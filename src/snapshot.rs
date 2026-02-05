use std::{
    sync::{LazyLock, RwLock},
    time::Instant,
};

use crate::{
    battery::Battery, bluetooth::Bluetooth, brightness::Brightness, error::DaemonError, fan_profile::FanProfile, ram::Ram,
    volume::Volume,
};

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub battery: Battery,
    pub bluetooth: Bluetooth,
    pub brightness: Brightness,
    pub fan_profile: FanProfile,
    pub ram: Ram,
    pub volume: Volume,
    pub timestamp: Instant,
}

// TODO
#[allow(clippy::default_constructed_unit_structs)]
impl Default for Snapshot {
    fn default() -> Self {
        Self {
            battery: Battery::default(),
            bluetooth: Bluetooth::default(),
            brightness: Brightness::default(),
            fan_profile: FanProfile::default(),
            ram: Ram::default(),
            volume: Volume::default(),
            timestamp: Instant::now(),
        }
    }
}

static CURRENT_SNAPSHOT: LazyLock<RwLock<Snapshot>> = LazyLock::new(|| RwLock::new(Snapshot::default()));

/// # Errors
/// Returns an error if the current snapshot cannot be read due to `RwLock` Poisoning
pub fn current_snapshot() -> Result<Snapshot, DaemonError> {
    CURRENT_SNAPSHOT
        .read()
        .map_or(Err(DaemonError::RwLockError), |snap| Ok(snap.clone()))
}

/// # Errors
/// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
pub fn set_snapshot_battery(battery: Battery) -> Result<(), DaemonError> {
    CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.battery = battery;

    Ok(())
}

/// # Errors
/// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
pub fn set_snapshot_bluetooth(bluetooth: Bluetooth) -> Result<(), DaemonError> {
    CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.bluetooth = bluetooth;

    Ok(())
}

/// # Errors
/// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
pub fn set_snapshot_brightness(brightness: Brightness) -> Result<(), DaemonError> {
    CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.brightness = brightness;

    Ok(())
}

/// # Errors
/// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
pub fn set_snapshot_fan_profile(fan_profile: FanProfile) -> Result<(), DaemonError> {
    CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.fan_profile = fan_profile;

    Ok(())
}

/// # Errors
/// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
pub fn set_snapshot_ram(ram: Ram) -> Result<(), DaemonError> {
    CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.ram = ram;

    Ok(())
}

/// # Errors
/// Returns an error if the current snapshot can't be written to due to `RwLock` Poisoning
pub fn set_snapshot_volume(volume: Volume) -> Result<(), DaemonError> {
    CURRENT_SNAPSHOT.write().map_err(|_| DaemonError::RwLockError)?.volume = volume;

    Ok(())
}
