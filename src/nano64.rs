use crate::{NANO64_STRING_LENGTH, Nano64Error, RANDOM_BITS, RANDOM_MASK, TIMESTAMP_MASK, compare};
use hex::FromHex;
use std::{
    cmp::Ordering,
    fmt, str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub struct Nano64 {
    pub(crate) value: u64,
}

impl Nano64 {
    pub(crate) fn new(value: u64) -> Self {
        Self { value }
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn timestamp(&self) -> u64 {
        (self.value >> RANDOM_BITS) & TIMESTAMP_MASK
    }

    /// Get random part from id
    pub fn random(&self) -> u32 {
        (self.value & RANDOM_MASK) as u32
    }

    /// Get underlying vaue as be_bytes
    pub fn to_be_bytes(&self) -> [u8; 8] {
        self.value.to_be_bytes()
    }

    /// Returns ID as hexadecimal string in the following format: TIMESTAMP-RANDOM
    /// For example, given the following nano64 id : `199C01B6659-5861C`,
    /// the TIMESTAMP is `199C01B6659` and the RANDOM is `5861C`.
    pub fn hex(&self) -> String {
        let full = format!("{:016X}", self.value);
        const SPLIT: usize = 11;
        // Format = TIMESTAMP-RANDOM
        format!("{}-{}", &full[..SPLIT], &full[SPLIT..])
    }

    pub fn date(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.timestamp())
    }

    pub fn equals(&self, other: &Nano64) -> bool {
        compare(self, other) == Ordering::Equal
    }
}

impl From<Nano64> for String {
    fn from(nano64: Nano64) -> String {
        nano64.to_string()
    }
}

impl From<Nano64> for u64 {
    fn from(nano64: Nano64) -> Self {
        nano64.value
    }
}

impl From<u64> for Nano64 {
    fn from(value: u64) -> Self {
        Self { value }
    }
}

impl From<[u8; 8]> for Nano64 {
    fn from(be_bytes: [u8; 8]) -> Self {
        Self {
            value: u64::from_be_bytes(be_bytes),
        }
    }
}

// From hex string
impl str::FromStr for Nano64 {
    type Err = Nano64Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut clean = value.replace("-", "");
        if let Some(stripped) = clean
            .strip_prefix("0x")
            .or_else(|| clean.strip_prefix("0X"))
        {
            clean = stripped.to_string();
        }

        if clean.len() != NANO64_STRING_LENGTH {
            return Err(Nano64Error::HexStringInvalidLength(clean.len()));
        }

        let bytes: [u8; 8] = Vec::<u8>::from_hex(&clean)?
            .try_into()
            .map_err(|bytes: Vec<u8>| Nano64Error::IdInvalidBytesLength(bytes.len()))?;

        let value = u64::from_be_bytes(bytes);
        Ok(Self { value })
    }
}

// From hex string
impl TryFrom<&str> for Nano64 {
    type Error = Nano64Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse::<Nano64>()
    }
}

// From hex stringg
impl TryFrom<String> for Nano64 {
    type Error = Nano64Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse::<Nano64>()
    }
}

