use crate::{
    ClockImpl, Hex, MAX_TIMESTAMP, Nano64EncryptionFactory, Nano64Error, RANDOM_BITS, RANDOM_MASK,
    RandomNumberGeneratorImpl, TIMESTAMP_MASK, TIMESTAMP_SHIFT, compare, default_rng,
    monotonic_refs::*, time_now_since_epoch_ms,
};
use std::{
    fmt, str,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Debug)]
pub struct Nano64 {
    pub(crate) value: u64,
}

impl Default for Nano64 {
    fn default() -> Self {
        Self {
            value: time_now_since_epoch_ms(),
        }
    }
}

impl From<Nano64> for String {
    fn from(n64: Nano64) -> String {
        n64.to_string()
    }
}

impl From<Nano64> for u64 {
    fn from(n64: Nano64) -> Self {
        n64.value
    }
}

impl From<u64> for Nano64 {
    fn from(value: u64) -> Self {
        Self { value }
    }
}

impl From<[u8; 8]> for Nano64 {
    fn from(bytes: [u8; 8]) -> Self {
        Self {
            value: u64::from_be_bytes(bytes),
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

        if clean.len() != 16 {
            return Err(Nano64Error::Error(format!(
                "hex must be 16 chars after removing dash, got {}",
                clean.len()
            )));
        }

        let bytes_vec = Hex::to_bytes(&clean)?;
        if bytes_vec.len() != 8 {
            return Err(Nano64Error::Error(format!(
                "hex must decode to 8 bytes, got {}",
                bytes_vec.len()
            )));
        }

        let bytes: [u8; 8] = bytes_vec
            .try_into()
            .map_err(|_| Nano64Error::Error("hex must decode to exactly 8 bytes".into()))?;

        let value = u64::from_be_bytes(bytes);
        Ok(Self { value })
    }
}

// From hex string
impl TryFrom<&'_ str> for Nano64 {
    type Error = Nano64Error;

    fn try_from(s: &'_ str) -> Result<Self, Self::Error> {
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
            "Nano64{{value={}, timestamp={}, random={}}}",
            self.value,
            self.get_timestamp(),
            self.get_random()
        )
    }
}

impl Nano64 {
    pub fn new(value: u64) -> Self {
        Self { value }
    }

    pub fn generate_default() -> Result<Self, Nano64Error> {
        Self::generate_now(Some(default_rng))
    }

    pub fn generate_now(rng: Option<RandomNumberGeneratorImpl>) -> Result<Self, Nano64Error> {
        Self::generate(time_now_since_epoch_ms(), rng)
    }

    pub fn generate_monotonic_now(
        rng: Option<RandomNumberGeneratorImpl>,
    ) -> Result<Self, Nano64Error> {
        Self::generate_monotonic(time_now_since_epoch_ms(), rng)
    }

    pub fn generate_monotonic_default() -> Result<Self, Nano64Error> {
        Self::generate_monotonic_now(Some(default_rng))
    }

    pub fn encrypted_factory(
        key: &[u8],
        clock: Option<ClockImpl>,
        rng: Option<RandomNumberGeneratorImpl>,
    ) -> Result<Nano64EncryptionFactory, Nano64Error> {
        return Nano64EncryptionFactory::new(key, clock, rng);
    }

    pub fn get_timestamp(&self) -> u64 {
        (self.value >> TIMESTAMP_SHIFT) & TIMESTAMP_MASK
    }

    pub fn get_random(&self) -> u32 {
        (self.value & RANDOM_MASK) as u32
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        self.value.to_be_bytes()
    }

    pub fn to_hex(&self) -> String {
        let full = format!("{:016X}", self.value);
        const SPLIT: usize = 11;
        format!("{}-{}", &full[..SPLIT], &full[SPLIT..])
    }

    pub fn to_date(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(self.get_timestamp())
    }

    pub fn u64_value(&self) -> u64 {
        self.value
    }

    pub fn equals(&self, other: &Nano64) -> bool {
        compare(self, other) == 0
    }

    pub(crate) fn generate(
        timestamp: u64,
        rng: Option<RandomNumberGeneratorImpl>,
    ) -> Result<Self, Nano64Error> {
        if timestamp > MAX_TIMESTAMP {
            return Err(Nano64Error::TimeStampExceedsBitRange(timestamp));
        }

        let rng = if let Some(_rng) = rng {
            _rng
        } else {
            default_rng
        };

        let random_value = rng(RANDOM_BITS as u32)?;
        let ms = timestamp & TIMESTAMP_MASK;
        let random = (random_value as u64) & RANDOM_MASK;
        let value = (ms << TIMESTAMP_SHIFT) | random;

        Ok(Self { value })
    }

