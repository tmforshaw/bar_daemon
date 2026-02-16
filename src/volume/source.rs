use std::str::SplitWhitespace;

use itertools::Itertools;
use tracing::instrument;

use super::Volume;
use crate::{
    command,
    error::DaemonError,
    log_linear::{linear_to_logarithmic, logarithmic_to_linear},
    monitored::Monitored,
    observed::Observed::{self, Unavailable, Valid},
    snapshot::{current_snapshot, update_snapshot},
    volume,
};

pub trait VolumeSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Observed<Volume>, DaemonError>> + std::marker::Send;

    // Change values of source
    fn set_percent(&self, percent_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + std::marker::Send;
    fn set_mute(&self, mute_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + std::marker::Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl VolumeSource {
    WpctlVolume
}

// ---------------- Wpctl Source ---------------

#[derive(Debug)]
pub struct WpctlVolume;

impl VolumeSource for WpctlVolume {
    // Read from commands

    #[instrument]
    async fn read(&self) -> Result<Observed<Volume>, DaemonError> {
        fn read_inner() -> Result<Volume, DaemonError> {
            let output = get_wpctl_output()?;
            let output_split = get_wpctl_split(&output);

            let percent = get_linear_percent_from_wpctl_split(output_split.clone())?;

            let mute = get_mute_from_wpctl_split(output_split);

            Ok(Volume { percent, mute })
        }

        // Set as unavailable if the inner function threw an error
        let volume: Observed<_> = read_inner().into();

        // Update current snapshot
        let _update = update_snapshot(volume.clone()).await;

        Ok(volume)
    }

    // Set source values

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn set_percent(&self, percent_str: &str) -> Result<(), DaemonError> {
        // Get the current snapshot values
        let volume_observed = match current_snapshot().await.volume {
            Valid(volume) => Valid(volume),
            Unavailable => Volume::latest().await?,
        };
        let volume = volume_observed.clone().unwrap_or_default();

        // If the percentage is a change, figure out the true percentage
        let linear_percent = if percent_str.starts_with('+') || percent_str.starts_with('-') {
            // Get the value of the percentage
            let delta_percent = i32::try_from(
                percent_str
                    .trim_start_matches('+')
                    .trim_start_matches('-')
                    .to_string()
                    .parse::<u32>()?,
            )?;

            // Calculate the new percentage based on the state's current value
            (i32::try_from(volume.percent)?
                + match percent_str.chars().next() {
                    Some('+') => delta_percent,
                    Some('-') => -delta_percent,
                    _ => 0,
                })
            .clamp(0, 100) as u32
        } else {
            percent_str.parse::<u32>()?
        };

        // Update the volume in the snapshot
        let update = update_snapshot(volume_observed.map(|volume| Volume {
            percent: linear_percent,
            ..volume
        }))
        .await;

        // Do a notification
        volume::notify(update).await?;

        // Set the volume internally as a logarithmic value
        let logarithmic_percent = linear_to_logarithmic(f64::from(linear_percent));

        // Set the volume
        let _ = command::run(
            "wpctl",
            &["set-volume", "@DEFAULT_SINK@", format!("{logarithmic_percent}%").as_str()],
        )?;

        Ok(())
    }

    #[instrument]
    async fn set_mute(&self, mute_str: &str) -> Result<(), DaemonError> {
        let volume_observed = match current_snapshot().await.volume {
            Valid(volume) => Valid(volume),
            Unavailable => Volume::latest().await?,
        };
        let volume = volume_observed.clone().unwrap_or_default();

        let new_mute;

        let mute = if mute_str == "toggle" {
            new_mute = !volume.mute;

            mute_str.to_string()
        } else {
            let mute = mute_str.parse::<bool>()?;
            new_mute = mute;

            u8::from(mute).to_string()
        };

        // Set the mute state
        let _ = command::run("wpctl", &["set-mute", "@DEFAULT_SINK@", mute.as_str()])?;

        // Update the volume in the snapshot
        let update = update_snapshot(volume_observed.map(|volume| Volume {
            mute: new_mute,
            ..volume
        }))
        .await;

        // Do a notification
        volume::notify(update).await?;

        Ok(())
    }
}

fn get_wpctl_output() -> Result<String, DaemonError> {
    // Get the volume and mute status as a string
    command::run("wpctl", &["get-volume", "@DEFAULT_SINK@"])
}

fn get_wpctl_split(output: &str) -> SplitWhitespace<'_> {
    // Left with only volume number, and muted status
    output.trim_start_matches("Volume: ").split_whitespace()
}

fn get_mute_from_wpctl_split(mut split: SplitWhitespace) -> bool {
    // Get the mute state from the second part of the split
    split.nth(1).is_some()
}

#[instrument(skip(split))]
fn get_linear_percent_from_wpctl_split(mut split: SplitWhitespace) -> Result<u32, DaemonError> {
    // Take the first part of the split (The numerical part) then convert to linear percentage
    if let Some(volume_str) = split.next() {
        Ok(logarithmic_to_linear(volume_str.parse::<f64>()? * 100.) as u32)
    } else {
        Err(DaemonError::ParseError(split.join(" ")))
    }
}
