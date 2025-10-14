//!
//! [Please see our README for more info!](https://github.com/matthewoestreich/rs-nano64)
//!
use std::time::{SystemTime, UNIX_EPOCH};

mod errors;
mod hex;
mod monotonic_refs;
mod nano64;
mod nano64_encrypted;

pub use errors::*;
pub use hex::*;
pub use nano64::*;
pub use nano64_encrypted::*;

pub const IV_LENGTH: usize = 12;
pub const PAYLOAD_LENGTH: usize = IV_LENGTH + 8 + 16;
// TIMESTAMP_BITS is the number of bits allocated to the millisecond timestamp (0..2^44-1).
pub const TIMESTAMP_BITS: u64 = 44;
// RANDOM_BITS is the number of bits allocated to the random field per millisecond (0..2^20-1).
pub const RANDOM_BITS: u64 = 20;
// TIMESTAMP_SHIFT is the bit shift used to position the timestamp above the random field.
pub(crate) const TIMESTAMP_SHIFT: u64 = RANDOM_BITS;
// TIMESTAMP_MASK is the mask for extracting the 44-bit timestamp from a u64 value.
pub(crate) const TIMESTAMP_MASK: u64 = (1 << TIMESTAMP_BITS) - 1;
// RANDOM_MASK is the mask for the 20-bit random field.
pub(crate) const RANDOM_MASK: u64 = (1 << RANDOM_BITS) - 1;
// MAX_TIMESTAMP is the maximum timestamp value (2^44 - 1).
pub(crate) const MAX_TIMESTAMP: u64 = TIMESTAMP_MASK;

// Compare compares two IDs as unsigned 64-bit numbers.
// Returns -1 if a < b, 0 if a == b, 1 if a > b.
pub fn compare(a: &Nano64, b: &Nano64) -> i64 {
    if a.value < b.value {
        return -1;
    } else if a.value > b.value {
        return 1;
    }
    return 0;
}

// A function that returns a random unsigned integer containing a specified number of random bits.
// {bits} The number of random bits to generate (must be between 1 and 32).
pub type RandomNumberGeneratorImpl = fn(bits: u32) -> Result<u32, Nano64Error>;

pub type ClockImpl = fn() -> u64;

// Gets time now since epoch in ms
fn time_now_since_epoch_ms() -> u64 {
    return SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64;
}

// Default cryptographically-secure RNG.
// `bits` must be in the 1-32 range.
fn default_rng(bits: u32) -> Result<u32, Nano64Error> {
    if bits == 0 || bits > 32 {
        return Err(Nano64Error::Error(format!("bits must be 1-32, got {bits}")));
    }

    // Generate 4 random bytes
    let mut buf = [0u8; 4];
    rand::fill(&mut buf);

    // Convert bytes to u32
    let mut val = u32::from_be_bytes(buf);

    // Mask to requested number of bits
    if bits < 32 {
        val &= (1u32 << bits) - 1;
    }

    Ok(val)
}
