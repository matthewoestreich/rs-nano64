use std::{
    error,
    fmt::{Display, Formatter, Result},
};

#[derive(Debug)]
pub enum Nano64Error {
    Error(String),
    TimeStampRangeError,
    TimeStampExceedsBitRange(u64),
    RNGOutOfBounds(u32),
    HexStringNotEvenCharacters,
    HexStringContainsNonHexChars,
}

impl Display for Nano64Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        use Nano64Error::*;
        match self {
            Error(s) => write!(f, "{s}"),
            TimeStampRangeError => write!(f, "Start must be less than or equal to end!"),
            TimeStampExceedsBitRange(got) => write!(f, "Timestamp exceeds the 44-bit range. Got={got}"),
            RNGOutOfBounds(got) => write!(f, "RNG bits must be between 1 and 32. Got {got}"),
            HexStringNotEvenCharacters => write!(f, "Hex string must contain an even amount of characters!"),
            HexStringContainsNonHexChars => write!(f, "Hex string contains non-hex characters!"),
        }
    }
}

impl error::Error for Nano64Error {}
