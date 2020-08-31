use crate::Status;
use std::{error::Error as StdError, ffi::NulError, fmt, num::ParseIntError};

#[derive(Debug)]
pub enum Error {
    Status(Status),
    NulError(NulError),
    ParseIntError(ParseIntError),
    Message(String),
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Error::NulError(e) => Some(e),
            Error::ParseIntError(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Status(ref e) => write!(f, "{}", e.decode()),
            Error::NulError(ref e) => write!(f, "{}", e),
            Error::ParseIntError(ref e) => write!(f, "{}", e),
            Error::Message(ref e) => write!(f, "{}", e),
        }
    }
}

impl From<Status> for Error {
    fn from(e: Status) -> Self {
        Error::Status(e)
    }
}

impl From<NulError> for Error {
    fn from(e: NulError) -> Self {
        Error::NulError(e)
    }
}

impl From<ParseIntError> for Error {
    fn from(e: ParseIntError) -> Self {
        Error::ParseIntError(e)
    }
}

impl From<&str> for Error {
    fn from(e: &str) -> Self {
        Error::Message(e.to_owned())
    }
}

impl From<String> for Error {
    fn from(e: String) -> Self {
        Error::Message(e)
    }
}
