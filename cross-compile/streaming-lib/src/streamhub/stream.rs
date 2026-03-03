use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub enum StreamIdentifier {
    #[default]
    Unknown,
    #[serde(rename = "rtsp")]
    Rtsp { stream_path: String },
}

impl fmt::Display for StreamIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StreamIdentifier::Rtsp { stream_path } => {
                write!(f, "RTSP - stream_name: {stream_path}")
            }
            StreamIdentifier::Unknown => {
                write!(f, "Unknown")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StreamIdentifier;
    use std::collections::HashSet;

    #[test]
    fn test_stream_identifier_unknown() {
        let id = StreamIdentifier::Unknown;
        assert_eq!(id, StreamIdentifier::Unknown);
        assert_eq!(format!("{}", id), "Unknown");
    }

    #[test]
    fn test_stream_identifier_rtsp() {
        let id = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        match &id {
            StreamIdentifier::Rtsp { stream_path } => {
                assert_eq!(stream_path, "/stream1");
            }
            _ => panic!("Expected Rtsp variant"),
        }
        assert_eq!(format!("{}", id), "RTSP - stream_name: /stream1");
    }

    #[test]
    fn test_stream_identifier_equality() {
        let id1 = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        let id2 = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        let id3 = StreamIdentifier::Rtsp {
            stream_path: "/other".to_string(),
        };

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_stream_identifier_hash() {
        let mut set = HashSet::new();
        let id1 = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        let id2 = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        let id3 = StreamIdentifier::Unknown;

        set.insert(id1.clone());
        set.insert(id2.clone());
        set.insert(id3.clone());

        // id1 and id2 are equal, so only 2 unique items
        assert_eq!(set.len(), 2);
        assert!(set.contains(&id1));
        assert!(set.contains(&id3));
    }

    #[test]
    fn test_stream_identifier_clone() {
        let id1 = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_stream_identifier_default() {
        let id = StreamIdentifier::default();
        assert_eq!(id, StreamIdentifier::Unknown);
    }

    #[test]
    fn test_stream_identifier_serialize_rtsp() {
        let id = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.contains("rtsp"));
        assert!(json.contains("/stream1"));
    }

    #[test]
    fn test_stream_identifier_deserialize_rtsp() {
        let json = r#"{"rtsp":{"stream_path":"/stream1"}}"#;
        let id: StreamIdentifier = serde_json::from_str(json).unwrap();
        match id {
            StreamIdentifier::Rtsp { stream_path } => {
                assert_eq!(stream_path, "/stream1");
            }
            _ => panic!("Expected RTSP variant"),
        }
    }

    #[test]
    fn test_stream_identifier_round_trip_serialization() {
        let identifiers = vec![
            StreamIdentifier::Unknown,
            StreamIdentifier::Rtsp {
                stream_path: "/stream1".to_string(),
            },
        ];

        for id in identifiers {
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: StreamIdentifier = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }
}
