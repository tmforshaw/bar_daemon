use std::{collections::HashMap, path::Path, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
    sync::{Mutex, Notify, mpsc},
};
use uuid::Uuid;

use crate::{
    battery::{self, BatteryItem},
    bluetooth::{self, BluetoothItem},
    brightness::{self, BrightnessItem, KEYBOARD_ID, MONITOR_ID},
    error::DaemonError,
    fan_profile::{self, FanProfileItem},
    listener::{Client, ClientMessage, SharedClients, handle_clients, poll_values},
    ram::{self, RamItem},
    shutdown::shutdown_signal,
    tuples::get_all_tuples,
    volume::{self, VolumeItem},
};

pub const SOCKET_PATH: &str = "/tmp/bar_daemon.sock";
pub const BUFFER_SIZE: usize = 1024;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonMessage {
    Set { item: DaemonItem, value: String },
    Get { item: DaemonItem },
    Update { item: DaemonItem },
    Listen,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonReply {
    Value {
        item: DaemonItem,
        value: String,
    },
    Tuples {
        item: DaemonItem,
        tuples: Vec<(String, String)>,
    },
    AllTuples {
        tuples: Vec<(String, Vec<(String, String)>)>,
    },
    Error(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum DaemonItem {
    Volume(VolumeItem),
    Brightness(BrightnessItem),
    Bluetooth(BluetoothItem),
    Battery(BatteryItem),
    Ram(RamItem),
    FanProfile(FanProfileItem),
    All,
}

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if ``UnixListener`` cannot be bound
/// Returns an error if socket cannot be accepted
pub async fn do_daemon() -> Result<(), DaemonError> {
    // Remove existing socket file
    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    // Create a future which waits for shutdown request
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);

    // Create Notify for broadcasting shutdown to all tasks
    let notify = Arc::new(Notify::new());

    // Create new UnixListener at SOCKET_PATH
    let listener = UnixListener::bind(SOCKET_PATH)?;

    // Enable back and forth communication from each socket handler and the client handler
    let (clients_tx, mut clients_rx) = mpsc::unbounded_channel::<ClientMessage>();

    // Remember listener clients to broadcast to
    let clients: SharedClients = Arc::new(Mutex::new(HashMap::new()));

    // Spawn a task which handles listener clients
    let clients_clone = clients.clone();
    let notify_clone = notify.clone();
    tokio::spawn(async move { handle_clients(clients_clone, &mut clients_rx, notify_clone).await });

    // Create a task which polls the state of certain values
    let clients_clone = clients.clone();
    let clients_tx_clone = clients_tx.clone();

    let notify_clone = notify.clone();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                () = poll_values(clients_clone.clone(), clients_tx_clone.clone()) => {}
                () = notify_clone.notified() => {
                    println!("Shutdown notified, cleaning up poll loop");
                }
            }
        }
    });

    // Handle sockets
    loop {
        tokio::select! {
            () = &mut shutdown => {
                println!("Shutdown signal received, stopping connection accept loop");

                notify.notify_waiters();

                break;
            },
            accept_result = listener.accept() => {
                let (stream, _) = accept_result?;

                // Spawn a task which handles this socket
                let clients_clone = clients.clone();
                let clients_tx_clone = clients_tx.clone();
                let notify_clone = notify.clone();
                tokio::spawn(async move { handle_socket(stream, clients_clone, clients_tx_clone, notify_clone).await });
            }
        }
    }

    // Remove socket file after shutdown
    if Path::new(SOCKET_PATH).exists() {
        std::fs::remove_file(SOCKET_PATH)?;
    }

    println!("Daemon shutdown cleanly");

    Ok(())
}

