use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tracing::{error, instrument};

use crate::{
    ICON_END, ICON_EXT,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_monitored,
    monitored::{Monitored, MonitoredUpdate},
    notification::Notify,
    observed::Observed::{self, Recovering, Unavailable, Valid},
    polled::Polled,
    snapshot::{IntoSnapshotEvent, Snapshot, SnapshotEvent, current_snapshot},
    tuples::ToTuples,
};

use super::source::RamSource;

#[derive(Subcommand)]
pub enum RamGetCommands {
    #[command(alias = "tot", alias = "t")]
    Total,
    #[command(alias = "u")]
    Used,
    #[command(alias = "per", alias = "p")]
    Percent,
    #[command(alias = "i")]
    Icon,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RamItem {
    Total,
    Used,
    Percent,
    Icon,
    All,
}

#[derive(
    Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq, bar_daemon_derive::Polled, bar_daemon_derive::IntoSnapshotEvent,
)]
pub struct Ram {
    pub total: u64,
    pub used: u64,
    pub percent: u32,
}

impl_monitored!(Ram, ram, ram);

impl Ram {
    #[must_use]
    pub fn get_icon() -> String {
        format!("nvidia-ram{ICON_END}")
    }
}

impl ToTuples for Ram {
    fn to_tuple_names() -> Vec<String> {
        vec![
            "total".to_string(),
            "used".to_string(),
            "percent".to_string(),
            "icon".to_string(),
        ]
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `total` then returned as an `Ok()`
    /// Returns an error if the requested value could not be parsed
    #[instrument]
    fn to_tuples(&self) -> Vec<(String, String)> {
        let icon = Self::get_icon();

        // Create list of values for tuples
        let str_values = {
            let Self { total, used, percent } = self;

            vec![
                total.to_string(),
                used.to_string(),
                percent.to_string(),
                format!("{icon}{ICON_EXT}"),
            ]
        };

        // Zip list of values with list of value names
        Self::to_tuple_names().into_iter().zip(str_values).collect::<Vec<_>>()
    }
}

// Implement default trait Impl for Notify
impl Notify<Self> for Ram {}

/// # Errors
/// Returns an error if the requested value could not be evaluated
#[instrument]
pub async fn evaluate_item(item: DaemonItem, ram_item: &RamItem) -> Result<DaemonReply, DaemonError> {
    Ok(
        // Get value
        match ram_item {
            RamItem::Total => DaemonReply::Value {
                item,
                value: match current_snapshot().await.ram {
                    Valid(ram) => ram.total.to_string(),
                    Unavailable | Recovering => Ram::latest().await?.map(|ram| ram.total).to_string(),
                },
            },
            RamItem::Used => DaemonReply::Value {
                item,
                value: match current_snapshot().await.ram {
                    Valid(ram) => ram.used.to_string(),
                    Unavailable | Recovering => Ram::latest().await?.map(|ram| ram.used).to_string(),
                },
            },
            RamItem::Percent => DaemonReply::Value {
                item,
                value: match current_snapshot().await.ram {
                    Valid(ram) => ram.percent.to_string(),
                    Unavailable | Recovering => Ram::latest().await?.map(|ram| ram.percent).to_string(),
                },
            },
            RamItem::Icon => DaemonReply::Value {
                item,
                value: Ram::get_icon(),
            },
            RamItem::All => DaemonReply::Tuples {
                item,
                tuples: Ram::latest().await?.to_tuples(),
            },
        },
    )
}

#[must_use]
pub const fn match_get_commands(commands: &Option<RamGetCommands>) -> DaemonMessage {
    DaemonMessage::Get {
        item: match commands {
            Some(commands) => match commands {
                RamGetCommands::Total => DaemonItem::Ram(RamItem::Total),
                RamGetCommands::Used => DaemonItem::Ram(RamItem::Used),
                RamGetCommands::Percent => DaemonItem::Ram(RamItem::Percent),
                RamGetCommands::Icon => DaemonItem::Ram(RamItem::Icon),
            },
            None => DaemonItem::Ram(RamItem::All),
        },
    }
}
