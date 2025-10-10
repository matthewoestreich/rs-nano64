use crate::{Hex, Nano64};
use aes_gcm::Aes256Gcm;

pub const IV_LENGTH: usize = 12;
pub const PAYLOAD_LENGTH: usize = IV_LENGTH + 8 + 16;

#[allow(dead_code)]
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
