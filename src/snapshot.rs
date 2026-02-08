use std::{sync::LazyLock, time::Instant};

use tokio::sync::RwLock;

use crate::{
    battery::Battery,
    bluetooth::Bluetooth,
    brightness::Brightness,
    error::DaemonError,
    fan_profile::FanProfile,
    monitored::{Monitored, MonitoredUpdate, update_monitored},
    ram::Ram,
    volume::Volume,
};

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub battery: Option<Battery>,
    pub bluetooth: Option<Bluetooth>,
    pub brightness: Option<Brightness>,
    pub fan_profile: Option<FanProfile>,
    pub ram: Option<Ram>,
    pub volume: Option<Volume>,
    pub timestamp: Option<Instant>,
}

static CURRENT_SNAPSHOT: LazyLock<RwLock<Snapshot>> = LazyLock::new(|| RwLock::new(Snapshot::default()));

/// # Errors
/// Returns an error if the current snapshot cannot be read due to `RwLock` Poisoning
pub async fn current_snapshot() -> Snapshot {
    CURRENT_SNAPSHOT.read().await.clone()
}

pub async fn update_snapshot<M: Monitored>(new_value: M) -> Result<MonitoredUpdate<M>, DaemonError> {
    let mut snapshot = CURRENT_SNAPSHOT.write().await; // exclusive access
    update_monitored(&mut snapshot, new_value) // centralized mutation
}

// /// # Errors
// /// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
// pub fn set_snapshot_battery(battery: Battery) -> Result<(), DaemonError> {
//     CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.battery = battery;

//     Ok(())
// }

// /// # Errors
// /// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
// pub fn set_snapshot_bluetooth(bluetooth: Bluetooth) -> Result<(), DaemonError> {
//     CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.bluetooth = bluetooth;

//     Ok(())
// }

// /// # Errors
// /// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
// pub fn set_snapshot_brightness(brightness: Brightness) -> Result<(), DaemonError> {
//     CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.brightness = brightness;

//     Ok(())
// }

// /// # Errors
// /// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
// pub fn set_snapshot_fan_profile(fan_profile: FanProfile) -> Result<(), DaemonError> {
//     CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.fan_profile = fan_profile;

//     Ok(())
// }

// /// # Errors
// /// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
// pub fn set_snapshot_ram(ram: Ram) -> Result<(), DaemonError> {
//     CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.ram = ram;

//     Ok(())
// }

// /// # Errors
// /// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
// pub fn set_snapshot_volume(volume: Volume) -> Result<(), DaemonError> {
//     CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.volume = volume;

//     Ok(())
// }
