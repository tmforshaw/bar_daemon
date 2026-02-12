use tracing::error;
use tracing_subscriber::EnvFilter;

/// # Panics
/// Panics if the log file can't be found or created
pub fn init_logging() {
    // Set panic hook
    std::panic::set_hook(Box::new(|info| {
        error!("Panic: {info}");
    }));

    // Filter the logs to the specified level (Use TRACE as default)
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .without_time()
        .with_ansi(true)
        .init();
}