/// # Errors
/// Returns an error if socket cannot be read
/// Returns an error if ``DaemonMessage`` could not be created from bytes
/// Returns an error if requested value cannot be found or parsed
/// Returns an error if socket could not be wrote to
pub async fn handle_socket(
    mut stream: UnixStream,
    clients: SharedClients,
    clients_tx: mpsc::UnboundedSender<ClientMessage>,
    notify: Arc<Notify>,
) -> Result<(), DaemonError> {
    let mut buf = [0; BUFFER_SIZE];
    loop {
        tokio::select! {
            read_result = stream.read(&mut buf) => {
                let n = match read_result? {
                    // Stream closed
                    0 => break,
                    n => n,
                };

                let message: DaemonMessage = postcard::from_bytes(&buf[..n])?;

                let reply = match message {
                    DaemonMessage::Set { item, value }=> {
                        // Broadcast which value has been updated
                        clients_tx.send(match item {
                            DaemonItem::Volume(_) => ClientMessage::UpdateVolume,
                            DaemonItem::Brightness(_) => ClientMessage::UpdateBrightness,
                            DaemonItem::Bluetooth(_) => ClientMessage::UpdateBluetooth,
                            DaemonItem::Battery(_) => ClientMessage::UpdateBattery,
                            DaemonItem::Ram(_) => ClientMessage::UpdateRam,
                            DaemonItem::FanProfile(_) => ClientMessage::UpdateFanProfile,
                            DaemonItem::All => ClientMessage::UpdateAll,
                        })?;

                        match_set_command(item.clone(), value.clone()).await?
                    }
                    DaemonMessage::Get { item } => match_get_command(item.clone()).await?,
                    DaemonMessage::Update {item} => {
                        // Broadcast which value has been updated
                        clients_tx.send(match item {
                            DaemonItem::Volume(_) => {
                                volume::notify().await?;

                                ClientMessage::UpdateVolume
                            },
                            DaemonItem::Brightness(_) => {
                                // TODO
                                brightness::notify(MONITOR_ID).await?;
                                brightness::notify(KEYBOARD_ID).await?;

                                ClientMessage::UpdateBrightness
                            },
                            DaemonItem::Bluetooth(_) => {
                                bluetooth::notify().await?;

                                ClientMessage::UpdateBluetooth
                            },
                            DaemonItem::Battery(_) => {
                                battery::notify(u32::MAX).await?;

                                ClientMessage::UpdateBattery
                            },
                            DaemonItem::Ram(_) => ClientMessage::UpdateRam,
                            DaemonItem::FanProfile(_) => {
                                fan_profile::notify().await?;

                                ClientMessage::UpdateFanProfile
                            },
                            DaemonItem::All => ClientMessage::UpdateAll,
                        })?;

                        match_get_command(item.clone()).await?
                    }
                    DaemonMessage::Listen => {
                        // Add the client writer and their uuid to clients
                        let client_id = Uuid::new_v4();
                        clients.lock().await.insert(client_id, Client { id: client_id, stream });

                        return Ok(());
                    }
                };

                // Send the reply back
                stream.write_all(&postcard::to_stdvec(&reply)?).await?;
            },
            () = notify.notified() => {
                println!("Socket handler received shutdown notification");
                break;
            }
        }
    }

    println!("Socket handler shutdown successfuly");

    Ok(())
}

/// # Errors
/// Returns an error if ``SOCKET_PATH`` cannot be found
/// Returns an error if socket cannot be read
/// Returns an error if socket could not be wrote to
pub async fn send_daemon_messaage(message: DaemonMessage) -> Result<DaemonReply, DaemonError> {
    // Connect to the daemon
    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    // Write the serialized message to the daemon
    stream.write_all(&postcard::to_stdvec(&message)?).await?;

    // Get the response from the daemon
    let mut buf = vec![0u8; BUFFER_SIZE];
    let n = stream.read(&mut buf).await?;

    // Serialize reply into DaemonMessage
    Ok(postcard::from_bytes(&buf[..n])?)
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub async fn match_set_command(item: DaemonItem, value: String) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => volume::evaluate_item(item, &volume_item, Some(value)).await?,
        DaemonItem::Brightness(brightness_item) => brightness::evaluate_item(item, &brightness_item, Some(value)).await?,
        DaemonItem::Bluetooth(bluetooth_item) => bluetooth::evaluate_item(item, &bluetooth_item, Some(value)).await?,
        DaemonItem::FanProfile(fan_profile_item) => fan_profile::evaluate_item(item, &fan_profile_item, Some(value)).await?,
        _ => DaemonReply::Value { item, value },
    };

    Ok(message)
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub async fn match_get_command(item: DaemonItem) -> Result<DaemonReply, DaemonError> {
    let message = match item.clone() {
        DaemonItem::Volume(volume_item) => volume::evaluate_item(item.clone(), &volume_item, None).await?,
        DaemonItem::Brightness(brightness_item) => brightness::evaluate_item(item.clone(), &brightness_item, None).await?,
        DaemonItem::Bluetooth(bluetooth_item) => bluetooth::evaluate_item(item.clone(), &bluetooth_item, None).await?,
        DaemonItem::Battery(battery_item) => battery::evaluate_item(item.clone(), &battery_item).await?,
        DaemonItem::Ram(ram_item) => ram::evaluate_item(item.clone(), &ram_item).await?,
        DaemonItem::FanProfile(fan_profile_item) => fan_profile::evaluate_item(item.clone(), &fan_profile_item, None).await?,
        DaemonItem::All => DaemonReply::AllTuples {
            tuples: get_all_tuples().await?,
        },
    };

    Ok(message)
}
