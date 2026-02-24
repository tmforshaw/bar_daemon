use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;
use tracing::warn;
use zbus::MatchRule;
use zbus::{Connection, MessageStream};

/// Spawn a background task that sends () whenever `UPower` says something changed.
pub fn spawn_upower_listener(tx: Sender<()>) {
    tokio::spawn(async move {
        if let Err(e) = run_dbus_listener(tx, "/org/freedesktop/UPower/devices").await {
            tracing::error!("UPower listener failed: {e}");
        }
    });
}

async fn run_dbus_listener<S: AsRef<str>>(tx: Sender<()>, listen_folder: S) -> zbus::Result<()> {
    // Listen on the dbus
    let conn = Connection::system().await?;

    // Match *any* property change on battery devices
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace(listen_folder.as_ref())?
        .build();

    let mut stream = MessageStream::for_match_rule(rule, &conn, None).await?;

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(_) => {
                // Send an event that something has changed
                if tx.send(()).await.is_err() {
                    break; // No more receivers, exit
                }
            }
            Err(e) => warn!("D-Bus receive error: {e}"),
        }
    }

    Ok(())
}
