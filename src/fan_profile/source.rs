use tracing::instrument;

use crate::{
    command,
    error::DaemonError,
    fan_profile,
    observed::Observed::{self, Unavailable, Valid},
    snapshot::{current_snapshot, update_snapshot},
};

use super::{FanProfile, FanState};

pub const FAN_STATE_STRINGS: &[&str] = &["Performance", "Balanced", "Quiet"];

// TODO
#[allow(dead_code)]
pub trait FanProfileSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Observed<FanProfile>, DaemonError>> + Send;

    // Change values of source
    fn set_profile(&self, profile_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl FanProfileSource {
    AsusctlFanProfile
}

pub async fn latest() -> Result<Observed<FanProfile>, DaemonError> {
    default_source().read().await
}

// ---------------- Wpctl Source ---------------

#[derive(Debug)]
pub struct AsusctlFanProfile;

impl FanProfileSource for AsusctlFanProfile {
    /// # Errors
    /// Returns an error if the command can't be ran
    /// Returns an error if the correct line can't be found
    /// Returns an error if the correct part of the line can't be found
    /// Returns an error if the profile string can't be converted to ``FanState``
    #[instrument]
    async fn read(&self) -> Result<Observed<FanProfile>, DaemonError> {
        fn read_inner() -> Result<FanProfile, DaemonError> {
            // Read the profile from the output of asusctl
            let profile = get_asusctl_profile()?;

            Ok(FanProfile { profile })
        }

        // Set as unavailable if the inner function threw an error
        let fan_profile: Observed<_> = read_inner().into();

        // Update snapshot
        let update = update_snapshot(fan_profile.clone()).await;

        // Do a notification
        fan_profile::notify(update).await?;

        Ok(fan_profile)
    }

    /// # Errors
    /// Returns an error if the given value is not a valid profile
    /// Returns an error if the set command can't be ran
    #[instrument]
    async fn set_profile(&self, profile_str: &str) -> Result<(), DaemonError> {
        let fan_profile = match current_snapshot().await.fan_profile {
            Valid(fan_profile) => Valid(fan_profile),
            Unavailable => latest().await?,
        };

        let new_profile_idx;

        let new_profile = if let Some(index) = FAN_STATE_STRINGS.iter().position(|&profile| profile == profile_str.trim()) {
            // Set the new_profile_id
            new_profile_idx = index;

            // A new profile has been set
            profile_str.trim()
        } else {
            let profile = fan_profile.clone().unwrap_or_default().profile;

            // Profile is set via cyclic function
            match profile_str {
                "next" => {
                    // Calculate the new profile's index
                    new_profile_idx = (profile as usize + 1) % FAN_STATE_STRINGS.len();

                    FAN_STATE_STRINGS[new_profile_idx]
                }
                "prev" => {
                    new_profile_idx = (profile as usize).checked_sub(1).unwrap_or(FAN_STATE_STRINGS.len() - 1);

                    FAN_STATE_STRINGS[new_profile_idx]
                }
                incorrect => return Err(DaemonError::ParseError(incorrect.to_string())),
            }
        };

        // Set the profile using asusctl
        command::run("asusctl", &["profile", "set", new_profile])?;

        // Update snapshot
        let update = update_snapshot(fan_profile.map(|_| FanProfile {
            profile: new_profile_idx.into(),
        }))
        .await;

        // Do a notification
        fan_profile::notify(update).await?;

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

#[instrument]
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
