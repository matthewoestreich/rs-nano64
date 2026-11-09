//!
//! [Please see our README for more info!](https://github.com/matthewoestreich/rs-nano64)
//!

mod encryption;
mod errors;
mod nano64;

pub use encryption::*;
pub use errors::*;
pub use nano64::*;

use rand::RngExt as _;
use std::{
    cmp::Ordering,
    sync::{LazyLock, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

/// To hold global state for monotonic id generation
pub(crate) struct MonotonicRefs {
    pub(crate) last_timestamp: u128,
    pub(crate) last_random: u64,
}

/// Global monotonic references
pub(crate) static MONOTONIC_REFS: LazyLock<Mutex<MonotonicRefs>> = LazyLock::new(|| {
    Mutex::new(MonotonicRefs {
        last_timestamp: 0,
        last_random: 0,
    })
});

pub(crate) const IV_LENGTH: usize = 12;
pub(crate) const PAYLOAD_LENGTH: usize = IV_LENGTH + 8 + 16;
/// TIMESTAMP_BITS is the number of bits allocated to the millisecond timestamp (0..2^44-1).
pub(crate) const TIMESTAMP_BITS: u64 = 44;
/// TIMESTAMP_MASK is the mask for extracting the 44-bit timestamp from a u64 value.
pub(crate) const TIMESTAMP_MASK: u64 = (1 << TIMESTAMP_BITS) - 1;
/// Number of hexadecimal characters used to encode the timestamp portion of a Nano64 ID.
pub(crate) const TIMESTAMP_HEX_LENGTH: usize = 11;
/// MAX_TIMESTAMP is the maximum timestamp value (2^44 - 1).
pub(crate) const MAX_TIMESTAMP: u128 = TIMESTAMP_MASK as u128;
/// RANDOM_BITS is the number of bits allocated to the random field per millisecond (0..2^20-1).
pub(crate) const RANDOM_BITS: u64 = 20;
/// RANDOM_MASK is the mask for the 20-bit random field.
pub(crate) const RANDOM_MASK: u64 = (1 << RANDOM_BITS) - 1;
/// Length of valid nano64 hex string WITHOUT dash
pub(crate) const NANO64_STRING_LENGTH: usize = 16;
/// Number of bytes that make up an ID
pub(crate) const NANO64_BYTES_LENGTH: usize = 8;

/// Compare compares two IDs as unsigned 64-bit integers.
/// Returns:
///   `Ordering::Less` (if a < b)
///   `Ordering::Equal` (if a == b)
///   `Ordering::Greater` (if a > b)
///
/// # Examples
///
/// ```rust
/// use std::cmp::Ordering;
/// use std::time::{Duration, SystemTime};
/// use std::ops::Sub;
/// use nano64;
///
/// let now = SystemTime::now();
/// let past = SystemTime::now().sub(Duration::from_millis(200));
///
/// let a = nano64::generate_at(now).unwrap();
/// let b = nano64::generate_at(past).unwrap();
///
/// let comparison = nano64::compare(&a, &b);
///
/// assert_eq!(comparison, Ordering::Greater);
/// ```
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
///
/// # Examples
///
/// ```rust
/// #![allow(clippy::absurd_extreme_comparisons, unused_comparisons)]
///
/// use std::time::UNIX_EPOCH;
/// use nano64;
///
/// let id = nano64::generate().unwrap();
///
/// assert!(id.date() > UNIX_EPOCH);
/// assert!(id.timestamp() > 0);
/// assert!(id.random() >= 0);
/// assert!(id.hex() != "");
/// assert!(id.value() >= 0);
/// ```
pub fn generate() -> Result<Nano64, Nano64Error> {
    generate_with_timestamp_and_rng(SystemTime::now(), &mut rand::rng())
}

/// Generate a new ID using your own timestamp and default RNG.
///
/// # Examples
///
/// ```rust
/// use std::time::{Duration, SystemTime};
/// use std::ops::Sub;
/// use nano64;
///
/// let time = SystemTime::now().sub(Duration::from_secs(10));
/// let id = nano64::generate_at(time).unwrap();
/// ```
pub fn generate_at(timestamp: SystemTime) -> Result<Nano64, Nano64Error> {
    generate_with_timestamp_and_rng(timestamp, &mut rand::rng())
}

/// Generate a new ID using the current timestamp and your own RNG.
///
/// # Examples
///
/// ```rust
/// use std::convert::Infallible;
/// use nano64;
/// use rand;
///
/// struct AlwaysOneRNG;
///
/// impl rand::TryRng for AlwaysOneRNG {
///     type Error = Infallible;
///
///     fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_fill_bytes(&mut self, _dst: &mut [u8]) -> Result<(), Self::Error> {
///         Ok(())
///     }
/// }
///
/// let mut rng = AlwaysOneRNG {};
/// let id = nano64::generate_with_rng(&mut rng).unwrap();
/// ```
pub fn generate_with_rng<RNG>(rng: &mut RNG) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_with_timestamp_and_rng(SystemTime::now(), rng)
}

/// Generate a new ID using your own timestamp and RNG.
///
/// # Examples
///
/// ```rust
/// use std::time::{Duration, SystemTime};
/// use std::ops::Sub;
/// use std::convert::Infallible;
/// use nano64;
/// use rand;
///
/// struct AlwaysOneRNG;
///
/// impl rand::TryRng for AlwaysOneRNG {
///     type Error = Infallible;
///
///     fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_fill_bytes(&mut self, _dst: &mut [u8]) -> Result<(), Self::Error> {
///         Ok(())
///     }
/// }
///
/// let time = SystemTime::now().sub(Duration::from_secs(10));
/// let mut rng = AlwaysOneRNG {};
///
/// let id = nano64::generate_with(time, &mut rng).unwrap();
/// ```
pub fn generate_with<RNG>(timestamp: SystemTime, rng: &mut RNG) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_with_timestamp_and_rng(timestamp, rng)
}

