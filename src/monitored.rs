use crate::{error::DaemonError, snapshot::Snapshot, volume::Volume};

struct MonitoredUpdate<M: Monitored> {
    old: Option<M::Value>,
    new: M::Value,
}

trait Monitored: Sized {
    type Value: Clone + PartialEq;

    fn get(snapshot: &Snapshot) -> Option<Self::Value>;
    fn set(snapshot: &mut Snapshot, new: Self::Value);
}

fn update_monitored<M: Monitored>(snapshot: &mut Snapshot, new: M::Value) -> Result<MonitoredUpdate<M>, DaemonError> {
    // Get the old value from the snapshot, then replace with the new value
    let old = M::get(snapshot);
    M::set(snapshot, new.clone());

    Ok(MonitoredUpdate { old, new })
}

/// Generate the `Impl` for `Monitored` using the given `type_name` and `field_name`
macro_rules! impl_monitored {
    ($type_name:ident, $field_name:ident) => {
        impl Monitored for $type_name {
            type Value = $type_name;

            fn get(snapshot: &Snapshot) -> Option<Self::Value> {
                snapshot.$field_name.clone()
            }

            fn set(snapshot: &mut Snapshot, new: Self::Value) {
                snapshot.$field_name = Some(new);
            }
        }
    };
}

impl_monitored!(Volume, volume);
