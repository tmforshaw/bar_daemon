use tracing::{debug, instrument};

use crate::{
    changed::{Changed, ChangedConstructor},
    error::DaemonError,
    observed::Observed::{self, Recovering, Unavailable, Valid},
    snapshot::{IntoSnapshotEvent, Snapshot, broadcast_snapshot_event},
};

pub trait Monitored: std::fmt::Debug + Sized + Clone + Send + PartialEq + Eq + 'static {
    fn get(snapshot: &Snapshot) -> Observed<Self>;
    fn set(snapshot: &mut Snapshot, new: Observed<Self>);

    fn latest() -> impl std::future::Future<Output = Result<Observed<Self>, DaemonError>> + Send;
}

/// # Documentation
/// Updates the `Monitored` value within the `Snapshot` and returns a `MonitoredUpdate` with the change
#[must_use]
#[instrument(skip(snapshot, new))]
pub fn update_monitored<M: Monitored + IntoSnapshotEvent>(snapshot: &mut Snapshot, new: Observed<M>) -> MonitoredUpdate<M> {
    // Get the old value from the snapshot
    let old = M::get(snapshot);

    let update = MonitoredUpdate { old, new: new.clone() };

    // Check that the update changed the data, but don't allow updating to Unavailable from Recovering
    if update.old != update.new && !(update.old == Recovering && update.new == Unavailable) {
        // Replace monitored value in the snapshot
        M::set(snapshot, new);

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
                match $crate::$module_name::source::default_source().read().await {
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

#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq)]
pub struct MonitoredUpdate<M: Monitored> {
    pub old: Observed<M>,
    pub new: Observed<M>,
}

impl<M> MonitoredUpdate<M>
where
    M: Changed + Monitored,
    M::ChangedType: ChangedConstructor,
{
    pub fn changed(&self) -> M::ChangedType {
        if let Self {
            old: Valid(old),
            new: Valid(new),
        } = self
        {
            old.changed(new)
        } else if self.old == self.new {
            M::ChangedType::all_false()
        } else {
            M::ChangedType::all_true()
        }
    }
}
