use std::fs::OpenOptions;

use time::macros::format_description;
use tracing::error;
use tracing_subscriber::{EnvFilter, fmt::time::LocalTime};

use crate::{config::get_config, error::DaemonError};

/// # Panics
/// Panics if the log file can't be found or created
pub fn init_logging() {
    // Set panic hook
    std::panic::set_hook(Box::new(|info| {
        error!("Panic: {info}");
    }));

    // Filter the logs to the specified level (Use TRACE as default)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    // Access the log file and begin to add logs
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(get_config().log_file)
        .unwrap_or_else(|e| {
            eprintln!("{}", DaemonError::PathRwError(e.to_string()));
            panic!()
        });

    // Initialise the tracing subscriber with desired features, including file to log to (specified in config)
    tracing_subscriber::fmt()
        .with_writer(move || {
            log_file
                .try_clone()
                .unwrap_or_else(|e| panic!("{}", DaemonError::PathRwError(e.to_string())))
        })
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_level(true)
        // Custom Time format
        .with_timer(LocalTime::new(format_description!(
            "[day]-[month repr:short]-[year] [hour]:[minute];[second].[subsecond digits:9] [offset_hour]:[offset_minute]"
        )))
        .init();
}
