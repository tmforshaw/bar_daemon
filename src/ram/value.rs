use clap::Subcommand;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{
    ICON_END, ICON_EXT,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    impl_monitored,
    monitored::Monitored,
    snapshot::{Snapshot, current_snapshot},
};

use super::{RamSource, default_source, latest};

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

#[derive(Clone, Debug, Default, PartialEq, PartialOrd, Ord, Eq)]
pub struct Ram {
    pub total: u64,
    pub used: u64,
    pub percent: u32,
}

impl_monitored!(Ram, ram);

impl Ram {
    #[must_use]
    pub fn get_icon() -> String {
        format!("nvidia-ram{ICON_END}")
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `total` then returned as an `Ok()`
    /// Returns an error if the requested value could not be parsed
    #[must_use]
    #[instrument]
    pub fn to_tuples(&self) -> Vec<(String, String)> {
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
        vec![
            "total".to_string(),
            "used".to_string(),
            "percent".to_string(),
            "icon".to_string(),
        ]
        .into_iter()
        .zip(str_values)
        .collect::<Vec<_>>()
    }
}

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
                    Some(ram) => Ok(ram.total),
                    None => default_source().read_total().await,
                }?
                .to_string(),
            },
            RamItem::Used => DaemonReply::Value {
                item,
                value: match current_snapshot().await.ram {
                    Some(ram) => Ok(ram.used),
                    None => default_source().read_used().await,
                }?
                .to_string(),
            },
            RamItem::Percent => DaemonReply::Value {
                item,
                value: match current_snapshot().await.ram {
                    Some(ram) => Ok(ram.percent),
                    None => default_source().read_percent().await,
                }?
                .to_string(),
            },
            RamItem::Icon => DaemonReply::Value {
                item,
                value: Ram::get_icon(),
            },
            RamItem::All => DaemonReply::Tuples {
                item,
                tuples: current_snapshot().await.ram.unwrap_or(latest().await?).to_tuples(),
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
