use crate::error::DaemonError;
use tracing::error;

use source::BluetoothSource;
use value::notify;

pub use source::default_source;
pub use value::{
    Bluetooth, BluetoothGetCommands, BluetoothItem, BluetoothSetCommands, evaluate_item, match_get_commands, match_set_commands,
};

mod source;
mod value;

/// # Errors
/// Returns an error if the latest `Bluetooth` can't be read due to parsing errors
pub async fn latest() -> Result<Bluetooth, DaemonError> {
    match source::latest().await {
        Ok(latest) => Ok(latest),
        Err(e) => {
            error!("{e}");
            Err(e)
        }
    }
}
