use crate::{error::DaemonError, snapshot::Snapshot};

pub struct MonitoredUpdate<M: Monitored> {
    old: Option<M>,
    new: M,
}

pub trait Monitored: Sized + Clone {
    fn get(snapshot: &Snapshot) -> Option<Self>;
    fn set(snapshot: &mut Snapshot, new: Self);
}

pub fn update_monitored<M: Monitored>(snapshot: &mut Snapshot, new: M) -> Result<MonitoredUpdate<M>, DaemonError> {
    // Get the old value from the snapshot, then replace with the new value
    let old = M::get(snapshot);
    M::set(snapshot, new.clone());

    Ok(MonitoredUpdate { old, new })
}

/// Generate the `Impl` for `Monitored` using the given `type_name` and `field_name`
#[macro_export]
macro_rules! impl_monitored {
    ($type_name:ident, $field_name:ident) => {
        impl Monitored for $type_name {
            fn get(snapshot: &Snapshot) -> Option<Self> {
                snapshot.$field_name.clone()
            }

            fn set(snapshot: &mut Snapshot, new: Self) {
                snapshot.$field_name = Some(new);
            }
        }
    };
}
