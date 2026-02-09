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
    // Get the HOME directory for the user
    let home = get_home_dir();

    // Combine $HOME and CONFIG_PATH to create a full path to the config file
    let mut config = get_config_from_file(format!("{home}/{CONFIG_PATH}"));

    // Initialise the log file, setting log_file in Config to absolute path of current log_file
    init_log_file(&mut config, home);

    config
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

fn init_log_file<S: AsRef<str>>(config: &mut Config, home: S) {
    // TODO Decide whether to add PID to log_file path

    // let log_path = {

    // Create the absolute log file path from the config
    let log_path_str = format!("{}/{}", home.as_ref(), config.log_file);
    let log_path = Path::new(log_path_str.as_str());

    //     // Get the stem and extension of this path
    //     let log_stem = log_path.file_stem().unwrap_or_else(|| {
    //         panic!(
    //             "{}",
    //             DaemonError::PathCreateError(String::from("Could not get file stem of log_path"))
    //         )
    //     });
    //     let log_ext = log_path.extension().unwrap_or_else(|| {
    //         panic!(
    //             "{}",
    //             DaemonError::PathCreateError(String::from("Could not get file extension of log_path"))
    //         )
    //     });

    //     // Add the PID to the end of the filename
    //     let mut new_log_filename = log_stem.to_os_string();
    //     new_log_filename.push(format!("_{}", std::process::id()));

    //     // Set this as the filename for this new path, and readd the extension
    //     let mut new_log_path = log_path.to_path_buf();
    //     new_log_path.set_file_name(new_log_filename);
    //     new_log_path.set_extension(log_ext);

    //     new_log_path
    // };

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

    // Get the string representation of the log_file path
    let log_path_str = log_path
        .to_str()
        .unwrap_or_else(|| {
            panic!(
                "{}",
                DaemonError::PathCreateError(String::from("Could not create log_path_str from log_path"))
            )
        })
        .to_string();

    // Replace the log_file in Config with the absolute path
    config.log_file = log_path_str;
}

pub fn get_config() -> Config {
    CONFIG.clone()
}
