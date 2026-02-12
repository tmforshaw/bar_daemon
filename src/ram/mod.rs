use crate::{error::DaemonError, observed::Observed};
use source::{RamSource, default_source};

use tracing::error;
pub use value::{Ram, RamGetCommands, RamItem, evaluate_item, match_get_commands};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Ram` can't be read due to parsing errors
pub async fn latest() -> Result<Observed<Ram>, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
