use std::slice::Iter;

use clap::Subcommand;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    command,
    daemon::{DaemonItem, DaemonMessage, DaemonReply},
    error::DaemonError,
    ICON_END, ICON_EXT,
};

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

pub struct Ram;

impl Ram {
    fn get() -> Result<(u64, u64, u64), DaemonError> {
        // Trim and split the output, getting to the numerical values
        let output_split = Self::get_output_split()?;

        let total = Self::get_total_from_split(output_split.iter())?;
        let used = Self::get_used_from_split(output_split.iter())?;

        let percent = Self::get_percent_from_used_total(used, total);

        Ok((total, used, percent))
    }

    fn get_output_split() -> Result<Vec<String>, DaemonError> {
        // Parse the output into lines
        let output = command::run("free", &["-b"])?;
        let output_lines = output.lines();

        // Choose the second line, and split based on whitespace
        Ok(output_lines
            .clone()
            .nth(1)
            .ok_or_else(|| DaemonError::ParseError(output_lines.collect::<String>()))?
            .trim_start_matches("Mem:")
            .split_whitespace()
            .map(ToString::to_string)
            .collect::<Vec<_>>())
    }

    fn get_total_from_split(mut output_split: Iter<String>) -> Result<u64, DaemonError> {
        // Get the total bytes from the spllit, parsing into u64
        output_split
            .next()
            .ok_or_else(|| DaemonError::ParseError(output_split.join(" ")))?
            .trim()
            .parse::<u64>()
            .map_err(Into::into)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_total() -> Result<u64, DaemonError> {
        let output_split = Self::get_output_split()?;

        Self::get_total_from_split(output_split.iter())
    }

    fn get_used_from_split(mut output_split: Iter<String>) -> Result<u64, DaemonError> {
        // Get the used bytes from the spllit, parsing into u64
        output_split
            .nth(1)
            .ok_or_else(|| DaemonError::ParseError(output_split.join(" ")))?
            .trim()
            .parse::<u64>()
            .map_err(Into::into)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_used() -> Result<u64, DaemonError> {
        let output_split = Self::get_output_split()?;

        Self::get_used_from_split(output_split.iter())
    }

    fn get_percent_from_used_total(used: u64, total: u64) -> u64 {
        ((used as f64 * 100.) / total as f64) as u64
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    pub fn get_percent() -> Result<u64, DaemonError> {
        let (_, _, percent) = Self::get()?;

        Ok(percent)
    }

    #[must_use]
    pub fn get_icon() -> String {
        format!("nvidia-ram{ICON_END}")
    }

    /// # Errors
    /// Errors are turned into `String` and set as value of `total` then returned as an `Ok()`
    /// Returns an error if the requested value could not be parsed
    pub fn get_tuples() -> Result<Vec<(String, String)>, DaemonError> {
        let icon = Self::get_icon();

        let str_values = match Self::get() {
            Ok((total, used, percent)) => {
                vec![
                    total.to_string(),
                    used.to_string(),
                    percent.to_string(),
                    format!("{icon}{ICON_EXT}"),
                ]
            }
            Err(e) => {
                vec![e.to_string(), 0.to_string(), 0.to_string(), format!("{icon}{ICON_EXT}")]
            }
        };

        Ok(vec![
            "total".to_string(),
            "used".to_string(),
            "percent".to_string(),
            "icon".to_string(),
        ]
        .into_iter()
        .zip(str_values)
        .collect::<Vec<_>>())
    }

    /// # Errors
    /// Returns an error if the requested value could not be parsed
    pub fn parse_item(item: DaemonItem, ram_item: &RamItem) -> Result<DaemonReply, DaemonError> {
        Ok(
            // Get value
            match ram_item {
                RamItem::Total => DaemonReply::Value {
                    item,
                    value: Self::get_total()?.to_string(),
                },
                RamItem::Used => DaemonReply::Value {
                    item,
                    value: Self::get_used()?.to_string(),
                },
                RamItem::Percent => DaemonReply::Value {
                    item,
                    value: Self::get_percent()?.to_string(),
                },
                RamItem::Icon => DaemonReply::Value {
                    item,
                    value: Self::get_icon(),
                },
                RamItem::All => DaemonReply::Tuples {
                    item,
                    tuples: Self::get_tuples()?,
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
}
