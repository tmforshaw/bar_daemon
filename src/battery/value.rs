use std::sync::LazyLock;

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::instrument;

use super::{default_source, latest};

use crate::{
    ICON_EXT, NOTIFICATION_ID,
    battery::BatterySource,
    command,
    config::get_config,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_into_snapshot_event, impl_monitored, impl_polled,
    monitored::{Monitored, MonitoredUpdate},
    observed::Observed::{self, Valid},
    polled::Polled,
    snapshot::{IntoSnapshotEvent, Snapshot, SnapshotEvent},
};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default, PartialOrd, Ord)]
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
const BAT_LOW_NOTIFY_THRESHOLDS: &[u32] = &[20, 15, 10, 5];
const BAT_HIGH_NOTIFY_THRESHOLD: u32 = 80;

#[derive(Debug, Default, Clone)]
struct BatteryNotifyState {
    low: [bool; BAT_LOW_NOTIFY_THRESHOLDS.len()],
    high: bool,
    not_charging: bool,
}

static BAT_NOTIFY_STATE: LazyLock<RwLock<BatteryNotifyState>> = LazyLock::new(|| RwLock::new(BatteryNotifyState::default()));

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct Battery {
    pub state: BatteryState,
    pub percent: u32,
    pub time: String,
}

impl_monitored!(Battery, battery);
impl_into_snapshot_event!(Battery);
impl_polled!(Battery, battery);

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
    #[instrument]
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
#[instrument]
pub async fn notify(update: MonitoredUpdate<Battery>) -> Result<(), DaemonError> {
    fn do_notification(battery: &Battery) -> Result<(), DaemonError> {
        command::run(
            "dunstify",
            &[
                "-u",
                "-normal",
                "-t",
                get_config().notification_timeout.to_string().as_str(),
                "-i",
                battery.get_icon().as_str(),
                "-r",
                NOTIFICATION_ID.to_string().as_str(),
                "-h",
                format!("int:value:{}", battery.percent).as_str(),
                "Battery: ",
            ],
        )?;

        Ok(())
    }

    // Only perform checks if the update changed something
    if update.old != Valid(update.clone().new) {
        // The state changed in this update
        if update.old.is_unavailable_or(|old| old.state != update.new.state) {
            // Mark all notifications as non-completed
            *(BAT_NOTIFY_STATE.write().await) = BatteryNotifyState::default();
        }

        // Check to see if any of the desired threhsolds have been reached for the first time
        let notify_state = BAT_NOTIFY_STATE.read().await.clone();
        match update.new.state {
            BatteryState::Charging | BatteryState::FullyCharged => {
                // Check if the high threshold has just been reached for the first time
                if update.new.percent >= BAT_HIGH_NOTIFY_THRESHOLD && !notify_state.high {
                    // Mark high threshold as complete
                    BAT_NOTIFY_STATE.write().await.high = true;

                    // Perform the notification
                    do_notification(&update.new)?;
                }
            }
            BatteryState::Discharging => {
                // Check each of the thresholds to see if it was just reached for the first time
                for (i, &threshold) in BAT_LOW_NOTIFY_THRESHOLDS.iter().enumerate() {
                    // The threshold was crossed and hasn't been crossed before
                    if update.new.percent <= threshold && !notify_state.low[i] {
                        // Mark this threshold as complete
                        BAT_NOTIFY_STATE.write().await.low[i] = true;

                        // Perform the notification
                        do_notification(&update.new)?;
                    }
                }
            }
            BatteryState::NotCharging => {
                // If this hasn't caused a notification already
                if !notify_state.not_charging {
                    // Mark not charging notification as complete
                    BAT_NOTIFY_STATE.write().await.not_charging = true;

                    // Perform the notification
                    do_notification(&update.new)?;
                }
            }
        }
    }

    Ok(())
}

/// # Errors
/// Returns an error if the requested value could not be parsed
#[instrument]
pub async fn evaluate_item(item: DaemonItem, battery_item: &BatteryItem) -> Result<DaemonReply, DaemonError> {
    Ok(
        // Get value (use latest() since this value changes without bar_daemon changing it)
        match battery_item {
            BatteryItem::State => DaemonReply::Value {
                item,
                value: BAT_STATE_STRINGS[default_source().read_state().await? as usize].to_string(),
            },
            BatteryItem::Percent => DaemonReply::Value {
                item,
                value: default_source().read_percent().await?.to_string(),
            },
            BatteryItem::Time => DaemonReply::Value {
                item,
                value: default_source().read_time().await?,
            },
            BatteryItem::Icon => DaemonReply::Value {
                item,
                value: latest().await?.get_icon(),
            },
            BatteryItem::All => DaemonReply::Tuples {
                item,
                tuples: latest().await?.to_tuples(),
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
