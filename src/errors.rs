use std::{
    error,
    fmt::{Display, Formatter, Result},
    time::SystemTimeError,
};

#[derive(Debug)]
pub enum Nano64Error {
    Custom(String),
    TimeStampRangeError,
    TimestampExceedsMax,
    TimestampExceedsBitRange(u128),
    RNGOutOfBounds(u32),
    HexStringNotEvenCharacters,
    HexStringContainsNonHexChars,
    SystemTime(SystemTimeError),
}

impl Display for Nano64Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use Nano64Error::*;
        match self {
            Custom(s) => write!(f, "{s}"),
            TimestampExceedsMax => write!(f, "Timestamp exceeds maximum"),
            TimeStampRangeError => write!(f, "Start must be less than or equal to end!"),
            TimestampExceedsBitRange(got) => {
                write!(f, "Timestamp exceeds the 44-bit range. Got={got}")
            }
            RNGOutOfBounds(got) => write!(f, "RNG bits must be between 1 and 32. Got {got}"),
            HexStringNotEvenCharacters => {
                write!(f, "Hex string must contain an even amount of characters!")
            }
            HexStringContainsNonHexChars => write!(f, "Hex string contains non-hex characters!"),
            SystemTime(error) => write!(f, "System time error {error}"),
        }
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
            _ => None,
        }
    }
}
