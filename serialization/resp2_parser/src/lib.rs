use std::fmt::Display;
use std::num::ParseIntError;
use std::str::Utf8Error;

use crate::parser::MAX_BULK_LEN;

pub mod decoder;
pub mod encoder;
pub mod frame;
pub(crate) mod parser;
pub mod server;

#[derive(Debug, PartialEq)]
pub enum Error {
    UnknownType(u8),
    InvalidInteger,
    InvalidUtf8,
    LengthTooLarge(usize),
    MissingCrlf,
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Self {
        Self::InvalidUtf8
    }
}

impl From<ParseIntError> for Error {
    fn from(_: ParseIntError) -> Self {
        Self::InvalidInteger
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownType(byte) => write!(f, "unknown frame type byte {byte:#04x}"),
            Self::InvalidInteger => write!(f, "not a valid integer"),
            Self::InvalidUtf8 => write!(f, "not valid UTF-8"),
            Self::LengthTooLarge(len) => write!(f, "declared length {len} exceeds {MAX_BULK_LEN}"),
            Self::MissingCrlf => write!(f, "expected a CRLF terminator"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_format_for_humans() {
        assert_eq!(
            Error::UnknownType(b'?').to_string(),
            "unknown frame type byte 0x3f"
        );
        assert_eq!(Error::InvalidUtf8.to_string(), "not valid UTF-8");
    }
}