/// Generates a strictly increasing ID using the current timestamp and default RNG.
///
/// Monotonic state is shared globally across all calls. If multiple IDs are
/// generated within the same millisecond, the random field is incremented
/// to ensure each ID is greater than the previous one. If the random field
/// is exhausted, the timestamp is advanced by one millisecond.
///
/// # Examples
///
/// ```rust
/// use nano64;
///
/// let first = nano64::generate_monotonic().unwrap();
/// let second = nano64::generate_monotonic().unwrap();
///
/// assert!(nano64::compare(&first, &second).is_lt());
/// ```
pub fn generate_monotonic() -> Result<Nano64, Nano64Error> {
    generate_monotonic_with_timestamp_and_rng(SystemTime::now(), &mut rand::rng())
}

/// Generates a strictly increasing ID using the provided timestamp and default RNG.
///
/// If the provided timestamp is earlier than the last generated timestamp,
/// the generator uses the last timestamp to preserve monotonicity.
///
/// # Examples
///
/// ```rust
/// use std::time::{Duration, SystemTime};
/// use nano64;
///
/// let timestamp = SystemTime::now() + Duration::from_secs(1);
///
/// let first = nano64::generate_monotonic_at(timestamp).unwrap();
/// let second = nano64::generate_monotonic_at(timestamp).unwrap();
///
/// assert!(nano64::compare(&first, &second).is_lt());
/// ```
pub fn generate_monotonic_at(timestamp: SystemTime) -> Result<Nano64, Nano64Error> {
    generate_monotonic_with_timestamp_and_rng(timestamp, &mut rand::rng())
}

/// Generates a strictly increasing ID using the current timestamp and a custom RNG.
///
/// The custom RNG is used when generating a new random field. When multiple
/// IDs are generated within the same millisecond, the random field is
/// incremented instead of generating another random value.
///
/// # Examples
///
/// ```rust
/// use std::convert::Infallible;
/// use nano64;
/// use rand;
///
/// struct AlwaysOneRNG;
///
/// impl rand::TryRng for AlwaysOneRNG {
///     type Error = Infallible;
///
///     fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_fill_bytes(&mut self, _dst: &mut [u8]) -> Result<(), Self::Error> {
///         Ok(())
///     }
/// }
///
/// let mut rng = AlwaysOneRNG;
///
/// let first = nano64::generate_monotonic_with_rng(&mut rng).unwrap();
/// let second = nano64::generate_monotonic_with_rng(&mut rng).unwrap();
///
/// assert!(nano64::compare(&first, &second).is_lt());
/// ```
pub fn generate_monotonic_with_rng<RNG>(rng: &mut RNG) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_monotonic_with_timestamp_and_rng(SystemTime::now(), rng)
}

