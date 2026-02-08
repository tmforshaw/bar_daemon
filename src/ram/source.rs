use std::str::SplitWhitespace;

use crate::{
    command,
    error::DaemonError,
    snapshot::{current_snapshot, update_snapshot},
};

use super::Ram;

pub trait RamSource {
    // Read from commands (Get latest values)
    async fn read(&self) -> Result<Ram, DaemonError>;
    async fn read_total(&self) -> Result<u64, DaemonError>;
    async fn read_used(&self) -> Result<u64, DaemonError>;
    async fn read_percent(&self) -> Result<u32, DaemonError>;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl RamSource {
    ProcpsRam
}

pub async fn latest() -> Result<Ram, DaemonError> {
    default_source().read().await
}

// ---------------- Procps Source --------------

pub struct ProcpsRam;

impl RamSource for ProcpsRam {
    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    async fn read(&self) -> Result<Ram, DaemonError> {
        let output = get_procps_output()?;
        let output_split = get_procps_output_split(&output)?;

        let total = get_procps_total_from_split(output_split.clone())?;
        let used = get_procps_used_from_split(output_split)?;

        let percent = get_percent_from_used_total(used, total);

        // Update snapshot
        let ram = Ram { total, used, percent };
        update_snapshot(ram.clone()).await?;

        Ok(ram)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    async fn read_total(&self) -> Result<u64, DaemonError> {
        let output = get_procps_output()?;
        let output_split = get_procps_output_split(&output)?;

        let total = get_procps_total_from_split(output_split)?;

        // Update snapshot
        let ram = current_snapshot().await.ram.unwrap_or_default();
        update_snapshot(Ram { total, ..ram }).await?;

        Ok(total)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    async fn read_used(&self) -> Result<u64, DaemonError> {
        let output = get_procps_output()?;
        let output_split = get_procps_output_split(&output)?;

        let used = get_procps_used_from_split(output_split)?;

        // Update snapshot
        let ram = current_snapshot().await.ram.unwrap_or_default();
        update_snapshot(Ram { used, ..ram }).await?;

        Ok(used)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    async fn read_percent(&self) -> Result<u32, DaemonError> {
        let output = get_procps_output()?;
        let output_split = get_procps_output_split(&output)?;

        let total = get_procps_total_from_split(output_split.clone())?;
        let used = get_procps_used_from_split(output_split)?;

        let percent = get_percent_from_used_total(used, total);

        // Update snapshot
        let ram = current_snapshot().await.ram.unwrap_or_default();
        update_snapshot(Ram { percent, ..ram }).await?;

        Ok(percent)
    }
}

fn get_procps_output() -> Result<String, DaemonError> {
    // Get the output of free so it can be parsed
    command::run("free", &["-b"])
}

fn get_procps_output_split(output: &str) -> Result<SplitWhitespace<'_>, DaemonError> {
    // Parse the output into lines
    let output_lines = output.lines();

    // Choose the second line, and split based on whitespace
    Ok(output_lines
        .clone()
        .nth(1)
        .ok_or_else(|| DaemonError::ParseError(output_lines.collect::<String>()))?
        .trim_start_matches("Mem:")
        .split_whitespace())
}

fn get_procps_total_from_split(mut split: SplitWhitespace) -> Result<u64, DaemonError> {
    // Get the total bytes from the spllit, parsing into u64
    split
        .next()
        .ok_or_else(|| DaemonError::ParseError(split.collect()))?
        .trim()
        .parse::<u64>()
        .map_err(Into::into)
}

fn get_procps_used_from_split(mut split: SplitWhitespace) -> Result<u64, DaemonError> {
    // Get the used bytes from the spllit, parsing into u64
    split
        .nth(1)
        .ok_or_else(|| DaemonError::ParseError(split.collect()))?
        .trim()
        .parse::<u64>()
        .map_err(Into::into)
}

// ------------- Helper Functions --------------

fn get_percent_from_used_total(used: u64, total: u64) -> u32 {
    ((used as f64 * 100.) / total as f64) as u32
}
