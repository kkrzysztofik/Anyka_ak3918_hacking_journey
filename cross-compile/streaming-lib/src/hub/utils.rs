use portable_atomic::{AtomicU64, Ordering};
use serde::{Serialize, Serializer};
use std::fmt;
use std::time::SystemTime;

static UUID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Copy)]
pub struct Uuid {
    value: [char; 16],
    random_count: RandomDigitCount,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, Copy)]
pub enum RandomDigitCount {
    #[default]
    Zero = 0,
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

fn u8_to_enum(digit: u8) -> RandomDigitCount {
    match digit {
        0 => RandomDigitCount::Zero,
        1 => RandomDigitCount::One,
        2 => RandomDigitCount::Two,
        3 => RandomDigitCount::Three,
        4 => RandomDigitCount::Four,
        5 => RandomDigitCount::Five,
        6 => RandomDigitCount::Six,
        _ => RandomDigitCount::Zero,
    }
}

impl Serialize for Uuid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl Uuid {
    fn parse_uuid(uuid: &str) -> Option<Uuid> {
        let length = uuid.len();
        if !(10..=16).contains(&length) {
            return None;
        }

        let random_count = u8_to_enum(length as u8 - 10);
        let mut value: [char; 16] = ['\0'; 16];
        for (i, c) in uuid.chars().enumerate() {
            value[i] = c;
        }

        Some(Uuid {
            value,
            random_count,
        })
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(uuid: &str) -> Option<Uuid> {
        Self::parse_uuid(uuid)
    }

    pub fn from_str2(uuid: &str) -> Option<Uuid> {
        Self::from_str(uuid)
    }

    pub fn new(random_digit_count: RandomDigitCount) -> Self {
        let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);
        let seconds = match duration {
            Ok(result) => result.as_secs(),
            _ => 0,
        };

        let seconds_str = seconds.to_string();
        let mut value: [char; 16] = ['\0'; 16];
        for (i, c) in seconds_str.chars().enumerate() {
            if i >= 10 {
                break;
            }
            value[i] = c;
        }

        let random_size = random_digit_count as usize;
        if random_size > 0 {
            let max = 10_u64.saturating_pow(random_size as u32);
            let counter = UUID_COUNTER.fetch_add(1, Ordering::Relaxed) % max.max(1);
            let counter_str = format!("{:0width$}", counter, width = random_size);
            for (i, c) in counter_str.chars().enumerate() {
                value[10 + i] = c;
            }
        }

        // let count = random_digit_count as u8;
        // let mut rng = rand::thread_rng();
        // let random_digit_str: String = (0..count)
        //     .map(|_| rng.gen_range(0..=9).to_string())
        //     .collect();

        // let seconds_str = format!("{}", seconds);
        // let random_num_str = if count > 0 {
        //     format!("-{}", random_digit_str)
        // } else {
        //     String::from("")
        // };

        Self {
            value,
            random_count: random_digit_count,
        }
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let val: String = self
            .value
            .iter()
            .take(10 + self.random_count as usize)
            .collect();
        write!(f, "{val}")
    }
}

impl std::str::FromStr for Uuid {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_uuid(s).ok_or(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== RandomDigitCount Tests ==========

    #[test]
    fn test_random_digit_count_default() {
        let count = RandomDigitCount::default();
        assert_eq!(count, RandomDigitCount::Zero);
    }

    #[test]
    fn test_random_digit_count_values() {
        assert_eq!(RandomDigitCount::Zero as usize, 0);
        assert_eq!(RandomDigitCount::One as usize, 1);
        assert_eq!(RandomDigitCount::Two as usize, 2);
        assert_eq!(RandomDigitCount::Three as usize, 3);
        assert_eq!(RandomDigitCount::Four as usize, 4);
        assert_eq!(RandomDigitCount::Five as usize, 5);
        assert_eq!(RandomDigitCount::Six as usize, 6);
    }

    #[test]
    fn test_u8_to_enum() {
        assert_eq!(u8_to_enum(0), RandomDigitCount::Zero);
        assert_eq!(u8_to_enum(1), RandomDigitCount::One);
        assert_eq!(u8_to_enum(2), RandomDigitCount::Two);
        assert_eq!(u8_to_enum(3), RandomDigitCount::Three);
        assert_eq!(u8_to_enum(4), RandomDigitCount::Four);
        assert_eq!(u8_to_enum(5), RandomDigitCount::Five);
        assert_eq!(u8_to_enum(6), RandomDigitCount::Six);
    }

    #[test]
    fn test_u8_to_enum_out_of_range() {
        assert_eq!(u8_to_enum(7), RandomDigitCount::Zero);
        assert_eq!(u8_to_enum(100), RandomDigitCount::Zero);
        assert_eq!(u8_to_enum(255), RandomDigitCount::Zero);
    }

    // ========== Uuid Construction Tests ==========

    #[test]
    fn test_uuid_new_zero_random() {
        let uuid = Uuid::new(RandomDigitCount::Zero);
        let s = uuid.to_string();
        assert_eq!(s.len(), 10);
    }

    #[test]
    fn test_uuid_new_four_random() {
        let uuid = Uuid::new(RandomDigitCount::Four);
        let s = uuid.to_string();
        assert_eq!(s.len(), 14);
    }

    #[test]
    fn test_uuid_new_six_random() {
        let uuid = Uuid::new(RandomDigitCount::Six);
        let s = uuid.to_string();
        assert_eq!(s.len(), 16);
    }

    #[test]
    fn test_uuid_contains_only_digits() {
        let uuid = Uuid::new(RandomDigitCount::Four);
        let s = uuid.to_string();
        assert!(s.chars().all(|c| c.is_ascii_digit()));
    }

    // ========== Uuid Parsing Tests ==========

    #[test]
    fn test_uuid_from_str2_valid() {
        let uuid = Uuid::from_str("1234567890");
        assert!(uuid.is_some());
        let uuid = uuid.unwrap();
        assert_eq!(uuid.to_string(), "1234567890");
    }

    #[test]
    fn test_uuid_from_str2_with_random_digits() {
        let uuid = Uuid::from_str("12345678901234");
        assert!(uuid.is_some());
        let uuid = uuid.unwrap();
        assert_eq!(uuid.to_string(), "12345678901234");
    }

    #[test]
    fn test_uuid_from_str2_too_short() {
        let uuid = Uuid::from_str("123456789");
        assert!(uuid.is_none());
    }

    #[test]
    fn test_uuid_from_str2_too_long() {
        let uuid = Uuid::from_str("12345678901234567");
        assert!(uuid.is_none());
    }

    #[test]
    fn test_uuid_from_str2_empty() {
        let uuid = Uuid::from_str("");
        assert!(uuid.is_none());
    }

    #[test]
    fn test_uuid_from_str2_min_length() {
        let uuid = Uuid::from_str("1234567890");
        assert!(uuid.is_some());
    }

    #[test]
    fn test_uuid_from_str2_max_length() {
        let uuid = Uuid::from_str("1234567890123456");
        assert!(uuid.is_some());
    }

    // ========== Uuid Serialization Tests ==========

    #[test]
    fn test_uuid_serialize() {
        let uuid = Uuid::new(RandomDigitCount::Four);
        let serialized = serde_json::to_string(&uuid).unwrap();
        assert!(serialized.starts_with('"'));
        assert!(serialized.ends_with('"'));
    }

    // ========== Uuid Round-Trip Tests ==========

    #[test]
    fn test_uuid_roundtrip() {
        let uuid = Uuid::new(RandomDigitCount::Four);
        let s = uuid.to_string();
        let parsed = Uuid::from_str(&s);
        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().to_string(), s);
    }

