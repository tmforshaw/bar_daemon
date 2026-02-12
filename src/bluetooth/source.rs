use tracing::instrument;

use crate::{
    bluetooth, command,
    error::DaemonError,
    observed::Observed::{self, Valid},
    snapshot::{current_snapshot, update_snapshot},
};

use super::Bluetooth;

pub trait BluetoothSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Observed<Bluetooth>, DaemonError>> + Send;
    fn read_state(&self) -> impl std::future::Future<Output = Result<Observed<bool>, DaemonError>> + Send;

    // Change values of source
    fn set_state(&self, state_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl BluetoothSource {
    BluezBluetooth
}

pub async fn latest() -> Result<Observed<Bluetooth>, DaemonError> {
    default_source().read().await
}

// ---------------- Bluez Source ---------------

#[derive(Debug)]
pub struct BluezBluetooth;

impl BluetoothSource for BluezBluetooth {
    #[instrument]
    async fn read(&self) -> Result<Observed<Bluetooth>, DaemonError> {
        fn read_inner() -> Result<Bluetooth, DaemonError> {
            // Get output for bluetooth command (From Bluez)
            let output = command::run("bluetooth", &[])?;

            // Split the output and check if it is on or off
            output
                .clone()
                .split_whitespace()
                .nth(2)
                .map_or(Err(DaemonError::ParseError(output)), |state| {
                    Ok(Bluetooth { state: state == "on" })
                })
        }

        // Set as unavailable if the inner function threw an error
        let bluetooth: Observed<_> = read_inner().into();

        // Update current snapshot
        let _update = update_snapshot(bluetooth.clone()).await;

        Ok(bluetooth)
    }

    #[instrument]
    async fn read_state(&self) -> Result<Observed<bool>, DaemonError> {
        // If there was an error, keep as unavailable, if not then map to bluetooth.state
        self.read().await.map(|bluetooth| bluetooth.map(|bluetooth| bluetooth.state))
    }

    #[instrument]
    async fn set_state(&self, state_str: &str) -> Result<(), DaemonError> {
        let new_state;

        // Allow toggling of the bluetooth state
        let state = match state_str {
            "toggle" => {
                new_state = !current_snapshot()
                    .await
                    .bluetooth
                    .unwrap_or(latest().await?.unwrap_or_default())
                    .state;

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
        let update = update_snapshot(Valid(Bluetooth { state: new_state })).await;

        // Do a notification
        bluetooth::notify(update).await?;

        Ok(())
    }
}