    pub(crate) fn generate_monotonic(
        timestamp: u64,
        rng: Option<RandomNumberGeneratorImpl>,
    ) -> Result<Self, Nano64Error> {
        if timestamp > MAX_TIMESTAMP {
            return Err(Nano64Error::TimeStampExceedsBitRange(timestamp));
        }

        let rng = if let Some(_rng) = rng {
            _rng
        } else {
            default_rng
        };

        let monotonic_refs = get_monotonic_refs();
        let mut refs = monotonic_refs
            .lock()
            .map_err(|_| Nano64Error::Error("Error unlocking refs".into()))?;

        // Enforce nondecreasing time
        let mut ts = timestamp;
        if ts < refs.last_timestamp {
            ts = refs.last_timestamp;
        }

        let random: u64;
        if ts == refs.last_timestamp {
            // Same ms → increment
            random = (refs.last_random + 1) & RANDOM_MASK;
            if random == 0 {
                ts += 1;
                if ts > MAX_TIMESTAMP {
                    return Err(Nano64Error::Error(
                        "timestamp overflow after incrementing for monotonic generation".into(),
                    ));
                }
                refs.last_timestamp = ts;
                refs.last_random = 0;
                let ms = ts & TIMESTAMP_MASK;
                let value = ms << TIMESTAMP_SHIFT;
                return Ok(Self { value });
            }
        } else {
            let random_value = rng(RANDOM_BITS as u32)?;
            random = (random_value as u64) & RANDOM_MASK;
        }

        refs.last_timestamp = ts;
        refs.last_random = random;
        let ms = ts & TIMESTAMP_MASK;
        let value = (ms << TIMESTAMP_SHIFT) | random;
        return Ok(Self { value });
    }
}

#[cfg(test)]
mod tests {

    use std::{
        collections::HashSet,
        sync::{Mutex, OnceLock},
        thread,
        time::UNIX_EPOCH,
    };

    use rand::Rng;

    use crate::{
        Nano64, Nano64Error, RANDOM_BITS, TIMESTAMP_BITS, compare, default_rng,
        monotonic_refs::get_monotonic_refs,
        nano64::{MAX_TIMESTAMP, RANDOM_MASK},
        time_now_since_epoch_ms,
    };

