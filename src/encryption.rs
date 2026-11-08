use crate::{ID_BYTES_LENGTH, IV_LENGTH, Nano64, Nano64Error, PAYLOAD_LENGTH};
use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, generic_array::GenericArray},
};
use rand::Rng;
use std::time::SystemTime;

pub struct Nano64Cipher {
    cipher: Aes256Gcm,
}

impl Nano64Cipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new(&Key::<Aes256Gcm>::from(key)),
        }
    }

    pub fn encrypt(&self, id: Nano64) -> Result<EncryptedNano64, Nano64Error> {
        let iv = Self::generate_iv();
        let nonce = Nonce::from_slice(&iv);

        let ciphertext = self
            .cipher
            .encrypt(nonce, id.to_be_bytes().as_ref())
            .map_err(|_| Nano64Error::EncryptionFailed)?;

        let mut payload = [0u8; PAYLOAD_LENGTH];
        payload[..IV_LENGTH].copy_from_slice(&iv);
        payload[IV_LENGTH..].copy_from_slice(&ciphertext);

        Ok(EncryptedNano64 { payload })
    }

    pub fn decrypt(&self, encrypted: &EncryptedNano64) -> Result<Nano64, Nano64Error> {
        let bytes = encrypted.to_bytes();
        let iv = &bytes[..IV_LENGTH];
        let ciphertext = &bytes[IV_LENGTH..];
        let nonce = GenericArray::from_slice(iv);

        let bytes_decrypted: [u8; ID_BYTES_LENGTH] = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| Nano64Error::DecryptionFailed)?
            .try_into()
            .map_err(|b: Vec<u8>| Nano64Error::IdInvalidBytesLength(b.len()))?;

        let value = u64::from_be_bytes(bytes_decrypted);
        Ok(Nano64 { value })
    }

    /// Generate encrypted id using default rng and current timestamp
    pub fn generate_encrypted(&self) -> Result<EncryptedNano64, Nano64Error> {
        self.generate_encrypted_with_timestamp_and_rng(SystemTime::now(), &mut rand::rng())
    }

    /// Generate encrypted id using default rng and provided timestamp
    pub fn generate_encrypted_at(
        &self,
        timestamp: SystemTime,
    ) -> Result<EncryptedNano64, Nano64Error> {
        self.generate_encrypted_with_timestamp_and_rng(timestamp, &mut rand::rng())
    }

    /// Generate encrypted id using provided rng and default timestamp
    pub fn generate_encrypted_with_rng<RNG>(
        &self,
        rng: &mut RNG,
    ) -> Result<EncryptedNano64, Nano64Error>
    where
        RNG: rand::Rng,
    {
        self.generate_encrypted_with_timestamp_and_rng(SystemTime::now(), rng)
    }

    // Private helper method
    fn generate_encrypted_with_timestamp_and_rng<RNG>(
        &self,
        timestamp: SystemTime,
        rng: &mut RNG,
    ) -> Result<EncryptedNano64, Nano64Error>
    where
        RNG: rand::Rng,
    {
        let id = crate::generate_with_timestamp_and_rng(timestamp, rng)?;
        self.encrypt(id)
    }

    fn generate_iv() -> [u8; IV_LENGTH] {
        let mut iv = [0u8; IV_LENGTH];
        rand::rng().fill_bytes(&mut iv);
        iv
    }
}

#[derive(Debug)]
pub struct EncryptedNano64 {
    payload: [u8; PAYLOAD_LENGTH],
}

impl EncryptedNano64 {
    pub fn from_bytes(bytes: [u8; PAYLOAD_LENGTH]) -> Self {
        Self { payload: bytes }
    }

    pub fn from_hex(hex: &str) -> Result<Self, Nano64Error> {
        let bytes: [u8; PAYLOAD_LENGTH] = hex::decode(hex)?
            .try_into()
            .map_err(|bytes: Vec<u8>| Nano64Error::IdInvalidBytesLength(bytes.len()))?;

        Ok(Self::from_bytes(bytes))
    }

    pub fn to_bytes(&self) -> &[u8; PAYLOAD_LENGTH] {
        &self.payload
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.payload)
    }
}

#[cfg(test)]
mod tests {
    use crate::encryption::Nano64Cipher;

    const DEFAULT_KEY: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];

    #[test]
    fn encrypt_provided_id() {
        // Provided id
        let id = crate::generate().unwrap();
        let c = Nano64Cipher::new(DEFAULT_KEY);
        let encrypted_id = c.encrypt(id).expect("encrypt");
        println!("{encrypted_id:?}");
        let decrypted_id = c.decrypt(&encrypted_id).unwrap();
        println!("{decrypted_id}");
    }

    #[test]
    fn gen_encrypted() {
        let cipher = Nano64Cipher::new(DEFAULT_KEY);
        let encrypted_id = cipher.generate_encrypted().unwrap();
        println!("{encrypted_id:?}");
    }
}
