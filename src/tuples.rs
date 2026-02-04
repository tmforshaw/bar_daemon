use crate::{
    battery::{self},
    bluetooth::Bluetooth,
    brightness::Brightness,
    error::DaemonError,
    fan_profile::FanProfile,
    ram::Ram,
    snapshot::current_state,
};

pub const TUPLE_NAMES: &[&str] = &["volume", "brightness", "bluetooth", "battery", "ram", "fan_profile"];

#[derive(Copy, Clone)]
pub enum TupleName {
    Volume = 0,
    Brightness = 1,
    Bluetooth = 2,
    Battery = 3,
    Ram = 4,
    FanProfile = 5,
}

impl TryFrom<usize> for TupleName {
    type Error = DaemonError;

    /// # Errors
    /// Fails if a number too high is provided
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Volume),
            1 => Ok(Self::Brightness),
            2 => Ok(Self::Bluetooth),
            3 => Ok(Self::Battery),
            4 => Ok(Self::Ram),
            5 => Ok(Self::FanProfile),
            _ => Err(Self::Error::TupleNameError),
        }
    }
}

/// # Errors
/// Returns an error if the specified tuples can't be gotten
pub fn tuple_name_to_tuples(tuple_name: &TupleName) -> Result<Vec<(String, String)>, DaemonError> {
    // TODO use latest() for polled values and current_state() for values which don't change without user intervention
    match tuple_name {
        TupleName::Volume => Ok(current_state()?.volume.to_tuples()),
        TupleName::Brightness => Brightness::get_tuples(),
        TupleName::Bluetooth => Bluetooth::get_tuples(),
        TupleName::Battery => Ok(battery::latest()?.to_tuples()),
        TupleName::Ram => Ram::get_tuples(),
        TupleName::FanProfile => FanProfile::get_tuples(),
    }
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub async fn get_all_tuples() -> Result<Vec<(String, Vec<(String, String)>)>, DaemonError> {
    TUPLE_NAMES
        .iter()
        .enumerate()
        .map(|(i, &name)| {
            TupleName::try_from(i).map(|tuple_name| {
                // Convert the name to the respective tuples
                tuple_name_to_tuples(&tuple_name).map(|tuples| {
                    // Pair the name with the respective tuples
                    (name.to_string(), tuples)
                })
            })
        })
        .collect::<Result<Result<Vec<_>, DaemonError>, DaemonError>>()?
}
