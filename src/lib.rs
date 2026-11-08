//!
//! [Please see our README for more info!](https://github.com/matthewoestreich/rs-nano64)
//!

mod errors;
mod nano64;
// mod nano64_encrypted;

pub use errors::*;
pub use nano64::*;
// pub use nano64_encrypted::*;

use rand::RngExt as _;
use std::{
    cmp::Ordering,
    sync::{LazyLock, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// Global monotonic references
pub(crate) struct MonotonicRefs {
    pub(crate) last_timestamp: u128,
    pub(crate) last_random: u64,
}

pub(crate) static MONOTONIC_REFS: LazyLock<Mutex<MonotonicRefs>> = LazyLock::new(|| {
    Mutex::new(MonotonicRefs {
        last_timestamp: 0,
        last_random: 0,
    })
});

pub(crate) const IV_LENGTH: usize = 12;
pub(crate) const PAYLOAD_LENGTH: usize = IV_LENGTH + 8 + 16;
// TIMESTAMP_BITS is the number of bits allocated to the millisecond timestamp (0..2^44-1).
pub(crate) const TIMESTAMP_BITS: u64 = 44;
// RANDOM_BITS is the number of bits allocated to the random field per millisecond (0..2^20-1).
pub(crate) const RANDOM_BITS: u64 = 20;
// TIMESTAMP_MASK is the mask for extracting the 44-bit timestamp from a u64 value.
pub(crate) const TIMESTAMP_MASK: u64 = (1 << TIMESTAMP_BITS) - 1;
// MAX_TIMESTAMP is the maximum timestamp value (2^44 - 1).
pub(crate) const MAX_TIMESTAMP: u128 = TIMESTAMP_MASK as u128;
// RANDOM_MASK is the mask for the 20-bit random field.
pub(crate) const RANDOM_MASK: u64 = (1 << RANDOM_BITS) - 1;
// Length of valid nano64 hex string WITHOUT dash
pub(crate) const HEX_STRING_LENGTH: usize = 16;
// Number of bytes that make up a decoded hex string
pub(crate) const HEX_BYTES_LENGTH: usize = 8;

/// Compare compares two IDs as unsigned 64-bit integers.
/// Returns:
///   `Ordering::Less` (if a < b)
///   `Ordering::Equal` (if a == b)
///   `Ordering::Greater` (if a > b)
pub fn compare(a: &Nano64, b: &Nano64) -> Ordering {
    if a.value < b.value {
        return Ordering::Less;
    }
    if a.value > b.value {
        return Ordering::Greater;
    }
    Ordering::Equal
}

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

/// Generate a new monotonic ID using current timestamp and default RNG.
pub fn generate_monotonic() -> Result<Nano64, Nano64Error> {
    generate_monotonic_with_timestamp_and_rng(SystemTime::now(), &mut rand::rng())
}

/// Generate a new monotonic ID using your own timestamp and default RNG.
pub fn generate_monotonic_at(timestamp: SystemTime) -> Result<Nano64, Nano64Error> {
    generate_monotonic_with_timestamp_and_rng(timestamp, &mut rand::rng())
}

/// Generate a new monotonic ID using the current timestamp and your own RNG.
pub fn generate_monotonic_with_rng<RNG>(rng: &mut RNG) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_monotonic_with_timestamp_and_rng(SystemTime::now(), rng)
}

/// Generate a new monotonic ID using your own timestamp and RNG.
pub fn generate_monotonic_with<RNG>(
    timestamp: SystemTime,
    rng: &mut RNG,
) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_monotonic_with_timestamp_and_rng(timestamp, rng)
}

// Private helper to generate ID using provided timestamp and rng
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

    let rand = rng.random::<u64>() & RANDOM_MASK;
    let value = ((ts as u64) << RANDOM_BITS) | rand;

    Ok(Nano64::new(value))
}

// Private helper to generate monotonic ID using provided timestamp and rng
fn generate_monotonic_with_timestamp_and_rng<RNG>(
    timestamp: SystemTime,
    rng: &mut RNG,
) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    let mut ts = timestamp.duration_since(UNIX_EPOCH)?.as_millis();

    if ts > MAX_TIMESTAMP {
        return Err(Nano64Error::TimestampExceedsBitRange(ts));
    }

    let mut refs = MONOTONIC_REFS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());

    // Enforce nondecreasing time
    ts = ts.max(refs.last_timestamp);

    let random = if ts != refs.last_timestamp {
        rng.random::<u64>() & RANDOM_MASK
    } else {
        // Multiple timestamps in same ms
        match refs.last_random.checked_add(1) {
            // Make sure we don't overflow
            Some(rand) if rand <= RANDOM_MASK => rand,
            // Handle overflow
            _ => {
                // Random field exhausted; synthesize a future timestamp.
                ts += 1;
                if ts > MAX_TIMESTAMP {
                    return Err(Nano64Error::TimestampExceedsBitRange(ts));
                }
                0
            }
        }
    };

    refs.last_timestamp = ts;
    refs.last_random = random;

    let value = ((ts as u64) << RANDOM_BITS) | random;
    Ok(Nano64::new(value))
}

// Generates cryptographically secure random `u32` via `rand::rng().random()`
fn generate_random_u32() -> u32 {
    rand::rng().random()
}
