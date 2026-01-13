// Integration tests for stream routing
// Tests end-to-end stream hub operations, publisher/subscriber routing, and data flow

use std::collections::HashMap;
use streaming_lib::streamhub::define::{PublishType, SubscribeType};
use streaming_lib::streamhub::stream::StreamIdentifier;

#[tokio::test]
async fn test_stream_identifier_creation() {
    // Test stream identifier creation for different protocols
    let rtmp_id = StreamIdentifier::Rtmp {
        app_name: "live".to_string(),
        stream_name: "test".to_string(),
    };

    let rtsp_id = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let webrtc_id = StreamIdentifier::WebRTC {
        app_name: "live".to_string(),
        stream_name: "test".to_string(),
    };

    match rtmp_id {
        StreamIdentifier::Rtmp {
            app_name,
            stream_name,
        } => {
            assert_eq!(app_name, "live");
            assert_eq!(stream_name, "test");
        }
        _ => panic!("Expected RTMP identifier"),
    }

    match rtsp_id {
        StreamIdentifier::Rtsp { stream_path } => {
            assert_eq!(stream_path, "/stream1");
        }
        _ => panic!("Expected RTSP identifier"),
    }

    match webrtc_id {
        StreamIdentifier::WebRTC {
            app_name,
            stream_name,
        } => {
            assert_eq!(app_name, "live");
            assert_eq!(stream_name, "test");
        }
        _ => panic!("Expected WebRTC identifier"),
    }
}

#[tokio::test]
async fn test_stream_identifier_hashmap_usage() {
    // Test stream identifier in HashMap (for stream hub)
    let mut streams: HashMap<StreamIdentifier, u32> = HashMap::new();

    let id1 = StreamIdentifier::Rtmp {
        app_name: "live".to_string(),
        stream_name: "test".to_string(),
    };

    let id2 = StreamIdentifier::Rtmp {
        app_name: "live".to_string(),
        stream_name: "test".to_string(),
    };

    streams.insert(id1.clone(), 12345);

    // Same identifier should map to same value
    assert_eq!(streams.get(&id2), Some(&12345));

    // Different identifier should not exist
    let id3 = StreamIdentifier::Rtmp {
        app_name: "live".to_string(),
        stream_name: "other".to_string(),
    };
    assert_eq!(streams.get(&id3), None);
}

#[tokio::test]
async fn test_subscribe_type_variants() {
    // Test all subscribe type variants
    let subscribe_types = vec![
        SubscribeType::RtmpPull,
        SubscribeType::RtmpRemux2HttpFlv,
        SubscribeType::RtmpRemux2Hls,
        SubscribeType::RtmpRelay,
        SubscribeType::RtspPull,
        SubscribeType::RtspRemux2Rtmp,
        SubscribeType::RtspRelay,
        SubscribeType::WhepPull,
        SubscribeType::WebRTCRemux2Rtmp,
        SubscribeType::WhipRelay,
        SubscribeType::RtpPull,
    ];

    // Verify all variants are distinct
    for (i, st1) in subscribe_types.iter().enumerate() {
        for (j, st2) in subscribe_types.iter().enumerate() {
            if i != j {
                assert_ne!(st1, st2);
            }
        }
    }
}

#[tokio::test]
async fn test_publish_type_variants() {
    // Test all publish type variants
    let publish_types = vec![
        PublishType::RtmpPush,
        PublishType::RtmpRelay,
        PublishType::RtspPush,
        PublishType::RtspRelay,
        PublishType::WhipPush,
        PublishType::WhepRelay,
        PublishType::RtpPush,
    ];

    // Verify all variants are distinct
    for (i, pt1) in publish_types.iter().enumerate() {
        for (j, pt2) in publish_types.iter().enumerate() {
            if i != j {
                assert_ne!(pt1, pt2);
            }
        }
    }
}

#[tokio::test]
async fn test_stream_identifier_serialization() {
    // Test stream identifier serialization (for API/notifications)
    let id = StreamIdentifier::Rtmp {
        app_name: "live".to_string(),
        stream_name: "test".to_string(),
    };

    let json = serde_json::to_string(&id).unwrap();
    assert!(json.contains("rtmp"));
    assert!(json.contains("live"));
    assert!(json.contains("test"));

    let deserialized: StreamIdentifier = serde_json::from_str(&json).unwrap();
    assert_eq!(id, deserialized);
}

#[tokio::test]
async fn test_stream_identifier_display() {
    // Test stream identifier Display implementation
    let rtmp_id = StreamIdentifier::Rtmp {
        app_name: "live".to_string(),
        stream_name: "test".to_string(),
    };

    let display_str = format!("{}", rtmp_id);
    assert!(display_str.contains("RTMP"));
    assert!(display_str.contains("live"));
    assert!(display_str.contains("test"));

    let rtsp_id = StreamIdentifier::Rtsp {
        stream_path: "/stream1".to_string(),
    };

    let display_str2 = format!("{}", rtsp_id);
    assert!(display_str2.contains("RTSP"));
    assert!(display_str2.contains("/stream1"));
}
