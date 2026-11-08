use crate::{HEX_STRING_LENGTH, ID_BYTES_LENGTH};
use std::{
    error,
    fmt::{Display, Formatter, Result},
    time::SystemTimeError,
};

#[derive(Debug)]
pub enum Nano64Error {
    Custom(String),
    TimestampExceedsBitRange(u128),
    HexStringInvalidLength(usize),
    IdInvalidBytesLength(usize),
    SystemTime(SystemTimeError),
    FromHex(hex::FromHexError),
    EncryptionFailed,
    DecryptionFailed,
}

impl Display for Nano64Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use Nano64Error::*;
        match self {
            Custom(s) => write!(f, "{s}"),
            TimestampExceedsBitRange(got) => {
                write!(f, "Timestamp exceeds the 44-bit range. Got={got}")
            }
            HexStringInvalidLength(got) => {
                write!(
                    f,
                    "Hex string invalid length, expected {HEX_STRING_LENGTH} got {got}"
                )
            }
            IdInvalidBytesLength(got) => {
                write!(
                    f,
                    "Invalid bytes length, expected {ID_BYTES_LENGTH} got {got}"
                )
            }
            SystemTime(error) => write!(f, "System time error {error}"),
            FromHex(error) => write!(f, "Error converting from hex strng {error}"),
            EncryptionFailed => write!(f, "Encryption failed"),
            DecryptionFailed => write!(f, "Decryption failed"),
        }
    }
}

impl From<hex::FromHexError> for Nano64Error {
    fn from(error: hex::FromHexError) -> Self {
        Self::FromHex(error)
    }
}

impl From<SystemTimeError> for Nano64Error {
    fn from(error: SystemTimeError) -> Self {
        Self::SystemTime(error)
    }
}

impl error::Error for Nano64Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::SystemTime(error) => Some(error),
            Self::FromHex(error) => Some(error),
            _ => None,
        }
    }
}
