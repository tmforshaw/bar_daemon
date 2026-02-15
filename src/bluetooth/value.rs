use clap::{ArgAction, Subcommand};
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};

use crate::{
    ICON_END, ICON_EXT, NOTIFICATION_ID,
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

use super::{BluetoothSource, default_source};

const NOTIFICATION_OFFSET: u32 = 1;

#[derive(Subcommand)]
pub enum BluetoothGetCommands {
    #[command(alias = "s")]
    State,
    #[command(alias = "i")]
    Icon,
}

#[derive(Subcommand)]
pub enum BluetoothSetCommands {
    #[command(alias = "s")]
    State {
        #[arg(action = ArgAction::Set, value_parser = parse_bool)]
        value: Option<bool>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub enum BluetoothItem {
    State,
    Icon,
    All,
}

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct Bluetooth {
    pub state: bool,
}

impl_monitored!(Bluetooth, bluetooth, bluetooth);
impl_into_snapshot_event!(Bluetooth);

impl Bluetooth {
    #[must_use]
    pub fn get_icon(&self) -> String {
        format!("bluetooth-{}{ICON_END}", if self.state { "active" } else { "disabled" })
    }
}

impl ToTuples for Bluetooth {
    fn to_tuple_names() -> Vec<String> {
        vec!["state".to_string(), "icon".to_string()]
    }
    /// # Errors
    /// Errors are turned into `String` and set as value of `state` then returned as an `Ok()`
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    fn to_tuples(&self) -> Vec<(String, String)> {
        // Create list of values for tuples
        let str_values = {
            let icon = self.get_icon();

            vec![self.state.to_string(), format!("{icon}{ICON_EXT}")]
        };

        // Zip list of values with list of value names
        Self::to_tuple_names().into_iter().zip(str_values).collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if `CURRENT_SNAPSHOT` could not be read
/// Returns an error if notification command could not be run
#[instrument]
pub async fn notify(update: MonitoredUpdate<Bluetooth>) -> Result<(), DaemonError> {
    fn do_notification(new: &Bluetooth) -> Result<(), DaemonError> {
        command::run(
            "dunstify",
            &[
                "-u",
                "normal",
                "-r",
                (NOTIFICATION_ID + NOTIFICATION_OFFSET).to_string().as_str(),
                "-i",
                new.get_icon().trim(),
                "-t",
                get_config().notification_timeout.to_string().as_str(),
                format!("Bluetooth: {}", if new.state { "on" } else { "off" }).as_str(),
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
                (NOTIFICATION_ID + NOTIFICATION_OFFSET).to_string().as_str(),
                "-t",
                get_config().notification_timeout.to_string().as_str(),
                "Bluetooth Unavailable",
            ],
        )?;

        Ok(())
    }

    // Only create notification if the update changed something
    if update.old != update.new {
        // If the new values are valid
        match update.new {
            Valid(new) => do_notification(&new)?,
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
    bluetooth_item: &BluetoothItem,
    value: Option<String>,
) -> Result<DaemonReply, DaemonError> {
    Ok(if let Some(value) = value {
        // Set value
        if bluetooth_item == &BluetoothItem::State {
            default_source().set_state(value.as_str()).await?;
        }

        DaemonReply::Value { item, value }
    } else {
        // Get value
        match bluetooth_item {
            BluetoothItem::State => DaemonReply::Value {
                item,
                value: match current_snapshot().await.bluetooth {
                    Valid(bluetooth) => bluetooth.state.to_string(),
                    Unavailable => default_source().read_state().await?.to_string(),
                },
            },
            BluetoothItem::Icon => DaemonReply::Value {
                item,
                value: match current_snapshot().await.bluetooth {
                    Valid(bluetooth) => bluetooth.get_icon(),
                    Unavailable => Bluetooth::latest().await?.map(|bluetooth| bluetooth.get_icon()).to_string(),
                },
            },
            BluetoothItem::All => DaemonReply::Tuples {
                item,
                tuples: Bluetooth::latest().await?.to_tuples(),
            },
        }
    })
}

#[must_use]
pub const fn match_get_commands(commands: &Option<BluetoothGetCommands>) -> DaemonMessage {
    DaemonMessage::Get {
        item: match commands {
            Some(commands) => match commands {
                BluetoothGetCommands::State => DaemonItem::Bluetooth(BluetoothItem::State),
                BluetoothGetCommands::Icon => DaemonItem::Bluetooth(BluetoothItem::Icon),
            },
            None => DaemonItem::Bluetooth(BluetoothItem::All),
        },
    }
}

#[must_use]
pub fn match_set_commands(commands: &BluetoothSetCommands) -> DaemonMessage {
    match commands {
        BluetoothSetCommands::State { value } => DaemonMessage::Set {
            item: DaemonItem::Bluetooth(BluetoothItem::State),
            value: value.map_or("toggle".to_string(), |value| value.to_string()),
        },
    }
}
