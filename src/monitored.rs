use tracing::{info, instrument};

use crate::snapshot::Snapshot;

// TODO
#[allow(dead_code)]
#[derive(Debug)]
pub struct MonitoredUpdate<M: Monitored> {
    old: Option<M>,
    new: M,
}

pub trait Monitored: std::fmt::Debug + Sized + Clone + Send + PartialEq + Eq + 'static {
    fn get(snapshot: &Snapshot) -> Option<Self>;
    fn set(snapshot: &mut Snapshot, new: Self);
    // fn notify(update: MonitoredUpdate<Self>);
}

/// # Documentation
/// Updates the `Monitored` value within the `Snapshot` and returns a `MonitoredUpdate`
#[must_use]
#[instrument(skip(snapshot, new))]
pub fn update_monitored<M: Monitored>(snapshot: &mut Snapshot, new: M) -> MonitoredUpdate<M> {
    // Get the old value from the snapshot, then replace with the new value
    let old = M::get(snapshot);
    M::set(snapshot, new.clone());

    let update = MonitoredUpdate { old, new };

    // Check that the update changed the data
    if update.old != Some(update.new.clone()) {
        // Log the update
        info!("Monitored Value Updated: {update:?}");

        // Notify change
    }

    update
}

/// # Documentation
/// Generate the `Impl` for `Monitored` using the given `type_name` and `field_name`
/// For notifications to work the `file_name` which the `notify()` function is in must be the same as `field_name` for this `type_name`
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

            // fn notify(update: $crate::monitored::MonitoredUpdate<Self>) {
            //     $crate::$field_name::notify();
            // }
        }
    };
}
