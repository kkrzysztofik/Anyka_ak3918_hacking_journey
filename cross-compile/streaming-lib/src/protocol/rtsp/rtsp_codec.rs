use lazy_static::lazy_static;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Hash, Eq, PartialEq)]
pub enum RtspCodecId {
    #[default]
    H264,
    H265,
    AAC,
    G711A,
}

lazy_static! {
    pub static ref RTSP_CODEC_ID_2_NAME: HashMap<RtspCodecId, &'static str> = {
        let mut m = HashMap::new();
        m.insert(RtspCodecId::H264, "h264");
        m.insert(RtspCodecId::H265, "h265");
        m.insert(RtspCodecId::AAC, "mpeg4-generic");
        m.insert(RtspCodecId::G711A, "pcma");
        m
    };
    pub static ref RTSP_CODEC_NAME_2_ID: HashMap<&'static str, RtspCodecId> = {
        let mut m = HashMap::new();
        m.insert("h264", RtspCodecId::H264);
        m.insert("h265", RtspCodecId::H265);
        m.insert("mpeg4-generic", RtspCodecId::AAC);
        m.insert("pcma", RtspCodecId::G711A);
        m
    };
}
#[derive(Debug, Clone, Default)]
pub struct RtspCodecInfo {
    pub codec_id: RtspCodecId,
    pub payload_type: u8,
    pub sample_rate: u32,
    pub channel_count: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // RtspCodecId Tests
    // ============================================

    #[test]
    fn test_rtsp_codec_id_default() {
        let codec_id = RtspCodecId::default();
        assert!(matches!(codec_id, RtspCodecId::H264));
    }

    #[test]
    fn test_rtsp_codec_id_clone() {
        let codec_id = RtspCodecId::H265;
        let cloned = codec_id.clone();
        assert_eq!(codec_id, cloned);
    }

    #[test]
    fn test_rtsp_codec_id_equality() {
        assert_eq!(RtspCodecId::H264, RtspCodecId::H264);
        assert_ne!(RtspCodecId::H264, RtspCodecId::H265);
        assert_ne!(RtspCodecId::AAC, RtspCodecId::G711A);
    }

    // ============================================
    // Codec ID to Name Mapping Tests
    // ============================================

    #[test]
    fn test_codec_id_to_name_h264() {
        assert_eq!(RTSP_CODEC_ID_2_NAME.get(&RtspCodecId::H264), Some(&"h264"));
    }

    #[test]
    fn test_codec_id_to_name_h265() {
        assert_eq!(RTSP_CODEC_ID_2_NAME.get(&RtspCodecId::H265), Some(&"h265"));
    }

    #[test]
    fn test_codec_id_to_name_aac() {
        assert_eq!(
            RTSP_CODEC_ID_2_NAME.get(&RtspCodecId::AAC),
            Some(&"mpeg4-generic")
        );
    }

    #[test]
    fn test_codec_id_to_name_g711a() {
        assert_eq!(RTSP_CODEC_ID_2_NAME.get(&RtspCodecId::G711A), Some(&"pcma"));
    }

    // ============================================
    // Codec Name to ID Mapping Tests
    // ============================================

