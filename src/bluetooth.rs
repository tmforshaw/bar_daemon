use clap::{ArgAction, Subcommand};
use serde::{Deserialize, Serialize};

use crate::{
    cli::parse_bool,
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
};

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

pub struct Bluetooth;

impl Bluetooth {
    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_state() -> Result<bool, DaemonError> {
        let output = command::run("bluetooth", &[])?;

        // Split the output and check if it is on or off
        output
            .clone()
            .split_whitespace()
            .nth(2)
            .map_or(Err(DaemonError::ParseError(output)), |state| Ok(state == "on"))
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    pub fn set_state(state: &str) -> Result<(), DaemonError> {
        // Allow toggling of the bluetooth state
        let state = match state {
            "toggle" => "toggle",
            _ => {
                if state.parse::<bool>()? {
                    "on"
                } else {
                    "off"
                }
            }
        };

        command::run("bluetooth", &[state])?;

        Ok(())
    }

    #[must_use]
    pub fn get_icon(state: bool) -> String {
        format!("bluetooth-{}{ICON_END}", if state { "active" } else { "disabled" })
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `state` then returned as an `Ok()`
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let str_values = match Self::get_state() {
            Ok(state) => {
                let icon = Self::get_icon(state);

                vec![state.to_string(), format!("{icon}{ICON_EXT}")]
            }
            Err(e) => {
                let icon = Self::get_icon(false);

                vec![e.to_string(), format!("{icon}{ICON_EXT}")]
            }
        };

        Ok(vec!["state".to_string(), "icon".to_string()]
            .into_iter()
            .zip(str_values)
            .collect::<Vec<_>>())
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn parse_item(
        item: DaemonItem,
        bluetooth_item: &BluetoothItem,
        value: Option<String>,
    ) -> Result<DaemonReply, DaemonError> {
        Ok(if let Some(value) = value {
            let prev_state = Self::get_state()?;

            // Set value
            if bluetooth_item == &BluetoothItem::State {
                Self::set_state(value.as_str())?;
            }

            let new_state = Self::get_state()?;

            if prev_state != new_state {
                // Do a notification
                Self::notify()?;
            }

            DaemonReply::Value { item, value }
        } else {
            // Get value
            match bluetooth_item {
                BluetoothItem::State => DaemonReply::Value {
                    item,
                    value: Self::get_state()?.to_string(),
                },
                BluetoothItem::Icon => {
                    let state = Self::get_state()?;

                    DaemonReply::Value {
                        item,
                        value: Self::get_icon(state),
                    }
                }
                BluetoothItem::All => DaemonReply::Tuples {
                    item,
                    tuples: Self::get_tuples()?,
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

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn notify() -> Result<(), DaemonError> {
        let state = Self::get_state()?;

        let icon = Self::get_icon(state);

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
                format!("Bluetooth: {}", if state { "on" } else { "off" }).as_str(),
            ],
        )?;

        Ok(())
    }
}
