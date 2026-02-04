use crate::error::DaemonError;

use source::default_source;
pub use value::{Battery, BatteryGetCommands, BatteryItem, evaluate_item, match_get_commands, notify};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Battery` can't be read due to `RwLock` Poisoning
/// Returns an error if the latest `Battery` can't be read due to parsing errors
pub fn latest() -> Result<Battery, DaemonError> {
    source::latest()
}
