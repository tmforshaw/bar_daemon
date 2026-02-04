use std::str::SplitWhitespace;

use super::Volume;
use crate::{
    command,
    error::DaemonError,
    log_linear::{linear_to_logarithmic, logarithmic_to_linear},
    snapshot::{current_state, set_state_volume},
};

pub trait VolumeSource {
    // Read from commands (Get latest values)
    fn read(&self) -> Result<Volume, DaemonError>;
    fn read_percent(&self) -> Result<u32, DaemonError>;
    fn read_mute(&self) -> Result<bool, DaemonError>;

    // Change values of source
    fn set_percent(&self, percent_str: &str) -> Result<(), DaemonError>;
    fn set_mute(&self, mute_str: &str) -> Result<(), DaemonError>;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl VolumeSource {
    WpctlVolume
}

pub fn latest() -> Result<Volume, DaemonError> {
    default_source().read()
}

// ---------------- Wpctl Source ---------------

pub struct WpctlVolume;

impl VolumeSource for WpctlVolume {
    // Read from commands

    fn read(&self) -> Result<Volume, DaemonError> {
        let output = get_wpctl_output()?;
        let output_split = get_wpctl_split(&output);

        let percent = get_linear_percent_from_wpctl_split(output_split.clone())?;

        let mute = get_mute_from_wpctl_split(output_split);

        Ok(Volume { percent, mute })
    }

    fn read_percent(&self) -> Result<u32, DaemonError> {
        let output = get_wpctl_output()?;
        let output_split = get_wpctl_split(&output);

        get_linear_percent_from_wpctl_split(output_split.clone())
    }

    fn read_mute(&self) -> Result<bool, DaemonError> {
        let output = get_wpctl_output()?;
        let output_split = get_wpctl_split(&output);

        Ok(get_mute_from_wpctl_split(output_split.clone()))
    }

    // Set source values

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    fn set_percent(&self, percent_str: &str) -> Result<(), DaemonError> {
        // Get the current state values within the snapshot
        let state = current_state()?;

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
            (i32::try_from(state.volume.percent)?
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
        set_state_volume(Volume {
            percent: linear_percent,
            ..state.volume
        })?;

        // Set the volume internally as a logarithmic value
        let logarithmic_percent = linear_to_logarithmic(f64::from(linear_percent));

        // Set the volume
        let _ = command::run(
            "wpctl",
            &["set-volume", "@DEFAULT_SINK@", format!("{logarithmic_percent}%").as_str()],
        )?;

        Ok(())
    }

    fn set_mute(&self, mute_str: &str) -> Result<(), DaemonError> {
        let mute = if mute_str == "toggle" {
            mute_str.to_string()
        } else {
            u8::from(mute_str.parse::<bool>()?).to_string()
        };

        // Set the mute state
        let _ = command::run("wpctl", &["set-mute", "@DEFAULT_SINK@", mute.as_str()])?;

        // TODO update Snapshot

        Ok(())
    }
}

fn get_wpctl_output() -> Result<String, DaemonError> {
    // Get the volume and mute status as a string
    command::run("wpctl", &["get-volume", "@DEFAULT_SINK@"])
}

fn get_wpctl_split(output: &str) -> std::str::SplitWhitespace<'_> {
    // Left with only volume number, and muted status
    output.trim_start_matches("Volume: ").split_whitespace()
}

fn get_mute_from_wpctl_split(mut split: SplitWhitespace) -> bool {
    // Get the mute state from the second part of the split
    split.nth(1).is_some()
}

fn get_linear_percent_from_wpctl_split(mut split: SplitWhitespace) -> Result<u32, DaemonError> {
    // Take the first part of the split (The numerical part) then convert to linear percentage
    if let Some(volume_str) = split.next() {
        Ok(logarithmic_to_linear(volume_str.parse::<f64>()? * 100.) as u32)
    } else {
        Err(DaemonError::ParseError(split.collect()))
    }
}
