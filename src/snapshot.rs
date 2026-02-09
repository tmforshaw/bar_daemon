use std::{
    sync::{Arc, LazyLock},
    time::Instant,
};

use tokio::sync::RwLock;
use tracing::instrument;

use crate::{
    battery::Battery,
    bluetooth::Bluetooth,
    brightness::Brightness,
    fan_profile::FanProfile,
    monitored::{Monitored, MonitoredUpdate, update_monitored},
    ram::Ram,
    volume::Volume,
};

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub battery: Option<Battery>,
    pub bluetooth: Option<Bluetooth>,
    pub brightness: Option<Brightness>,
    pub fan_profile: Option<FanProfile>,
    pub ram: Option<Ram>,
    pub volume: Option<Volume>,
    pub timestamp: Instant,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            battery: None,
            bluetooth: None,
            brightness: None,
            fan_profile: None,
            ram: None,
            volume: None,
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
pub async fn update_snapshot<M: Monitored>(new_value: M) -> MonitoredUpdate<M> {
    let mut snapshot = CURRENT_SNAPSHOT.write().await;
    update_monitored(&mut snapshot, new_value)
}
