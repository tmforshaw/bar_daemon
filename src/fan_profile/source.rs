use crate::{
    command,
    error::DaemonError,
    snapshot::{current_snapshot, update_snapshot},
};

use super::{FanProfile, FanState};

pub const FAN_STATE_STRINGS: &[&str] = &["Performance", "Balanced", "Quiet"];

pub trait FanProfileSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<FanProfile, DaemonError>> + Send;
    async fn read_profile(&self) -> Result<FanState, DaemonError>;

    // Change values of source
    fn set_profile(&self, profile_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl FanProfileSource {
    AsusctlFanProfile
}

pub async fn latest() -> Result<FanProfile, DaemonError> {
    default_source().read().await
}

// ---------------- Wpctl Source ---------------

pub struct AsusctlFanProfile;

impl FanProfileSource for AsusctlFanProfile {
    /// # Errors
    /// Returns an error if the command can't be ran
    /// Returns an error if the correct line can't be found
    /// Returns an error if the correct part of the line can't be found
    /// Returns an error if the profile string can't be converted to ``FanState``
    async fn read(&self) -> Result<FanProfile, DaemonError> {
        let profile = self.read_profile().await?;

        Ok(FanProfile { profile })
    }

    /// # Errors
    /// Returns an error if the command can't be ran
    /// Returns an error if the correct line can't be found
    /// Returns an error if the correct part of the line can't be found
    /// Returns an error if the profile string can't be converted to ``FanState``
    async fn read_profile(&self) -> Result<FanState, DaemonError> {
        // Read the profile from the output of asusctl
        let profile = get_asusctl_profile()?;

        // Update snapshot
        update_snapshot(FanProfile { profile }).await;

        Ok(profile)
    }

    /// # Errors
    /// Returns an error if the given value is not a valid profile
    /// Returns an error if the set command can't be ran
    async fn set_profile(&self, profile_str: &str) -> Result<(), DaemonError> {
        let new_profile_idx;

        let new_profile = if let Some(index) = FAN_STATE_STRINGS.iter().position(|&profile| profile == profile_str.trim()) {
            // Set the new_profile_id
            new_profile_idx = index;

            // A new profile has been set
            profile_str.trim()
        } else {
            // Profile is set via cyclic function
            let current_profile = current_snapshot().await.fan_profile.unwrap_or_default().profile;

            match profile_str {
                "next" => {
                    // Calculate the new profile's index
                    new_profile_idx = (current_profile as usize + 1) % FAN_STATE_STRINGS.len();

                    FAN_STATE_STRINGS[new_profile_idx]
                }
                "prev" => {
                    new_profile_idx = (current_profile as usize)
                        .checked_sub(1)
                        .unwrap_or(FAN_STATE_STRINGS.len() - 1);

                    FAN_STATE_STRINGS[new_profile_idx]
                }
                incorrect => return Err(DaemonError::ParseError(incorrect.to_string())),
            }
        };

        command::run("asusctl", &["profile", "set", new_profile])?;

        // Update snapshot
        update_snapshot(FanProfile {
            profile: new_profile_idx.into(),
        })
        .await;

        Ok(())
    }
}

fn get_asusctl_output() -> Result<String, DaemonError> {
    // Get the profile output from asusctl
    command::run("asusctl", &["profile", "get"])
}

fn get_asusctl_split(output: &str) -> Result<&str, DaemonError> {
    output
        .lines()
        .next()
        .ok_or_else(|| DaemonError::ParseError(output.to_string()))
}

fn get_asusctl_profile() -> Result<FanState, DaemonError> {
    // Find the correct line where the fan profile is
    let output = get_asusctl_output()?;
    let output_line = get_asusctl_split(&output)?;

    // Match the profile string
    Ok(
        match output_line
            .split_whitespace()
            .nth(2)
            .ok_or_else(|| DaemonError::ParseError(output_line.to_string()))?
        {
            "Performance" => FanState::Performance,
            "Balanced" => FanState::Balanced,
            "Quiet" => FanState::Quiet,
            incorrect => Err(DaemonError::ParseError(incorrect.to_string()))?,
        },
    )
}
