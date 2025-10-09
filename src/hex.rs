use std::str;

use crate::errors::*;

pub struct Hex;

impl Hex {
    pub fn from_bytes(bytes: &[u8; 8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join("")
    }

    pub fn to_bytes(hex: &str) -> Result<[u8; 8], Nano64Error> {
        let h = hex.strip_prefix("0x").unwrap_or(hex);

        if h.len() % 2 != 0 {
            return Err(Nano64Error::HexStringNotEvenCharacters);
        }

        let mut bytes = [0u8; 8];
        for (i, chunk) in h.as_bytes().chunks(2).enumerate() {
            let s = str::from_utf8(chunk).map_err(|_| Nano64Error::HexStringContainsNonHexChars)?;
            bytes[i] = u8::from_str_radix(s, 16)
                .map_err(|_| Nano64Error::HexStringContainsNonHexChars)?;
        }

        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::{Hex, Nano64Error};

    #[test]
    fn test_deserialize() {
      let og_string = "ABCD";
      let og_string_bytes= Hex::to_bytes(og_string).unwrap();
      let og_string_deserial = Hex::from_bytes(&og_string_bytes);
      assert!(og_string_deserial.starts_with(og_string));
    }

    #[test]
    fn test_to_bytes_valid_hex() {
        let hex = "0x12AB34";
        let bytes = Hex::to_bytes(hex).unwrap();
        assert_eq!(bytes, [0x12, 0xAB, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn test_to_bytes_no_prefix() {
        let hex = "12AB34";
        let bytes = Hex::to_bytes(hex).unwrap();
        assert_eq!(bytes, [0x12, 0xAB, 0x34, 0x00, 0x00, 0x00, 0x00, 0x00]);
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