/// Generates a strictly increasing ID using the provided timestamp and custom RNG.
///
/// If the provided timestamp is earlier than the last generated timestamp,
/// the generator uses the last timestamp to preserve monotonicity. The custom
/// RNG is used when a new random field needs to be generated.
///
/// # Examples
///
/// ```rust
/// use std::convert::Infallible;
/// use std::time::{Duration, SystemTime};
/// use nano64;
/// use rand;
///
/// struct AlwaysOneRNG;
///
/// impl rand::TryRng for AlwaysOneRNG {
///     type Error = Infallible;
///
///     fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
///         Ok(1)
///     }
///
///     fn try_fill_bytes(&mut self, _dst: &mut [u8]) -> Result<(), Self::Error> {
///         Ok(())
///     }
/// }
///
/// let timestamp = SystemTime::now() + Duration::from_secs(1);
/// let mut rng = AlwaysOneRNG;
///
/// let first = nano64::generate_monotonic_with(timestamp, &mut rng).unwrap();
/// let second = nano64::generate_monotonic_with(timestamp, &mut rng).unwrap();
///
/// assert!(nano64::compare(&first, &second).is_lt());
/// ```
pub fn generate_monotonic_with<RNG>(
    timestamp: SystemTime,
    rng: &mut RNG,
) -> Result<Nano64, Nano64Error>
where
    RNG: rand::Rng,
{
    generate_monotonic_with_timestamp_and_rng(timestamp, rng)
}

/// Generates the start and end byte arrays for a database query based on a timestamp range.
///
/// # Examples
///
/// ```rust
///  use nano64;
///
/// let max_timestamp: u64 = 17592186044415;
/// let ts_start = max_timestamp - 10;
/// let ts_end = ts_start + 9;
/// let got = nano64::time_range_to_bytes(ts_start, ts_end).expect("valid range");
/// let expect: [[u8; 8]; 2] = [
///     [255, 255, 255, 255, 255, 80, 0, 0],
///     [255, 255, 255, 255, 255, 239, 255, 255],
/// ];
/// assert_eq!(expect, got);
/// ```
pub fn time_range_to_bytes(
    timestamp_start: u64,
    timestamp_end: u64,
) -> Result<[[u8; NANO64_BYTES_LENGTH]; 2], Nano64Error> {
    let range = range_u64(timestamp_start, timestamp_end)?;
    let range_bytes = [range[0].to_be_bytes(), range[1].to_be_bytes()];
    Ok(range_bytes)
}

/// Crate-visible helper that computes the inclusive u64 bounds for a given millisecond timestamp range.
/// This is a utility for creating database range queries.
///
/// Returns array with 2 elements, where array[0] is the low end of the range
/// and array[1] is the high end of the range.
/// The low and high represent the full range of possible Nano64 IDs
pub(crate) fn range_u64(start: u64, end: u64) -> Result<[u64; 2], Nano64Error> {
    if start as u128 > MAX_TIMESTAMP {
        return Err(Nano64Error::TimestampExceedsBitRange(start as u128));
    }
    if end as u128 > MAX_TIMESTAMP {
        return Err(Nano64Error::TimestampExceedsBitRange(end as u128));
    }
    if start > end {
        return Err(Nano64Error::Custom(format!(
            "Parameter `start` must be less than or equal to parameter `end`. Got : start={start} end={end}"
        )));
    }

    // Start of the range has random bits set to 0.
    let lo = start << RANDOM_BITS;
    // End of the range has random bits set to 1.
    let hi = (end << RANDOM_BITS) | RANDOM_MASK;

    Ok([lo, hi])
}

/// Private helper to generate ID using provided timestamp and rng
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

/// Private helper to generate monotonic ID using provided timestamp and rng
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
