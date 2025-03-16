use std::{error::Error as StdErr, num::ParseIntError};

#[derive(Debug)]
pub enum Error {
    Serenity(serenity::Error),
    Libsql(libsql::Error),
    Serde(serde::de::value::Error),
    Io(std::io::Error),
    ParseInt(ParseIntError),
}
use core::result::Result as StdResult;

pub type Result<T = ()> = StdResult<T, Error>;

impl StdErr for Error {
    fn source(&self) -> Option<&(dyn StdErr + 'static)> {
        match self {
            Error::Serenity(error) => error.source(),
            Error::Libsql(error) => error.source(),
            Error::Serde(error) => error.source(),
            Error::Io(error) => error.source(),
            Error::ParseInt(error) => error.source(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Error::Serenity(error) => write!(f, "{error}"),
            Error::Libsql(error) => write!(f, "{error}"),
            Error::Serde(error) => write!(f, "{error}"),
            Error::Io(error) => write!(f, "{error}"),
            Error::ParseInt(error) => write!(f, "{error}"),
        }
    }
}

impl From<ParseIntError> for Error {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<libsql::Error> for Error {
    fn from(value: libsql::Error) -> Self {
        Self::Libsql(value)
    }
}

impl From<serde::de::value::Error> for Error {
    fn from(value: serde::de::value::Error) -> Self {
        Self::Serde(value)
    }
}
