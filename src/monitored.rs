use tracing::{debug, instrument};

use crate::snapshot::{IntoSnapshotEvent, Snapshot, broadcast_snapshot_event};

#[derive(Clone, Debug)]
pub struct MonitoredUpdate<M: Monitored> {
    pub old: Option<M>,
    pub new: M,
}

pub trait Monitored: std::fmt::Debug + Sized + Clone + Send + PartialEq + Eq + 'static {
    fn get(snapshot: &Snapshot) -> Option<Self>;
    fn set(snapshot: &mut Snapshot, new: Self);
}

/// # Documentation
/// Updates the `Monitored` value within the `Snapshot` and returns a `MonitoredUpdate`
#[must_use]
#[instrument(skip(snapshot, new))]
pub fn update_monitored<M: Monitored + IntoSnapshotEvent>(snapshot: &mut Snapshot, new: M) -> MonitoredUpdate<M> {
    // Get the old value from the snapshot, then replace with the new value
    let old = M::get(snapshot);
    M::set(snapshot, new.clone());

    let update = MonitoredUpdate { old, new };

    // Check that the update changed the data
    if update.old != Some(update.new.clone()) {
        // Log the update
        debug!("Monitored Value Updated: {update:?}");

        // Broadcast update
        broadcast_snapshot_event(M::into_event(update.clone()));
    }

    update
}

/// # Documentation
/// Generate the `Impl` for `Monitored` using the given `type_name` and `field_name`
#[macro_export]
macro_rules! impl_monitored {
    ($type_name:ident, $field_name:ident) => {
        impl Monitored for $type_name {
            fn get(snapshot: &Snapshot) -> Option<Self> {
                // Get the given field
                snapshot.$field_name.clone()
            }

            fn set(snapshot: &mut Snapshot, new: Self) {
                // Set the given field to the new value
                snapshot.$field_name = Some(new);

                // Show that this snapshot happened now
                snapshot.timestamp = std::time::Instant::now();
            }
        }
    };
}
