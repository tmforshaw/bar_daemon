use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use tokio::sync::{RwLock, broadcast};
use tracing::{info, instrument};

use crate::{
    battery::Battery,
    bluetooth::Bluetooth,
    brightness::Brightness,
    fan_profile::FanProfile,
    monitored::{Monitored, MonitoredUpdate, update_monitored},
    observed::{
        Observed::{self, Unavailable},
        spawn_read_until_valid,
    },
    ram::Ram,
    volume::Volume,
};

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub battery: Observed<Battery>,
    pub bluetooth: Observed<Bluetooth>,
    pub brightness: Observed<Brightness>,
    pub fan_profile: Observed<FanProfile>,
    pub ram: Observed<Ram>,
    pub volume: Observed<Volume>,
    pub timestamp: Instant,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            battery: Unavailable,
            bluetooth: Unavailable,
            brightness: Unavailable,
            fan_profile: Unavailable,
            ram: Unavailable,
            volume: Unavailable,
            timestamp: Instant::now(),
        }
    }
}

static CURRENT_SNAPSHOT: LazyLock<Arc<RwLock<Snapshot>>> = LazyLock::new(|| Arc::new(RwLock::new(Snapshot::default())));

#[must_use]
#[instrument]
pub async fn current_snapshot() -> Snapshot {
    CURRENT_SNAPSHOT.read().await.clone()
}

#[must_use]
#[instrument]
pub async fn update_snapshot<M: Monitored + IntoSnapshotEvent>(new_value: Observed<M>) -> MonitoredUpdate<M> {
    let update = {
        let mut snapshot = CURRENT_SNAPSHOT.write().await;
        update_monitored(&mut snapshot, new_value)
    };

    // Spawn task to run read_until_valid if the new value is Unavailable (If the value isn't recovering)
    if update.new == Unavailable && !update.old.is_recovering() {
        info!(
            "Spawning task to read {} until it is Valid: {update:?}",
            std::any::type_name::<M>()
        );

        spawn_read_until_valid::<M>();
    }

    update
}

static SNAPSHOT_EVENTS: LazyLock<broadcast::Sender<SnapshotEvent>> = LazyLock::new(|| {
    let (tx, _) = broadcast::channel(64);
    tx
});

pub fn subscribe_snapshot() -> broadcast::Receiver<SnapshotEvent> {
    SNAPSHOT_EVENTS.subscribe()
}

pub fn broadcast_snapshot_event(event: SnapshotEvent) {
    // Drop since only returns Err when there are no receivers
    let _ = SNAPSHOT_EVENTS.send(event);
}

#[derive(Clone, Debug)]
pub enum SnapshotEvent {
    Battery(MonitoredUpdate<Battery>),
    Bluetooth(MonitoredUpdate<Bluetooth>),
    Brightness(MonitoredUpdate<Brightness>),
    FanProfile(MonitoredUpdate<FanProfile>),
    Ram(MonitoredUpdate<Ram>),
    Volume(MonitoredUpdate<Volume>),
}

pub trait IntoSnapshotEvent: Monitored {
    fn into_event(update: MonitoredUpdate<Self>) -> SnapshotEvent;
}

/// # Documentation
/// Generate the `Impl` for `IntoSnapshotEvent` using the given `type_name`
/// Generate the `Impl` `From<MonitoredUpdate<$type_name>> for SnapshotEvent` using the given `type_name`
#[macro_export]
macro_rules! impl_into_snapshot_event {
    ($type_name:ident) => {
        impl IntoSnapshotEvent for $type_name {
            fn into_event(update: MonitoredUpdate<Self>) -> SnapshotEvent {
                SnapshotEvent::$type_name(update)
            }
        }

        impl From<MonitoredUpdate<$type_name>> for SnapshotEvent {
            fn from(update: MonitoredUpdate<$type_name>) -> Self {
                IntoSnapshotEvent::into_event(update)
            }
        }
    };
}
