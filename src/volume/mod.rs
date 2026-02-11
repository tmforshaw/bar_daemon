use crate::error::DaemonError;
use source::{VolumeSource, default_source};
use value::notify;

use tracing::error;

pub use value::{
    Volume, VolumeGetCommands, VolumeItem, VolumeSetCommands, evaluate_item, match_get_commands, match_set_commands,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Volume` can't be read due to parsing errors
pub async fn latest() -> Result<Volume, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
