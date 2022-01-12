use std::convert::From;

#[derive(Debug)]
pub(crate) enum BargeError {
    StdIoError(std::io::Error),
    StdStrUtf8Error(std::str::Utf8Error),
    SerdeJsonError(serde_json::Error),
    ClapError(clap::Error),
    NoneOption(&'static str),
    InvalidValue(&'static str),
}

impl From<std::io::Error> for BargeError {
    fn from(error: std::io::Error) -> BargeError {
        BargeError::StdIoError(error)
    }
}

impl From<std::str::Utf8Error> for BargeError {
    fn from(error: std::str::Utf8Error) -> BargeError {
        BargeError::StdStrUtf8Error(error)
    }
}

impl From<serde_json::Error> for BargeError {
    fn from(error: serde_json::Error) -> BargeError {
        BargeError::SerdeJsonError(error)
    }
}

impl From<clap::Error> for BargeError {
    fn from(error: clap::Error) -> BargeError {
        BargeError::ClapError(error)
    }
}

pub(crate) type Result<T> = std::result::Result<T, BargeError>;
