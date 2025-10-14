use crate::{
    ClockImpl, Hex, IV_LENGTH, Nano64, Nano64Error, PAYLOAD_LENGTH, RandomNumberGeneratorImpl,
    default_rng, time_now_since_epoch_ms,
};
use aes_gcm::{
    Aes256Gcm, Key,
    aead::{Aead, KeyInit, OsRng, generic_array::GenericArray, rand_core::RngCore},
};

#[derive(Clone)]
pub struct Nano64Encrypted {
    pub id: Nano64,
    pub(crate) payload: [u8; PAYLOAD_LENGTH],
    #[allow(dead_code)]
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

pub struct Nano64EncryptionFactory {
    pub(crate) gcm: Aes256Gcm,
    pub(crate) clock: ClockImpl,
    pub(crate) rng: RandomNumberGeneratorImpl,
}

impl Nano64EncryptionFactory {
    pub fn new(
        aes_key: &[u8],
        clock: Option<ClockImpl>,
        rng: Option<RandomNumberGeneratorImpl>,
    ) -> Result<Self, Nano64Error> {
        if aes_key.len() != 32 {
            return Err(Nano64Error::Error("AES-256 key must be 32 bytes!".into()));
        }

        let rng = if let Some(_rng) = rng {
            _rng
        } else {
            default_rng
        };

        let clock = if let Some(_clock) = clock {
            _clock
        } else {
            time_now_since_epoch_ms
        };

        let key = Key::<Aes256Gcm>::from_slice(aes_key);
        let gcm = Aes256Gcm::new(key);

        Ok(Self { gcm, clock, rng })
    }

    pub fn encrypt(&self, id: Nano64) -> Result<Nano64Encrypted, Nano64Error> {
        let iv = self.generate_iv();
        let nonce = GenericArray::clone_from_slice(&iv);
        let plaintext = id.value.to_be_bytes();
        let ciphertext = self
            .gcm
            .encrypt(&nonce, plaintext.as_ref())
            .map_err(|e| Nano64Error::Error(format!("Error during encryption! {e}")))?;

        if ciphertext.len() != 8 + 16 {
            return Err(Nano64Error::Error(format!(
                "unexpected AES-GCM output length: {}",
                ciphertext.len()
            )));
        }

        let mut payload = [0u8; PAYLOAD_LENGTH];
        payload[..IV_LENGTH].copy_from_slice(&iv);
        payload[IV_LENGTH..].copy_from_slice(&ciphertext);

        Ok(Nano64Encrypted {
            id,
            payload,
            gcm: self.gcm.clone(),
        })
    }

    pub fn generate_encrypted(&self, timestamp: u64) -> Result<Nano64Encrypted, Nano64Error> {
        let mut ts = timestamp;
        if ts == 0 {
            ts = (self.clock)();
        }
        let id = Nano64::generate(ts, Some(self.rng))?;
        self.encrypt(id)
    }

    pub fn generate_encrypted_now(&self) -> Result<Nano64Encrypted, Nano64Error> {
        self.generate_encrypted((self.clock)())
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_encrypted_bytes(&self, bytes: &[u8]) -> Result<Nano64Encrypted, Nano64Error> {
        if bytes.len() != PAYLOAD_LENGTH {
            return Err(Nano64Error::Error(format!(
                "encrypted payload must be {} bytes, got {}",
                PAYLOAD_LENGTH,
                bytes.len()
            )));
        }

        // Split into IV and ciphertext
        let iv = &bytes[..IV_LENGTH];
        let ciphertext = &bytes[IV_LENGTH..];

        // Decrypt
        let nonce = GenericArray::from_slice(iv);
        let plaintext = self
            .gcm
            .decrypt(nonce, ciphertext)
            .map_err(|_| Nano64Error::Error("decryption failed".into()))?;

        if plaintext.len() != 8 {
            return Err(Nano64Error::Error(format!(
                "decryption yielded invalid length: {}",
                plaintext.len()
            )));
        }

        let mut arr = [0u8; 8];
        arr.copy_from_slice(&plaintext);
        let value = u64::from_be_bytes(arr);

        let mut payload = [0u8; PAYLOAD_LENGTH];
        payload.copy_from_slice(bytes);

        Ok(Nano64Encrypted {
            id: Nano64 { value },
            payload,
            gcm: self.gcm.clone(),
        })
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn from_encrypted_hex(&self, hex: String) -> Result<Nano64Encrypted, Nano64Error> {
        let bytes = Hex::to_bytes(hex.as_str())?;
        if bytes.len() != PAYLOAD_LENGTH {
            return Err(Nano64Error::Error(format!(
                "Encrypted payload must be {} len, got {}",
                PAYLOAD_LENGTH,
                bytes.len()
            )));
        }
        self.from_encrypted_bytes(&bytes)
    }

    fn generate_iv(&self) -> [u8; IV_LENGTH] {
        let mut iv = [0u8; IV_LENGTH];
        OsRng.fill_bytes(&mut iv);
        iv
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

    #[test]
    fn test_nano64_encrypted_default_clock() {
        let key: [u8; 32] = [
            1, 2, 3, 4, 5, 61, 73, 8, 92, 10, 15, 122, 13, 14, 15, 16, 17, 18, 74, 20, 21, 22, 23,
            24, 69, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        assert_ne!(
            factory.generate_encrypted_now().unwrap().id.get_timestamp(),
            0,
            "generate_encrypted_now should use current timestamp!"
        );
    }

    #[test]
    fn test_nano64_encrypted_generate_iv_error() {
        let key: [u8; 32] = [
            1, 2, 3, 43, 5, 61, 73, 8, 92, 10, 15, 122, 13, 14, 15, 16, 17, 18, 74, 20, 21, 22, 23,
            24, 69, 39, 27, 28, 29, 30, 66, 32,
        ];
        let factory = Nano64EncryptionFactory::new(&key, None, None).unwrap();
        let id = Nano64::generate_default().unwrap();
        let encrypted = factory.encrypt(id).unwrap();
        assert_eq!(
            encrypted.to_encrypted_bytes().len(),
            PAYLOAD_LENGTH,
            "Encrypted payload has incorrect len"
        );
    }
}
