use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    ICON_END, ICON_EXT, NOTIFICATION_ID, command,
    config::get_config,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_monitored,
    monitored::Monitored,
    snapshot::{Snapshot, current_snapshot},
};

use super::{BrightnessSource, MONITOR_ID, default_source, latest};

#[derive(Subcommand)]
pub enum BrightnessGetCommands {
    #[command(alias = "mon", alias = "m")]
    Monitor,
    #[command(alias = "key", alias = "k")]
    Keyboard,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum BrightnessSetCommands {
    #[command(alias = "mon", alias = "m")]
    Monitor {
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    #[command(alias = "key", alias = "k")]
    Keyboard {
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
}

#[derive(Subcommand)]
pub enum BrightnessUpdateCommands {
    #[command(alias = "mon", alias = "m")]
    Monitor,
    #[command(alias = "key", alias = "k")]
    Keyboard,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BrightnessItem {
    Monitor,
    Keyboard,
    Icon,
    All,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct Brightness {
    pub monitor: u32,
    pub keyboard: u32,
}

impl_monitored!(Brightness, brightness);

impl Brightness {
    #[must_use]
    pub fn get_icon(&self, device_id: &str) -> String {
        if device_id == MONITOR_ID {
            format!(
                "display-brightness-{}{ICON_END}",
                match self.monitor {
                    0 => "off",
                    1..=33 => "low",
                    34..=67 => "medium",
                    68.. => "high",
                }
            )
        } else {
            let strength = match self.keyboard {
                0 => "off",
                1..=33 => "medium",
                34..=67 => "",
                68.. => "high",
            };

            format!(
                "keyboard-brightness{}{ICON_END}",
                if strength.is_empty() {
                    String::new()
                } else {
                    format!("-{strength}")
                }
            )
        }
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `monitor_percent` then returned as an `Ok()`
    /// Returns an error if values in the output of the command cannot be parsed
    #[must_use]
    #[instrument]
    pub fn to_tuples(&self) -> Vec<(String, String)> {
        let str_values = {
            let icon = self.get_icon(MONITOR_ID);

            vec![self.monitor.to_string(), format!("{icon}{ICON_EXT}")]
        };

        vec!["monitor_percent".to_string(), "icon".to_string()]
            .into_iter()
            .zip(str_values)
            .collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if the requested value could not be parsed
#[instrument]
pub async fn notify(device_id: &str) -> Result<(), DaemonError> {
    let brightness = current_snapshot().await.brightness.unwrap_or(latest().await?);

    let percent = if device_id == MONITOR_ID {
        brightness.monitor
    } else {
        brightness.keyboard
    };

    let icon = brightness.get_icon(device_id);

    command::run(
        "dunstify",
        &[
            "-u",
            "normal",
            "-r",
            format!("{NOTIFICATION_ID}").as_str(),
            "-i",
            icon.as_str(),
            "-t",
            get_config().notification_timeout.to_string().as_str(),
            "-h",
            format!("int:value:{percent}").as_str(),
            format!("{}: ", if device_id == MONITOR_ID { "Monitor" } else { "Keyboard" }).as_str(),
        ],
    )?;

    Ok(())
}

#[must_use]
pub const fn match_get_commands(commands: &Option<BrightnessGetCommands>) -> DaemonMessage {
    DaemonMessage::Get {
        item: match commands {
            Some(commands) => match commands {
                BrightnessGetCommands::Monitor => DaemonItem::Brightness(BrightnessItem::Monitor),
                BrightnessGetCommands::Keyboard => DaemonItem::Brightness(BrightnessItem::Keyboard),
                BrightnessGetCommands::Icon => DaemonItem::Brightness(BrightnessItem::Icon),
            },
            None => DaemonItem::Brightness(BrightnessItem::All),
        },
    }
}

#[must_use]
pub fn match_set_commands(commands: BrightnessSetCommands) -> DaemonMessage {
    match commands {
        BrightnessSetCommands::Monitor { value } => DaemonMessage::Set {
            item: DaemonItem::Brightness(BrightnessItem::Monitor),
            value,
        },
        BrightnessSetCommands::Keyboard { value } => DaemonMessage::Set {
            item: DaemonItem::Brightness(BrightnessItem::Keyboard),
            value,
        },
    }
}

#[must_use]
pub const fn match_update_commands(commands: &BrightnessUpdateCommands) -> DaemonMessage {
    match commands {
        BrightnessUpdateCommands::Monitor => DaemonMessage::Update {
            item: DaemonItem::Brightness(BrightnessItem::Monitor),
        },
        BrightnessUpdateCommands::Keyboard => DaemonMessage::Update {
            item: DaemonItem::Brightness(BrightnessItem::Keyboard),
        },
    }
}

/// # Errors
/// Returns an error if the requested value could not be evaluated
#[instrument]
pub async fn evaluate_item(
    item: DaemonItem,
    brightness_item: &BrightnessItem,
    value: Option<String>,
) -> Result<DaemonReply, DaemonError> {
    Ok(if let Some(value) = value {
        // Set value
        match brightness_item {
            BrightnessItem::Monitor => default_source().set_monitor(value.as_str()).await?,
            BrightnessItem::Keyboard => default_source().set_keyboard(value.as_str()).await?,
            _ => {}
        }

        // Notifications are done in the set_* functions

        DaemonReply::Value { item, value }
    } else {
        // Get value TODO Probably could use current_snapshot instead of default_source()
        match brightness_item {
            BrightnessItem::Monitor => DaemonReply::Value {
                item,
                value: match current_snapshot().await.brightness {
                    Some(brightness) => Ok(brightness.monitor),
                    None => default_source().read_monitor().await,
                }?
                .to_string(),
            },
            BrightnessItem::Keyboard => DaemonReply::Value {
                item,
                value: match current_snapshot().await.brightness {
                    Some(brightness) => Ok(brightness.keyboard),
                    None => default_source().read_keyboard().await,
                }?
                .to_string(),
            },
            BrightnessItem::Icon | BrightnessItem::All => {
                let brightness = current_snapshot().await.brightness.unwrap_or(latest().await?);

                match brightness_item {
                    BrightnessItem::Icon => DaemonReply::Value {
                        item,
                        value: brightness.get_icon(MONITOR_ID),
                    },
                    _ => DaemonReply::Tuples {
                        item,
                        tuples: brightness.to_tuples(),
                    },
                }
            }
        }
    })
}
