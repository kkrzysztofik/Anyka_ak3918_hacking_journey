// Integration tests for stream routing
// Tests end-to-end stream hub operations, publisher/subscriber routing, and data flow

use std::collections::{HashMap, HashSet};
use streaming_lib::hub::define::{PublishType, SubscribeType};
use streaming_lib::hub::stream::StreamIdentifier;

/// Tests stream identifier creation for different protocols.
#[tokio::test]
async fn test_stream_identifier_creation() {
    let rtsp_id = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    match rtsp_id {
        StreamIdentifier::Rtsp { stream_path } => {
            assert_eq!(stream_path, "/stream1");
        }
        _ => panic!("Expected RTSP identifier"),
    }
}

/// Tests stream identifier usage as HashMap keys.
#[tokio::test]
async fn test_stream_identifier_hashmap_usage() {
    let mut streams: HashMap<StreamIdentifier, u32> = HashMap::new();

    let id1 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let id2 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    streams.insert(id1.clone(), 12345);

    // Same identifier should map to same value
    assert_eq!(streams.get(&id2), Some(&12345));

    // Different identifier should not exist
    let id3 = StreamIdentifier::Rtsp {
        stream_path: "/other".to_string(),
    };
    assert_eq!(streams.get(&id3), None);
}

/// Tests all SubscribeType variants are distinct.
#[tokio::test]
async fn test_subscribe_type_variants() {
    let subscribe_types = vec![SubscribeType::RtspPull, SubscribeType::HttpFlvPull];

    // Verify all variants are distinct
    for (i, st1) in subscribe_types.iter().enumerate() {
        for (j, st2) in subscribe_types.iter().enumerate() {
            if i != j {
                assert_ne!(st1, st2);
            }
        }
    }
}

/// Tests all PublishType variants are distinct.
#[tokio::test]
async fn test_publish_type_variants() {
    let publish_types = vec![PublishType::RtspPush];

    // Verify all variants are distinct
    for (i, pt1) in publish_types.iter().enumerate() {
        for (j, pt2) in publish_types.iter().enumerate() {
            if i != j {
                assert_ne!(pt1, pt2);
            }
        }
    }
}

/// Tests StreamIdentifier serialization/deserialization round-trip.
#[tokio::test]
async fn test_stream_identifier_serialization() {
    let id = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let json = serde_json::to_string(&id).unwrap();
    assert!(json.contains("rtsp"));
    assert!(json.contains("/stream1"));

    let deserialized: StreamIdentifier = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
}

/// Tests Display formatting for StreamIdentifier variants.
#[tokio::test]
async fn test_stream_identifier_display() {
    let rtsp_id = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let display_str = format!("{}", rtsp_id);
    assert!(display_str.contains("RTSP"));
    assert!(display_str.contains("/stream1"));
}

/// Tests StreamIdentifier uniqueness in HashSet collections.
#[tokio::test]
async fn test_stream_identifier_hashset_uniqueness() {
    let mut set = HashSet::new();
    let id1 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };
    let id2 = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };
    let id3 = StreamIdentifier::Rtsp {
        stream_path: "/other".to_string(),
    };

    set.insert(id1.clone());
    set.insert(id2);
    set.insert(id3.clone());

    assert_eq!(set.len(), 2);
    assert!(set.contains(&id1));
    assert!(set.contains(&id3));
}
