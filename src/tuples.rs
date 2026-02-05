use crate::{
    battery::{self},
    error::DaemonError,
    fan_profile,
    ram::{self},
    snapshot::current_snapshot,
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
    // use latest() for polled values and current_snapshot() for values which don't change without user intervention
    Ok(match tuple_name {
        TupleName::Volume => current_snapshot()?.volume.to_tuples(),
        TupleName::Brightness => current_snapshot()?.brightness.to_tuples(),
        TupleName::Bluetooth => current_snapshot()?.bluetooth.to_tuples(),
        TupleName::Battery => battery::latest()?.to_tuples(),
        TupleName::Ram => ram::latest()?.to_tuples(),
        TupleName::FanProfile => fan_profile::latest()?.to_tuples(), // Special case since the OS changes fan mode when plugging/unplugging AC
    })
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
