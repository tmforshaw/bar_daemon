use crate::{
    command,
    error::DaemonError,
    snapshot::{current_snapshot, update_snapshot},
};

use super::Bluetooth;

pub trait BluetoothSource {
    // Read from commands (Get latest values)
    async fn read(&self) -> Result<Bluetooth, DaemonError>;
    async fn read_state(&self) -> Result<bool, DaemonError>;

    // Change values of source
    async fn set_state(&self, state_str: &str) -> Result<(), DaemonError>;
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

pub struct BluezBluetooth;

impl BluetoothSource for BluezBluetooth {
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
        update_snapshot(bluetooth.clone()).await?;

        Ok(bluetooth)
    }

    async fn read_state(&self) -> Result<bool, DaemonError> {
        self.read().await.map(|bluetooth| bluetooth.state)
    }

    async fn set_state(&self, state_str: &str) -> Result<(), DaemonError> {
        let new_state;

        // Allow toggling of the bluetooth state
        let state = match state_str {
            "toggle" => {
                new_state = !current_snapshot().await.bluetooth.unwrap_or_default().state;

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
        update_snapshot(Bluetooth { state: new_state }).await?;

        Ok(())
    }
}
