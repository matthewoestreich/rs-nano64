use crate::errors::*;
use hex::FromHex;
use std::str;

pub struct Hex;

impl Hex {
    pub fn from_bytes(bytes: &[u8]) -> String {
        hex::encode_upper(bytes)
    }

    pub fn to_bytes(hex_str: &str) -> Result<Vec<u8>, Nano64Error> {
        let h = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        if h.len() % 2 != 0 {
            return Err(Nano64Error::HexStringNotEvenCharacters);
        }
        Vec::from_hex(h).map_err(|_| Nano64Error::HexStringContainsNonHexChars)
    }
}

#[cfg(test)]
mod tests {
    use super::{Hex, Nano64Error};

    #[test]
    fn test_deserialize() {
        let og_string = "ABCD";
        let og_string_bytes = Hex::to_bytes(og_string).unwrap();
        let og_string_deserial = Hex::from_bytes(&og_string_bytes);
        assert!(og_string_deserial.starts_with(og_string));
    }

    #[test]
    fn test_to_bytes_valid_hex() {
        let hex = "0x12AB34";
        let bytes = Hex::to_bytes(hex).unwrap();
        assert_eq!(bytes, [0x12, 0xAB, 0x34]);
    }

    #[test]
    fn test_to_bytes_no_prefix() {
        let hex = "12AB34";
        let bytes = Hex::to_bytes(hex).unwrap();
        assert_eq!(bytes, [0x12, 0xAB, 0x34]);
    }

    #[test]
    fn test_to_bytes_odd_length() {
        let hex = "123";
        let err = Hex::to_bytes(hex).unwrap_err();
        assert!(matches!(err, Nano64Error::HexStringNotEvenCharacters));
    }

    #[test]
    fn test_to_bytes_non_hex_chars() {
        let hex = "12G4";
        let err = Hex::to_bytes(hex).unwrap_err();
        assert!(matches!(err, Nano64Error::HexStringContainsNonHexChars));
    }
}
