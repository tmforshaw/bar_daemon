use crate::error::DaemonError;
use source::{RamSource, default_source};

use tracing::error;
pub use value::{Ram, RamGetCommands, RamItem, evaluate_item, match_get_commands};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Ram` can't be read due to parsing errors
pub async fn latest() -> Result<Ram, DaemonError> {
    let latest = source::latest().await;

    if let Err(e) = latest {
        error!("{e}");

        Err(e)
    } else {
        latest
    }
}
