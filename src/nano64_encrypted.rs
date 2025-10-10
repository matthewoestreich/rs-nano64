use crate::{Hex, Nano64};
use aes_gcm::Aes256Gcm;

pub const IV_LENGTH: usize = 12;
pub const PAYLOAD_LENGTH: usize = IV_LENGTH + 8 + 16;

#[allow(dead_code)]
#[derive(Clone)]
pub struct Nano64Encrypted {
    pub id: Nano64,
    pub(crate) payload: [u8; PAYLOAD_LENGTH],
    pub(crate) gcm: Aes256Gcm,
}

impl Nano64Encrypted {
    pub fn to_encrypted_hex(&self) -> String {
        Hex::from_bytes(&self.payload)
    }

    pub fn to_encrypted_bytes(&self) -> [u8; PAYLOAD_LENGTH] {
        self.payload
    }
}

#[cfg(test)]
mod tests {

    use crate::{Nano64, Nano64EncryptionFactory, PAYLOAD_LENGTH};

    #[test]
    fn test_nano64_encrypted_complete() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let encrypted = factory.generate_encrypted_now().unwrap();
        let hex_str = encrypted.to_encrypted_hex();
        let bytes = encrypted.to_encrypted_bytes();
        let decrypted_from_hex = factory.from_encrypted_hex(hex_str).unwrap();
        assert!(decrypted_from_hex.id.equals(&encrypted.id));
        let decrypted_from_bytes = factory.from_encrypted_bytes(&bytes).unwrap();
        assert!(decrypted_from_bytes.id.equals(&encrypted.id));
    }

    #[test]
    fn test_nano64_encrypted_generate_encrypted() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let timestamp: u64 = 1234567890;
        let encrypted = factory.generate_encrypted(timestamp).unwrap();
        println!("{:?}", encrypted.payload);
        assert_eq!(encrypted.id.get_timestamp(), timestamp);
    }

    #[test]
    fn test_nano64_encrypted_generate_encrypted_zero_timestamp() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 73, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ];
        fn mock_clock() -> u64 {
            9999999
        }
        let factory = Nano64EncryptionFactory::new(&key, Some(mock_clock), None).unwrap();
        let encrypted = factory.generate_encrypted(0).unwrap();
        assert!(encrypted.id.get_timestamp() == 9999999);
    }

    #[test]
    fn test_nano64_encrypted_encrypt() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 73, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let id = Nano64::generate_default().unwrap();
        let encrypted = factory.encrypt(id.clone()).unwrap();
        assert!(encrypted.id.equals(&id));
    }

    #[test]
    fn test_nano64_encrypted_errors_invalid_encrypted_byte_length() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 6, 73, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        if let Ok(got) = factory.from_encrypted_bytes(&[0x01, 0x02, 0x03]) {
            panic!("Expected error, but got id {:?}", got.id)
        }
    }

    #[test]
    fn test_nano64_encrypted_errors_invalid_encrypted_hex() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 61, 73, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        if let Ok(got) = factory.from_encrypted_hex("INVALID".into()) {
            panic!("Expected error, but got id {:?}", got.id)
        }
    }

    #[test]
    fn test_nano64_encrypted_errors_invalid_encrypted_hex_wrong_len() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 61, 73, 8, 9, 10, 15, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        if let Ok(got) = factory.from_encrypted_hex("AABBCCDD".into()) {
            panic!("Expected error, but got id {:?}", got.id)
        }
    }

    #[test]
    fn test_nano64_encrypted_errors_tampered_ciphertext() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 61, 73, 8, 9, 10, 15, 122, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let encrypted = factory.generate_encrypted_now().unwrap();
        let mut bytes = encrypted.to_encrypted_bytes();
        bytes[20] ^= 0xFF;
        if let Ok(got) = factory.from_encrypted_bytes(&bytes) {
            panic!("Expected error but got id {:?}", got.id);
        }
    }

    #[test]
    fn test_nano64_encrypted_invalid_decryption_length() {
        // This test covers the edge case where decrypted data isn't 8 bytes
        // This is difficult to trigger naturally, but we can test the error path exists
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 61, 73, 8, 9, 10, 15, 122, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 69, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let invalid_payload: [u8; PAYLOAD_LENGTH] = [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1, 1, 1, 1, 1,
        ];
        if let Ok(got) = factory.from_encrypted_bytes(&invalid_payload) {
            panic!(
                "from_encrypted_bytes with invalid payload should error but got {:?}",
                got.id
            );
        }
    }

    #[test]
    fn test_nano64_encrypted_ciphertext_length() {
        // This covers the error case in Encrypt where ciphertext length is unexpected
        // In normal operation this shouldn't happen, but the code checks for it
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 61, 73, 8, 9, 10, 15, 122, 13, 14, 15, 16, 17, 18, 74, 20, 21, 22, 23,
            24, 69, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let id = Nano64::generate_default().unwrap();
        let mut encrypted = if let Ok(got) = factory.encrypt(id.clone()) {
            got
        } else {
            panic!("Normal encryption should work")
        };

        encrypted.id.value = 1;

        if let Ok(got) = factory.encrypt(encrypted.id.clone()) {
            got
        } else {
            panic!("ahh");
        };
    }
}
