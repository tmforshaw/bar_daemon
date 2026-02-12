use std::str::SplitWhitespace;

use tracing::instrument;

use crate::{
    command,
    error::DaemonError,
    observed::Observed::{self, Unavailable, Valid},
    snapshot::{current_snapshot, update_snapshot},
};

use super::Ram;

pub trait RamSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Observed<Ram>, DaemonError>> + Send;
    fn read_total(&self) -> impl std::future::Future<Output = Result<Observed<u64>, DaemonError>> + Send;
    fn read_used(&self) -> impl std::future::Future<Output = Result<Observed<u64>, DaemonError>> + Send;
    fn read_percent(&self) -> impl std::future::Future<Output = Result<Observed<u32>, DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl RamSource {
    ProcpsRam
}

pub async fn latest() -> Result<Observed<Ram>, DaemonError> {
    default_source().read().await
}

// ---------------- Procps Source --------------

#[derive(Debug)]
pub struct ProcpsRam;

impl RamSource for ProcpsRam {
    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read(&self) -> Result<Observed<Ram>, DaemonError> {
        fn read_inner() -> Result<Ram, DaemonError> {
            let output = get_procps_output()?;
            let output_split = get_procps_output_split(&output)?;

            let total = get_procps_total_from_split(output_split.clone())?;
            let used = get_procps_used_from_split(output_split)?;

            let percent = get_percent_from_used_total(used, total);

            Ok(Ram { total, used, percent })
        }

        // Set as unavailable if the inner function threw an error
        let ram: Observed<_> = read_inner().into();

        // Update snapshot
        let _update = update_snapshot(ram.clone()).await;

        Ok(ram)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read_total(&self) -> Result<Observed<u64>, DaemonError> {
        fn read_total_inner() -> Result<u64, DaemonError> {
            let output = get_procps_output()?;
            let output_split = get_procps_output_split(&output)?;

            get_procps_total_from_split(output_split)
        }

        // If there was an error, keep as unavailable, if not then map to monitored value
        let ram = match read_total_inner().into() {
            Valid(total) => {
                let ram = current_snapshot().await.ram.unwrap_or_default();

                Valid(Ram { total, ..ram })
            }
            Unavailable => Unavailable,
        };

        // Update snapshot
        let _update = update_snapshot(ram.clone()).await;

        Ok(ram.map(|ram| ram.total))
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read_used(&self) -> Result<Observed<u64>, DaemonError> {
        fn read_used_inner() -> Result<u64, DaemonError> {
            let output = get_procps_output()?;
            let output_split = get_procps_output_split(&output)?;

            get_procps_used_from_split(output_split)
        }

        // If there was an error, keep as unavailable, if not then map to monitored value
        let ram = match read_used_inner().into() {
            Valid(used) => {
                let ram = current_snapshot().await.ram.unwrap_or_default();

                Valid(Ram { used, ..ram })
            }
            Unavailable => Unavailable,
        };

        // Update snapshot
        let _update = update_snapshot(ram.clone()).await;

        Ok(ram.map(|ram| ram.used))
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read_percent(&self) -> Result<Observed<u32>, DaemonError> {
        fn read_percent_inner() -> Result<u32, DaemonError> {
            let output = get_procps_output()?;
            let output_split = get_procps_output_split(&output)?;

            let total = get_procps_total_from_split(output_split.clone())?;
            let used = get_procps_used_from_split(output_split)?;

            Ok(get_percent_from_used_total(used, total))
        }

        // If there was an error, keep as unavailable, if not then map to monitored value
        let ram = match read_percent_inner().into() {
            Valid(percent) => {
                let ram = current_snapshot().await.ram.unwrap_or_default();

                Valid(Ram { percent, ..ram })
            }
            Unavailable => Unavailable,
        };

        // Update snapshot
        let _update = update_snapshot(ram.clone()).await;

        Ok(ram.map(|ram| ram.percent))
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

#[instrument(skip(split))]
fn get_procps_total_from_split(mut split: SplitWhitespace) -> Result<u64, DaemonError> {
    // Get the total bytes from the spllit, parsing into u64
    split
        .next()
        .ok_or_else(|| DaemonError::ParseError(split.collect()))?
        .trim()
        .parse::<u64>()
        .map_err(Into::into)
}

#[instrument(skip(split))]
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
