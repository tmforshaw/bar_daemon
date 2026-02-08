use std::str::Split;

use super::value::{Battery, BatteryState};
use crate::{
    command,
    error::DaemonError,
    snapshot::{current_snapshot, update_snapshot},
};

pub trait BatterySource {
    // Read from commands (Get latest values)
    async fn read(&self) -> Result<Battery, DaemonError>;
    async fn read_state(&self) -> Result<BatteryState, DaemonError>;
    async fn read_percent(&self) -> Result<u32, DaemonError>;
    async fn read_time(&self) -> Result<String, DaemonError>;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl BatterySource {
    AcpiBattery
}

pub async fn latest() -> Result<Battery, DaemonError> {
    default_source().read().await
}

// ---------------- ACPI Source ----------------

pub struct AcpiBattery;

// TODO update snapshot when getting
impl BatterySource for AcpiBattery {
    async fn read(&self) -> Result<Battery, DaemonError> {
        // Get ACPI output and split it into sections
        let output = get_acpi_output()?;
        let output_split = get_acpi_split(&output);

        // Parse the state, percentage, and time remaining
        let battery = Battery {
            state: get_state_from_acpi_split(output_split.clone())?,
            percent: get_percent_from_acpi_split(output_split.clone())?,
            time: get_time_from_acpi_split(output_split)?,
        };

        // Update current snapshot
        update_snapshot(battery.clone()).await?;

        Ok(battery)
    }

    async fn read_state(&self) -> Result<BatteryState, DaemonError> {
        // Get ACPI output and split it into sections
        let output = get_acpi_output()?;
        let output_split = get_acpi_split(&output);

        let state = get_state_from_acpi_split(output_split)?;

        // Update current snapshot TODO Replace with soft error
        let battery = current_snapshot().await.battery.unwrap_or_default();
        update_snapshot(Battery { state, ..battery }).await?;

        Ok(state)
    }

    async fn read_percent(&self) -> Result<u32, DaemonError> {
        // Get ACPI output and split it into sections
        let output = get_acpi_output()?;
        let output_split = get_acpi_split(&output);

        let percent = get_percent_from_acpi_split(output_split)?;

        // Update current snapshot TODO Replace with soft error
        let battery = current_snapshot().await.battery.unwrap_or_default();
        update_snapshot(Battery { percent, ..battery }).await?;

        Ok(percent)
    }

    async fn read_time(&self) -> Result<String, DaemonError> {
        // Get ACPI output and split it into sections
        let output = get_acpi_output()?;
        let output_split = get_acpi_split(&output);

        let time = get_time_from_acpi_split(output_split)?;

        // Update current snapshot TODO Replace with soft error
        let battery = current_snapshot().await.battery.unwrap_or_default();
        update_snapshot(Battery {
            time: time.clone(),
            ..battery
        })
        .await?;

        Ok(time)
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

fn get_percent_from_acpi_split(mut output_split: std::str::Split<char>) -> Result<u32, DaemonError> {
    // Parse the percentage from split and convert to u32
    Ok(output_split
        .nth(1)
        .ok_or_else(|| DaemonError::ParseError(output_split.collect::<String>()))?
        .trim()
        .trim_end_matches('%')
        .parse::<u32>()?)
}

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
