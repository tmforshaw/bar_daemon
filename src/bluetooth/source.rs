use tracing::instrument;

use crate::{
    command,
    error::DaemonError,
    snapshot::{current_snapshot, update_snapshot},
};

use super::Bluetooth;

pub trait BluetoothSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Bluetooth, DaemonError>> + Send;
    fn read_state(&self) -> impl std::future::Future<Output = Result<bool, DaemonError>> + Send;

    // Change values of source
    fn set_state(&self, state_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl BluetoothSource {
    BluezBluetooth
}

pub async fn latest() -> Result<Bluetooth, DaemonError> {
    default_source().read().await
}

// ---------------- Bluez Source ---------------

#[derive(Debug)]
pub struct BluezBluetooth;

impl BluetoothSource for BluezBluetooth {
    #[instrument]
    async fn read(&self) -> Result<Bluetooth, DaemonError> {
        // Get output for bluetooth command (From Bluez)
        let output = command::run("bluetooth", &[])?;

        // Split the output and check if it is on or off
        let bluetooth = output
            .clone()
            .split_whitespace()
            .nth(2)
            .map_or(Err(DaemonError::ParseError(output)), |state| {
                Ok(Bluetooth { state: state == "on" })
            })?;

        // Update current snapshot
        let _update = update_snapshot(bluetooth.clone()).await;

        Ok(bluetooth)
    }

    #[instrument]
    async fn read_state(&self) -> Result<bool, DaemonError> {
        self.read().await.map(|bluetooth| bluetooth.state)
    }

    #[instrument]
    async fn set_state(&self, state_str: &str) -> Result<(), DaemonError> {
        let new_state;

        // Allow toggling of the bluetooth state
        let state = match state_str {
            "toggle" => {
                new_state = !current_snapshot().await.bluetooth.unwrap_or(latest().await?).state;

                "toggle"
            }
            _ => {
                if state_str.parse::<bool>()? {
                    new_state = true;

                    "on"
                } else {
                    new_state = false;

                    "off"
                }
            }
        };

        command::run("bluetooth", &[state])?;

        // Change the value within the snapshot
        let _update = update_snapshot(Bluetooth { state: new_state }).await;

        Ok(())
    }
}
