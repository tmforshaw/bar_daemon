use tokio::signal::unix::{SignalKind, signal};
use tracing::{info, instrument};

pub enum ShutdownMessage {
    Shutdown,
}

/// # Panics
/// Panics if the signal can't be created from ``SignalKind``
#[allow(clippy::unwrap_used)]
#[instrument]
pub async fn shutdown_signal() {
    let mut sigint = signal(SignalKind::interrupt()).unwrap();
    let mut sigterm = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    }

    info!("Shutdown signal received");
}