impl fmt::Display for Nano64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Nano64{{ value={}, timestamp={}, random={} }}",
            self.value,
            self.timestamp(),
            self.random()
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{Nano64, Nano64Error};
    use rand::RngExt;
    use std::{
        assert_matches,
        cmp::Ordering,
        collections::HashSet,
        ops::Sub,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    struct DeterministicRNG;
    impl rand::TryRng for DeterministicRNG {
        type Error = std::convert::Infallible;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Ok(1)
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Ok(1)
        }
        fn try_fill_bytes(&mut self, _dst: &mut [u8]) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    pub fn get_timestamp_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("get_timestamp_now error")
            .as_millis() as u64
    }

    #[test]
    fn echo_test_threads() {
        let env_threads = std::env::var("RUST_TEST_THREADS")
            .unwrap_or_else(|_| "Not explicitly set (defaults to logical CPUs)".to_string());

        println!("\n--- TEST THREAD INFO ---");
        println!("RUST_TEST_THREADS env: {}", env_threads);
        println!("------------------------\n");
    }

    #[test]
    fn test_nano64_new() {
        let _zero = 0;
        let _max = !0u64;
        let _random = 0x123456789ABCDEF0;
        let id_zero = Nano64::new(_zero);
        let id_max = Nano64::new(_max);
        let id_random = Nano64::new(_random);
        assert_eq!(id_zero.value(), _zero);
        assert_eq!(id_max.value(), _max);
        assert_eq!(id_random.value(), _random);
    }

    #[test]
    fn test_nano64_generate() {
        let id = crate::generate().unwrap();
        let timestamp = get_timestamp_now();
        let id_timestamp = id.timestamp();
        let abs_timestamp_diff = ((timestamp as i64) - (id_timestamp as i64)).unsigned_abs();
        let expected_diff_max = 5u64;
        // Test that the timestamp from the id is within 5ms of timestamp of "now"
        assert!(
            abs_timestamp_diff <= expected_diff_max,
            "id.get_timestamp() = {id_timestamp} | timestamp = {timestamp} | expected_max_diff = {expected_diff_max} got {abs_timestamp_diff}"
        );

        // WARNING : this will fail! Need to figure out a way to "predict" the random value
        // let id_random = id.get_random();
        // let expected_random = 1;
        // assert_eq!(
        //     expected_random, id_random,
        //     "expected random = {expected_random} got = {id_random}"
        // );
    }

    #[test]
    fn test_nano64_generate_monotonic() {
        {
            let mut refs = crate::MONOTONIC_REFS.lock().unwrap();
            refs.last_timestamp = 0;
            refs.last_random = 0;
        }

        // Generate id's
        let id_1 = crate::generate_monotonic().expect("monotonic id");
        let id_2 = crate::generate_monotonic().expect("monotonic id");
        // Second id should be greater than first
        assert_eq!(crate::compare(&id_2, &id_1), Ordering::Greater);
        // Both shoulld have same timestamp
        assert_eq!(id_1.timestamp(), id_2.timestamp());
    }

    #[test]
    fn test_exhaust_random_values_in_single_millisecond() {
        let timestamp = UNIX_EPOCH + Duration::from_millis(1_000);
        let mut rng = DeterministicRNG {};
        let mut last_id = crate::generate_monotonic_with(timestamp, &mut rng).unwrap();

        for i in 1..crate::RANDOM_MASK {
            let id = crate::generate_monotonic_with(timestamp, &mut rng).unwrap();
            assert_eq!(
                id.timestamp(),
                1000,
                "iteration {i} (max iters={}) expected 1000, got {}",
                crate::RANDOM_MASK,
                id.timestamp()
            );
            assert!(id > last_id);
            last_id = id;
        }

        // The next ID should advance the timestamp by 1 ms.
        let next_id = crate::generate_monotonic_with(timestamp, &mut rng).unwrap();

        assert_eq!(next_id.timestamp(), 1001);
        assert_eq!(next_id.random(), 0);
    }

    #[test]
    fn test_nano64_to_hex() {
        let _zero = 0;
        let _zero_expect = "00000000000-00000";
        let _max = !0u64;
        let _max_expect = "FFFFFFFFFFF-FFFFF";
        let _example = 0x123456789ABCDEF0;
        let _example_expect = "123456789AB-CDEF0";
        let id_zero = Nano64::new(_zero);
        let id_max = Nano64::new(_max);
        let id_example = Nano64::new(_example);
        assert_eq!(id_zero.hex(), _zero_expect);
        assert_eq!(id_max.hex(), _max_expect);
        assert_eq!(id_example.hex(), _example_expect);
    }

    #[test]
    fn test_nano64_from_hex() {
        struct TestCase {
            name: String,
            hex: String,
            want: u64,
            want_err: bool,
        }

        let test_cases: Vec<TestCase> = vec![
            TestCase {
                name: "zero".into(),
                hex: "00000000000-00000".into(),
                want: 0,
                want_err: false,
            },
            TestCase {
                name: "max".into(),
                hex: "FFFFFFFFFFF-FFFFF".into(),
                want: !0u64,
                want_err: false,
            },
            TestCase {
                name: "example".into(),
                hex: "123456789AB-CDEF0".into(),
                want: 0x123456789ABCDEF0,
                want_err: false,
            },
            TestCase {
                name: "no dash".into(),
                hex: "123456789ABCDEF0".into(),
                want: 0x123456789ABCDEF0,
                want_err: false,
            },
            TestCase {
                name: "lowercase".into(),
                hex: "123456789ab-cdef0".into(),
                want: 0x123456789ABCDEF0,
                want_err: false,
            },
            TestCase {
                name: "0x prefix".into(),
                hex: "0x123456789ABCDEF0".into(),
                want: 0x123456789ABCDEF0,
                want_err: false,
            },
            TestCase {
                name: "invalid length".into(),
                hex: "123".into(),
                want: 0,
                want_err: true,
            },
            TestCase {
                name: "invalid char".into(),
                hex: "123456789AB-CDEFG".into(),
                want: 0,
                want_err: true,
            },
        ];

        for test_case in test_cases {
            match test_case.hex.parse::<Nano64>() {
                Ok(got) => {
                    if test_case.want_err {
                        panic!(
                            "[{}] from_hex() want_err={}, but did not get err",
                            test_case.name, test_case.want_err
                        );
                    }
                    assert_eq!(got.value(), test_case.want);
                }
                Err(e) => {
                    if !test_case.want_err {
                        panic!(
                            "[{}] from_hex() error = {e} | want_err = {}",
                            test_case.name, test_case.want_err
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_nano64_to_bytes_from_bytes() {
        let original = crate::generate().unwrap();
        let bytes = original.to_be_bytes();
        let parsed = Nano64::from(bytes);
        assert_eq!(parsed.value(), original.value());
    }

    #[test]
    fn test_nano64_compare() {
        let mut rng = DeterministicRNG {};
        let now = SystemTime::now();
        let now_minus_200ms = SystemTime::now().sub(Duration::from_millis(200));

        let id_1 = crate::generate_with(now, &mut rng).unwrap();
        let id_2 = crate::generate_with(now_minus_200ms, &mut rng).unwrap();
        let id_3 = crate::generate_with(now, &mut rng).unwrap();

        assert!(crate::compare(&id_1, &id_2) == Ordering::Greater);
        assert!(crate::compare(&id_2, &id_1) == Ordering::Less);
        assert!(crate::compare(&id_1, &id_3) == Ordering::Equal);
    }

    #[test]
    fn test_nano64_equals() {
        let mut rng = DeterministicRNG {};
        let now = SystemTime::now();
        let now_minus_200ms = SystemTime::now().sub(Duration::from_millis(200));

        let id_1 = crate::generate_with(now, &mut rng).unwrap();
        let id_2 = crate::generate_with(now_minus_200ms, &mut rng).unwrap();
        let id_3 = crate::generate_with(now, &mut rng).unwrap();

        assert!(!id_1.equals(&id_2));
        assert!(id_1.equals(&id_3));
    }

    #[test]
    fn test_nano64_string_conversions() {
        let s_slice = "199E4C62AD4-DAEFC";
        let s_string = "199E4C62AD4-DAEFC".to_string();
        let n_1 = s_slice.parse::<Nano64>().unwrap();
        let n_2 = Nano64::try_from(s_slice).unwrap();
        let n_3 = s_string.parse::<Nano64>().unwrap();
        let n_4 = Nano64::try_from(s_string.clone()).unwrap();
        assert!(n_1.equals(&n_2) && n_3.equals(&n_4) && n_1.equals(&n_3) && n_2.equals(&n_4));
    }

    #[test]
    fn test_nano64_to_date() {
        let timestamp = SystemTime::now();
        let id = crate::generate_at(timestamp).expect("id");
        // Need to compare using millisecond precision!
        // Comparing `timestamp` and `id.to_date()` directly is not apples to apples since
        // `timestamp` will have nanosecond precision where id.to_date() will have millisecond precision!
        let id_date_ms = id.date().duration_since(UNIX_EPOCH).unwrap().as_millis();
        let timestamp_ms = timestamp.duration_since(UNIX_EPOCH).unwrap().as_millis();
        assert_eq!(id_date_ms, timestamp_ms);
    }

    #[test]
    fn test_nano64_string() {
        let id = crate::generate().unwrap();
        let str = id.to_string();
        assert_ne!(str, "");
        assert!(str.contains("Nano64"));
    }

    #[test]
    fn test_nano64_from_u64() {
        #[allow(dead_code)]
        struct TestCase {
            name: String,
            value: u64,
        }

        let test_cases: Vec<TestCase> = vec![
            TestCase {
                name: "zero".into(),
                value: 0,
            },
            TestCase {
                name: "small value".into(),
                value: 12345,
            },
            TestCase {
                name: "large value".into(),
                value: 0xFFFFFFFFFFFFFFFF,
            },
        ];

        for tc in test_cases {
            let id = Nano64::from(tc.value);
            assert_eq!(id.value(), tc.value);
        }
    }

    // #[test]
    //fn test_nano64_monotonic_now() {
    // TODO : pretty sure we already test ffor this above!
    //
    // let _guard = get_monotonic_lock_for_tests().lock().unwrap();
    // set_monotonic_refs_to(0, 0);
    // let id_1: Nano64 = match Nano64::generate_monotonic_now(None) {
    //     Ok(got) => got,
    //     Err(e) => panic!("[id_1] did not expect error {e}"),
    // };
    // let id_2 = match Nano64::generate_monotonic_now(None) {
    //     Ok(got) => got,
    //     Err(e) => panic!("[id_2] did not expect error {e}"),
    // };
    // assert!(id_1.value() < id_2.value());
    //}

    #[test]
    fn test_monotonic_race() {
        let min_threads = 5;
        let max_threads = 17;
        let num_ids_to_create = 100_000;
        let num_threads = rand::rng().random_range(min_threads..=max_threads);
        assert!(
            num_threads >= min_threads && num_threads <= max_threads,
            "Expected num_threads to be between {min_threads} and {max_threads} inclusive. Got {num_threads}."
        );
        let ids_per_thread = num_ids_to_create / num_threads;
        let remainder = num_ids_to_create % num_threads;

        let mut handles = Vec::new();

        for i in 0..num_threads {
            // It's impossible for remainder to be > num_threads.
            let total_ids_to_create = if i < remainder {
                ids_per_thread + 1
            } else {
                ids_per_thread
            };

            handles.push(thread::spawn(move || {
                let mut local_ids = Vec::<Nano64>::new();
                for _ in 0..total_ids_to_create {
                    local_ids.push(crate::generate_monotonic().unwrap());
                }
                local_ids
            }));
        }

        let mut global_ids = Vec::<Nano64>::new();

        for handle in handles {
            let mut thread_ids = handle.join().unwrap();
            global_ids.append(&mut thread_ids);
        }

        assert_eq!(
            num_ids_to_create,
            global_ids.len(),
            "Expected {num_ids_to_create} total ids (including duplicates), got {}",
            global_ids.len()
        );

        global_ids.sort_by_key(|id| id.value());

        for pair in global_ids.windows(2) {
            assert!(
                pair[0].value() < pair[1].value(),
                "IDs not strictly increasing"
            );
        }

        let unique_count = global_ids
            .iter()
            .map(|id| id.value())
            .collect::<HashSet<_>>()
            .len();

        assert_eq!(unique_count, global_ids.len(), "Duplicate IDs detected!");
    }

    // TODO : this test
    //
    // #[test]
    // fn test_nano64_monotonic_overflow() {
    //     let _guard = get_monotonic_lock_for_tests().lock().unwrap();
    //     // Set refs to maximums, simulate exhaustion.
    //     set_monotonic_refs_to(RANDOM_MASK, MAX_TIMESTAMP);
    //     if let Ok(got) = Nano64::generate_monotonic(MAX_TIMESTAMP, None) {
    //         panic!(
    //             "`generate_monotonic` called with max timestamp and exhausted random should error but got {got:?}"
    //         );
    //     }
    // }

    #[test]
    #[ignore]
    fn test_nano64_monotonic_backwards_time() {
        // Warm up caches
        for _ in 0..10 {
            _ = crate::generate_monotonic();
        }

        let in_the_past = SystemTime::now().sub(Duration::from_secs(10));
        let id = crate::generate_monotonic_at(in_the_past).unwrap();
        assert!(id.timestamp() >= 1000000);
    }

    #[test]
    fn test_nano64_from_bytes_error() {
        let bytes: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let id = Nano64::from(bytes);
        assert!(id.hex() != "");
    }

    #[test]
    fn test_nano64_from_hex_edge_case_too_short_after_prefix_removal() {
        let result = "0xABCD".parse::<Nano64>();
        assert_matches!(result, Err(Nano64Error::HexStringInvalidLength(_)));
    }

    #[test]
    fn test_nano64_from_hex_edge_case_too_long() {
        let result = "0x00112233445566778899".parse::<Nano64>();
        assert_matches!(result, Err(Nano64Error::HexStringInvalidLength(_)));
    }

    #[test]
    fn test_nano64_monotonic_same_timestamp_increment() {
        let now = SystemTime::now();
        let id_1 = crate::generate_monotonic_at(now).unwrap();
        let id_2 = crate::generate_monotonic_at(now).unwrap();
        assert!(
            crate::compare(&id_2, &id_1) == Ordering::Greater,
            "should increment random field in same ms. id_2 ({}) should be > id_1 ({})",
            id_2.random(),
            id_1.random()
        );
        assert!(
            id_2.random() > id_1.random(),
            "should increment random field in same ms. id_2 ({}) should be > id_1 ({})",
            id_2.random(),
            id_1.random()
        );
    }

    /*
    #[test]
    fn test_nano64_default_rng_bitmask() {
        // Test that 1-bit RNG only returns 0 or 1
        for _ in 0..100 {
            let rng_val = default_rng(1).unwrap();
            assert!(
                rng_val <= 1,
                "default_rng(1) returned {rng_val}, expected 0 or 1"
            );
        }
        // Test that 2-bit RNG only returns 0-3
        for _ in 0..100 {
            let rng_val = default_rng(2).unwrap();
            assert!(
                rng_val <= 3,
                "default_rng(2) returned {rng_val}, expected 0, 1, 2, or 3"
            );
        }
    }
    */

    /*
    *
    * WARNING : PRETTY SURE WE ALREADY TEST FOR THIS
    *
    #[test]
    fn test_nano64_monotonic_default() {
        let _guard = get_monotonic_lock_for_tests().lock().unwrap();
        set_monotonic_refs_to(0, 0);
        let id = match Nano64::generate_monotonic_default() {
            Ok(got) => got,
            Err(e) => panic!("unexpected error {e}"),
        };
        assert_ne!(id.value(), 0);
    }
    */

    /*
    #[test]
    fn test_default_rng() {
        struct TestCase {
            name: String,
            bits: u32,
            want_err: bool,
        }

        let test_cases: Vec<TestCase> = vec![
            TestCase {
                name: "valid 1 bit".into(),
                bits: 1,
                want_err: false,
            },
            TestCase {
                name: "valid 20 bit".into(),
                bits: 20,
                want_err: false,
            },
            TestCase {
                name: "valid 32 bit".into(),
                bits: 32,
                want_err: false,
            },
            TestCase {
                name: "invalid 0 bit".into(),
                bits: 0,
                want_err: true,
            },
            TestCase {
                name: "invalid 33 bit".into(),
                bits: 33,
                want_err: true,
            },
        ];

        for tc in test_cases {
            match default_rng(tc.bits) {
                Ok(got) => {
                    if tc.want_err {
                        panic!(
                            "[{}] default_rng() wanted error but didn't get one. wantErr={}",
                            tc.name, tc.want_err
                        );
                    }
                    let max_val = ((1u64 << tc.bits) - 1) as u32;
                    assert!(got <= max_val);
                }
                Err(e) => {
                    if !tc.want_err {
                        panic!(
                            "[{}] default_rng() error={}, wantErr={}",
                            tc.name, e, tc.want_err
                        );
                    }
                }
            }
        }
    }
    */

    /*
    #[test]
    fn test_generate_errors() {
        struct TestCase {
            name: String,
            timestamp: u64,
            want_err: bool,
        }

        let test_cases: Vec<TestCase> = vec![
            TestCase {
                name: "valid timestamp".into(),
                timestamp: 1234567890123,
                want_err: false,
            },
            TestCase {
                name: "max timestamp".into(),
                timestamp: (1 << TIMESTAMP_BITS) - 1,
                want_err: false,
            },
            TestCase {
                name: "overflow timestamp".into(),
                timestamp: 1 << TIMESTAMP_BITS,
                want_err: true,
            },
        ];

        fn _rng(_bits: u32) -> Result<u32, Nano64Error> {
            Ok(0)
        }

        for tc in test_cases {
            match Nano64::generate(tc.timestamp, Some(_rng)) {
                Ok(_got) => {
                    if tc.want_err {
                        panic!(
                            "[{}] generate() err. want_err={} | wanted error but did not get one",
                            tc.name, tc.want_err
                        );
                    }
                }
                Err(e) => {
                    if !tc.want_err {
                        panic!(
                            "[{}] generate() err. want_err={} | unexpected err={}",
                            tc.name, tc.want_err, e
                        );
                    }
                }
            }
        }
    }
    */
}
