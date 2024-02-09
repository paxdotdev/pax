use std;
use std::fmt::{self, Display};

use serde::{de, ser};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Message(String),

    UnsupportedType(String),
    TrailingCharacters,
    UnsupportedMethod,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(msg) => formatter.write_str(msg),
            Error::UnsupportedType(t) => {
                formatter.write_str(format!("unsupported type: {}", t).as_str())
            }
            Error::TrailingCharacters => {
                formatter.write_str("trailing characters after deserialization")
            }
            _ => formatter.write_str("unknown error"),
        }
    }
}

impl std::error::Error for Error {}
