use crate::{
    error::DaemonError,
    monitored::{Monitored, MonitoredUpdate},
    snapshot::IntoSnapshotEvent,
};

pub trait Notify<M: Monitored + IntoSnapshotEvent> {
    fn notify(update: MonitoredUpdate<M>) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send {
        // Temporary binding to show that it is used in other implementations
        let _ = update;

        async { Ok(()) }
    }
}
