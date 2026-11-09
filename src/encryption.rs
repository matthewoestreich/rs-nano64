use crate::{IV_LENGTH, NANO64_BYTES_LENGTH, Nano64, Nano64Error, PAYLOAD_LENGTH};
use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, KeyInit, generic_array::GenericArray},
};
use rand::Rng;
use std::time::SystemTime;

/// Provides methods for generating, encrypting, and decrypting Nano64 IDs.
pub struct Nano64Cipher {
    cipher: Aes256Gcm,
}

impl Nano64Cipher {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            cipher: Aes256Gcm::new(&Key::<Aes256Gcm>::from(key)),
        }
    }

    /// Encrypt an existing Nano64 ID.
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

        Ok(EncryptedNano64::from_bytes(payload))
    }

    /// Decrypts an encrypted Nano64 ID and verifies its authenticity.
    pub fn decrypt(&self, encrypted: &EncryptedNano64) -> Result<Nano64, Nano64Error> {
        let bytes = encrypted.to_bytes();
        let iv = &bytes[..IV_LENGTH];
        let ciphertext = &bytes[IV_LENGTH..];
        let nonce = GenericArray::from_slice(iv);

        let bytes_decrypted: [u8; NANO64_BYTES_LENGTH] = self
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

    /// Generate encrypted ID using your own timestamp and RNG
    pub fn generate_encrypted_with<RNG>(
        &self,
        timestamp: SystemTime,
        rng: &mut RNG,
    ) -> Result<EncryptedNano64, Nano64Error>
    where
        RNG: rand::Rng,
    {
        self.generate_encrypted_with_timestamp_and_rng(timestamp, rng)
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

/// Represents an encrypted Nano64 ID.
///
/// Contains a 12-byte nonce, 8-byte ciphertext, and 16-byte authentication tag.
#[derive(Debug)]
pub struct EncryptedNano64 {
    payload: [u8; PAYLOAD_LENGTH],
}

#[allow(dead_code)]
impl EncryptedNano64 {
    pub(crate) fn from_bytes(bytes: [u8; PAYLOAD_LENGTH]) -> Self {
        Self { payload: bytes }
    }

    pub(crate) fn from_hex(hex: &str) -> Result<Self, Nano64Error> {
        let bytes: [u8; PAYLOAD_LENGTH] = hex::decode(hex)?
            .try_into()
            .map_err(|bytes: Vec<u8>| Nano64Error::IdInvalidBytesLength(bytes.len()))?;

        Ok(Self::from_bytes(bytes))
    }

    /// Get the encrypted payload as raw bytes
    pub fn to_bytes(&self) -> &[u8; PAYLOAD_LENGTH] {
        &self.payload
    }

    /// Get the 72-char encrypted payload as a hexadecimal string
    pub fn to_hex(&self) -> String {
        hex::encode(self.payload)
    }
}

#[cfg(test)]
mod tests {
    use crate::Nano64Cipher;

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
    fn encryption_round_trip() {
        let cipher = Nano64Cipher::new(DEFAULT_KEY);
        let id = crate::generate().expect("id");
        let id_clone = id.clone();
        let enc_id = cipher.encrypt(id).expect("enc");
        let dec_id = cipher.decrypt(&enc_id).expect("dec");
        assert_eq!(
            id_clone, dec_id,
            "expected to decrypt back to same id! expected = {id_clone} got = {dec_id}"
        );
    }

    #[test]
    fn gen_encrypted() {
        let cipher = Nano64Cipher::new(DEFAULT_KEY);
        let encrypted_id = cipher.generate_encrypted().unwrap();
        println!("{encrypted_id:?}");
        println!("{}", encrypted_id.to_hex().to_ascii_uppercase());
    }

    #[test]
    fn encryption_code_from_readme() {
        use crate::Nano64Cipher;

        // Create 32-byte key (we use AES-256)
        let key: [u8; 32] = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32, 0x54, 0x76, 0x98, 0xBA,
            0xDC, 0xFE, 0x0F, 0x1E, 0x2D, 0x3C, 0x4B, 0x5A, 0x69, 0x78, 0x87, 0x96, 0xA5, 0xB4,
            0xC3, 0xD2, 0xE1, 0xF0,
        ];

        // Create a cipher, this is what allows you to encrypt/generate encrypted IDs
        let cipher = Nano64Cipher::new(key);

        // Generate and encrypt using default RNG and current timestamp
        let encrypted_now = cipher.generate_encrypted().expect("id");
        println!("encrypted_now = {encrypted_now:?}");
        // encrypted_now = EncryptedNano64 { payload: [26, 38, 236, ...] }

        let bytes_payload = encrypted_now.to_bytes();
        println!("Payload bytes = {bytes_payload:?}");
        // Payload bytes = [26, 38, 236, ...]

        let hex_payload = encrypted_now.to_hex().to_ascii_uppercase();
        println!("72-char hex payload = {hex_payload}");
        // 72-char hex payload = 1A26EC84662107B7A09C0BB0039D1B869A87620520724461233C13C808593B35AC7169AB

        // Decrypt later...
        let decrypted_now = cipher.decrypt(&encrypted_now).expect("decrypted id");
        println!("Decrypted ID = {decrypted_now}");
        // Decrypted ID = Nano64{ value=1881315126699803069, timestamp=1794161917400, random=180669 }

        // Encrypt existing ID
        let existing = crate::generate().expect("id");
        let encrypted_existing = cipher.encrypt(existing).expect("encrypted id");
        println!("Encrypted existing = {encrypted_existing:?}");
        // Encrypted existing = EncryptedNano64 { payload: [16, 43, 93, ...] }
    }
}
