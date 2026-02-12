use std::{sync::Arc, time::Duration};

use tokio::sync::Notify;
use tracing::error;

use crate::{
    config::get_config,
    error::DaemonError,
    monitored::Monitored,
    observed::Observed,
    snapshot::{IntoSnapshotEvent, update_snapshot},
};

pub trait Polled: Monitored {
    // Get the latest information about this value
    fn poll() -> impl std::future::Future<Output = Result<Observed<Self>, DaemonError>> + Send;

    // TODO Can add seperate polling rates for each polled value
    #[must_use]
    fn interval() -> Duration {
        Duration::from_millis(get_config().polling_rate)
    }
}

/// # Documentation
/// Generate the `Impl` for `Polled` using the given `type_name` and `module_name`
#[macro_export]
macro_rules! impl_polled {
    ($type_name:ident, $module_name:ident) => {
        impl Polled for $type_name {
            async fn poll() -> Result<Observed<Self>, DaemonError> {
                $crate::$module_name::latest().await
            }
        }
    };
}

pub fn spawn_poller<P: Polled + IntoSnapshotEvent>(shutdown_notify: Arc<Notify>) {
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(P::interval());

        loop {
            tokio::select! {
                // For every tick of the timer
                _ = timer.tick() => {
                    // Match the polled value, and ask to update_snapshot (Will be broadcast as SnapshotEvent if there is a change)
                    match P::poll().await {
                        Ok(new_value) => {let _update= update_snapshot(new_value).await;}
                        Err(e) => error!("Polling Failed: {e}")
                    }
                }

                () = shutdown_notify.notified() => {
                    break;
                }
            }
        }
    });
}
