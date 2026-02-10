use thiserror::Error;
use tokio::sync::mpsc;

use crate::listener::ClientMessage;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Socket Error:\t\"{0}\"")]
    SocketError(#[from] tokio::io::Error),

    #[error("Postcard Serialize/Deserialize Error:\t\"{0}\"")]
    PostcardError(#[from] postcard::Error),

    #[error("Command '{name}' With Args '{args:?}' Could Not Run:\t\"{e}\"")]
    CommandError { name: String, args: Vec<String>, e: String },

    #[error("Bytes Could Not Convert To String:\t\"{0}\"")]
    IntegerFromByteString(#[from] std::string::FromUtf8Error),

    #[error("String Could Not Convert To Integer:\t\"{0}\"")]
    IntegerFromString(#[from] std::num::ParseIntError),

    #[error("String Could Not Convert To Bool:\t\"{0}\"")]
    BoolFromString(#[from] std::str::ParseBoolError),

    #[error("String Could Not Convert To Float:\t\"{0}\"")]
    StringToFloatError(#[from] std::num::ParseFloatError),

    #[error("String could not parse enough arguments:\t\"{0}\"")]
    ParseError(String),

    #[error("Serde JSON Serialization Failed:\t\"{0}\"")]
    JsonError(#[from] serde_json::Error),

    #[error("Mpsc Could Not Send ClientMessage:\t\"{0}\"")]
    MpscSendError(#[from] mpsc::error::SendError<ClientMessage>),

    #[error("Could not convert between int types:\t\"{0}\"")]
    IntError(#[from] std::num::TryFromIntError),

    #[error("RwLock couldn't be locked")]
    RwLockError,

    #[error("Could not convert usize to TupleName")]
    TupleNameError,

    #[error("Could not create path:\t\"{0}\"")]
    PathCreateError(String),

    #[error("Could not read/write to path:\t\"{0}\"")]
    PathRwError(String),

    #[error("Monitored value of type '{0}' in Snapshot was None")]
    MonitoredEmptyError(&'static str),
}
