use std::{
    collections::{HashMap, HashSet},
    thread,
    time::{Duration, Instant},
};

use nano64::*;

fn main() {
    let high_speed_count = 10_000_000;
    let max_throughput_duration = Duration::from_millis(1000);
    let sustained_rate_count = 145_000;
    let sustained_rate_duration = Duration::from_millis(10_000);

    println!("\nTesting max throughput [{max_throughput_duration:?} burst]:");
    test_max_throughput(max_throughput_duration);

    println!(
        "\nTesting high speed generation: Generating {} IDs as fast as possible.",
        with_commas(high_speed_count)
    );
    test_high_speed_generation(high_speed_count);

    println!(
        "\nTesting sustained rate: {} IDs/sec for {sustained_rate_duration:?}",
        with_commas(sustained_rate_count)
    );
    test_sustained_rate(sustained_rate_count, sustained_rate_duration);
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
    let rate = format!("{:.2}", (seen.len() + collisions) as f64 / elapsed);
    let unique_ids = seen.len() as f64;
    let collision_prob = collisions as f64 / unique_ids * 100.0;

    println!("  Generated {} IDs", with_commas(count));
    println!("  Time: {:.6}ms", with_commas(start.elapsed().as_millis()));
    println!("  Rate: {} IDs/Second", with_commas(rate));
    println!("  Collisions: {}", with_commas(collisions));
    println!(
        "  Collision probability: {:.6}%",
        with_commas(collision_prob)
    );
}

// Will try to rate limit to `target_rate` id's per second, over `duration` time period.
fn test_sustained_rate(target_rate: u64, duration: Duration) {
    let mut seen = HashSet::<u64>::new();
    let mut collisions = 0;
    let mut ms_stats = HashMap::<u64, u64>::new();
    let mut max_per_ms = 0u64;
    let start = Instant::now();
    let mut next_tick = start;
    let interval = Duration::from_secs_f64(1.0 / target_rate as f64);

    while Instant::now() - start < duration {
        let id = Nano64::generate_default().unwrap();
        let value = id.u64_value();
        let timestamp = id.get_timestamp();

        if !seen.insert(value) {
            collisions += 1;
        }

        let count = ms_stats.entry(timestamp).or_insert(0);
        *count += 1;
        if *count > max_per_ms {
            max_per_ms = *count;
        }

        // Schedule next tick (rate limit)
        next_tick += interval;
        let now = Instant::now();
        if next_tick > now {
            let sleep_time = next_tick - now;
            if sleep_time > Duration::from_micros(200) {
                thread::sleep(sleep_time - Duration::from_micros(100));
            }
            while Instant::now() < next_tick {}
        }
    }

    let elapsed = start.elapsed();
    let total_generated = seen.len() + collisions;
    let actual_rate = format!("{:.2}", total_generated as f64 / elapsed.as_secs_f64());

    println!("  Target Rate: {} IDs/s", with_commas(target_rate));
    println!("  Duration: {duration:?}");
    println!("  Generated: {}", with_commas(total_generated));
    println!("  Actual Rate: {} IDs/s", with_commas(actual_rate));
    println!(
        "  Collisions: {} ({:.6}%)",
        with_commas(collisions),
        with_commas(collisions as f64 / total_generated as f64 * 100.0)
    );
    println!("  Unique IDs: {}", with_commas(seen.len()));
    println!("  Max IDs in a single ms: {}", with_commas(max_per_ms));
    println!("  Milliseconds with IDs: {}", with_commas(ms_stats.len()));
}

fn test_max_throughput(duration: Duration) {
    let mut seen = HashSet::<u64>::new();
    let mut collisions = 0;
    let mut ids_per_ms = HashMap::<u64, u64>::new();

    const TIME_CHECK_INTERVAL: u64 = 1000;
    let start = Instant::now();

    loop {
        // Generate in a tight loop.
        // Try to keep calls to Instant.now() to a min (checking if `Instant::now() < start+duration` is expensive)
        for _ in 0..TIME_CHECK_INTERVAL {
            let id = Nano64::generate_default().unwrap();
            let value = id.u64_value();
            let timestamp = id.get_timestamp();
            if !seen.insert(value) {
                collisions += 1;
            }
            *ids_per_ms.entry(timestamp).or_insert(0) += 1;
        }
        // Only check time every TIME_CHECK_INTERVAL iterations.
        // This keeps expensive calls to a min.
        if start.elapsed() >= duration {
            break;
        }
    }

    let elapsed = start.elapsed();

    // Sort timestamps by value (highest to lowest).
    let mut sorted_ids_per_ms: Vec<(&u64, &u64)> = ids_per_ms.iter().collect();
    sorted_ids_per_ms.sort_by(|a, b| b.1.cmp(a.1));

    let total_generated_ids = seen.len() + collisions;
    let timestamp_with_most_ids = sorted_ids_per_ms[0];
    let timestamp_with_fewest_ids = sorted_ids_per_ms[sorted_ids_per_ms.len() - 1];
    let collision_prob = collisions as f64 / (seen.len() as f64) * 100.0;
    let rate = format!("{:.2}", total_generated_ids as f64 / elapsed.as_secs_f64());

    println!("  Duration : {:.6}ms", with_commas(elapsed.as_millis()));
    println!("  Rate : {} IDs/sec", with_commas(rate));
    println!(
        "  Total Generated IDs : {}",
        with_commas(total_generated_ids)
    );
    println!("  Unique IDs : {}", with_commas(seen.len()));
    println!(
        "  Collisions : {} ({:.6}%)",
        with_commas(collisions),
        with_commas(collision_prob)
    );
    println!("  Most IDs in a single ms (timestamp, count) : {timestamp_with_most_ids:?}");
    println!("  Fewest IDs in a single ms (timestamp, count) : {timestamp_with_fewest_ids:?}");
}

fn with_commas<T: ToString>(value: T) -> String {
    let s = value.to_string();
    let parts: Vec<&str> = s.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1).map(|d| format!(".{d}")).unwrap_or_default();
    #[allow(clippy::manual_strip)]
    let (sign, integer_digits) = if integer_part.starts_with('-') {
        (&integer_part[..1], &integer_part[1..])
    } else {
        ("", integer_part)
    };
    let mut result = String::new();
    let digits = integer_digits.chars().rev().enumerate();
    for (i, c) in digits {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    format!(
        "{}{}{}",
        sign,
        result.chars().rev().collect::<String>(),
        decimal_part
    )
}
