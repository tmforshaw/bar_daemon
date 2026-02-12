use crate::{
    ICON_EXT, NOTIFICATION_ID,
    cli::parse_bool,
    command,
    config::get_config,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_into_snapshot_event, impl_monitored,
    monitored::{Monitored, MonitoredUpdate},
    observed::Observed::{self, Unavailable, Valid},
    snapshot::{IntoSnapshotEvent, Snapshot, SnapshotEvent, current_snapshot},
    tuples::ToTuples,
};

use super::{VolumeSource, default_source, latest};

use clap::{ArgAction, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::instrument;

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
impl_into_snapshot_event!(Volume);

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
}

impl ToTuples for Volume {
    fn to_tuple_names() -> Vec<String> {
        vec!["percent".to_string(), "mute_state".to_string(), "icon".to_string()]
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `percent` then returned as an `Ok()`
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    fn to_tuples(&self) -> Vec<(String, String)> {
        // Create list of values for tuples
        let str_values = {
            let Self { percent, mute } = self;
            let icon = self.get_icon();

            vec![percent.to_string(), mute.to_string(), format!("{icon}{ICON_EXT}")]
        };

        // Zip list of values with list of value names
        Self::to_tuple_names().into_iter().zip(str_values).collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if `CURRENT_SNAPSHOT` could not be read
/// Returns an error if notification command could not be run
#[instrument]
pub async fn notify(update: MonitoredUpdate<Volume>) -> Result<(), DaemonError> {
    fn do_notification(new: Volume) -> Result<(), DaemonError> {
        command::run(
            "dunstify",
            &[
                "-u",
                "normal",
                "-r",
                format!("{NOTIFICATION_ID}").as_str(),
                "-i",
                new.get_icon().trim(),
                "-t",
                get_config().notification_timeout.to_string().as_str(),
                "-h",
                format!("int:value:{}", new.percent).as_str(),
                "Volume: ",
            ],
        )?;

        Ok(())
    }

    fn do_notification_unavailable() -> Result<(), DaemonError> {
        command::run(
            "dunstify",
            &[
                "-u",
                "normal",
                "-r",
                format!("{NOTIFICATION_ID}").as_str(),
                "-t",
                get_config().notification_timeout.to_string().as_str(),
                "Volume Unavailable",
            ],
        )?;

        Ok(())
    }
    // Only create notification if the update changed something
    if update.old != update.new {
        // If the new values are valid
        match update.new {
            Valid(new) => do_notification(new)?,
            Unavailable => do_notification_unavailable()?,
        }
    }

    Ok(())
}

/// # Errors
/// Returns an error if the requested value could not be evaluated
#[instrument]
pub async fn evaluate_item(
    item: DaemonItem,
    volume_item: &VolumeItem,
    value: Option<String>,
) -> Result<DaemonReply, DaemonError> {
    Ok(if let Some(value) = value {
        // Set value
        match volume_item {
            VolumeItem::Percent => default_source().set_percent(value.as_str()).await?,
            VolumeItem::Mute => default_source().set_mute(value.as_str()).await?,
            _ => {}
        }

        DaemonReply::Value { item, value }
    } else {
        // Get value (use current_snapshot since this won't change without bar_daemon changing it) (Use latest when current_snapshot is empty)
        match volume_item {
            VolumeItem::Percent => DaemonReply::Value {
                item,
                value: match current_snapshot().await.volume {
                    Valid(volume) => volume.percent.to_string(),
                    Unavailable => default_source().read_percent().await?.to_string(),
                },
            },
            VolumeItem::Mute => DaemonReply::Value {
                item,
                value: match current_snapshot().await.volume {
                    Valid(volume) => volume.mute.to_string(),
                    Unavailable => default_source().read_mute().await?.to_string(),
                },
            },
            VolumeItem::Icon => DaemonReply::Value {
                item,
                value: match current_snapshot().await.volume {
                    Valid(volume) => volume.get_icon(),
                    Unavailable => latest().await?.map(|volume| volume.get_icon()).to_string(),
                },
            },
            VolumeItem::All => DaemonReply::Tuples {
                item,
                tuples: latest().await?.to_tuples(),
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
