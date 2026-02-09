use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{
    ICON_END, ICON_EXT, NOTIFICATION_ID, command,
    config::get_config,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_monitored,
    monitored::Monitored,
    snapshot::{Snapshot, current_snapshot},
};

use super::{FAN_STATE_STRINGS, FanProfileSource, default_source};

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Ord, Eq)]
pub enum FanState {
    Performance = 0,
    Balanced = 1,
    #[default]
    Quiet = 2,
}

impl From<usize> for FanState {
    fn from(value: usize) -> Self {
        match value {
            0 => Self::Performance,
            1 => Self::Balanced,
            _ => Self::Quiet,
        }
    }
}

#[derive(Subcommand)]
pub enum FanProfileGetCommands {
    #[command(alias = "prof", alias = "p")]
    Profile,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum FanProfileSetCommands {
    #[command(alias = "prof", alias = "p")]
    Profile {
        #[arg()]
        value: String,
    },
}

#[derive(Subcommand)]
pub enum FanProfileUpdateCommands {
    #[command(alias = "prof", alias = "p")]
    Profile,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FanProfileItem {
    Profile,
    Icon,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct FanProfile {
    pub profile: FanState,
}

impl_monitored!(FanProfile, fan_profile);

impl FanProfile {
    #[must_use]
    pub fn get_icon() -> String {
        format!("sensors-fan{ICON_END}")
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `profile` then returned as an `Ok()`
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[must_use]
    pub fn to_tuples(&self) -> Vec<(String, String)> {
        let str_values = {
            let Self { profile } = self.clone();

            vec![
                FAN_STATE_STRINGS[profile as usize].to_string(),
                format!("{}{ICON_EXT}", Self::get_icon()),
            ]
        };

        vec!["profile".to_string(), "icon".to_string()]
            .into_iter()
            .zip(str_values)
            .collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub async fn notify() -> Result<(), DaemonError> {
    let profile = current_snapshot().await.fan_profile.unwrap_or_default().profile;
    let icon = FanProfile::get_icon();

    command::run(
        "dunstify",
        &[
            "-u",
            "-normal",
            "-t",
            get_config().notification_timeout.to_string().as_str(),
            "-i",
            icon.as_str(),
            "-r",
            NOTIFICATION_ID.to_string().as_str(),
            format!("Fan Profile: {}", FAN_STATE_STRINGS[profile as usize]).as_str(),
        ],
    )?;

    Ok(())
}

/// # Errors
/// Returns an error if the requested value could not be evaluated
pub async fn evaluate_item(
    item: DaemonItem,
    fan_profile_item: &FanProfileItem,
    value: Option<String>,
) -> Result<DaemonReply, DaemonError> {
    Ok(if let Some(value) = value {
        let prev_fan_profile = current_snapshot().await.fan_profile.unwrap_or_default();

        // Set value
        if matches!(fan_profile_item, FanProfileItem::Profile) {
            default_source().set_profile(value.as_str()).await?;
        }

        let new_profile = FanProfile {
            profile: FAN_STATE_STRINGS
                .iter()
                .position(|&profile| profile == value)
                .ok_or_else(|| DaemonError::ParseError(value.clone()))?
                .into(),
        };

        if prev_fan_profile != new_profile {
            // Do a notification
            notify().await?;
        }

        DaemonReply::Value { item, value }
    } else {
        // Get value
        let fan_profile = current_snapshot().await.fan_profile.unwrap_or_default();

        match fan_profile_item {
            FanProfileItem::Profile => DaemonReply::Value {
                item,
                value: FAN_STATE_STRINGS[fan_profile.profile as usize].to_string(),
            },
            FanProfileItem::Icon => DaemonReply::Value {
                item,
                value: FanProfile::get_icon(),
            },
        }
    })
}

#[must_use]
pub const fn match_get_commands(commands: &FanProfileGetCommands) -> DaemonMessage {
    DaemonMessage::Get {
        item: match commands {
            FanProfileGetCommands::Profile => DaemonItem::FanProfile(FanProfileItem::Profile),
            FanProfileGetCommands::Icon => DaemonItem::FanProfile(FanProfileItem::Icon),
        },
    }
}

#[must_use]
pub fn match_set_commands(commands: FanProfileSetCommands) -> DaemonMessage {
    match commands {
        FanProfileSetCommands::Profile { value } => DaemonMessage::Set {
            item: DaemonItem::FanProfile(FanProfileItem::Profile),
            value,
        },
    }
}

#[must_use]
pub const fn match_update_commands(commands: &FanProfileUpdateCommands) -> DaemonMessage {
    match commands {
        FanProfileUpdateCommands::Profile => DaemonMessage::Update {
            item: DaemonItem::FanProfile(FanProfileItem::Profile),
        },
    }
}
