use clap::Subcommand;
use serde::{Deserialize, Serialize};

use super::{default_source, latest};

use crate::{
    ICON_EXT, NOTIFICATION_ID, NOTIFICATION_TIMEOUT,
    battery::BatterySource,
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    snapshot::current_snapshot,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum BatteryState {
    FullyCharged = 0,
    Charging = 1,
    Discharging = 2,
    #[default]
    NotCharging = 3,
}

#[derive(Subcommand)]
pub enum BatteryGetCommands {
    #[command(alias = "s")]
    State,
    #[command(alias = "per", alias = "p")]
    Percent,
    #[command(alias = "t")]
    Time,
    #[command(alias = "i")]
    Icon,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum BatteryItem {
    State,
    Percent,
    Time,
    Icon,
    All,
}

const BAT_STATE_STRINGS: &[&str] = &["Fully Charged", "Charging", "Discharging", "Not Charging"];
const BAT_NOTIFY_VALUES: &[u32] = &[5, 15, 20, 30];

#[derive(Clone, Debug, Default)]
pub struct Battery {
    pub state: BatteryState,
    pub percent: u32,
    pub time: String,
}

impl Battery {
    #[must_use]
    pub fn get_icon(&self) -> String {
        if self.state == BatteryState::NotCharging {
            "battery-missing".to_string()
        } else {
            format!(
                "battery-{:0>3}{}",
                self.percent / 10 * 10,
                match self.state {
                    BatteryState::Charging => "-charging",
                    // BatteryState::FullyCharged => "-charged",
                    _ => "",
                }
            )
        }
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `state` then returned as an `Ok()`
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[must_use]
    pub fn to_tuples(&self) -> Vec<(String, String)> {
        // Create list of values for tuples
        let str_values = {
            let Self { state, percent, time } = self;
            let icon = self.get_icon();

            vec![
                BAT_STATE_STRINGS[*state as usize].to_string(),
                percent.to_string(),
                time.clone(),
                format!("{icon}{ICON_EXT}"),
            ]
        };

        // Zip list of values with list of value names
        vec![
            "state".to_string(),
            "percent".to_string(),
            "time".to_string(),
            "icon".to_string(),
        ]
        .into_iter()
        .zip(str_values)
        .collect::<Vec<_>>()
    }
}

/// # Errors
/// Returns an error if `CURRENT_SNAPSHOT` could not be read
/// Returns an error if notification command could not be run
pub fn notify(prev_percent: u32) -> Result<(), DaemonError> {
    let battery = current_snapshot()?.battery;

    let current_percent = battery.percent;

    if current_percent < prev_percent && battery.state == BatteryState::Discharging {
        for &value in BAT_NOTIFY_VALUES.iter().rev() {
            if current_percent == value {
                command::run(
                    "dunstify",
                    &[
                        "-u",
                        "-normal",
                        "-t",
                        NOTIFICATION_TIMEOUT.to_string().as_str(),
                        "-i",
                        battery.get_icon().as_str(),
                        "-r",
                        NOTIFICATION_ID.to_string().as_str(),
                        "-h",
                        format!("int:value:{current_percent}").as_str(),
                        "Battery: ",
                    ],
                )?;
            }
        }
    }

    Ok(())
}

/// # Errors
/// Returns an error if the requested value could not be parsed
pub fn evaluate_item(item: DaemonItem, battery_item: &BatteryItem) -> Result<DaemonReply, DaemonError> {
    Ok(
        // Get value (use latest() since this value changes without bar_daemon changing it)
        match battery_item {
            BatteryItem::State => DaemonReply::Value {
                item,
                value: BAT_STATE_STRINGS[default_source().read_state()? as usize].to_string(),
            },
            BatteryItem::Percent => DaemonReply::Value {
                item,
                value: default_source().read_percent()?.to_string(),
            },
            BatteryItem::Time => DaemonReply::Value {
                item,
                value: default_source().read_time()?,
            },
            BatteryItem::Icon => DaemonReply::Value {
                item,
                value: latest()?.get_icon(),
            },
            BatteryItem::All => DaemonReply::Tuples {
                item,
                tuples: latest()?.to_tuples(),
            },
        },
    )
}

#[must_use]
pub const fn match_get_commands(commands: &Option<BatteryGetCommands>) -> DaemonMessage {
    DaemonMessage::Get {
        item: match commands {
            Some(commands) => match commands {
                BatteryGetCommands::State => DaemonItem::Battery(BatteryItem::State),
                BatteryGetCommands::Percent => DaemonItem::Battery(BatteryItem::Percent),
                BatteryGetCommands::Time => DaemonItem::Battery(BatteryItem::Time),
                BatteryGetCommands::Icon => DaemonItem::Battery(BatteryItem::Icon),
            },
            None => DaemonItem::Battery(BatteryItem::All),
        },
    }
}
