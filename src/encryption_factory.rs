use crate::{
    Clock, Hex, IV_LENGTH, Nano64, Nano64Encrypted, Nano64Error, PAYLOAD_LENGTH,
    RandomNumberGeneratorImpl, default_rng, time_now_since_epoch_ms,
};
use aes_gcm::{
    Aes256Gcm, Key,
    aead::{Aead, KeyInit, OsRng, generic_array::GenericArray, rand_core::RngCore},
};

pub struct Nano64EncryptionFactory {
    gcm: Aes256Gcm,
    clock: Clock,
    rng: RandomNumberGeneratorImpl,
}

impl Nano64EncryptionFactory {
    pub fn new(
        aes_key: &[u8],
        clock: Option<Clock>,
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
        let nonce = GenericArray::from_slice(&iv);
        let plaintext = id.value.to_be_bytes();
        let ciphertext = self
            .gcm
            .encrypt(nonce, plaintext.as_ref())
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
