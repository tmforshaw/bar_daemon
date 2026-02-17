use std::{any::type_name, time::Duration};

use Observed::{Recovering, Unavailable, Valid};
use tokio::time::Interval;
use tracing::{info, instrument, warn};

use crate::{
    error::DaemonError,
    monitored::{Monitored, MonitoredUpdate},
    snapshot::{IntoSnapshotEvent, current_snapshot, update_snapshot},
    tuples::ToTuples,
};

const READ_ATTEMPTS: u32 = 10;
const READ_ATTEMPT_INTERVAL: Duration = Duration::from_micros(500);

/// # Errors
/// Error if `M::latest().await` returns an Err
#[instrument(skip(timer))]
async fn read_until_valid<M: Monitored + IntoSnapshotEvent>(
    timer: &mut Interval,
) -> Result<(MonitoredUpdate<M>, u32), DaemonError> {
    // Set the value as recovering in the snapshot
    let _update = update_snapshot::<M>(Recovering).await;

    let snapshot = current_snapshot().await;
    let mut current: Observed<M> = M::get(&snapshot);

    let mut attempts_num = 1;
    while current.is_unavailable() {
        // Only run READ_ATTEMPTS number of times
        if attempts_num == READ_ATTEMPTS {
            warn!("Did not read Valid within max number of attempts ({READ_ATTEMPTS}): {current:?}");
            break;
        }

        // Get the latest value of this type
        current = M::latest().await?;

        attempts_num += 1;

        // Wait for the timer to tick before progressing the loop
        timer.tick().await;
    }

    if current.is_valid() {
        Ok((update_snapshot(current).await, attempts_num))
    } else {
        Err(DaemonError::MonitoredReadAttemptFail(
            type_name::<M>().to_string(),
            attempts_num,
        ))
    }
}

/// # Documentation
/// Create a task which (asynchronously) keeps polling the latest value of this type, and updates the snapshot when it is Valid
#[instrument]
pub fn spawn_read_until_valid<M: Monitored + IntoSnapshotEvent>() {
    tokio::spawn(async {
        let mut timer = tokio::time::interval(READ_ATTEMPT_INTERVAL);

        match read_until_valid::<M>(&mut timer).await {
            Ok((update, attempts)) => info!("Read Until Available Returned: '{:?}' after {attempts} attempts", update.new),
            Err(e) => warn!("{e}"),
        }
    });
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Observed<T> {
    Valid(T),
    Unavailable,
    Recovering,
}

impl<T: ToTuples> Observed<T> {
    pub fn to_tuples(self) -> Vec<(String, String)> {
        match self {
            Valid(v) => v.to_tuples(),
            Unavailable | Recovering => {
                // Generate a fake tuple with "?" instead of real data
                let tuple_names = T::to_tuple_names();
                tuple_names.into_iter().map(|name| (name, String::from("?"))).collect()
            }
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for Observed<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Valid(v) => write!(f, "{v:?}"),
            Unavailable | Recovering => write!(f, "?"),
        }
    }
}

// Unwrap-style functions

impl<T> Observed<T> {
    /// # Panics
    /// Panics if `self` is `Observed::Unavailable`
    pub fn unwrap(self) -> T {
        match self {
            Valid(v) => v,
            Unavailable | Recovering => panic!("Called 'unwrap()' on 'Unavailable'"),
        }
    }

    /// # Panics
    /// Panics if `self` is `Unavailable`
    #[must_use]
    pub fn expect(self, msg: &str) -> T {
        match self {
            Valid(v) => v,
            Unavailable | Recovering => panic!("{msg}"),
        }
    }

    #[must_use]
    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Valid(v) => v,
            Unavailable | Recovering => default,
        }
    }

    #[must_use]
    pub fn unwrap_or_else<F: Fn() -> T>(self, f: F) -> T {
        match self {
            Valid(v) => v,
            Unavailable | Recovering => f(),
        }
    }

    #[must_use]
    pub const fn is_valid(&self) -> bool {
        matches!(self, Valid(_))
    }

    /// # Documentation
    /// This treats `Unavailable` and `Recovering` as the same
    #[must_use]
    pub const fn is_unavailable(&self) -> bool {
        matches!(self, Unavailable | Recovering)
    }

    #[must_use]
    pub const fn is_recovering(&self) -> bool {
        matches!(self, Recovering)
    }

    #[must_use]
    pub fn is_valid_or<F: Fn() -> bool>(self, f: F) -> bool {
        match self {
            Valid(_) => true,
            Unavailable | Recovering => f(),
        }
    }

    /// # Documentation
    /// This treats `Unavailable` and `Recovering` as the same
    #[must_use]
    pub fn is_unavailable_or<F: Fn(T) -> bool>(self, f: F) -> bool {
        match self {
            Valid(v) => f(v),
            Unavailable | Recovering => true,
        }
    }

    #[must_use]
    pub fn is_recovering_or<U: Fn() -> bool, V: Fn(T) -> bool>(self, valid_fn: V, unavailable_fn: U) -> bool {
        match self {
            Valid(v) => valid_fn(v),
            Unavailable => unavailable_fn(),
            Recovering => true,
        }
    }
}

impl<T: Default> Observed<T> {
    #[must_use]
    pub fn unwrap_or_default(self) -> T {
        match self {
            Valid(v) => v,
            Unavailable | Recovering => T::default(),
        }
    }
}

// Result-like functions

impl<T> Observed<T> {
    #[must_use]
    pub fn from_result<E: std::fmt::Display>(res: Result<T, E>) -> Self {
        match res {
            Ok(v) => Valid(v),
            Err(e) => {
                warn!("{e}");
                Unavailable
            }
        }
    }
}

impl<T, E: std::fmt::Display> From<Result<T, E>> for Observed<T> {
    fn from(value: Result<T, E>) -> Self {
        Self::from_result(value)
    }
}

// Map functions
impl<T> Observed<T> {
    #[must_use]
    pub fn map<F: Fn(T) -> U, U>(self, f: F) -> Observed<U> {
        match self {
            Valid(v) => Valid(f(v)),
            Unavailable => Unavailable,
            Recovering => Recovering,
        }
    }

    /// # Documentation
    /// This treats `Unavailable` and `Recovering` as the same
    #[must_use]
    pub fn map_unavailable<F: Fn() -> T>(self, f: F) -> T {
        match self {
            Valid(v) => v,
            Unavailable | Recovering => f(),
        }
    }

    #[must_use]
    pub fn map_recovering<F: Fn() -> T>(self, f: F) -> Self {
        match self {
            Valid(v) => Valid(v),
            Unavailable => Unavailable,
            Recovering => Valid(f()),
        }
    }

    /// # Documentation
    /// This treats `Unavailable` and `Recovering` as the same
    #[must_use]
    pub fn map_or<F: Fn(T) -> U, U>(self, default: U, f: F) -> U {
        match self {
            Valid(v) => f(v),
            Unavailable | Recovering => default,
        }
    }

    /// # Documentation
    /// This treats `Unavailable` and `Recovering` as the same
    #[must_use]
    pub fn map_or_else<F: Fn(T) -> U, D: Fn() -> U, U>(self, default: D, f: F) -> U {
        match self {
            Valid(v) => f(v),
            Unavailable | Recovering => default(),
        }
    }
}
