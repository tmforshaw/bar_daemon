use std::str::Split;

use tracing::{instrument, warn};

use super::value::{Battery, BatteryState};
use crate::{
    command,
    error::DaemonError,
    observed::Observed::{self},
    snapshot::update_snapshot,
};

pub trait BatterySource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Observed<Battery>, DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl BatterySource {
    AcpiBattery
}

// ---------------- ACPI Source ----------------

#[derive(Debug)]
pub struct AcpiBattery;

impl BatterySource for AcpiBattery {
    #[instrument]
    async fn read(&self) -> Result<Observed<Battery>, DaemonError> {
        fn read_inner() -> Result<Battery, DaemonError> {
            // Get ACPI output and split it into sections
            let output = get_acpi_output()?;
            let output_split = get_acpi_split(&output);

            // Parse the state, percentage, and time remaining
            Ok(Battery {
                state: get_state_from_acpi_split(output_split.clone())?,
                percent: get_percent_from_acpi_split(output_split.clone())?,
                time: get_time_from_acpi_split(output_split)?,
            })
        }

        // Set as unavailable if the inner function threw an error
        let battery: Observed<_> = read_inner().into();

        // Update current snapshot
        let _update = update_snapshot(battery.clone()).await;

        Ok(battery)
    }
}

fn get_acpi_output() -> Result<String, DaemonError> {
    // Get the output of the 'acpi -b' command
    command::run("acpi", &["-b"])
}

fn get_acpi_split(output: &str) -> Split<'_, char> {
    // Split the output based on commas
    output.split(',')
}

#[instrument(skip(output_split))]
fn get_state_from_acpi_split(mut output_split: Split<char>) -> Result<BatteryState, DaemonError> {
    // Get the state from the split and convert it to a BatteryState enum
    match output_split
        .next()
        .ok_or_else(|| DaemonError::ParseError(output_split.collect::<String>()))?
        .trim_start_matches("Battery 0: ")
    {
        "Fully charged" => Ok(BatteryState::FullyCharged),
        "Charging" => Ok(BatteryState::Charging),
        "Discharging" => Ok(BatteryState::Discharging),
        "Not charging" => Ok(BatteryState::NotCharging),
        state_string => Err(DaemonError::ParseError(state_string.to_string())),
    }
}

#[instrument(skip(output_split))]
fn get_percent_from_acpi_split(mut output_split: std::str::Split<char>) -> Result<u32, DaemonError> {
    // Parse the percentage from split and convert to u32
    Ok(output_split
        .nth(1)
        .ok_or_else(|| DaemonError::ParseError(output_split.collect::<String>()))?
        .trim()
        .trim_end_matches('%')
        .parse::<u32>()?)
}

#[instrument(skip(output_split))]
fn get_time_from_acpi_split(mut output_split: std::str::Split<char>) -> Result<String, DaemonError> {
    // Return empty string if the time part of the output_split is not present
    let Some(time_string_unsplit) = output_split.nth(2) else {
        return Ok(String::new());
    };

    // Get the time portion of the split
    Ok(time_string_unsplit
        .split_whitespace()
        .next()
        .ok_or_else(|| DaemonError::ParseError(output_split.collect::<String>()))?
        .to_string())
}
