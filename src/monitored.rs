use std::{any::type_name, time::Duration};

use tokio::time::Interval;
use tracing::{debug, info, instrument, warn};

use crate::{
    error::DaemonError,
    observed::Observed::{self},
    snapshot::{IntoSnapshotEvent, Snapshot, broadcast_snapshot_event, current_snapshot, update_snapshot},
};

#[derive(Clone, Debug)]
pub struct MonitoredUpdate<M: Monitored> {
    pub old: Observed<M>,
    pub new: Observed<M>,
}

pub trait Monitored: std::fmt::Debug + Sized + Clone + Send + PartialEq + Eq + 'static {
    fn get(snapshot: &Snapshot) -> Observed<Self>;
    fn set(snapshot: &mut Snapshot, new: Observed<Self>);

    fn latest() -> impl std::future::Future<Output = Result<Observed<Self>, DaemonError>> + Send;
}

/// # Documentation
/// Updates the `Monitored` value within the `Snapshot` and returns a `MonitoredUpdate`
#[must_use]
#[instrument(skip(snapshot, new))]
pub fn update_monitored<M: Monitored + IntoSnapshotEvent>(snapshot: &mut Snapshot, new: Observed<M>) -> MonitoredUpdate<M> {
    // Get the old value from the snapshot, then replace with the new value
    let old = M::get(snapshot);
    M::set(snapshot, new.clone());

    let update = MonitoredUpdate { old, new };

    // Check that the update changed the data
    if update.old != update.new {
        // Log the update
        debug!("Monitored Value Updated: {update:?}");

        // Broadcast update
        broadcast_snapshot_event(M::into_event(update.clone()));
    }

    update
}

/// # Documentation
/// Generate the `Impl` for `Monitored` using the given `type_name`, `field_name`, and `module_name`
#[macro_export]
macro_rules! impl_monitored {
    ($type_name:ident, $field_name:ident, $module_name:ident) => {
        impl Monitored for $type_name {
            fn get(snapshot: &Snapshot) -> Observed<Self> {
                // Get the given field
                snapshot.$field_name.clone()
            }

            fn set(snapshot: &mut Snapshot, new: Observed<Self>) {
                // Set the given field to the new value
                snapshot.$field_name = new;

                // Show that this snapshot happened now
                snapshot.timestamp = std::time::Instant::now();
            }

            /// # Errors
            /// Returns an error if the latest value of `Monitored` can't be read due to parsing errors
            async fn latest() -> Result<Observed<Self>, DaemonError> {
                match $crate::$module_name::source::latest().await {
                    Ok(latest) => Ok(latest),
                    Err(e) => {
                        error!("{e}");
                        Err(e)
                    }
                }
            }
        }
    };
}

const READ_ATTEMPTS: u32 = 10;
const READ_ATTEMPT_INTERVAL: Duration = Duration::from_micros(500);

/// # Documentation
/// A function for asynchronously reading the value until it is available (Meant to be used in a `tokio::spawn`)
/// # Errors
/// Error if `M::latest().await` returns an Err
async fn read_until_available<M: Monitored + IntoSnapshotEvent>(
    timer: &mut Interval,
) -> Result<(MonitoredUpdate<M>, u32), DaemonError> {
    let snapshot = current_snapshot().await;
    let mut current: Observed<M> = M::get(&snapshot);

    let mut attempts_num = READ_ATTEMPTS;
    for i in 0..READ_ATTEMPTS {
        if current.is_unavailable() {
            current = M::latest().await?;
        } else {
            attempts_num = i + 1;
            break;
        }

        // Wait for the timer to tick before progressing the loop
        timer.tick().await;
    }

    if current.is_valid() {
        Ok((update_snapshot(current).await, attempts_num))
    } else {
        Err(DaemonError::MonitoredReadAttemptFail(
            type_name::<M>().to_string(),
            attempts_num,
        ))
    }
}

/// # Documentation
/// Create a task which (asynchronously) keeps getting the latest value of this type, and updates the snapshot when it is Valid
pub fn spawn_read_until_available<M: Monitored + IntoSnapshotEvent>() {
    tokio::spawn(async {
        let mut timer = tokio::time::interval(READ_ATTEMPT_INTERVAL);

        match read_until_available::<M>(&mut timer).await {
            Ok((update, attempts)) => info!("Read Until Available Returned: '{:?}' after {attempts} attempts", update.new),
            Err(e) => warn!("{e}"),
        }
    });
}
