use crate::{error::DaemonError, observed::Observed};
use tracing::error;

use source::{BrightnessSource, default_source};
use value::notify;

pub use source::{KEYBOARD_ID, MONITOR_ID};
pub use value::{
    Brightness, BrightnessGetCommands, BrightnessItem, BrightnessSetCommands, evaluate_item, match_get_commands,
    match_set_commands,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Brightness` can't be read due to parsing errors
pub async fn latest() -> Result<Observed<Brightness>, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
