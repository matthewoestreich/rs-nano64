use std::time::{SystemTime, UNIX_EPOCH};

mod encryption_factory;
mod errors;
mod hex;
mod monotonic_refs;
mod nano64;
mod nano64_encrypted;

pub use errors::*;
pub use hex::*;
pub use nano64::*;

pub(crate) use encryption_factory::*;
pub(crate) use nano64_encrypted::*;

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

pub type Clock = fn() -> u64;

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
