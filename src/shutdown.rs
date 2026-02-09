use tokio::signal::unix::{SignalKind, signal};

pub enum ShutdownMessage {
    Shutdown,
}

/// # Panics
/// Panics if the signal can't be created from ``SignalKind``
#[allow(clippy::unwrap_used)]
pub async fn shutdown_signal() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    }

    println!("Shutdown signal received");
}
