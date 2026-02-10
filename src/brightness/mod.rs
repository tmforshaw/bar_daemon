use crate::error::DaemonError;
use source::{BrightnessSource, default_source};

pub use source::{KEYBOARD_ID, MONITOR_ID};
use tracing::error;
pub use value::{
    Brightness, BrightnessGetCommands, BrightnessItem, BrightnessSetCommands, BrightnessUpdateCommands, evaluate_item,
    match_get_commands, match_set_commands, match_update_commands, notify,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Brightness` can't be read due to parsing errors
pub async fn latest() -> Result<Brightness, DaemonError> {
    let latest = source::latest().await;

    if let Err(e) = latest {
        error!("{e}");

        Err(e)
    } else {
        latest
    }
}
