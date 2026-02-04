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

static CURRENT_STATE: LazyLock<RwLock<Snapshot>> = LazyLock::new(|| RwLock::new(Snapshot::default()));

/// # Errors
/// Returns an error if the current state cannot be read due to `RwLock` Poisoning
pub fn current_state() -> Result<Snapshot, DaemonError> {
    CURRENT_STATE
        .read()
        .map_or(Err(DaemonError::RwLockError), |snap| Ok(snap.clone()))
}

/// # Errors
/// Returns an error if the current state can't be written to due to `RwLock` Poisoning
pub fn set_state_volume(volume: Volume) -> Result<(), DaemonError> {
    CURRENT_STATE.write().map_err(|_| DaemonError::RwLockError)?.volume = volume;

    Ok(())
}
