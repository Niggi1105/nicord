use crate::framing::Frameable;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;

/// Error that is returned to the Client as a Response::Error(ServerError)
#[derive(Serialize, Deserialize, Debug)]
pub enum ServerError {
    InternalServerError,
    PermissionDenied,
    ConnectionTimeout,
}

impl Frameable for ServerError {}

#[derive(Debug)]
pub enum FramingError {
    FromUtf8Error(std::string::FromUtf8Error),
    SerializationError(serde_json::Error),
    ParseIntError(core::num::ParseIntError),
    MaximumFrameSizeExceeded,
}

impl fmt::Display for FramingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "crittical Error while framing")
    }
}

impl From<serde_json::Error> for FramingError {
    fn from(err: serde_json::Error) -> Self {
        FramingError::SerializationError(err)
    }
}

impl From<std::string::FromUtf8Error> for FramingError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        FramingError::FromUtf8Error(err)
    }
}

impl From<core::num::ParseIntError> for FramingError {
    fn from(err: core::num::ParseIntError) -> Self {
        FramingError::ParseIntError(err)
    }
}

impl Error for FramingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            FramingError::FromUtf8Error(er) => Some(er),
            FramingError::MaximumFrameSizeExceeded => None,
            FramingError::ParseIntError(er) => Some(er),
            FramingError::SerializationError(er) => Some(er),
        }
    }
}