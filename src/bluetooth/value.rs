use clap::{ArgAction, Subcommand};
use serde::{Deserialize, Serialize};

use crate::{
    ICON_END, ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
    cli::parse_bool,
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_monitored,
    monitored::Monitored,
    snapshot::{Snapshot, current_snapshot},
};

use super::{BluetoothSource, default_source, latest};

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

#[derive(Subcommand)]
pub enum BluetoothUpdateCommands {
    #[command(alias = "s")]
    State,
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

impl_monitored!(Bluetooth, bluetooth);

impl Bluetooth {
    #[must_use]
    pub fn get_icon(&self) -> String {
        format!("bluetooth-{}{ICON_END}", if self.state { "active" } else { "disabled" })
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `state` then returned as an `Ok()`
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[must_use]
    pub fn to_tuples(&self) -> Vec<(String, String)> {
        // Create list of values for tuples
        let str_values = {
            let icon = self.get_icon();

            vec![self.state.to_string(), format!("{icon}{ICON_EXT}")]
        };

        // Zip list of values with list of value names
        vec!["state".to_string(), "icon".to_string()]
            .into_iter()
            .zip(str_values)
            .collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if `CURRENT_SNAPSHOT` could not be read
/// Returns an error if notification command could not be run
pub async fn notify() -> Result<(), DaemonError> {
    let bluetooth = current_snapshot().await.bluetooth.unwrap_or_default();

    let icon = bluetooth.get_icon();

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
            format!("Bluetooth: {}", if bluetooth.state { "on" } else { "off" }).as_str(),
        ],
    )?;

    Ok(())
}
/// # Errors
/// Returns an error if the requested value could not be evaluated
pub async fn evaluate_item(
    item: DaemonItem,
    bluetooth_item: &BluetoothItem,
    value: Option<String>,
) -> Result<DaemonReply, DaemonError> {
    Ok(if let Some(value) = value {
        let prev_state = current_snapshot().await.bluetooth.unwrap_or_default();

        // Set value
        if bluetooth_item == &BluetoothItem::State {
            default_source().set_state(value.as_str()).await?;
        }

        let new_state = latest().await?;

        if prev_state != new_state {
            // Do a notification
            notify().await?;
        }

        DaemonReply::Value { item, value }
    } else {
        // Get value
        match bluetooth_item {
            BluetoothItem::State => DaemonReply::Value {
                item,
                value: default_source().read_state().await?.to_string(),
            },
            BluetoothItem::Icon => DaemonReply::Value {
                item,
                value: latest().await?.get_icon(),
            },
            BluetoothItem::All => DaemonReply::Tuples {
                item,
                tuples: latest().await?.to_tuples(),
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

#[must_use]
pub const fn match_update_commands(commands: &BluetoothUpdateCommands) -> DaemonMessage {
    match commands {
        BluetoothUpdateCommands::State => DaemonMessage::Update {
            item: DaemonItem::Bluetooth(BluetoothItem::State),
        },
    }
}
