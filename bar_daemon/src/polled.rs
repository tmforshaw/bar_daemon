use std::{sync::Arc, time::Duration};

use tracing::error;

use crate::{
    config::get_config,
    monitored::Monitored,
    notification::Notify,
    snapshot::{IntoSnapshotEvent, update_snapshot},
    trigger::{IntervalTrigger, Trigger},
};

pub trait Polled: Monitored {
    // TODO Can add seperate polling rates for each polled value
    #[must_use]
    fn interval() -> Duration {
        Duration::from_millis(get_config().polling_rate)
    }
}

pub fn spawn_poller<P: Polled + IntoSnapshotEvent + Notify<P>>(shutdown_notify: Arc<tokio::sync::Notify>) {
    // Create an IntervalTrigger for this poller
    let trigger = IntervalTrigger::new(P::interval());

    // Spawn the polling loop, triggered by a timer
    spawn_poll_on_trigger::<P, _>(trigger, shutdown_notify);
}

pub fn spawn_poll_or_listen<P: Polled + IntoSnapshotEvent + Notify<P>>(shutdown_notify: Arc<tokio::sync::Notify>) {
    // Create an IntervalTrigger for this poller
    let trigger = IntervalTrigger::new(P::interval());

    // Spawn the polling loop, triggered by a timer
    spawn_poll_on_trigger::<P, _>(trigger, shutdown_notify);
}

pub fn spawn_poll_on_trigger<M: Monitored + IntoSnapshotEvent + Notify<M>, T: Trigger + 'static>(
    mut trigger: T,
    shutdown_notify: Arc<tokio::sync::Notify>,
) {
    tokio::spawn(async move {
        loop {
            tokio::select! {
                // For every event from Trigger
                () = trigger.wait() => {
                    // TODO update_snapshot() probably unneeded
                    // Match the latest value, and ask to update_snapshot (Will be broadcast as SnapshotEvent if there is a change)
                    match M::latest().await {
                        Ok(new_value) => {let _update= update_snapshot(new_value).await;}
                        Err(e) => error!("Poll on Trigger Failed: {e}")
                    }
                }

                () = shutdown_notify.notified() => {
                    break;
                }
            }
        }
    });
}
