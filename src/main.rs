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

// TODO Re-add the soft errors back in so that errors can easily be found

use crate::{cli::match_cli, error::DaemonError};

pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod cli;
pub mod command;
pub mod daemon;
pub mod error;
pub mod fan_profile;
pub mod json;
pub mod listener;
pub mod log_linear;
pub mod ram;
pub mod shutdown;
pub mod snapshot;
pub mod tuples;
pub mod volume;

pub const ICON_END: &str = "-symbolic";
pub const ICON_EXT: &str = ""; // ".svg"

pub const NOTIFICATION_ID: u32 = 42069;
pub const NOTIFICATION_TIMEOUT: u32 = 1000;

pub const POLLING_RATE: u64 = 2000;

#[tokio::main]
async fn main() -> Result<(), DaemonError> {
    match_cli().await?;

    Ok(())
}
