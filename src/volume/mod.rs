use crate::error::DaemonError;
use source::{VolumeSource, default_source};

use tracing::error;
pub use value::{
    Volume, VolumeGetCommands, VolumeItem, VolumeSetCommands, VolumeUpdateCommands, evaluate_item, match_get_commands,
    match_set_commands, match_update_commands, notify,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Volume` can't be read due to parsing errors
pub async fn latest() -> Result<Volume, DaemonError> {
    let latest = source::latest().await;

    if let Err(e) = latest {
        error!("{e}");

        Err(e)
    } else {
        latest
    }
}
