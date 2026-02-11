use crate::error::DaemonError;

use source::{FAN_STATE_STRINGS, FanProfileSource, default_source};
use tracing::error;
use value::FanState;

pub use value::{
    FanProfile, FanProfileGetCommands, FanProfileItem, FanProfileSetCommands, evaluate_item, match_get_commands,
    match_set_commands, notify,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `FanProfile` can't be read due to parsing errors
pub async fn latest() -> Result<FanProfile, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