    #[test]
    fn test_codec_name_to_id_h264() {
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("h264"), Some(&RtspCodecId::H264));
    }

    #[test]
    fn test_codec_name_to_id_h265() {
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("h265"), Some(&RtspCodecId::H265));
    }

    #[test]
    fn test_codec_name_to_id_aac() {
        assert_eq!(
            RTSP_CODEC_NAME_2_ID.get("mpeg4-generic"),
            Some(&RtspCodecId::AAC)
        );
    }

    #[test]
    fn test_codec_name_to_id_g711a() {
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("pcma"), Some(&RtspCodecId::G711A));
    }

    #[test]
    fn test_codec_name_to_id_unknown() {
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("unknown"), None);
    }

    // ============================================
    // Round-trip Tests (ID -> Name -> ID)
    // ============================================

    #[test]
    fn test_codec_id_name_roundtrip_h264() {
        let codec_id = RtspCodecId::H264;
        let name = RTSP_CODEC_ID_2_NAME.get(&codec_id).unwrap();
        let back_to_id = RTSP_CODEC_NAME_2_ID.get(name).unwrap();
        assert_eq!(&codec_id, back_to_id);
    }

    #[test]
    fn test_codec_id_name_roundtrip_h265() {
        let codec_id = RtspCodecId::H265;
        let name = RTSP_CODEC_ID_2_NAME.get(&codec_id).unwrap();
        let back_to_id = RTSP_CODEC_NAME_2_ID.get(name).unwrap();
        assert_eq!(&codec_id, back_to_id);
    }

    #[test]
    fn test_codec_id_name_roundtrip_aac() {
        let codec_id = RtspCodecId::AAC;
        let name = RTSP_CODEC_ID_2_NAME.get(&codec_id).unwrap();
        let back_to_id = RTSP_CODEC_NAME_2_ID.get(name).unwrap();
        assert_eq!(&codec_id, back_to_id);
    }

    #[test]
    fn test_codec_id_name_roundtrip_g711a() {
        let codec_id = RtspCodecId::G711A;
        let name = RTSP_CODEC_ID_2_NAME.get(&codec_id).unwrap();
        let back_to_id = RTSP_CODEC_NAME_2_ID.get(name).unwrap();
        assert_eq!(&codec_id, back_to_id);
    }

    #[test]
    fn test_codec_id_name_roundtrip_all() {
        let codecs = vec![
            RtspCodecId::H264,
            RtspCodecId::H265,
            RtspCodecId::AAC,
            RtspCodecId::G711A,
        ];

        for codec_id in codecs {
            let name = RTSP_CODEC_ID_2_NAME.get(&codec_id).unwrap();
            let back_to_id = RTSP_CODEC_NAME_2_ID.get(name).unwrap();
            assert_eq!(
                &codec_id, back_to_id,
                "Round-trip failed for {:?}",
                codec_id
            );
        }
    }

    // ============================================
    // RtspCodecInfo Tests
    // ============================================

    #[test]
    fn test_rtsp_codec_info_default() {
        let info = RtspCodecInfo::default();
        assert!(matches!(info.codec_id, RtspCodecId::H264));
        assert_eq!(info.payload_type, 0);
        assert_eq!(info.sample_rate, 0);
        assert_eq!(info.channel_count, 0);
    }

    #[test]
    fn test_rtsp_codec_info_clone() {
        let info = RtspCodecInfo {
            codec_id: RtspCodecId::H265,
            payload_type: 96,
            sample_rate: 90000,
            channel_count: 2,
        };
        let cloned = info.clone();
        assert_eq!(info.codec_id, cloned.codec_id);
        assert_eq!(info.payload_type, cloned.payload_type);
        assert_eq!(info.sample_rate, cloned.sample_rate);
        assert_eq!(info.channel_count, cloned.channel_count);
    }

    #[test]
    fn test_rtsp_codec_info_h264() {
        let info = RtspCodecInfo {
            codec_id: RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            channel_count: 0, // Video has no channels
        };
        assert_eq!(info.codec_id, RtspCodecId::H264);
        assert_eq!(info.payload_type, 96);
    }

    #[test]
    fn test_rtsp_codec_info_aac() {
        let info = RtspCodecInfo {
            codec_id: RtspCodecId::AAC,
            payload_type: 97,
            sample_rate: 48000,
            channel_count: 2,
        };
        assert_eq!(info.codec_id, RtspCodecId::AAC);
        assert_eq!(info.sample_rate, 48000);
        assert_eq!(info.channel_count, 2);
    }

    #[test]
    fn test_rtsp_codec_info_h265() {
        let info = RtspCodecInfo {
            codec_id: RtspCodecId::H265,
            payload_type: 98,
            sample_rate: 90000,
            channel_count: 0,
        };
        assert_eq!(info.codec_id, RtspCodecId::H265);
    }

    #[test]
    fn test_rtsp_codec_info_g711a() {
        let info = RtspCodecInfo {
            codec_id: RtspCodecId::G711A,
            payload_type: 8,
            sample_rate: 8000,
            channel_count: 1,
        };
        assert_eq!(info.codec_id, RtspCodecId::G711A);
        assert_eq!(info.sample_rate, 8000);
        assert_eq!(info.channel_count, 1);
    }

    // ============================================
    // Codec Name Variations Tests
    // ============================================

    #[test]
    fn test_codec_name_case_sensitivity() {
        // Names should be case-sensitive
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("H264"), None); // Uppercase
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("h264"), Some(&RtspCodecId::H264)); // Lowercase
    }

    #[test]
    fn test_codec_name_exact_match() {
        // Should require exact match
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("h264"), Some(&RtspCodecId::H264));
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("h264 "), None); // With space
        assert_eq!(RTSP_CODEC_NAME_2_ID.get(" h264"), None); // With space
        assert_eq!(RTSP_CODEC_NAME_2_ID.get("h264x"), None); // With suffix
    }
}
