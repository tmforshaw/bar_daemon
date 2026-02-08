use crate::{
    ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
    cli::parse_bool,
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_monitored,
    monitored::Monitored,
    snapshot::{Snapshot, current_snapshot},
};

use super::{VolumeSource, default_source, latest};

use clap::{ArgAction, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub enum VolumeGetCommands {
    #[command(alias = "per", alias = "p")]
    Percent,
    #[command(alias = "m")]
    Mute,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum VolumeSetCommands {
    #[command(alias = "per", alias = "p")]
    Percent {
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    #[command(alias = "m")]
    Mute {
        #[arg(action = ArgAction::Set, value_parser = parse_bool)]
        value: Option<bool>,
    },
}

#[derive(Subcommand)]
pub enum VolumeUpdateCommands {
    #[command(alias = "per", alias = "p")]
    Percent,
    #[command(alias = "m")]
    Mute,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum VolumeItem {
    Percent,
    Mute,
    Icon,
    All,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct Volume {
    pub percent: u32,
    pub mute: bool,
}

impl_monitored!(Volume, volume);

impl Volume {
    #[must_use]
    pub const fn get_percent(&self) -> u32 {
        self.percent
    }

    #[must_use]
    pub const fn get_mute(&self) -> bool {
        self.mute
    }

    #[must_use]
    pub fn get_icon(&self) -> String {
        format!(
            "audio-volume-{}",
            if self.mute {
                "muted"
            } else {
                match self.percent {
                    0 => "muted",
                    1..=33 => "low",
                    34..=67 => "medium",
                    68..=100 => "high",
                    101.. => "overamplified",
                }
            }
        )
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `percent` then returned as an `Ok()`
    /// Returns an error if values in the output of the command cannot be parsed
    #[must_use]
    pub fn to_tuples(&self) -> Vec<(String, String)> {
        // Create list of values for tuples
        let str_values = {
            let Self { percent, mute } = self;
            let icon = self.get_icon();

            vec![percent.to_string(), mute.to_string(), format!("{icon}{ICON_EXT}")]
        };

        // Zip list of values with list of value names
        vec!["percent".to_string(), "mute_state".to_string(), "icon".to_string()]
            .into_iter()
            .zip(str_values)
            .collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if `CURRENT_SNAPSHOT` could not be read
/// Returns an error if notification command could not be run
pub fn notify() -> Result<(), DaemonError> {
    // Get the current volume from the state
    let volume = current_snapshot()?.volume;

    let percent = volume.percent;
    let icon = volume.get_icon();

    command::run(
        "dunstify",
        &[
            "-u",
            "normal",
            "-r",
            format!("{NOTIFICATION_ID}").as_str(),
            "-i",
            icon.trim().to_string().as_str(),
            "-t",
            format!("{NOTIFICATION_TIMEOUT}").as_str(),
            "-h",
            format!("int:value:{percent}").as_str(),
            "Volume: ",
        ],
    )?;

    Ok(())
}

/// # Errors
/// Returns an error if the requested value could not be evaluated
pub fn evaluate_item(item: DaemonItem, volume_item: &VolumeItem, value: Option<String>) -> Result<DaemonReply, DaemonError> {
    Ok(if let Some(value) = value {
        // Get the current volume before the change
        let prev_volume_obj = current_snapshot()?.volume;

        // Set value
        match volume_item {
            VolumeItem::Percent => default_source().set_percent(value.as_str())?,
            VolumeItem::Mute => default_source().set_mute(value.as_str())?,
            _ => {}
        }

        let new_volume_obj = latest()?;

        if prev_volume_obj != new_volume_obj {
            // Do a notification
            notify()?;
        }

        DaemonReply::Value { item, value }
    } else {
        // Get value (use current_snapshot since this won't change without bar_daemon changing it)
        match volume_item {
            VolumeItem::Percent => DaemonReply::Value {
                item,
                value: current_snapshot()?.volume.percent.to_string(),
            },
            VolumeItem::Mute => DaemonReply::Value {
                item,
                value: current_snapshot()?.volume.mute.to_string(),
            },
            VolumeItem::Icon => DaemonReply::Value {
                item,
                value: current_snapshot()?.volume.get_icon(),
            },
            VolumeItem::All => DaemonReply::Tuples {
                item,
                tuples: current_snapshot()?.volume.to_tuples(),
            },
        }
    })
}

#[must_use]
pub const fn match_get_commands(commands: &Option<VolumeGetCommands>) -> DaemonMessage {
    DaemonMessage::Get {
        item: match commands {
            Some(commands) => match commands {
                VolumeGetCommands::Percent => DaemonItem::Volume(VolumeItem::Percent),
                VolumeGetCommands::Mute => DaemonItem::Volume(VolumeItem::Mute),
                VolumeGetCommands::Icon => DaemonItem::Volume(VolumeItem::Icon),
            },
            None => DaemonItem::Volume(VolumeItem::All),
        },
    }
}

#[must_use]
pub fn match_set_commands(commands: VolumeSetCommands) -> DaemonMessage {
    match commands {
        VolumeSetCommands::Percent { value } => DaemonMessage::Set {
            item: DaemonItem::Volume(VolumeItem::Percent),
            value,
        },
        VolumeSetCommands::Mute { value } => DaemonMessage::Set {
            item: DaemonItem::Volume(VolumeItem::Mute),
            value: value.map_or_else(|| "toggle".to_string(), |value| value.to_string()),
        },
    }
}

#[must_use]
pub const fn match_update_commands(commands: &VolumeUpdateCommands) -> DaemonMessage {
    match commands {
        VolumeUpdateCommands::Percent => DaemonMessage::Update {
            item: DaemonItem::Volume(VolumeItem::Percent),
        },
        VolumeUpdateCommands::Mute => DaemonMessage::Update {
            item: DaemonItem::Volume(VolumeItem::Mute),
        },
    }
}
