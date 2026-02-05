use crate::error::DaemonError;
use source::{RamSource, default_source};

pub use value::{Ram, RamGetCommands, RamItem, evaluate_item, match_get_commands};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Ram` can't be read due to `RwLock` Poisoning
/// Returns an error if the latest `Ram` can't be read due to parsing errors
pub fn latest() -> Result<Ram, DaemonError> {
    source::latest()
}
