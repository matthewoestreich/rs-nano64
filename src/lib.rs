//!
//! [Please see our README for more info!](https://github.com/matthewoestreich/rs-nano64)
//!

use rand::RngExt as _;
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
// TIMESTAMP_MASK is the mask for extracting the 44-bit timestamp from a u64 value.
pub(crate) const TIMESTAMP_MASK: u64 = (1 << TIMESTAMP_BITS) - 1;
// MAX_TIMESTAMP is the maximum timestamp value (2^44 - 1).
pub(crate) const MAX_TIMESTAMP: u128 = TIMESTAMP_MASK as u128;
// RANDOM_MASK is the mask for the 20-bit random field.
pub(crate) const RANDOM_MASK: u64 = (1 << RANDOM_BITS) - 1;
// Length of valid nano64 hex string WITHOUT dash
pub(crate) const HEX_STRING_LENGTH_NO_DASH: usize = 16;
// Number of bytes that make up a decoded hex string
pub(crate) const HEX_STRING_LENGTH_BYTES_DECODED: usize = 8;

/// Generate a new ID using current timestamp and default RNG.
pub fn generate() -> Result<Nano64, Nano64Error> {
    generate_with_timestamp_and_rng(SystemTime::now(), &mut rand::rng())
}

/// Generate a new ID using your own timestamp and default RNG.
pub fn generate_at(timestamp: SystemTime) -> Result<Nano64, Nano64Error> {
    generate_with_timestamp_and_rng(timestamp, &mut rand::rng())
}

/// Generate a new ID using the current timestamp and your own RNG.
pub fn generate_with_rng<RNG>(rng: &mut RNG) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_with_timestamp_and_rng(SystemTime::now(), rng)
}

/// Generate a new ID using your own timestamp and RNG.
pub fn generate_with<RNG>(timestamp: SystemTime, rng: &mut RNG) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_with_timestamp_and_rng(timestamp, rng)
}

// Compare compares two IDs as unsigned 64-bit numbers.
// Returns -1 if a < b, 0 if a == b, 1 if a > b.
pub fn compare(a: &Nano64, b: &Nano64) -> i64 {
    if a.value < b.value {
        return -1;
    } else if a.value > b.value {
        return 1;
    }
    0
}

// Generate ID using provided timestamp and rng
fn generate_with_timestamp_and_rng<RNG>(
    timestamp: SystemTime,
    rng: &mut RNG,
) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    let ts = timestamp.duration_since(UNIX_EPOCH)?.as_millis();

    if ts > MAX_TIMESTAMP {
        return Err(Nano64Error::TimestampExceedsBitRange(ts));
    }

    let rand = (rng.random::<u32>() as u64) & RANDOM_MASK;
    let value = ((ts as u64) << RANDOM_BITS) | rand;

    Ok(Nano64::new(value))
}

// Generates cryptographically secure random `u32` via `rand::rng().random()`
fn generate_random_u32() -> u32 {
    rand::rng().random()
}
