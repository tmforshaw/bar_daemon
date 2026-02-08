use crate::error::DaemonError;

use source::{FAN_STATE_STRINGS, FanProfileSource, default_source};
use value::FanState;

pub use value::{
    FanProfile, FanProfileGetCommands, FanProfileItem, FanProfileSetCommands, FanProfileUpdateCommands, evaluate_item,
    match_get_commands, match_set_commands, match_update_commands, notify,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `FanProfile` can't be read due to `RwLock` Poisoning
/// Returns an error if the latest `FanProfile` can't be read due to parsing errors
pub async fn latest() -> Result<FanProfile, DaemonError> {
    source::latest().await
}
