use tracing::instrument;

use crate::{
    battery::{self},
    error::DaemonError,
    fan_profile,
    ram::{self},
    snapshot::current_snapshot,
};

pub const TUPLE_NAMES: &[&str] = &["volume", "brightness", "bluetooth", "battery", "ram", "fan_profile"];

#[derive(Debug, Copy, Clone)]
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
#[instrument]
pub async fn tuple_name_to_tuples(tuple_name: &TupleName) -> Result<Vec<(String, String)>, DaemonError> {
    // use latest() for polled values and current_snapshot() for values which don't change without user intervention
    Ok(match tuple_name {
        TupleName::Volume => current_snapshot().await.volume.unwrap_or_default().to_tuples(),
        TupleName::Brightness => current_snapshot().await.brightness.unwrap_or_default().to_tuples(),
        TupleName::Bluetooth => current_snapshot().await.bluetooth.unwrap_or_default().to_tuples(),
        TupleName::Battery => battery::latest().await?.to_tuples(),
        TupleName::Ram => ram::latest().await?.to_tuples(),
        TupleName::FanProfile => fan_profile::latest().await?.to_tuples(), // Special case since the OS changes fan mode when plugging/unplugging AC
    })
}

type TupleNameWithTuples = (String, Vec<(String, String)>);

/// # Errors
/// Returns an error if the requested value could not be parsed
#[instrument]
pub async fn get_all_tuples() -> Result<Vec<TupleNameWithTuples>, DaemonError> {
    // Validate and convert indices
    let tuple_names = TUPLE_NAMES
        .iter()
        .enumerate()
        .map(|(i, _)| TupleName::try_from(i))
        .collect::<Result<Vec<_>, DaemonError>>()?;

    // Convert names to their respective tuples (create async future for this)
    let futures = tuple_names.iter().zip(TUPLE_NAMES.iter()).map(|(tuple_name, &name)| {
        let name = name.to_string();

        async move {
            let tuples = tuple_name_to_tuples(tuple_name).await?;

            Ok::<_, DaemonError>((name, tuples))
        }
    });

    // Execute the futures concurrently to get the tuples
    futures::future::try_join_all(futures).await
}
