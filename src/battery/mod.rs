use crate::error::DaemonError;

use source::{BatterySource, default_source};
use tracing::{error, instrument};
pub use value::{Battery, BatteryGetCommands, BatteryItem, evaluate_item, match_get_commands, notify};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Battery` can't be read due to `RwLock` Poisoning
/// Returns an error if the latest `Battery` can't be read due to parsing errors
#[instrument]
pub async fn latest() -> Result<Battery, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
