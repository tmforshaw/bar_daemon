use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use tokio::sync::RwLock;

use crate::{
    battery::Battery,
    bluetooth::Bluetooth,
    brightness::Brightness,
    fan_profile::FanProfile,
    monitored::{Monitored, MonitoredUpdate, update_monitored},
    ram::Ram,
    volume::Volume,
};

#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub battery: Option<Battery>,
    pub bluetooth: Option<Bluetooth>,
    pub brightness: Option<Brightness>,
    pub fan_profile: Option<FanProfile>,
    pub ram: Option<Ram>,
    pub volume: Option<Volume>,
    pub timestamp: Option<Instant>,
}

static CURRENT_SNAPSHOT: LazyLock<Arc<RwLock<Snapshot>>> = LazyLock::new(|| Arc::new(RwLock::new(Snapshot::default())));

pub async fn current_snapshot() -> Snapshot {
    CURRENT_SNAPSHOT.read().await.clone()
}

pub async fn update_snapshot<M: Monitored>(new_value: M) -> MonitoredUpdate<M> {
    let mut snapshot = CURRENT_SNAPSHOT.write().await;
    update_monitored(&mut snapshot, new_value)
}
