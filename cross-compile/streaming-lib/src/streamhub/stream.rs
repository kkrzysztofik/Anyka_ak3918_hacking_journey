use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
pub enum StreamIdentifier {
    #[default]
    Unknown,
    #[serde(rename = "rtmp")]
    Rtmp {
        app_name: String,
        stream_name: String,
    },
    #[serde(rename = "rtsp")]
    Rtsp { stream_path: String },
    #[serde(rename = "webrtc")]
    WebRTC {
        app_name: String,
        stream_name: String,
    },
}
impl fmt::Display for StreamIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StreamIdentifier::Rtmp {
                app_name,
                stream_name,
            } => {
                write!(f, "RTMP - app_name: {app_name}, stream_name: {stream_name}")
            }
            StreamIdentifier::Rtsp {
                stream_path: stream_name,
            } => {
                write!(f, "RTSP - stream_name: {stream_name}")
            }
            StreamIdentifier::WebRTC {
                app_name,
                stream_name,
            } => {
                write!(
                    f,
                    "WebRTC - app_name: {app_name}, stream_name: {stream_name}"
                )
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
    fn test_stream_identifier_rtmp() {
        let id = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        match &id {
            StreamIdentifier::Rtmp { app_name, stream_name } => {
                assert_eq!(app_name, "live");
                assert_eq!(stream_name, "test");
            }
            _ => panic!("Expected Rtmp variant"),
        }
        assert_eq!(
            format!("{}", id),
            "RTMP - app_name: live, stream_name: test"
        );
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
    fn test_stream_identifier_webrtc() {
        let id = StreamIdentifier::WebRTC {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        match &id {
            StreamIdentifier::WebRTC { app_name, stream_name } => {
                assert_eq!(app_name, "live");
                assert_eq!(stream_name, "test");
            }
            _ => panic!("Expected WebRTC variant"),
        }
        assert_eq!(
            format!("{}", id),
            "WebRTC - app_name: live, stream_name: test"
        );
    }

    #[test]
    fn test_stream_identifier_equality() {
        let id1 = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        let id2 = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        let id3 = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "other".to_string(),
        };

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_stream_identifier_hash() {
        let mut set = HashSet::new();
        let id1 = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        let id2 = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        let id3 = StreamIdentifier::Rtsp {
            stream_path: "/stream1".to_string(),
        };

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
        let id1 = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
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
    fn test_stream_identifier_serialize_rtmp() {
        let id = StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.contains("rtmp"));
        assert!(json.contains("live"));
        assert!(json.contains("test"));
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
    fn test_stream_identifier_serialize_webrtc() {
        let id = StreamIdentifier::WebRTC {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        };
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.contains("webrtc"));
        assert!(json.contains("live"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_stream_identifier_deserialize_rtmp() {
        let json = r#"{"rtmp":{"app_name":"live","stream_name":"test"}}"#;
        let id: StreamIdentifier = serde_json::from_str(json).unwrap();
        match id {
            StreamIdentifier::Rtmp {
                app_name,
                stream_name,
            } => {
                assert_eq!(app_name, "live");
                assert_eq!(stream_name, "test");
            }
            _ => panic!("Expected RTMP variant"),
        }
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
    fn test_stream_identifier_deserialize_webrtc() {
        let json = r#"{"webrtc":{"app_name":"live","stream_name":"test"}}"#;
        let id: StreamIdentifier = serde_json::from_str(json).unwrap();
        match id {
            StreamIdentifier::WebRTC {
                app_name,
                stream_name,
            } => {
                assert_eq!(app_name, "live");
                assert_eq!(stream_name, "test");
            }
            _ => panic!("Expected WebRTC variant"),
        }
    }

    #[test]
    fn test_stream_identifier_round_trip_serialization() {
        let identifiers = vec![
            StreamIdentifier::Unknown,
            StreamIdentifier::Rtmp {
                app_name: "live".to_string(),
                stream_name: "test".to_string(),
            },
            StreamIdentifier::Rtsp {
                stream_path: "/stream1".to_string(),
            },
            StreamIdentifier::WebRTC {
                app_name: "live".to_string(),
                stream_name: "test".to_string(),
            },
        ];

        for id in identifiers {
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: StreamIdentifier = serde_json::from_str(&json).unwrap();
            assert_eq!(id, deserialized);
        }
    }
}
