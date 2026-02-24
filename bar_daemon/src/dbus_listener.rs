use futures_util::StreamExt;
use tokio::sync::mpsc::Sender;
use tracing::warn;
use zbus::MatchRule;
use zbus::{Connection, MessageStream};

pub fn spawn_upower_listener(tx: Sender<()>) {
    tokio::spawn(async move {
        // Store the Connection inside the task so it lives for the task's lifetime
        let conn = match Connection::system().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to connect to system bus: {e}");
                return;
            }
        };

        if let Err(e) = run_dbus_listener(tx, conn, "/org/freedesktop/UPower/devices/battery_BAT0").await {
            tracing::error!("UPower listener failed: {e}");
        }
    });
}

async fn run_dbus_listener<S: AsRef<str>>(tx: Sender<()>, conn: Connection, listen_folder: S) -> zbus::Result<()> {
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.DBus.Properties")?
        .member("PropertiesChanged")?
        .path_namespace(listen_folder.as_ref())?
        .build();

    let mut stream = MessageStream::for_match_rule(rule, &conn, None).await?;

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(_) => {
                if tx.send(()).await.is_err() {
                    break; // No more receivers, exit
                }
            }
            Err(e) => warn!("D-Bus receive error: {e}"),
        }
    }

    Ok(())
}

/// # Panics
/// Panics if `udev::MonitorBuilder::new()` fails
/// Panics if `match_subsystem()` fails for `MonitorBuilder`
/// Panics if `listen()` fails for `MonitorBuilder`
pub fn spawn_hwmon_listener(tx: Sender<()>) {
    tokio::task::spawn_blocking(move || {
        // Monitor the hwmon system
        let monitor = udev::MonitorBuilder::new()
            .unwrap_or_else(|e| panic!("{e}"))
            .match_subsystem("hwmon")
            .unwrap_or_else(|e| panic!("{e}"))
            .listen()
            .unwrap_or_else(|e| panic!("{e}"));

        for _event in monitor.iter() {
            // Any thermal / fan / profile change wakes your runner
            if tx.blocking_send(()).is_err() {
                break;
            }
        }
    });
}

use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;

/// # Panics
/// Panics if the proc file couldn't open
/// Panics if the `epoll` could not be created
/// Panics if the `epoll_ctl` could not be created
pub fn spawn_ram_listener(tx: Sender<()>) {
    std::thread::spawn(move || {
        let file = OpenOptions::new()
            .read(true)
            .write(true) // to set thresholds
            .open("/proc/pressure/memory")
            .unwrap_or_else(|e| panic!("failed to open /proc/pressure/memory:\t\"{e}\""));

        // Threshold example
        std::fs::write("/proc/pressure/memory", "some 100000 1000000\n").ok();

        let epfd = unsafe { libc::epoll_create1(0) };
        assert!(epfd >= 0, "epoll_create1 failed");

        let mut event = libc::epoll_event {
            events: libc::EPOLLPRI as u32,
            u64: 1,
        };

        let ret = unsafe { libc::epoll_ctl(epfd, libc::EPOLL_CTL_ADD, file.as_raw_fd(), &raw mut event) };
        assert!(ret == 0, "epoll_ctl failed");

        let mut events = [libc::epoll_event { events: 0, u64: 0 }; 4];

        // TODO
        #[allow(clippy::cast_possible_wrap)]
        loop {
            let n = unsafe { libc::epoll_wait(epfd, events.as_mut_ptr(), events.len() as i32, -1) };
            if n < 0 {
                continue;
            }

            if tx.blocking_send(()).is_err() {
                break; // channel closed
            }
        }
    });
}
