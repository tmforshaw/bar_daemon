use crate::{error::DaemonError, observed::Observed};
use tracing::{error, instrument};

use source::{BatterySource, default_source};
use value::notify;

pub use value::{Battery, BatteryGetCommands, BatteryItem, evaluate_item, match_get_commands};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Battery` can't be read due to `RwLock` Poisoning
/// Returns an error if the latest `Battery` can't be read due to parsing errors
#[instrument]
pub async fn latest() -> Result<Observed<Battery>, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
