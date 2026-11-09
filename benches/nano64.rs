#![allow(dead_code)]

use rand::RngExt as _;
use std::{
    hint::black_box,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const RANDOM_SPACE: f64 = (1 << 20) as f64;

fn main() {
    run_single_threaded_throughput();
    run_concurrent_multi_threaded_throughput();
    run_single_thread_collision_distribution();
    run_concurrent_multi_threaded_collision_distribution();
    run_sustained_generation();
}

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// I MADE THESE 'run_*' METHODS SO ITS EASIER TO COMMENT OUT
// BENCHMARKS WE DONT WANT TO RUN INSIDE OF 'main'!
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

fn run_single_threaded_throughput() {
    benchmark_single_thread_throughput(SingleThreadConfig {
        ids_per_run: 5_000_000,
        warmup_ids: 100_000,
        runs: 5,
    });
}

fn run_concurrent_multi_threaded_throughput() {
    let mut concurrent_thread_counts = vec![1, 2, 4, 8, 12, 16, 24, 32, 64];

    // Shuffle order we test threads in. Do this 10 times to really shake things up.
    for _ in 0..10 {
        shuffle_vec(&mut concurrent_thread_counts);
    }

    benchmark_concurrent_multi_threaded_throughput(ConcurrentThroughputConfig {
        ids_per_run: 5_000_000, // 100_000_000,
        warmup_ids: 1_000_000,
        runs: 5,
        thread_counts: concurrent_thread_counts,
    });
}

fn run_single_thread_collision_distribution() {
    benchmark_single_thread_collision_distribution(CollisionDistributionConfig {
        ids_to_generate: 10_000_000,
        runs: 5,
    });
}

fn run_concurrent_multi_threaded_collision_distribution() {
    benchmark_concurrent_multi_threaded_collision_distribution(
        ConcurrentCollisionDistributionConfig {
            ids_to_generate: 10_000_000,
            threads: 10,
            runs: 5,
        },
    );
}

fn run_sustained_generation() {
    benchmark_sustained_generation(SustainedGenerationConfig {
        target_ids_per_second: 145_000,
        duration: Duration::from_secs(10),
    });
}

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// BENCHMARK PARAMETER STRUCTS
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

struct BenchmarkRun {
    generated: u64,
    elapsed: Duration,
    ids_per_second: f64,
}

struct CollisionDistributionRun {
    total_ids: u64,
    unique_ids: u64,
    total_collisions: u64,
    milliseconds_occupied: u64,
    max_ids_per_ms: u64,
    max_collisions_per_ms: u64,
    expected_collisions: f64,
    elapsed: std::time::Duration,
}

struct SingleThreadConfig {
    ids_per_run: u64,
    warmup_ids: u64,
    runs: usize,
}

struct ConcurrentThroughputConfig {
    ids_per_run: u64,
    warmup_ids: u64,
    runs: usize,
    thread_counts: Vec<usize>,
}

struct ConcurrentCollisionDistributionConfig {
    ids_to_generate: u64,
    threads: usize,
    runs: usize,
}

struct CollisionDistributionConfig {
    ids_to_generate: u64,
    runs: usize,
}

struct SustainedGenerationConfig {
    target_ids_per_second: u64,
    duration: Duration,
}

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// BENCHMARK FUNCTIONS
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

fn benchmark_single_thread_throughput(config: SingleThreadConfig) {
    assert!(
        config.ids_per_run > 0,
        "ids_per_run must be greater than zero"
    );
    assert!(config.runs > 0, "runs must be greater than zero");

    let ids_per_run = with_commas(config.ids_per_run);

    print_separator();
    println!(
        "=== Single-thread throughput (generate IDs as fast as possible on a single thread) ==="
    );
    println!("  IDs per run: {ids_per_run}");
    println!("  Warmup IDs: {}", with_commas(config.warmup_ids));
    println!("  Runs: {}", config.runs);

    // Warmup
    for _ in 0..config.warmup_ids {
        let id = nano64::generate().unwrap();
        black_box(id.value());
    }

    let mut results = Vec::with_capacity(config.runs);

    // Measured runs
    for run in 1..=config.runs {
        let start = Instant::now();

        for _ in 0..config.ids_per_run {
            let id = nano64::generate().unwrap();
            black_box(id.value());
        }

        let elapsed = start.elapsed();
        let ids_per_second = config.ids_per_run as f64 / elapsed.as_secs_f64();

        let result = BenchmarkRun {
            generated: config.ids_per_run,
            elapsed,
            ids_per_second,
        };

        println!(
            "  Run {run}: {} IDs/sec ({:.2}ms)",
            with_commas(format!("{:.2}", result.ids_per_second)),
            result.elapsed.as_secs_f64() * 1000.0,
        );

        results.push(result);
    }

    // Sort by throughput to calculate median.
    results.sort_by(|a, b| a.ids_per_second.total_cmp(&b.ids_per_second));

    let median = results[results.len() / 2].ids_per_second;
    let min = results.first().unwrap().ids_per_second;
    let max = results.last().unwrap().ids_per_second;

    let median = with_commas(format!("{median:.2}"));
    let min = with_commas(format!("{min:.2}"));
    let max = with_commas(format!("{max:.2}"));

    let ids_per_run = with_commas(config.ids_per_run);

    println!("\n  Results:");
    println!("    Generated per run:  {}", ids_per_run);
    println!("    Median throughput:  {} IDs/sec", median);
    println!("    Minimum throughput: {} IDs/sec", min);
    println!("    Maximum throughput: {} IDs/sec", max);
}

fn benchmark_concurrent_multi_threaded_throughput(config: ConcurrentThroughputConfig) {
    assert!(
        config.ids_per_run > 0,
        "ids_per_run must be greater than zero"
    );
    assert!(config.runs > 0, "runs must be greater than zero");
    assert!(
        !config.thread_counts.is_empty(),
        "thread_counts cannot be empty"
    );
    assert!(
        config.thread_counts.iter().all(|&n| n > 0),
        "thread counts must be greater than zero"
    );

    print_separator();
    println!(
        "=== Concurrent throughput (generate IDs as fast as possible over multiple threads) ==="
    );
    println!("  IDs per run: {}", with_commas(config.ids_per_run));
    println!("  Warmup IDs: {}", with_commas(config.warmup_ids));
    println!("  Runs per thread count: {}", config.runs);
    println!("  Thread counts order: {:?}", config.thread_counts);

    for &num_threads in &config.thread_counts {
        println!("\n  Testing {num_threads} threads:");

        // Warmup
        run_concurrent_generation(config.warmup_ids, num_threads);

        let mut results = Vec::with_capacity(config.runs);

        // Measured runs
        for run in 1..=config.runs {
            let elapsed = run_concurrent_generation(config.ids_per_run, num_threads);

            let ids_per_second = config.ids_per_run as f64 / elapsed.as_secs_f64();

            let result = BenchmarkRun {
                generated: config.ids_per_run,
                elapsed,
                ids_per_second,
            };

            println!(
                "    Run {run}: {} IDs/sec ({:.2}ms)",
                with_commas(format!("{:.2}", result.ids_per_second)),
                result.elapsed.as_secs_f64() * 1000.0,
            );

            results.push(result);
        }

        // Sort by throughput to calculate median.
        results.sort_by(|a, b| a.ids_per_second.total_cmp(&b.ids_per_second));

        let median = results[results.len() / 2].ids_per_second;
        let min = results.first().unwrap().ids_per_second;
        let max = results.last().unwrap().ids_per_second;

        println!("\n    Results:");
        println!(
            "      Generated per run:  {}",
            with_commas(config.ids_per_run)
        );
        println!(
            "      Median throughput:  {} IDs/sec",
            with_commas(format!("{median:.2}"))
        );
        println!(
            "      Minimum throughput: {} IDs/sec",
            with_commas(format!("{min:.2}"))
        );
        println!(
            "      Maximum throughput: {} IDs/sec",
            with_commas(format!("{max:.2}"))
        );
    }
}

fn benchmark_single_thread_collision_distribution(config: CollisionDistributionConfig) {
    assert!(config.ids_to_generate > 0);
    assert!(config.runs > 0);

    print_separator();
    println!("=== Collision distribution (single thread) ===");
    println!("  IDs per run: {}", with_commas(config.ids_to_generate));
    println!("  Runs: {}", config.runs);
    println!("  Threads: 1");

    let mut results = Vec::with_capacity(config.runs);

    for run in 1..=config.runs {
        let mut ids = Vec::with_capacity(config.ids_to_generate as usize);

        let start = Instant::now();

        for _ in 0..config.ids_to_generate {
            let id = nano64::generate().unwrap();

            ids.push((id.timestamp(), id.random()));
            black_box(id.value());
        }

        let elapsed = start.elapsed();

        // Sort by timestamp, then random value.
        ids.sort_unstable();

        let mut unique_ids = 0u64;
        let mut total_collisions = 0u64;
        let mut milliseconds_occupied = 0u64;
        let mut max_ids_per_ms = 0u64;
        let mut max_collisions_per_ms = 0u64;
        let mut expected_collisions = 0.0f64;

        let mut index = 0;

        while index < ids.len() {
            let timestamp = ids[index].0;
            let bucket_start = index;

            // Find the end of this timestamp bucket.
            while index < ids.len() && ids[index].0 == timestamp {
                index += 1;
            }

            let bucket = &ids[bucket_start..index];
            let bucket_size = bucket.len() as u64;

            milliseconds_occupied += 1;
            max_ids_per_ms = max_ids_per_ms.max(bucket_size);

            // Expected number of duplicate random values in this bucket.
            // Formula comes from the birthday paradox.
            expected_collisions +=
                (bucket_size * (bucket_size.saturating_sub(1))) as f64 / (2.0 * RANDOM_SPACE);

            // Count duplicate random values.
            let mut bucket_collisions = 0u64;

            for pair in bucket.windows(2) {
                if pair[0].1 == pair[1].1 {
                    bucket_collisions += 1;
                } else {
                    unique_ids += 1;
                }
            }

            // The final random value in the bucket is unique unless
            // it matches the preceding value.
            if bucket_size > 0
                && (bucket_size == 1 || bucket[bucket.len() - 1].1 != bucket[bucket.len() - 2].1)
            {
                unique_ids += 1;
            }

            total_collisions += bucket_collisions;
            max_collisions_per_ms = max_collisions_per_ms.max(bucket_collisions);
        }

        let result = CollisionDistributionRun {
            total_ids: config.ids_to_generate,
            unique_ids,
            total_collisions,
            milliseconds_occupied,
            max_ids_per_ms,
            max_collisions_per_ms,
            expected_collisions,
            elapsed,
        };

        println!("\n  Run {run}: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!(
            "    Total IDs: _ _ _ _ _ _ _ _ _ _ {}",
            with_commas(result.total_ids)
        );
        println!(
            "    Unique IDs: __ _ _ _ _ _ _ _ _ {}",
            with_commas(result.unique_ids)
        );
        println!(
            "    Expected collisions: _ _ _ _ _ {:.2}",
            result.expected_collisions
        );
        println!(
            "    Actual (Total) collisions: _ _ {}",
            with_commas(result.total_collisions)
        );
        println!(
            "    Actual / expected: _ _ _ _ _ _ {:.2}x",
            result.total_collisions as f64 / result.expected_collisions
        );
        println!(
            "    Expected collision percentage: {:.6}%",
            result.expected_collisions / result.total_ids as f64 * 100.0
        );
        println!(
            "    Actual collision percentage: _ {:.6}%",
            result.total_collisions as f64 / result.total_ids as f64 * 100.0
        );
        println!(
            "    Milliseconds occupied: _ _ _ _ {}",
            with_commas(result.milliseconds_occupied)
        );
        println!(
            "    Average IDs/ms: __ _ _ _ _ _ _ {:.2}",
            result.total_ids as f64 / result.milliseconds_occupied as f64
        );
        println!(
            "    Maximum IDs/ms: __ _ _ _ _ _ _ {}",
            with_commas(result.max_ids_per_ms)
        );
        println!(
            "    Maximum collisions/ms: _ _ _ _ {}",
            with_commas(result.max_collisions_per_ms)
        );

        results.push(result);
    }
}

fn benchmark_concurrent_multi_threaded_collision_distribution(
    config: ConcurrentCollisionDistributionConfig,
) {
    assert!(config.ids_to_generate > 0);
    assert!(config.threads > 0);
    assert!(config.runs > 0);

    print_separator();
    println!("=== Collision Distribution (concurrent, multiple threaded) ===");
    println!("  IDs per run: {}", with_commas(config.ids_to_generate));
    println!("  Threads: {}", config.threads);
    println!("  Runs: {}", config.runs);

    let mut results = Vec::with_capacity(config.runs);

    for run in 1..=config.runs {
        let barrier = Arc::new(Barrier::new(config.threads + 1));
        let mut handles = Vec::with_capacity(config.threads);

        let ids_per_thread = config.ids_to_generate / config.threads as u64;

        let remainder = config.ids_to_generate % config.threads as u64;

        // Spawn workers and distribute the workload.
        for thread_index in 0..config.threads {
            let barrier = Arc::clone(&barrier);

            let count = ids_per_thread + u64::from(thread_index < remainder as usize);

            handles.push(thread::spawn(move || {
                let mut ids = Vec::with_capacity(count as usize);

                // Wait until all workers are ready.
                barrier.wait();

                for _ in 0..count {
                    let id = nano64::generate().unwrap();

                    ids.push((id.timestamp(), id.random()));
                    black_box(id.value());
                }

                ids
            }));
        }

        // Release all workers simultaneously.
        let start = Instant::now();
        barrier.wait();

        // Collect each worker's results.
        let mut thread_results = Vec::with_capacity(config.threads);

        for handle in handles {
            thread_results.push(handle.join().unwrap());
        }

        let elapsed = start.elapsed();

        // Merge results after generation has finished.
        let mut ids = Vec::with_capacity(config.ids_to_generate as usize);

        for thread_ids in thread_results {
            ids.extend(thread_ids);
        }

        // Sort globally so cross-thread collisions are detected too.
        ids.sort_unstable();

        let mut total_collisions = 0u64;
        let mut milliseconds_occupied = 0u64;
        let mut max_ids_per_ms = 0u64;
        let mut max_collisions_per_ms = 0u64;
        let mut expected_collisions = 0.0f64;

        let mut index = 0;

        while index < ids.len() {
            let timestamp = ids[index].0;
            let bucket_start = index;

            // Find the end of this timestamp bucket.
            while index < ids.len() && ids[index].0 == timestamp {
                index += 1;
            }

            let bucket = &ids[bucket_start..index];
            let bucket_size = bucket.len() as u64;

            milliseconds_occupied += 1;
            max_ids_per_ms = max_ids_per_ms.max(bucket_size);

            // Expected collisions using the birthday-problem approximation.
            expected_collisions +=
                (bucket_size * bucket_size.saturating_sub(1)) as f64 / (2.0 * RANDOM_SPACE);

            // Count duplicate random values.
            let bucket_collisions = bucket
                .windows(2)
                .filter(|pair| pair[0].1 == pair[1].1)
                .count() as u64;

            total_collisions += bucket_collisions;

            max_collisions_per_ms = max_collisions_per_ms.max(bucket_collisions);
        }

        let total_ids = ids.len() as u64;
        let unique_ids = total_ids - total_collisions;

        let result = CollisionDistributionRun {
            total_ids,
            unique_ids,
            total_collisions,
            milliseconds_occupied,
            max_ids_per_ms,
            max_collisions_per_ms,
            expected_collisions,
            elapsed,
        };

        println!("\n  Run {run}: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!(
            "    Total IDs: _ _ _ _ _ _ _ _ _ _ {}",
            with_commas(result.total_ids)
        );
        println!(
            "    Unique IDs: __ _ _ _ _ _ _ _ _ {}",
            with_commas(result.unique_ids)
        );
        println!(
            "    Expected collisions: _ _ _ _ _ {:.2}",
            result.expected_collisions
        );
        println!(
            "    Actual (Total) collisions: _ _ {}",
            with_commas(result.total_collisions)
        );
        println!(
            "    Actual / expected: _ _ _ _ _ _ {:.2}x",
            result.total_collisions as f64 / result.expected_collisions
        );
        println!(
            "    Expected collision percentage: {:.6}%",
            result.expected_collisions / result.total_ids as f64 * 100.0
        );
        println!(
            "    Actual collision percentage: _ {:.6}%",
            result.total_collisions as f64 / result.total_ids as f64 * 100.0
        );
        println!(
            "    Milliseconds occupied: _ _ _ _ {}",
            with_commas(result.milliseconds_occupied)
        );
        println!(
            "    Average IDs/ms: __ _ _ _ _ _ _ {:.2}",
            result.total_ids as f64 / result.milliseconds_occupied as f64
        );
        println!(
            "    Maximum IDs/ms: __ _ _ _ _ _ _ {}",
            with_commas(result.max_ids_per_ms)
        );
        println!(
            "    Maximum collisions/ms: _ _ _ _ {}",
            with_commas(result.max_collisions_per_ms)
        );
        println!(
            "    Throughput: __ _ _ _ _ _ _ _ _ {} IDs/sec",
            with_commas(format!(
                "{:.2}",
                result.total_ids as f64 / result.elapsed.as_secs_f64()
            ))
        );

        results.push(result);
    }
}

fn benchmark_sustained_generation(config: SustainedGenerationConfig) {
    assert!(config.target_ids_per_second > 0);

    print_separator();
    println!("=== Sustained Generation (sustained throttle of per second ID creation) ===");
    println!(
        "Target rate: {} IDs/sec",
        with_commas(config.target_ids_per_second)
    );
    println!("Duration: {:.2}s", config.duration.as_secs_f64());

    let mut ids: Vec<(u64, u32)> = Vec::new();

    let start = Instant::now();
    let end = start + config.duration;
    let mut generated = 0u64;

    // Generate IDs according to a fixed per-ID schedule.
    while Instant::now() < end {
        let next_deadline = start
            + Duration::from_nanos(
                (generated as u128 * 1_000_000_000 / config.target_ids_per_second as u128) as u64,
            );

        while Instant::now() < next_deadline {
            std::hint::spin_loop();
        }

        if Instant::now() >= end {
            break;
        }

        let id = nano64::generate().unwrap();

        ids.push((id.timestamp(), id.random()));
        black_box(id.value());

        generated += 1;
    }

    let elapsed = start.elapsed();

    // Sort by timestamp, then random value.
    // Collision analysis is deliberately outside the timed generation loop.
    ids.sort_unstable();

    let mut total_collisions = 0u64;
    let mut expected_collisions = 0.0;
    let mut occupied_timestamps = 0u64;
    let mut max_ids_per_ms = 0u64;
    let mut max_collisions_per_ms = 0u64;

    let mut i = 0;

    while i < ids.len() {
        let timestamp = ids[i].0;
        let mut j = i + 1;

        while j < ids.len() && ids[j].0 == timestamp {
            j += 1;
        }

        let bucket_size = (j - i) as u64;
        let mut collisions = 0u64;

        // Count duplicate random values within this millisecond.
        for k in (i + 1)..j {
            if ids[k].1 == ids[k - 1].1 {
                collisions += 1;
            }
        }

        total_collisions += collisions;
        expected_collisions += bucket_size as f64 * (bucket_size - 1) as f64 / (2.0 * RANDOM_SPACE);

        occupied_timestamps += 1;
        max_ids_per_ms = max_ids_per_ms.max(bucket_size);
        max_collisions_per_ms = max_collisions_per_ms.max(collisions);

        i = j;
    }

    let unique_ids = generated - total_collisions;
    let collision_percentage = total_collisions as f64 / generated as f64 * 100.0;

    let ids_per_second = generated as f64 / elapsed.as_secs_f64();
    let average_ids_per_ms = generated as f64 / occupied_timestamps as f64;

    println!("\nResults:");
    println!("  Generated IDs:          {}", with_commas(generated));
    println!("  Unique IDs:             {}", with_commas(unique_ids));
    println!("  Expected collisions:    {:.2}", expected_collisions);
    println!(
        "  Actual collisions:      {}",
        with_commas(total_collisions)
    );
    println!("  Collision percentage:   {:.4}%", collision_percentage);
    println!(
        "  Actual / expected:      {:.2}x",
        total_collisions as f64 / expected_collisions
    );

    println!("\nTiming:");
    println!("  Elapsed:                {:.3}s", elapsed.as_secs_f64());
    println!(
        "  Actual throughput:      {} IDs/sec",
        with_commas(format!("{:.2}", ids_per_second))
    );

    println!("\nTimestamp Distribution:");
    println!(
        "  Occupied milliseconds:  {}",
        with_commas(occupied_timestamps)
    );
    println!("  Average IDs/ms:         {:.2}", average_ids_per_ms);
    println!("  Maximum IDs/ms:         {}", with_commas(max_ids_per_ms));
    println!(
        "  Maximum collisions/ms:  {}",
        with_commas(max_collisions_per_ms)
    );
}

// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------
// HELPER FUNCTIONS
// ----------------------------------------------------------------------------------------------------
// ----------------------------------------------------------------------------------------------------

fn shuffle_vec<T>(values: &mut [T]) {
    let mut rng = rand::rng();

    for i in (1..values.len()).rev() {
        let j = rng.random_range(0..=i);
        values.swap(i, j);
    }
}

fn run_concurrent_generation(total_ids: u64, num_threads: usize) -> Duration {
    let barrier = Arc::new(Barrier::new(num_threads + 1));
    let mut handles = Vec::with_capacity(num_threads);

    let ids_per_thread = total_ids / num_threads as u64;
    let remainder = total_ids % num_threads as u64;

    for thread_index in 0..num_threads {
        let barrier = Arc::clone(&barrier);

        let count = ids_per_thread + u64::from(thread_index < remainder as usize);

        handles.push(thread::spawn(move || {
            // Wait until all workers are ready.
            barrier.wait();

            for _ in 0..count {
                let id = nano64::generate().unwrap();
                black_box(id.value());
            }
        }));
    }

    // Start timing immediately before releasing the workers.
    let start = Instant::now();

    barrier.wait();

    for handle in handles {
        handle.join().unwrap();
    }

    start.elapsed()
}

fn with_commas<T: std::fmt::Display>(value: T) -> String {
    let value = value.to_string();
    let (integer, decimal) = value.split_once('.').unwrap_or((&value, ""));

    let formatted = integer
        .chars()
        .rev()
        .enumerate()
        .fold(String::new(), |mut result, (i, c)| {
            if i > 0 && i % 3 == 0 {
                result.push(',');
            }
            result.push(c);
            result
        })
        .chars()
        .rev()
        .collect::<String>();

    if decimal.is_empty() {
        formatted
    } else {
        format!("{formatted}.{decimal}")
    }
}

fn print_separator() {
    println!("\n{}\n", "-".repeat(200));
}
