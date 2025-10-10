# Nano64 - 64‑bit Time‑Sortable Identifiers for Rust

**Nano64** is a lightweight library for generating time-sortable, globally unique IDs that offer the same practical guarantees as ULID or UUID in half the storage footprint; reducing index and I/O overhead while preserving cryptographic-grade randomness. Includes optional monotonic sequencing and AES-GCM encryption.

[![Crates.io](https://img.shields.io/crates/v/nano64.svg)](https://crates.io/crates/nano64)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

> **Note:** This is a Rust port of the original [Nano64 TypeScript/JavaScript library](https://github.com/only-cliches/nano64) by [@only-cliches](https://github.com/only-cliches). All credit for the original concept, design, and implementation goes to the original author. This port aims to bring the same powerful, compact ID generation capabilities to the Rust ecosystem.
> Also, a huge shout out to the [Go port](https://github.com/Codycody31/go-nano64/)!

## Features

- **Time‑sortable:** IDs order by creation time automatically.
- **Compact:** 8 bytes / 16 hex characters.
- **Deterministic format:** `[63‥20]=timestamp`, `[19‥0]=random`.
- **Cross‑database‑safe:** Big‑endian bytes preserve order in SQLite, Postgres, MySQL, etc.
- **AES-GCM encryption:** Optional encryption masks the embedded creation date.
- **Unsigned canonical form:** Single, portable representation (0..2⁶⁴‑1).

## Installation

```bash
cargo add nano64
```

## Usage

### Basic ID generation

```rust
use nano64::*;

fn main() -> Result<(), Nano64Error> {
    let id = Nano64::generate_default()?;

    println!("{}", id.to_hex()); // 17‑char uppercase hex TIMESTAMP-RANDOM
    // 199CB26E5C1-706DF
    println!("{:?}", id.to_bytes()); // [8]byte
    // [25, 156, 178, 110, 92, 23, 6, 223]
    println!("{}", id.get_timestamp()); // ms since epoch
    // 1760049948097
    Ok(())
}
```

### Monotonic generation

Ensures strictly increasing values even if created in the same millisecond.

```rust
fn main() -> Result<(), Nano64Error> {
    let a = Nano64::generate_monotonic_default()?;
    let b = Nano64::generate_monotonic_default()?;
    println!("{}", nano64::compare(&a, &b)); // -1

    Ok(())
}
```

### AES‑GCM encryption

IDs can easily be encrypted and decrypted to mask their timestamp value from public view.

```rust
fn main() -> Result<(), Nano64Error> {
    // Create 32-byte key (we use AES-256)
    let key: [u8; 32] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC,
        0xFE, 0x0F, 0x1E, 0x2D, 0x3C, 0x4B, 0x5A, 0x69, 0x78, 0x87, 0x96, 0xA5, 0xB4, 0xC3, 0xD2,
        0xE1, 0xF0,
    ];

    let factory = Nano64::encrypted_factory(&key, None, None)?;

    // Generate and encrypt
    let wrapped = factory.generate_encrypted_now()?;
    // or provide your own timestamp
    // let wrapped = factory.generate_encrypted(your_timestamp)?;

    println!("{}", wrapped.id.to_hex()); // Unencrypted ID
    // 199CB349B6C-F84AC
    println!("{}", wrapped.to_encrypted_hex()); // 72-char hex payload
    // D8A385F53E9AC7E13C04CDBA88C52629ED2A3B31422BF474569BBF3E482B7CCCC1605309

    // Decrypt later
    let restored = factory.from_encrypted_hex(wrapped.to_encrypted_hex())?;

    println!("{}", restored.id.u64_value() == wrapped.id.u64_value()); // true

    Ok(())
}
```

## Comparison with other identifiers

| Property               | **Nano64**                                | **ULID**                    | **UUIDv4**              | **Snowflake ID**             |
| ---------------------- | ----------------------------------------- | --------------------------- | ----------------------- | ---------------------------- |
| Bits total             | 64                                        | 128                         | 128                     | 64                           |
| Encoded timestamp bits | 44                                        | 48                          | 0                       | 41                           |
| Random / entropy bits  | 20                                        | 80                          | 122                     | 22 (per-node sequence)       |
| Sortable by time       | ✅ Yes (lexicographic & numeric)          | ✅ Yes                      | ❌ No                   | ✅ Yes                       |
| Collision risk (1%)    | ~145 IDs/ms (~0.04% at 145k/sec)          | ~26M/ms                     | Practically none        | None (central sequence)      |
| Typical string length  | 16 hex chars                              | 26 Crockford base32         | 36 hex+hyphens          | 18–20 decimal digits         |
| Encodes creation time  | ✅                                        | ✅                          | ❌                      | ✅                           |
| Can hide timestamp     | ✅ via AES-GCM encryption                 | ⚠️ Not built-in             | ✅ (no time field)      | ❌ Not by design             |
| Database sort order    | ✅ Stable with big-endian BLOB            | ✅ (lexical)                | ❌ Random               | ✅ Numeric                   |
| Cryptographic strength | 20-bit random, optional AES               | 80-bit random               | 122-bit random          | None (deterministic)         |
| Dependencies           | None (crypto optional)                    | None                        | None                    | Central service or worker ID |
| Target use             | Compact, sortable, optionally private IDs | Human-readable sortable IDs | Pure random identifiers | Distributed service IDs      |

## API Summary

### Generation Functions

- **`generate(timestamp: u64, rng: Option<RandomNumberGeneratorImpl>) -> Result<Nano64, Nano64Error>`** - Creates a new ID with specified timestamp and RNG
- **`generate_now(rng: Option<RandomNumberGeneratorImpl>) -> Result<Nano64, Nano64Error>`** - Creates an ID with current timestamp
- **`generate_default() -> Result<Nano64, Nano64Error>`** - Creates an ID with current timestamp and default RNG
- **`generate_monotonic(timestamp: u64, rng: Option<RandomNumberGeneratorImpl>) -> Result<Nano64, Nano64Error>`** - Creates monotonic ID (strictly increasing)
- **`generate_monotonic_now(rng: Option<RandomNumberGeneratorImpl>) -> Result<Nano64, Nano64Error>`** - Creates monotonic ID with current timestamp
- **`generate_monotonic_default() -> Result<Nano64, Nano64Error>`** - Creates monotonic ID with current timestamp and default RNG

### Parsing Functions

- **`from_hex(hex_str: String) -> Result<Nano64, Nano64Error>`** - Parse from 16-char hex string (with or without dash)
- **`from_bytes(bytes: [u8; 8]) -> Nano64`** - Parse from 8 big-endian bytes
- **`from_u64(value: u64) -> Nano64`** - Create from u64 value
- **`new(value: u64) -> Nano64`** - Create from u64 value (alias)

### ID Methods

- **`to_hex() -> String`** - Returns 17-char uppercase hex (TIMESTAMP-RANDOM)
- **`to_bytes() -> [u8; 8]`** - Returns 8-byte big-endian encoding
- **`to_date() -> SystemTime`** - Converts embedded timestamp to SystemTime
- **`get_timestamp() -> u64`** - Extracts embedded millisecond timestamp
- **`get_random() -> u32`** - Extracts 20-bit random field
- **`u64_value() -> u64`** - Returns raw u64 value

### Comparison Functions

- **`nano64::compare(a: &Nano64, b: &Nano64) -> i64`** - Compare two IDs (-1, 0, 1)
- **`<Nano64_Instance>.equals(other &Nano64) -> bool`** - Check equality

### Database Support

In-progress!

### Encrypted IDs

- **Create factory with 32-byte AES-256 key**
```rust
encrypted_factory(key: &[u8], clock: Option<Clock>, rng: Option<RandomNumberGeneratorImpl>) -> Result<Nano64EncryptionFactory, Nano64Error>
```

- **Generate and encrypt ID**
```rust
factory.generate_encrypted(timestamp: u64) -> Result<Nano64Encrypted, Nano64Error>
```

- **Encrypt existing ID**
```rust
factory.encrypt(id: Nano64) -> Result<Nano64Encrypted, Nano64Error>
```

- **Decrypt from hex**
```rust
factory.from_encrypted_hex(hex: String) -> Result<Nano64Encrypted, Nano64Error> 
```

- **Decrypt from bytes**
```rust
factory.from_encrypted_bytes(bytes: &[u8]) -> Result<Nano64Encrypted, Nano64Error>
```

## Design

| Bits | Field          | Purpose             | Range                 |
| ---- | -------------- | ------------------- | --------------------- |
| 44   | Timestamp (ms) | Chronological order | 1970–2527             |
| 20   | Random         | Collision avoidance | 1,048,576 patterns/ms |

## Benchmark

Run the collision resistance demonstration:

```bash
cargo run --release
```

**Benchmark Results:**

The collision resistance test performs four comprehensive scenarios:

1. **Single-threaded high-speed**: 5.3M IDs/sec with 0.29% collisions
2. **Concurrent generation**: TBD
3. **Sustained safe rate**: 145k IDs/sec over 10 seconds with <0.05% collisions
4. **Maximum throughput burst**: 4.9M IDs/sec with 0.21% collisions

## Tests

Run:

```bash
cargo test
```

All unit tests cover:

- Hex ↔ bytes conversions
- BigInt encoding
- Timestamp extraction and monotonic logic
- AES‑GCM encryption/decryption integrity
- Overflow edge cases

## License

MIT License
