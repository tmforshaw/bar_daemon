use thiserror::Error;
use tokio::sync::mpsc;

use crate::listener::ClientMessage;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Socket Error:\n\t\"{0}\"")]
    SocketError(#[from] tokio::io::Error),

    #[error("Postcard Serialize/Deserialize Error:\n\t\"{0}\"")]
    PostcardError(#[from] postcard::Error),

    #[error("Command '{name}' With Args '{args:?}' Could Not Run:\n\t\"{e}\"")]
    CommandError { name: String, args: Vec<String>, e: String },

    #[error("Bytes Could Not Convert To String:\n\t\"{0}\"")]
    IntegerFromByteString(#[from] std::string::FromUtf8Error),

    #[error("String Could Not Convert To Integer:\n\t\"{0}\"")]
    IntegerFromString(#[from] std::num::ParseIntError),

    #[error("String Could Not Convert To Bool:\n\t\"{0}\"")]
    BoolFromString(#[from] std::str::ParseBoolError),

    #[error("String Could Not Convert To Float:\n\t\"{0}\"")]
    StringToFloatError(#[from] std::num::ParseFloatError),

    #[error("String could not parse enough arguments:\n\t\"{0}\"")]
    ParseError(String),

    #[error("Serde JSON Serialization Failed:\n\t\"{0}\"")]
    JsonError(#[from] serde_json::Error),

    #[error("Mpsc Could Not Send ClientMessage:\n\t\"{0}\"")]
    MpscSendError(#[from] mpsc::error::SendError<ClientMessage>),

    #[error("Could not convert between int types:\n\t\"{0}\"")]
    IntError(#[from] std::num::TryFromIntError),

    #[error("RwLock couldn't be locked")]
    RwLockError,

    #[error("Could not convert usize to TupleName")]
    TupleNameError,

    #[error("Could not create path:\n\t\"{0}\"")]
    PathCreateError(String),

    #[error("Could not read/write to path:\n\t\"{0}\"")]
    PathRwError(String),
}
