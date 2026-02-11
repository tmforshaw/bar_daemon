use std::{collections::HashMap, path::Path, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::{Mutex, Notify, broadcast, mpsc},
};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    config::get_config,
    daemon::{DaemonMessage, SOCKET_PATH},
    error::DaemonError,
    json::tuples_to_json,
    snapshot::SnapshotEvent,
    tuples::{TUPLE_NAMES, TupleName, get_all_tuples},
};

#[derive(Debug)]
pub struct Client {
    pub id: Uuid,
    pub stream: UnixStream,
}

// TODO Replace this with the Snapshot MonitoredUpdate to ensure that updates are only sent when necessary
pub enum ClientMessage {
    UpdateVolume,
    UpdateBrightness,
    UpdateBluetooth,
    UpdateBattery,
    UpdateRam,
    UpdateFanProfile,
    UpdateAll,
}

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if ``UnixListener`` cannot be bound
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if socket cannot be read
/// Returns an error if socket could not be wrote to
#[instrument]
pub async fn listen() -> Result<(), DaemonError> {
    match listen_inner().await {
        Ok(()) => Ok(()),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}

async fn listen_inner() -> Result<(), DaemonError> {
    if !Path::new(SOCKET_PATH).exists() {
        error!("Socket not found ('{SOCKET_PATH}'). Is the daemon running?");
        return Err(DaemonError::PathRwError(SOCKET_PATH.to_string()));
    }

    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    // Tell the daemon that this client wants to listen
    stream.write_all(&postcard::to_stdvec(&DaemonMessage::Listen)?).await?;

    // Get the initial tuples, as JSON, and write to stdout
    let json = tuples_to_json(get_all_tuples().await?)?;
    println!("{json}");

    // Read the lines which the client sends
    let reader = BufReader::new(stream);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        println!("{line}");
    }

    Ok(())
}

pub type SharedClients = Arc<Mutex<HashMap<Uuid, Client>>>;

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if ``UnixListener`` cannot be bound
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if socket cannot be read
/// Returns an error if socket could not be wrote to
#[instrument(skip(clients, snapshot_rx, shutdown_notify))]
pub async fn handle_clients(
    clients: SharedClients,
    snapshot_rx: &mut broadcast::Receiver<SnapshotEvent>,
    shutdown_notify: Arc<Notify>,
) -> Result<(), DaemonError> {
    let tuples = Mutex::new(get_all_tuples().await?);

    loop {
        tokio::select! {
            Ok(event)= snapshot_rx.recv() => {
                let clients_empty = clients.lock().await.is_empty();

                if !clients_empty {
                    info!("SnapshotEvent Received: {event:?}");

                    let (index, new_tuples) = match event {
                        SnapshotEvent::Battery(update) => (TupleName::Battery as usize, update.new.to_tuples()),
                        SnapshotEvent::Bluetooth(update) => (TupleName::Bluetooth as usize, update.new.to_tuples()),
                        SnapshotEvent::Brightness(update) => (TupleName::Brightness as usize, update.new.to_tuples()),
                        SnapshotEvent::FanProfile(update) => (TupleName::FanProfile as usize, update.new.to_tuples()),
                        SnapshotEvent::Ram(update) => (TupleName::Ram as usize, update.new.to_tuples()),
                        SnapshotEvent::Volume(update) => (TupleName::Volume as usize, update.new.to_tuples()),
                    };

                    // Convert the updated tuples to JSON
                    let json = tuples_to_json({
                       // Update the inner of the tuples Mutex
                       let mut tuples_guard = tuples.lock().await;
                       (*tuples_guard)[index] = (
                            TUPLE_NAMES[index].to_string(),
                            new_tuples,
                        );

                       tuples_guard.clone()
                    })? + "\n";

                    // Broadcast to each client
                    let mut to_remove = vec![];
                    for (id, client) in clients.lock().await.iter_mut() {
                        if let Err(e) = client.stream.try_write(json.as_bytes()) {
                            error!("Write failed for {id}: {e}");
                            to_remove.push(*id);
                        }
                    }

                    // Remove dead clients
                    for id in to_remove {
                        clients.lock().await.remove(&id);
                        info!("Client {id} removed");
                    }
                }
            }
            () = shutdown_notify.notified() => {
                info!("Client handler received shutdown notification");
                break;
            }
        }
    }

    info!("Client handler shutdown successfuly");

    Ok(())
}

// TODO Add a Polled trait to simplify this and make it easier to poll values
#[instrument(skip(clients, clients_tx))]
pub async fn poll_values(clients: Arc<Mutex<HashMap<Uuid, Client>>>, clients_tx: mpsc::UnboundedSender<ClientMessage>) {
    let clients_empty = clients.lock().await.is_empty();

    // Only poll the values when there are listener clients
    if !clients_empty {
        clients_tx
            .send(ClientMessage::UpdateBattery)
            .unwrap_or_else(|e| error!("{}", Into::<DaemonError>::into(e)));

        clients_tx
            .send(ClientMessage::UpdateRam)
            .unwrap_or_else(|e| error!("{}", Into::<DaemonError>::into(e)));

        clients_tx
            .send(ClientMessage::UpdateFanProfile)
            .unwrap_or_else(|e| error!("{}", Into::<DaemonError>::into(e)));
    }

    // Set the polling rate
    tokio::time::sleep(tokio::time::Duration::from_millis(get_config().polling_rate)).await;
}
