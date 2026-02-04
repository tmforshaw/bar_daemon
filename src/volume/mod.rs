use crate::error::DaemonError;
use source::VolumeSource;

use source::default_source;
pub use value::{
    Volume, VolumeGetCommands, VolumeItem, VolumeSetCommands, VolumeUpdateCommands, evaluate_item, match_get_commands,
    match_set_commands, match_update_commands, notify,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Volume` can't be read due to `RwLock` Poisoning
/// Returns an error if the latest `Volume` can't be read due to parsing errors
pub fn latest() -> Result<Volume, DaemonError> {
    source::latest()
}
