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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            notification_timeout: 1000,
            polling_rate: 2000,
        }
    }
}

static CONFIG: LazyLock<Config> = LazyLock::new(init_config);

// TODO Paths in config are relative to $HOME but I could make it possible to be absolute or relative
#[instrument]
fn init_config() -> Config {
    // Get the HOME directory for the user
    let home = get_home_dir();

    // Combine $HOME and CONFIG_PATH to create a full path to the config file
    get_config_from_file(format!("{home}/{CONFIG_PATH}"))
}

fn get_home_dir() -> String {
    // Get the $HOME directory for this user
    let home_os_str =
        std::env::var_os("HOME").unwrap_or_else(|| panic!("{}", DaemonError::PathCreateError(String::from("env $HOME"))));

    // Convert $HOME to &str
    home_os_str
        .to_str()
        .unwrap_or_else(|| {
            panic!(
                "{}",
                DaemonError::PathCreateError(String::from("env $HOME could not convert to &str",))
            )
        })
        .to_string()
}

fn get_config_from_file<S: AsRef<str>>(file_path: S) -> Config {
    let config_path = Path::new(file_path.as_ref());

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
    toml::from_str(config.as_str()).unwrap_or_else(|e| panic!("{e}"))
}

pub fn get_config() -> Config {
    CONFIG.clone()
}
