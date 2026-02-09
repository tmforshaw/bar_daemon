use std::{fs, path::Path, sync::LazyLock};

use serde::Deserialize;
use tracing::instrument;

use crate::error::DaemonError;

const CONFIG_PATH: &str = ".config/bar_daemon/config.toml";
const DEFAULT_CONFIG_PATH: &str = "/etc/bar_daemon/config.toml";

/// # Documentation
/// The `Config` derived from the `config.toml` file
#[derive(Deserialize, Clone)]
pub struct Config {
    /// Timeout of notifications in milliseconds
    pub notification_timeout: u32,
    /// Polling rate for polled values in milliseconds
    pub polling_rate: u64,
    /// The location of the log file for this daemon
    pub log_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notification_timeout: 1000,
            polling_rate: 2000,
            log_file: String::from(".cache/bar_daemon/bar_daemon.log"),
        }
    }
}

static CONFIG: LazyLock<Config> = LazyLock::new(init_config);

// TODO This just panics right now since without CONFIG this daemon can't function
// TODO Paths in config are relative to $HOME but I could make it possible to be absolute or relative
#[instrument]
fn init_config() -> Config {
    // Get the $HOME directory for this user
    let home_os_str =
        std::env::var_os("HOME").unwrap_or_else(|| panic!("{}", DaemonError::PathCreateError(String::from("env $HOME"))));

    // Convert $HOME to &str
    let home = home_os_str.to_str().unwrap_or_else(|| {
        panic!(
            "{}",
            DaemonError::PathCreateError(String::from("env $HOME could not convert to &str",))
        )
    });

    // Combine $HOME and CONFIG_PATH to create a full path to the config file
    let config_path_str = format!("{home}/{CONFIG_PATH}");
    let config_path = Path::new(config_path_str.as_str());

    // If the config_path doesn't point to any file, copy it from /etc/
    if !config_path.exists() {
        // Create the config_path parent folders
        fs::create_dir_all(config_path.parent().unwrap_or_else(|| {
            panic!(
                "{}",
                DaemonError::PathCreateError(String::from("Could get parent of `config_path`"))
            )
        }))
        .unwrap_or_else(|e| panic!("{}", DaemonError::PathCreateError(e.to_string())));

        // Copy the default config from /etc/
        fs::copy(DEFAULT_CONFIG_PATH, config_path).unwrap_or_else(|e| panic!("{}", DaemonError::PathRwError(e.to_string())));
    }

    // Read the config file as a String (Converting Error to DaemonError::PathRwError)
    let config = fs::read_to_string(config_path)
        .map_err(|e| DaemonError::PathRwError(e.to_string()))
        .unwrap_or_else(|e| panic!("{e}"));

    // Convert the text in the config file to a Config struct using TOML
    let mut config: Config = toml::from_str(config.as_str()).unwrap_or_else(|e| panic!("{e}"));

    // Create the log file path from the config
    let log_path_str = format!("{home}/{}", config.log_file);
    let log_path = Path::new(log_path_str.as_str());

    // Create the log file parent directories if they don't exist
    if let Some(parent) = log_path.parent()
        && !parent.exists()
    {
        // Create the log path parent folders
        fs::create_dir_all(log_path.parent().unwrap_or_else(|| {
            panic!(
                "{}",
                DaemonError::PathCreateError(String::from("Could get parent of `log_path`"))
            )
        }))
        .unwrap_or_else(|e| panic!("{}", DaemonError::PathCreateError(e.to_string())));
    }

    // Replace the log_file in Config with the absolute path
    config.log_file = log_path_str;

    config
}

pub fn get_config() -> Config {
    CONFIG.clone()
}
