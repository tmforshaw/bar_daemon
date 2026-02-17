#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::similar_names)]
#![allow(clippy::implicit_hasher)]

use crate::{cli::evaluate_cli, error::DaemonError, logging::init_logging};

// TODO Implement some kind of "last-known value" system for the 'set' functions (currently uses unwrap_or_default)
// TODO notify listeners when the read_until_valid function finds a new valid value
// TODO Read functions update snapshot so it ends up in a mini loop of spawning the read_until_valid task

pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod cli;
pub mod command;
pub mod config;
pub mod daemon;
pub mod error;
pub mod fan_profile;
pub mod json;
pub mod listener;
pub mod log_linear;
pub mod logging;
pub mod monitored;
pub mod observed;
pub mod polled;
pub mod ram;
pub mod shutdown;
pub mod snapshot;
pub mod tuples;
pub mod volume;

pub const ICON_END: &str = "-symbolic";
pub const ICON_EXT: &str = ""; // ".svg"

pub const NOTIFICATION_ID: u32 = 42069;

#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    // Start the logging process
    init_logging();

    // Evaluate cli commands
    evaluate_cli().await?;

    Ok(())
}