    // ========== Uuid Equality Tests ==========

    #[test]
    fn test_uuid_equality() {
        let uuid1 = Uuid::from_str("1234567890").unwrap();
        let uuid2 = Uuid::from_str("1234567890").unwrap();
        assert_eq!(uuid1, uuid2);
    }

    #[test]
    fn test_uuid_inequality() {
        let uuid1 = Uuid::from_str("1234567890").unwrap();
        let uuid2 = Uuid::from_str("0987654321").unwrap();
        assert_ne!(uuid1, uuid2);
    }

    // ========== Uuid Clone and Debug Tests ==========

    #[test]
    fn test_uuid_clone() {
        let uuid = Uuid::new(RandomDigitCount::Four);
        let cloned = uuid.clone();
        assert_eq!(uuid, cloned);
    }

    #[test]
    fn test_uuid_debug() {
        let uuid = Uuid::new(RandomDigitCount::Zero);
        let debug_str = format!("{:?}", uuid);
        assert!(debug_str.contains("Uuid"));
        assert!(debug_str.contains("value"));
    }

    // ========== Uuid Hash Tests ==========

    #[test]
    fn test_uuid_hash_consistency() {
        use std::collections::HashSet;
        let uuid = Uuid::from_str("1234567890").unwrap();
        let mut set = HashSet::new();
        set.insert(uuid);
        assert!(set.contains(&uuid));
    }

    // ========== Original Test ==========

    #[test]
    fn test_uuid() {
        let id = Uuid::new(RandomDigitCount::Four);
        let s = id.to_string();
        let serialized = serde_json::to_string(&id).unwrap();
        assert!(!serialized.is_empty());
        assert!(!s.is_empty());

        if let Some(u) = Uuid::from_str(&s) {
            assert_eq!(u.to_string(), s);
        }
    }
}
