use std::str::Split;

use itertools::Itertools;
use tracing::instrument;

use crate::{
    brightness, command,
    error::DaemonError,
    snapshot::{current_snapshot, update_snapshot},
};

use super::Brightness;

pub const MONITOR_ID: &str = "nvidia_wmi_ec_backlight";
pub const KEYBOARD_ID: &str = "asus::kbd_backlight";

pub trait BrightnessSource {
    // Read from commands (Get latest values)
    fn read(&self) -> impl std::future::Future<Output = Result<Brightness, DaemonError>> + Send;
    fn read_monitor(&self) -> impl std::future::Future<Output = Result<u32, DaemonError>> + Send;
    fn read_keyboard(&self) -> impl std::future::Future<Output = Result<u32, DaemonError>> + Send;

    // Change values of source
    fn set_monitor(&self, percent_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send;
    fn set_keyboard(&self, percent_str: &str) -> impl std::future::Future<Output = Result<(), DaemonError>> + Send;
}

// -------------- Default Source ---------------

#[must_use]
pub fn default_source() -> impl BrightnessSource {
    BctlBrightness
}

pub async fn latest() -> Result<Brightness, DaemonError> {
    default_source().read().await
}

// ---------------- Bctl Source ----------------

#[derive(Debug)]
pub struct BctlBrightness;

impl BrightnessSource for BctlBrightness {
    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read(&self) -> Result<Brightness, DaemonError> {
        // Get the brightness via brightnessctl
        let monitor = read_bctl_device(MONITOR_ID)?;
        let keyboard = read_bctl_device(KEYBOARD_ID)?;

        // Update the snapshot
        let brightness = Brightness { monitor, keyboard };
        let _update = update_snapshot(brightness.clone()).await;

        Ok(brightness)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read_monitor(&self) -> Result<u32, DaemonError> {
        // Get the brightness via brightnessctl
        let monitor = read_bctl_device(MONITOR_ID)?;

        // Update the snapshot
        let brightness = current_snapshot()
            .await
            .brightness
            // .map_or_else(Monitored::couldnt_find_monitored, Ok)?;
            .unwrap_or_default();
        let _update = update_snapshot(Brightness { monitor, ..brightness }).await;

        Ok(monitor)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn read_keyboard(&self) -> Result<u32, DaemonError> {
        // Get the brightness via brightnessctl
        let keyboard = read_bctl_device(KEYBOARD_ID)?;

        // Update the snapshot
        let brightness = current_snapshot()
            .await
            .brightness
            // .map_or_else(Monitored::couldnt_find_monitored, Ok)?;
            .unwrap_or_default();
        let _update = update_snapshot(Brightness { keyboard, ..brightness }).await;

        Ok(keyboard)
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn set_monitor(&self, percent_str: &str) -> Result<(), DaemonError> {
        let prev_brightness = current_snapshot().await.brightness.unwrap_or(latest().await?);

        set_bctl_device(MONITOR_ID, percent_str).await?;

        let new_monitor = latest().await?.monitor;

        // Update snapshot
        let update = update_snapshot(Brightness {
            monitor: new_monitor,
            ..prev_brightness
        })
        .await;

        // Do a notification
        brightness::notify(update, MONITOR_ID).await?;

        Ok(())
    }

    /// # Errors
    /// Returns an error if the command cannot be spawned
    /// Returns an error if values in the output of the command cannot be parsed
    #[instrument]
    async fn set_keyboard(&self, percent_str: &str) -> Result<(), DaemonError> {
        let prev_brightness = current_snapshot().await.brightness.unwrap_or(latest().await?);

        set_bctl_device(KEYBOARD_ID, percent_str).await?;

        let new_keyboard = latest().await?.keyboard;

        // Update snapshot
        let update = update_snapshot(Brightness {
            keyboard: new_keyboard,
            ..prev_brightness
        })
        .await;

        // Do a notification
        brightness::notify(update, KEYBOARD_ID).await?;

        Ok(())
    }
}

fn get_bctl_output(device_id: &str) -> Result<String, DaemonError> {
    // Get brightness output of device
    command::run("brightnessctl", &["-m", "-d", device_id, "i"])
}

fn get_bctl_split(output: &str) -> Split<'_, char> {
    // Split the output by commas
    output.split(',')
}

#[instrument(skip(split))]
fn get_bctl_percentage_from_split(mut split: Split<'_, char>) -> Result<u32, DaemonError> {
    // Get the current and maximum brightness values
    let current_brightness = split.nth(2);
    let max_brightness = split.nth(1);

    // Parse the values into integers, then get the floating point percentage
    Ok(
        if let (Some(current_brightness), Some(max_brightness)) = (current_brightness, max_brightness) {
            let current_value = f64::from(current_brightness.parse::<u32>()?);
            let max_value = f64::from(max_brightness.parse::<u32>()?);

            ((current_value / max_value) * 100.) as u32
        } else {
            return Err(DaemonError::ParseError(split.join(" ")));
        },
    )
}

/// # Errors
/// Returns an error if the command cannot be spawned
/// Returns an error if values in the output of the command cannot be parsed
#[instrument]
fn read_bctl_device(device_id: &str) -> Result<u32, DaemonError> {
    let output = get_bctl_output(device_id)?;
    let output_split = get_bctl_split(&output);

    get_bctl_percentage_from_split(output_split)
}

#[instrument]
async fn set_bctl_device(device_id: &str, percent_str: &str) -> Result<(), DaemonError> {
    // Change the percentage based on the delta percentage
    let percent = if percent_str.starts_with('+') || percent_str.starts_with('-') {
        let current_brightness = current_snapshot().await.brightness.unwrap_or(latest().await?);

        let delta_percent = percent_str.parse::<f64>()?;

        // Get the current percentage of the device which is being modified
        let current_percent = f64::from(if device_id == MONITOR_ID {
            current_brightness.monitor
        } else {
            current_brightness.keyboard
        });

        // Depending on the first char, add or subtract the percentage
        (current_percent + delta_percent).clamp(0.0, 100.0)
    } else {
        percent_str.parse::<f64>()?
    };

    // Set the percentage
    command::run("brightnessctl", &["-d", device_id, "s", format!("{percent}%").as_str()])?;

    Ok(())
}