    // Rust tests run concurrently by default. Some tests reset or manipulate the global
    // monotonic refs to produce predictable results. Without coordination, these tests
    // can interfere with each other, causing failures that would not occur in normal usage.
    // This lock ensures only one test at a time can access or modify the global monotonic refs.
    static MONOTONIC_LOCK_FOR_TESTS: OnceLock<Mutex<()>> = OnceLock::new();
    fn get_monotonic_lock_for_tests() -> &'static Mutex<()> {
        MONOTONIC_LOCK_FOR_TESTS.get_or_init(|| Mutex::new(()))
    }

    fn set_monotonic_refs_to(last_random: u64, last_timestamp: u64) {
        let monotonic_refs = get_monotonic_refs();
        let mut refs = monotonic_refs.lock().unwrap();
        refs.last_random = last_random;
        refs.last_timestamp = last_timestamp;
    }

    fn acquire_monotonic_test_lock(func: fn()) {
        let _guard = get_monotonic_lock_for_tests().lock().unwrap();
        func();
    }

    #[test]
    fn test_nano64_new() {
        let _zero = 0;
        let _max = !0u64;
        let _random = 0x123456789ABCDEF0;
        let id_zero = Nano64::new(_zero);
        let id_max = Nano64::new(_max);
        let id_random = Nano64::new(_random);
        assert_eq!(id_zero.u64_value(), _zero);
        assert_eq!(id_max.u64_value(), _max);
        assert_eq!(id_random.u64_value(), _random);
    }

    #[test]
    fn test_nano64_generate() {
        let timestamp: u64 = 1234567890123;
        let expected_random = 0x12345;
        fn rng(_bits: u32) -> Result<u32, Nano64Error> {
            Ok(0x12345) // Same as expected_random!
        }
        let id = Nano64::generate(timestamp, Some(rng)).unwrap();
        assert_eq!(id.get_timestamp(), timestamp);
        assert_eq!(id.get_random(), expected_random);
    }

    #[test]
    fn test_nano64_generate_default() {
        let id = Nano64::generate_default().unwrap();
        let now = time_now_since_epoch_ms();
        // check timestamp is recent (within last min)
        let timestamp = id.get_timestamp();
        assert!((timestamp > (now - 60000)) || (timestamp < (now + 1000)));
        let random = id.get_random();
        assert!(random < (1 << RANDOM_BITS));
    }

    #[test]
    fn test_nano64_generate_monotonic() {
        acquire_monotonic_test_lock(test);
        fn test() {
            let timestamp: u64 = 1234567890123;
            fn _rng(_bits: u32) -> Result<u32, Nano64Error> {
                Ok(0x12345)
            }
            // Generate id's
            let id_1 = Nano64::generate_monotonic(timestamp, Some(_rng)).unwrap();
            let id_2 = Nano64::generate_monotonic(timestamp, Some(_rng)).unwrap();
            // Second id should be greater than first
            assert!(compare(&id_2, &id_1) >= 0);
            // both shoulld have same timestamp
            assert_eq!(id_1.get_timestamp(), id_2.get_timestamp());
        }
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
        assert_eq!(id_zero.to_hex(), _zero_expect);
        assert_eq!(id_max.to_hex(), _max_expect);
        assert_eq!(id_example.to_hex(), _example_expect);
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

        for tc in test_cases {
            match tc.hex.parse::<Nano64>() {
                Ok(got) => {
                    if tc.want_err {
                        panic!(
                            "[{}] from_hex() want_err={}, but did not get err",
                            tc.name, tc.want_err
                        );
                    }
                    assert_eq!(got.u64_value(), tc.want);
                }
                Err(e) => {
                    if !tc.want_err {
                        panic!(
                            "[{}] from_hex() error = {e} | want_err = {}",
                            tc.name, tc.want_err
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_nano64_to_bytes_from_bytes() {
        let original = Nano64::new(0x123456789ABCDEF0);
        let bytes = original.to_bytes();
        let parsed = Nano64::from(bytes);
        assert_eq!(parsed.u64_value(), original.u64_value());
    }

    #[test]
    fn test_nano64_compare() {
        let id_1 = Nano64::new(100);
        let id_2 = Nano64::new(200);
        let id_3 = Nano64::new(100);
        assert!(compare(&id_1, &id_2) == -1);
        assert!(compare(&id_2, &id_1) == 1);
        assert!(compare(&id_1, &id_3) == 0);
    }

    #[test]
    fn test_nano64_equals() {
        let id_1 = Nano64::new(100);
        let id_2 = Nano64::new(200);
        let id_3 = Nano64::new(100);
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
        let timestamp: u64 = 1234567890123;
        fn _rng(_bytes: u32) -> Result<u32, Nano64Error> {
            Ok(0)
        }
        let id = Nano64::generate(timestamp, Some(_rng)).unwrap();
        let date_u64 = id
            .to_date()
            .duration_since(UNIX_EPOCH)
            .expect("SystemTime before UNIX EPOCH!")
            .as_millis() as u64;
        assert_eq!(date_u64, timestamp);
    }

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

    #[test]
    fn test_nano64_string() {
        let id = Nano64::new(0x123456789ABCD);
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
            assert_eq!(id.u64_value(), tc.value);
        }
    }

    #[test]
    fn test_nano64_monotonic_now() {
        acquire_monotonic_test_lock(test);
        fn test() {
            set_monotonic_refs_to(0, 0);
            let id_1: Nano64 = match Nano64::generate_monotonic_now(None) {
                Ok(got) => got,
                Err(e) => panic!("[id_1] did not expect error {e}"),
            };
            let id_2 = match Nano64::generate_monotonic_now(None) {
                Ok(got) => got,
                Err(e) => panic!("[id_2] did not expect error {e}"),
            };
            assert!(id_1.u64_value() < id_2.u64_value());
        }
    }

    #[test]
    fn test_monotonic_race() {
        acquire_monotonic_test_lock(test);
        fn test() {
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
                        local_ids.push(Nano64::generate_monotonic_default().unwrap());
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

            global_ids.sort_by_key(|id| id.u64_value());

            for pair in global_ids.windows(2) {
                assert!(
                    pair[0].u64_value() < pair[1].u64_value(),
                    "IDs not strictly increasing"
                );
            }

            let unique_count = global_ids
                .iter()
                .map(|id| id.u64_value())
                .collect::<HashSet<_>>()
                .len();

            assert_eq!(unique_count, global_ids.len(), "Duplicate IDs detected!");
        }
    }

    #[test]
    fn test_nano64_monotonic_default() {
        acquire_monotonic_test_lock(test);
        fn test() {
            set_monotonic_refs_to(0, 0);
            let id = match Nano64::generate_monotonic_default() {
                Ok(got) => got,
                Err(e) => panic!("unexpected error {e}"),
            };
            assert_ne!(id.u64_value(), 0);
        }
    }

    #[test]
    fn test_nano64_monotonic_overflow() {
        acquire_monotonic_test_lock(test);
        fn test() {
            // Set refs to maximums, simulate exhaustion.
            set_monotonic_refs_to(RANDOM_MASK, MAX_TIMESTAMP);
            if let Ok(got) = Nano64::generate_monotonic(MAX_TIMESTAMP, None) {
                panic!(
                    "`generate_monotonic` called with max timestamp and exhausted random should error but got {got:?}"
                );
            }
        }
    }

    #[test]
    fn test_nano64_monotonic_backwards_time() {
        acquire_monotonic_test_lock(test);
        fn test() {
            set_monotonic_refs_to(100, 1000000);
            // Try to generate with an earlier timestamp
            let id = Nano64::generate_monotonic(500000, None).unwrap();
            // Should use the last timestamp, not provided one
            if id.get_timestamp() < 1000000 {
                panic!("Should not go backwards in time {}", id.get_timestamp());
            }
        }
    }

    #[test]
    fn test_nano64_from_bytes_error() {
        let bytes: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
        let id = Nano64::from(bytes);
        assert!(id.to_hex() != "");
    }

    #[test]
    fn test_nano64_from_hex_edge_case_too_short_after_prefix_removal() {
        if let Ok(id) = "0xABCD".parse::<Nano64>() {
            panic!("Expected error - hex string too short after prefix removal - but got {id:?}");
        }
    }

    #[test]
    fn test_nano64_from_hex_edge_case_too_long() {
        if let Ok(id) = "0x00112233445566778899".parse::<Nano64>() {
            panic!("Expected error - hex string too long - but got {id:?}");
        }
    }

    #[test]
    fn test_nano64_failing_rng() {
        fn rng(_bits: u32) -> Result<u32, Nano64Error> {
            Err(Nano64Error::Error("Simulated rng failure".into()))
        }
        if let Ok(got) = Nano64::generate(1122334455, Some(rng)) {
            panic!("Expected error - rng failure - but got {got:?}");
        }
    }

    #[test]
    fn test_nano64_monotonic_failing_rng() {
        acquire_monotonic_test_lock(test);
        fn test() {
            set_monotonic_refs_to(0, 1000);
            fn rng(_bits: u32) -> Result<u32, Nano64Error> {
                Err(Nano64Error::Error("Simulated rng failure".into()))
            }
            if let Ok(got) = Nano64::generate_monotonic(12345, Some(rng)) {
                panic!("Expected error - rng failure - but got {got:?}");
            }
        }
    }

    #[test]
    fn test_nano64_monotonic_same_timestamp_increment() {
        acquire_monotonic_test_lock(test);
        fn test() {
            set_monotonic_refs_to(50, 1000);
            let id_1 = Nano64::generate_monotonic(1000, None).unwrap();
            let id_2 = Nano64::generate_monotonic(1000, None).unwrap();
            if id_2.get_random() <= id_1.get_random() {
                panic!(
                    "should increment random field in same ms. id_2 ({}) should be > id_1 ({})",
                    id_2.get_random(),
                    id_1.get_random()
                );
            }
        }
    }

    #[test]
    fn test_nano64_generate_with_none_rng() {
        let timestamp = 12345;
        let id = if let Ok(got) = Nano64::generate(timestamp, None) {
            got
        } else {
            panic!("Expected 'None' rng to use default_rng under the hood!");
        };
        assert_eq!(id.get_timestamp(), timestamp);
    }

    #[test]
    fn test_nano64_monotonic_generate_with_none_rng() {
        acquire_monotonic_test_lock(test);
        fn test() {
            let timestamp = 12345;
            let id = if let Ok(got) = Nano64::generate_monotonic(timestamp, None) {
                got
            } else {
                panic!("Expected 'None' rng to use default_rng under the hood!");
            };
            assert_eq!(id.get_timestamp(), timestamp);
        }
    }

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
}
