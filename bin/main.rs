use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use nano64::Nano64;

fn main() {
    println!("Testing high speedd generation:");
    test_high_speed_generation(5_000_000);
    println!("\nTesting sustained rate:");
    test_sustained_rate(145000, Duration::from_secs(10));
    println!("\nTesting max throughput [1s burst]:");
    test_max_throughput(Duration::from_secs(1));
}

fn test_high_speed_generation(count: u64) {
    let mut seen = HashSet::<u64>::new();
    let mut collisions = 0;
    let start = Instant::now();

    for _ in 0..count {
        let id = Nano64::generate_default().unwrap();
        let value = id.u64_value();

        if !seen.insert(value) {
            collisions += 1;
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let rate = count as f64 / elapsed;
    let unique_ids = seen.len() as f64;
    let collision_prob = collisions as f64 / unique_ids * 100.0;

    println!("  Generated {count} IDs");
    println!("  Time: {elapsed:.6}s");
    println!("  Rate: {rate:.2} IDs/Second");
    println!("  Collisions: {collisions}");
    println!("  Collision probability: {collision_prob:.6}%");
}

fn test_sustained_rate(target_rate: u64, duration: Duration) {
    let mut seen: HashMap<u64, bool> = HashMap::new();
    let mut collisions = 0u64;
    let mut total_generated = 0u64;

    let start = Instant::now();
    let deadline = start + duration;

    // Track per-millisecond statistics
    let mut ms_stats: HashMap<u64, u64> = HashMap::new();
    let mut max_per_ms = 0u64;

    // Batch generation parameters
    let batch_size = 1000;
    let batch_interval = Duration::from_secs_f64(batch_size as f64 / target_rate as f64);

    while Instant::now() < deadline {
        let batch_start = Instant::now();

        for _ in 0..batch_size {
            if Instant::now() >= deadline {
                break;
            }

            // Generate ID
            let id = Nano64::generate_default().unwrap();

            let value = id.u64_value();

            if seen.contains_key(&value) {
                collisions += 1;
            }
            seen.insert(value, true);
            total_generated += 1;

            let timestamp = id.get_timestamp();
            let count = ms_stats.entry(timestamp).or_insert(0);
            *count += 1;
            if *count > max_per_ms {
                max_per_ms = *count;
            }
        }

        // Sleep to maintain target rate
        let elapsed = batch_start.elapsed();
        if batch_interval > elapsed {
            std::thread::sleep(batch_interval - elapsed);
        }
    }

    let elapsed = start.elapsed();
    let actual_rate = total_generated as f64 / elapsed.as_secs_f64();

    println!("  Target Rate: {target_rate} IDs/second");
    println!("  Duration: {duration:?}");
    println!("  Generated: {total_generated}");
    println!("  Actual Rate: {actual_rate:.0} IDs/second");
    println!(
        "  Collisions: {} ({:.6}%)",
        collisions,
        collisions as f64 / total_generated as f64 * 100.0
    );
    println!("  Unique IDs: {}", seen.len());
    println!("  Max IDs in single millisecond: {max_per_ms}");
    println!("  Milliseconds with IDs: {}", ms_stats.len());
}

fn test_max_throughput(duration: Duration) {
    let mut seen = HashMap::<u64, bool>::new();
    let mut collisions = 0;
    let mut total_generated = 0;

    let mut ms_stats = HashMap::<u64, u64>::new();
    let mut collisions_per_ms = HashMap::<u64, u64>::new();
    let mut max_per_ms = 0u64;

    let start = Instant::now();
    let deadline = start + duration;

    while Instant::now() < deadline {
        let id = match Nano64::generate_default() {
            Ok(id) => id,
            Err(_) => continue,
        };

        let value = id.u64_value();
        let timestamp = id.get_timestamp();

        if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(value) {
            e.insert(true);
        } else {
            collisions += 1;
            *collisions_per_ms.entry(timestamp).or_insert(0) += 1;
        }

        total_generated += 1;
        let count = ms_stats.entry(timestamp).or_insert(0);
        *count += 1;
        if *count > max_per_ms {
            max_per_ms = *count;
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let rate = total_generated as f64 / elapsed;

    let (max_collision_ms, max_collisions) = collisions_per_ms.iter().fold(
        (0u64, 0u64),
        |acc, (&ms, &count)| {
            if count > acc.1 { (ms, count) } else { acc }
        },
    );

    println!("Duration: {duration:?}");
    println!("Generated: {total_generated}");
    println!("Rate: {rate:.2} IDs/second");
    println!(
        "Collisions: {} ({:.6}%)",
        collisions,
        collisions as f64 / total_generated as f64 * 100.0
    );
    println!("Unique IDs: {}", seen.len());
    println!("Max IDs in single millisecond: {max_per_ms}");
    println!("Milliseconds with IDs: {}", ms_stats.len());
    if max_collisions > 0 {
        println!(
            "Max collisions in single ms: {} (at timestamp {}, had {} IDs)",
            max_collisions,
            max_collision_ms,
            ms_stats.get(&max_collision_ms).unwrap_or(&0)
        );
    }
}
