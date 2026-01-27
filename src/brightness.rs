use std::cmp;

use clap::Subcommand;
use serde::{Deserialize, Serialize};

use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
};

pub const MONITOR_ID: &str = "nvidia_wmi_ec_backlight";
pub const KEYBOARD_ID: &str = "asus::kbd_backlight";

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

pub struct Brightness;

impl Brightness {
    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    fn get(device_id: &str) -> Result<f32, DaemonError> {
        let output = command::run("brightnessctl", &["-m", "-d", device_id, "i"])?;

        // Split the output by commas
        let output_split = output.split(',').map(ToString::to_string).collect::<Vec<_>>();

        // Get the current and maximum brightness values
        let current_brightness = output_split.get(2);
        let max_brightness = output_split.get(4);

        // Parse the values into integers, then get the floating point percentage
        Ok(
            if let (Some(current_brightness), Some(max_brightness)) = (current_brightness, max_brightness) {
                let current_value = f64::from(current_brightness.parse::<u32>()?);
                let max_value = f64::from(max_brightness.parse::<u32>()?);

                ((current_value / max_value) * 100.) as f32
            } else {
                return Err(DaemonError::ParseError(output));
            },
        )
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_monitor() -> Result<f32, DaemonError> {
        Self::get(MONITOR_ID)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_keyboard() -> Result<f32, DaemonError> {
        Self::get(KEYBOARD_ID)
    }

    #[must_use]
    pub fn get_icon(device_id: &str, percent: f32) -> String {
        let percent = percent as u32;

        if device_id == MONITOR_ID {
            format!(
                "display-brightness-{}{ICON_END}",
                match percent {
                    0 => "off",
                    1..=33 => "low",
                    34..=67 => "medium",
                    68.. => "high",
                }
            )
        } else {
            let strength = match percent {
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

    fn set(device_id: &str, percent_string: &str) -> Result<(), DaemonError> {
        // Change the percentage based on the delta percentage
        let percent = if percent_string.starts_with('+') || percent_string.starts_with('-') {
            let delta_percent = percent_string.parse::<f64>()?;
            let current_percent = f64::from(Self::get(device_id)?);

            // Depending on the first char, add or subtract the percentage
            (current_percent + delta_percent).clamp(0.0, 100.0)
        } else {
            percent_string.parse::<f64>()?
        };

        // Set the percentage
        command::run("brightnessctl", &["-d", device_id, "s", format!("{percent}%").as_str()])?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn set_monitor(percent: &str) -> Result<(), DaemonError> {
        let prev_monitor = Self::get_monitor()?;

        Self::set(MONITOR_ID, percent)?;

        let new_monitor = Self::get_monitor()?;

        if prev_monitor.partial_cmp(&new_monitor) != Some(cmp::Ordering::Equal) {
            Self::notify(MONITOR_ID)?;
        }

        Ok(())
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn set_keyboard(percent: &str) -> Result<(), DaemonError> {
        let prev_keyboard = Self::get_keyboard()?;

        Self::set(KEYBOARD_ID, percent)?;

        let new_keyboard = Self::get_keyboard()?;

        if prev_keyboard.partial_cmp(&new_keyboard) != Some(cmp::Ordering::Equal) {
            Self::notify(KEYBOARD_ID)?;
        }

        Ok(())
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `monitor_percent` then returned as an `Ok()`
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let str_values = match Self::get_monitor() {
            Ok(monitor_percent) => {
                let icon = Self::get_icon(MONITOR_ID, monitor_percent);

                vec![(monitor_percent as u32).to_string(), format!("{icon}{ICON_EXT}")]
            }
            Err(e) => {
                let icon = Self::get_icon(MONITOR_ID, 0.);

                vec![e.to_string(), format!("{icon}{ICON_EXT}")]
            }
        };

        Ok(vec!["monitor_percent".to_string(), "icon".to_string()]
            .into_iter()
            .zip(str_values)
            .collect::<Vec<_>>())
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
    /// Returns an error if the requested value could not be parsed
    pub fn parse_item(
        item: DaemonItem,
        brightness_item: &BrightnessItem,
        value: Option<String>,
    ) -> Result<DaemonReply, DaemonError> {
        Ok(if let Some(value) = value {
            // Set value
            match brightness_item {
                BrightnessItem::Monitor => Self::set_monitor(value.as_str())?,
                BrightnessItem::Keyboard => Self::set_keyboard(value.as_str())?,
                _ => {}
            }

            // Notifications are done in the set_* functions

            DaemonReply::Value { item, value }
        } else {
            // Get value
            match brightness_item {
                BrightnessItem::Monitor => DaemonReply::Value {
                    item,
                    value: Self::get_monitor()?.to_string(),
                },
                BrightnessItem::Keyboard => DaemonReply::Value {
                    item,
                    value: Self::get_keyboard()?.to_string(),
                },
                BrightnessItem::Icon => {
                    let percent = Self::get_monitor()?;

                    DaemonReply::Value {
                        item,
                        value: Self::get_icon(MONITOR_ID, percent),
                    }
                }
                BrightnessItem::All => DaemonReply::Tuples {
                    item,
                    tuples: Self::get_tuples()?,
                },
            }
        })
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn notify(device_id: &str) -> Result<(), DaemonError> {
        let percent = Self::get(device_id)?;

        let icon = Self::get_icon(device_id, percent);

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
                format!("{NOTIFICATION_TIMEOUT}").as_str(),
                "-h",
                format!("int:value:{percent}").as_str(),
                format!("{}: ", if device_id == MONITOR_ID { "Monitor" } else { "Keyboard" }).as_str(),
            ],
        )?;

        Ok(())
    }
}
