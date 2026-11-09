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

### Basic ID Generation

```rust
use nano64;

let id = nano64::generate().expect("id");

println!("value       = {}", id.value());
println!("timestamp   = {}", id.timestamp());
println!("random      = {}", id.random());
println!("date        = {:?}", id.date());
println!("hex         = {}", id.hex());
println!("bytes       = {:?}", id.to_be_bytes());
// value       = 1881354876126052188
// timestamp   = 1794199825407
// random      = 81756
// date        = SystemTime { tv_sec: 1794199825, tv_nsec: 407000000 }
// hex         = 1A1BEA4ABFF-13F5C
// bytes       = [26, 27, 234, 74, 191, 241, 63, 92]
```

### Monotonic Generation

Ensures strictly increasing values even if created in the same millisecond.

```rust
use nano64;

let a = nano64::generate_monotonic().expect("id");
let b = nano64::generate_monotonic().expect("id");

println!("a = {a:?}");
println!("b = {b:?}");
```

### AES‑GCM Encryption

IDs can easily be encrypted and decrypted to mask their timestamp value from public view.

```rust
use nano64::{self, Nano64Cipher};

// Create 32-byte key (we use AES-256)
let key: [u8; 32] = [
    0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC,
    0xFE, 0x0F, 0x1E, 0x2D, 0x3C, 0x4B, 0x5A, 0x69, 0x78, 0x87, 0x96, 0xA5, 0xB4, 0xC3, 0xD2,
    0xE1, 0xF0,
];

// Create a cipher, this is what allows you to encrypt/decrypt/generate encrypted IDs
let cipher = Nano64Cipher::new(key);

// Generate and encrypt using default RNG and current timestamp
let encrypted_now = cipher.generate_encrypted().expect("id");
println!("encrypted_now = {encrypted_now:?}");
// encrypted_now = EncryptedNano64 { payload: [21, 169, 207, ...] }

let bytes_payload = encrypted_now.to_bytes();
println!("Payload bytes = {bytes_payload:?}");
// Payload bytes = [21, 169, 207, ...]

let hex_payload = encrypted_now.to_hex().to_ascii_uppercase();
println!("72-char hex payload = {hex_payload}");
// 72-char hex payload = 15A9CFD47DEB805A5E61210AEB9BDFE34BF91F019C57E18F83F53CDB1BAEE39E202E8785

// Decrypt later...
let decrypted_now = cipher.decrypt(&encrypted_now).expect("decrypted id");
println!("Decrypted ID = {decrypted_now}");
// Decrypted ID = Nano64{ value=1881354342969611461, timestamp=1794199316949, random=496837 }

// Encrypt existing ID
let existing = nano64::generate().expect("id");
let encrypted_existing = cipher.encrypt(existing).expect("encrypted id");
// Encrypted existing = EncryptedNano64 { payload: [16, 43, 93, ...] }
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

> Nano64 keeps the chronological behavior of ULIDs but in 64 bits instead of 128, cutting key size by half while accepting a lower random-entropy budget.

## Security and Uniqueness Boundaries

Raw Nano64 IDs contain only 20 random bits. They are appropriate for compact, time-sortable database keys where collision risk is understood and handled by normal uniqueness constraints or monotonic generation.

Do **not** use raw Nano64 values as session tokens, password reset tokens, API keys, invitation secrets, CSRF tokens, or other bearer credentials. Use UUIDv4, a larger random token, or another purpose-built secret format for those cases.

The AES-GCM wrapper hides the embedded timestamp, but possession of a valid encrypted ID should never grant access by itself. Treat it as an opaque identifier and enforce authorization separately.

## Database Usage

Nano64 IDs are time-sortable, enabling **index-only time-range queries** without needing a separate timestamp column.

### Time-Based Range Queries

Use `nano64::time_range_to_bytes(start_ms, end_ms)` to obtain the **lowest** and **highest** possible `Nano64` values for a given time window.
These can be used directly in a SQL `BETWEEN` clause to select all rows created within that range.

Using Nano64 as the **primary key** is recommended, since the database’s native index makes these range queries extremely fast.

### Storing Nano64 IDs in SQL

Store IDs as unsigned big-endian bytes using `id.to_be_bytes()` and a byte-ordered column type.

| DBMS        | Column Type | Notes                                                 |
| ----------- | ----------- | ----------------------------------------------------- |
| SQLite      | `BLOB(8)`   | Lexicographic byte order matches unsigned big-endian. |
| PostgreSQL  | `BYTEA(8)`  | Works with primary key indexes.                       |
| MySQL 8+    | `BINARY(8)` | Use binary collation.                                 |
| MariaDB     | `BINARY(8)` | Same as MySQL.                                        |
| SQL Server  | `BINARY(8)` | Clustered index sorts by bytes.                       |
| Oracle      | `RAW(8)`    | Bytewise comparison.                                  |
| CockroachDB | `BYTES(8)`  | Bytewise ordering.                                    |
| DuckDB      | `BLOB(8)`   | Bytewise ordering.                                    |

## API Summary

### ID Generation

`nano64::generate()`

Generates a new ID using the current timestamp and default RNG.

`nano64::generate_at(timestamp)`

Generates a new ID using a provided `SystemTime` and the default RNG.

`nano64::generate_with_rng(rng)`

Generates a new ID using the current timestamp and a custom RNG.

`nano64::generate_with(timestamp, rng)`

Generates a new ID using a provided timestamp and custom RNG.

### Monotonic ID Generation

**Monotonic ID generation is thread-safe!**

`nano64::generate_monotonic()`

Generates a monotonically increasing ID using the current timestamp and default RNG.

`nano64::generate_monotonic_at(timestamp)`

Generates a monotonically increasing ID using a provided timestamp and the default RNG.

`nano64::generate_monotonic_with_rng(rng)`

Generates a monotonically increasing ID using the current timestamp and a custom RNG.

`nano64::generate_monotonic_with(timestamp, rng)`

Generates a monotonically increasing ID using a provided timestamp and custom RNG.

Monotonic generation guarantees strictly increasing IDs, even when multiple IDs are generated within the same millisecond. If all 20-bit random values are exhausted, the timestamp is advanced by 1 ms.

### Nano64

`Nano64::from(value)` / `Nano64::try_from(hex)`

Constructs a Nano64 from a `u64`, an 8-byte big-endian array, or a hexadecimal string.

`id.value()` / `id.to_be_bytes()`

Access the underlying `u64` value or export it as 8 big-endian bytes.

`id.hex()` / `id.to_string()`

Export the ID as a hexadecimal string in `TIMESTAMP-RANDOM` format.

`id.date()` / `id.timestamp()` / `id.random()`

Extract the timestamp as a `SystemTime`, the Unix timestamp in milliseconds, or the 20-bit random component.

`id.equals(other)` / `nano64::compare(a, b)`

Compare two IDs for equality or ordering.

### Encryption

`Nano64Cipher::new(key)`

Creates an AES-256-GCM cipher using a 32-byte key.

`cipher.encrypt(id)` / `cipher.decrypt(encrypted)`

Encrypts a Nano64 ID or decrypts an `EncryptedNano64` payload.

`cipher.generate_encrypted()` / `cipher.generate_encrypted_at(timestamp)`

Generates and encrypts a new ID using the current or provided timestamp.

`cipher.generate_encrypted_with_rng(rng)`

Generates and encrypts a new ID using a custom RNG.

`cipher.generate_encrypted_with(timestamp, rng)`

Generates and encrypts a new ID using a custom timestamp and custom RNG.

### EncryptedNano64

`encrypted.to_bytes()` / `encrypted.to_hex()`

Exports the encrypted payload as bytes or a hexadecimal string.

## Design

| Bits | Field          | Purpose             | Range                 |
| ---- | -------------- | ------------------- | --------------------- |
| 44   | Timestamp (ms) | Chronological order | 1970–2527             |
| 20   | Random         | Collision avoidance | 1,048,576 patterns/ms |

## Benchmark

Run the collision resistance demonstration:

```bash
cargo bench --bench nano64
```

**Benchmark Results:**

The collision resistance test performs four comprehensive scenarios:

1. **Single-threaded high-speed**: 5.3M IDs/sec with 0.29% collisions
2. **Concurrent generation**: ~80M IDs/sec with 0.1642% collisions across 10 threads
3. **Sustained safe rate**: 145k IDs/sec over 10 seconds with <0.05% collisions
4. **Maximum throughput burst**: 4.9M IDs/sec with 0.21% collisions

## Tests

For unit tests run:

```bash
cargo test
```

For doc tests run:

```bash
cargo test --doc
```

## License

MIT License
