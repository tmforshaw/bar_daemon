use Observed::{Unavailable, Valid};
use tracing::warn;

use crate::{monitored::Monitored, tuples::ToTuples};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Observed<T> {
    Valid(T),
    Unavailable,
}

impl<T: ToTuples> Observed<T> {
    pub fn to_tuples(self) -> Vec<(String, String)> {
        match self {
            Valid(v) => v.to_tuples(),
            Unavailable => {
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
            Observed::Valid(v) => write!(f, "{v:?}"),
            Observed::Unavailable => write!(f, "?"),
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
            Unavailable => panic!("Called 'unwrap()' on 'Unavailable'"),
        }
    }

    /// # Panics
    /// Panics if `self` is `Unavailable`
    pub fn expect(self, msg: &str) -> T {
        match self {
            Valid(v) => v,
            Unavailable => panic!("{msg}"),
        }
    }

    pub fn unwrap_or(self, default: T) -> T {
        match self {
            Valid(v) => v,
            Unavailable => default,
        }
    }

    pub fn unwrap_or_else<F: Fn() -> T>(self, f: F) -> T {
        match self {
            Valid(v) => v,
            Unavailable => f(),
        }
    }

    pub fn is_valid(self) -> bool {
        matches!(self, Self::Valid(_))
    }

    pub fn is_unavailable(self) -> bool {
        matches!(self, Self::Unavailable)
    }

    pub fn is_valid_or<F: Fn() -> bool>(self, f: F) -> bool {
        match self {
            Valid(_) => true,
            Unavailable => f(),
        }
    }

    pub fn is_unavailable_or<F: Fn(T) -> bool>(self, f: F) -> bool {
        match self {
            Valid(v) => f(v),
            Unavailable => true,
        }
    }
}

impl<T: Default> Observed<T> {
    pub fn unwrap_or_default(self) -> T {
        match self {
            Valid(v) => v,
            Unavailable => T::default(),
        }
    }
}

// Result-like functions

impl<T> Observed<T> {
    pub fn from_result<E: std::fmt::Display>(res: Result<T, E>) -> Self {
        match res {
            Ok(v) => Observed::Valid(v),
            Err(e) => {
                warn!("{e}");
                Observed::Unavailable
            }
        }
    }
}

impl<T, E: std::fmt::Display> From<Result<T, E>> for Observed<T> {
    fn from(value: Result<T, E>) -> Self {
        Observed::from_result(value)
    }
}

// Map functions
impl<T> Observed<T> {
    pub fn map<F: Fn(T) -> U, U>(self, f: F) -> Observed<U> {
        match self {
            Valid(v) => Valid(f(v)),
            Unavailable => Unavailable,
        }
    }

    pub fn map_or<F: Fn(T) -> U, U>(self, default: U, f: F) -> U {
        match self {
            Valid(v) => f(v),
            Unavailable => default,
        }
    }

    pub fn map_or_else<F: Fn(T) -> U, D: Fn() -> U, U>(self, default: D, f: F) -> U {
        match self {
            Valid(v) => f(v),
            Unavailable => default(),
        }
    }
}
