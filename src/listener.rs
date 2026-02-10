use std::{collections::HashMap, path::Path, sync::Arc};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    sync::{Mutex, Notify, mpsc},
};
use tracing::{error, info, instrument};
use uuid::Uuid;

use crate::{
    config::get_config,
    daemon::{DaemonMessage, SOCKET_PATH},
    error::DaemonError,
    json::tuples_to_json,
    tuples::{TUPLE_NAMES, TupleName, get_all_tuples, tuple_name_to_tuples},
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
    if !Path::new(SOCKET_PATH).exists() {
        error!("Socket not found. Is the daemon running?");
        // TODO This should return and Err
        return Ok(());
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
#[instrument(skip(clients, clients_rx, notify))]
pub async fn handle_clients(
    clients: SharedClients,
    clients_rx: &mut mpsc::UnboundedReceiver<ClientMessage>,
    notify: Arc<Notify>,
) -> Result<(), DaemonError> {
    let mut tuples = Mutex::new(get_all_tuples().await?);

    loop {
        tokio::select! {
            client_message_result = clients_rx.recv() => {
                // Only show messages when the update has been asked for
                let Some(client_message) = client_message_result else {
                    continue;
                };

                let clients_empty = clients.lock().await.is_empty();

                if !clients_empty {
                    if matches!(client_message, ClientMessage::UpdateAll) {
                        // Get the tuples for all values
                        tuples = Mutex::new(get_all_tuples().await?);
                    } else {
                        // Get the TupleName for this message
                        let tuple_name = match client_message {
                            ClientMessage::UpdateVolume => TupleName::Volume,
                            ClientMessage::UpdateBrightness => TupleName::Brightness,
                            ClientMessage::UpdateBluetooth => TupleName::Bluetooth,
                            ClientMessage::UpdateBattery => TupleName::Battery,
                            ClientMessage::UpdateRam => TupleName::Ram,
                            ClientMessage::UpdateFanProfile => TupleName::FanProfile,
                            ClientMessage::UpdateAll => unreachable!(),
                        };

                        // Update the inner of the Mutex
                        let mut tuples = tuples.lock().await;
                        (*tuples)[tuple_name as usize] = (
                            TUPLE_NAMES[tuple_name as usize].to_string(),
                            tuple_name_to_tuples(&tuple_name).await?,
                        );
                    }

                    let mut to_remove = vec![];

                    let json = tuples_to_json(tuples.lock().await.clone())? + "\n";

                    // Broadcast to each client
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
            () = notify.notified() => {
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
