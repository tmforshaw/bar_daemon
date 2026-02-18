use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};

use crate::{
    ICON_END, ICON_EXT, NOTIFICATION_ID,
    brightness::KEYBOARD_ID,
    changed::{Changed, ChangedConstructor},
    command,
    config::get_config,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_into_snapshot_event, impl_monitored,
    monitored::{Monitored, MonitoredUpdate},
    notification::Notify,
    observed::Observed::{self, Recovering, Unavailable, Valid},
    snapshot::{IntoSnapshotEvent, Snapshot, SnapshotEvent, current_snapshot},
    tuples::ToTuples,
};

use super::{BrightnessSource, MONITOR_ID, default_source};

const NOTIFICATION_OFFSET: u32 = 2;

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BrightnessItem {
    Monitor,
    Keyboard,
    Icon,
    All,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq, bar_daemon_derive::Changed)]
pub struct Brightness {
    pub monitor: u32,
    pub keyboard: u32,
}

impl_monitored!(Brightness, brightness, brightness);
impl_into_snapshot_event!(Brightness);

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
}

impl ToTuples for Brightness {
    fn to_tuple_names() -> Vec<String> {
        vec!["monitor_percent".to_string(), "icon".to_string()]
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `monitor_percent` then returned as an `Ok()`
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    fn to_tuples(&self) -> Vec<(String, String)> {
        let str_values = {
            let icon = self.get_icon(MONITOR_ID);

            vec![self.monitor.to_string(), format!("{icon}{ICON_EXT}")]
        };

        Self::to_tuple_names().into_iter().zip(str_values).collect::<Vec<_>>()
    }
}

impl Notify<Self> for Brightness {
    /// # Errors
    /// Returns an error if the requested value could not be parsed
    #[instrument]
    async fn notify(update: MonitoredUpdate<Self>) -> Result<(), DaemonError> {
        fn do_notification(new: &Brightness, device_id: &str) -> Result<(), DaemonError> {
            command::run(
                "dunstify",
                &[
                    "-u",
                    "normal",
                    "-r",
                    (NOTIFICATION_ID + NOTIFICATION_OFFSET).to_string().as_str(),
                    "-i",
                    new.get_icon(device_id).trim(),
                    "-t",
                    get_config().notification_timeout.to_string().as_str(),
                    "-h",
                    // Select the percent of the device which is being notified
                    format!(
                        "int:value:{}",
                        if device_id == MONITOR_ID { new.monitor } else { new.keyboard }
                    )
                    .as_str(),
                    format!("{}: ", if device_id == MONITOR_ID { "Monitor" } else { "Keyboard" }).as_str(),
                ],
            )?;

            Ok(())
        }

        fn do_notification_unavailable(device_id: &str) -> Result<(), DaemonError> {
            command::run(
                "dunstify",
                &[
                    "-u",
                    "normal",
                    "-r",
                    (NOTIFICATION_ID + NOTIFICATION_OFFSET).to_string().as_str(),
                    "-t",
                    get_config().notification_timeout.to_string().as_str(),
                    format!("{}: ", if device_id == MONITOR_ID { "Monitor" } else { "Keyboard" }).as_str(),
                ],
            )?;

            Ok(())
        }

        // Get which device(s) changed
        let changed = update.changed();
        let device_ids = [changed.monitor.then_some(MONITOR_ID), changed.keyboard.then_some(KEYBOARD_ID)]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        // Perform a notification for all devices which changed
        for device_id in device_ids {
            // If the new values are valid
            match update.new {
                Valid(ref new) => do_notification(new, device_id)?,
                Unavailable | Recovering => do_notification_unavailable(device_id)?,
            }
        }

        Ok(())
    }
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

        DaemonReply::Value { item, value }
    } else {
        match brightness_item {
            BrightnessItem::Monitor => DaemonReply::Value {
                item,
                value: match current_snapshot().await.brightness {
                    Valid(brightness) => brightness.monitor.to_string(),
                    Unavailable | Recovering => Brightness::latest().await?.map(|brightness| brightness.monitor).to_string(),
                },
            },
            BrightnessItem::Keyboard => DaemonReply::Value {
                item,
                value: match current_snapshot().await.brightness {
                    Valid(brightness) => brightness.keyboard.to_string(),
                    Unavailable | Recovering => Brightness::latest().await?.map(|brightness| brightness.keyboard).to_string(),
                },
            },
            BrightnessItem::Icon => DaemonReply::Value {
                item,
                value: match current_snapshot().await.brightness {
                    Valid(brightness) => brightness.get_icon(MONITOR_ID),
                    Unavailable | Recovering => Brightness::latest()
                        .await?
                        .map(|brightness| brightness.get_icon(MONITOR_ID))
                        .to_string(),
                },
            },
            BrightnessItem::All => DaemonReply::Tuples {
                item,
                tuples: match current_snapshot().await.brightness {
                    Valid(brightness) => brightness.to_tuples(),
                    Unavailable | Recovering => Brightness::latest().await?.to_tuples(),
                },
            },
        }
    })
}
