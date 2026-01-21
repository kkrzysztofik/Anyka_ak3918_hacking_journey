use crate::rtsp::global_trait::{Marshal, Unmarshal};

#[derive(Debug, Clone, Default)]
pub struct RtpMap {
    pub payload_type: u16,
    pub encoding_name: String,
    pub clock_rate: u32,
    pub encoding_param: String,
}

impl Unmarshal for RtpMap {
    // a=rtpmap:96 H264/90000\r\n\
    // a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\

    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut rtpmap = RtpMap::default();

        let parts: Vec<&str> = raw_data.split(' ').collect();

        if let Some(part_0) = parts.first()
            && let Ok(payload_type) = part_0.parse::<u16>()
        {
            rtpmap.payload_type = payload_type;
        }

        if let Some(part_1) = parts.get(1) {
            let parameters: Vec<&str> = part_1.split('/').collect();

            if let Some(para_0) = parameters.first() {
                rtpmap.encoding_name = para_0.to_string();
            }

            if let Some(para_1) = parameters.get(1)
                && let Ok(clock_rate) = para_1.parse::<u32>()
            {
                rtpmap.clock_rate = clock_rate;
            }
            if let Some(para_2) = parameters.get(2) {
                rtpmap.encoding_param = para_2.to_string();
            }
        }

        Ok(rtpmap)
    }
}

impl Marshal for RtpMap {
    fn marshal(&self) -> String {
        let mut rtpmap = format!(
            "{} {}/{}",
            self.payload_type, self.encoding_name, self.clock_rate
        );
        if self.encoding_param != *"" {
            rtpmap = format!("{}/{}", rtpmap, self.encoding_param);
        }

        format!("{rtpmap}\r\n")
    }
}

#[cfg(test)]
mod tests {

    use crate::rtsp::global_trait::{Marshal, Unmarshal};

    use super::RtpMap;

    #[test]
    fn test_marshal_unmarshal_rtpmap() {
        let parser = RtpMap::unmarshal("97 MPEG4-GENERIC/44100/2").unwrap();

        println!(" parser: {parser:?}");

        assert_eq!(parser.payload_type, 97);
        assert_eq!(parser.encoding_name, "MPEG4-GENERIC".to_string());
        assert_eq!(parser.clock_rate, 44100);
        assert_eq!(parser.encoding_param, "2".to_string());

        print!("marshal str:{}", parser.marshal());

        let parser2 = RtpMap::unmarshal("96 H264/90000").unwrap();

        println!(" parser2: {parser2:?}");

        assert_eq!(parser2.payload_type, 96);
        assert_eq!(parser2.encoding_name, "H264".to_string());
        assert_eq!(parser2.clock_rate, 90000);
        assert_eq!(parser2.encoding_param, "".to_string());

        print!("marshal str2 :{}", parser2.marshal());
    }

    // ============================================
    // Construction and Default Tests
    // ============================================

    #[test]
    fn test_rtpmap_default() {
        let rtpmap = RtpMap::default();
        assert_eq!(rtpmap.payload_type, 0);
        assert_eq!(rtpmap.encoding_name, "");
        assert_eq!(rtpmap.clock_rate, 0);
        assert_eq!(rtpmap.encoding_param, "");
    }

    #[test]
    fn test_rtpmap_clone() {
        let rtpmap = RtpMap {
            payload_type: 96,
            encoding_name: "H264".to_string(),
            clock_rate: 90000,
            encoding_param: "".to_string(),
        };
        let cloned = rtpmap.clone();
        assert_eq!(rtpmap.payload_type, cloned.payload_type);
        assert_eq!(rtpmap.encoding_name, cloned.encoding_name);
        assert_eq!(rtpmap.clock_rate, cloned.clock_rate);
        assert_eq!(rtpmap.encoding_param, cloned.encoding_param);
    }

    // ============================================
    // Unmarshal Tests - H.264
    // ============================================

    #[test]
    fn test_unmarshal_h264() {
        let rtpmap = RtpMap::unmarshal("96 H264/90000").unwrap();
        assert_eq!(rtpmap.payload_type, 96);
        assert_eq!(rtpmap.encoding_name, "H264");
        assert_eq!(rtpmap.clock_rate, 90000);
        assert_eq!(rtpmap.encoding_param, "");
    }

    #[test]
    fn test_unmarshal_h264_lowercase() {
        let rtpmap = RtpMap::unmarshal("96 h264/90000").unwrap();
        assert_eq!(rtpmap.encoding_name, "h264");
    }

    // ============================================
    // Unmarshal Tests - AAC/MPEG4-GENERIC
    // ============================================

    #[test]
    fn test_unmarshal_aac_with_channels() {
        let rtpmap = RtpMap::unmarshal("97 MPEG4-GENERIC/48000/2").unwrap();
        assert_eq!(rtpmap.payload_type, 97);
        assert_eq!(rtpmap.encoding_name, "MPEG4-GENERIC");
        assert_eq!(rtpmap.clock_rate, 48000);
        assert_eq!(rtpmap.encoding_param, "2");
    }

    #[test]
    fn test_unmarshal_aac_mono() {
        let rtpmap = RtpMap::unmarshal("97 MPEG4-GENERIC/44100/1").unwrap();
        assert_eq!(rtpmap.clock_rate, 44100);
        assert_eq!(rtpmap.encoding_param, "1");
    }

    #[test]
    fn test_unmarshal_aac_stereo() {
        let rtpmap = RtpMap::unmarshal("97 MPEG4-GENERIC/48000/2").unwrap();
        assert_eq!(rtpmap.clock_rate, 48000);
        assert_eq!(rtpmap.encoding_param, "2");
    }

    #[test]
    fn test_unmarshal_aac_surround() {
        let rtpmap = RtpMap::unmarshal("97 MPEG4-GENERIC/48000/6").unwrap();
        assert_eq!(rtpmap.encoding_param, "6");
    }

    // ============================================
    // Unmarshal Tests - Other Codecs
    // ============================================

    #[test]
    fn test_unmarshal_pcma() {
        let rtpmap = RtpMap::unmarshal("8 PCMA/8000").unwrap();
        assert_eq!(rtpmap.payload_type, 8);
        assert_eq!(rtpmap.encoding_name, "PCMA");
        assert_eq!(rtpmap.clock_rate, 8000);
        assert_eq!(rtpmap.encoding_param, "");
    }

    #[test]
    fn test_unmarshal_pcmu() {
        let rtpmap = RtpMap::unmarshal("0 PCMU/8000").unwrap();
        assert_eq!(rtpmap.payload_type, 0);
        assert_eq!(rtpmap.encoding_name, "PCMU");
        assert_eq!(rtpmap.clock_rate, 8000);
    }

    #[test]
    fn test_unmarshal_h265() {
        let rtpmap = RtpMap::unmarshal("98 H265/90000").unwrap();
        assert_eq!(rtpmap.payload_type, 98);
        assert_eq!(rtpmap.encoding_name, "H265");
        assert_eq!(rtpmap.clock_rate, 90000);
    }

    // ============================================
    // Unmarshal Tests - Edge Cases
    // ============================================

    #[test]
    fn test_unmarshal_empty_string() {
        let rtpmap = RtpMap::unmarshal("");
        // Should return default
        assert!(rtpmap.is_some());
    }

    #[test]
    fn test_unmarshal_malformed() {
        let rtpmap = RtpMap::unmarshal("invalid");
        // Should handle gracefully
        assert!(rtpmap.is_some());
    }

    #[test]
    fn test_unmarshal_missing_clock_rate() {
        let rtpmap = RtpMap::unmarshal("96 H264").unwrap();
        assert_eq!(rtpmap.payload_type, 96);
        assert_eq!(rtpmap.encoding_name, "H264");
        assert_eq!(rtpmap.clock_rate, 0); // Default when missing
    }

    #[test]
    fn test_unmarshal_with_extra_params() {
        let rtpmap = RtpMap::unmarshal("96 H264/90000/extra/params").unwrap();
        assert_eq!(rtpmap.payload_type, 96);
        assert_eq!(rtpmap.encoding_name, "H264");
        assert_eq!(rtpmap.clock_rate, 90000);
        // Only first extra param is captured
        assert_eq!(rtpmap.encoding_param, "extra");
    }

    // ============================================
    // Marshal Tests
    // ============================================

    #[test]
    fn test_marshal_h264() {
        let rtpmap = RtpMap {
            payload_type: 96,
            encoding_name: "H264".to_string(),
            clock_rate: 90000,
            encoding_param: "".to_string(),
        };
        let result = rtpmap.marshal();
        assert!(result.contains("96"));
        assert!(result.contains("H264"));
        assert!(result.contains("90000"));
        assert!(result.ends_with("\r\n"));
    }

    #[test]
    fn test_marshal_aac_with_channels() {
        let rtpmap = RtpMap {
            payload_type: 97,
            encoding_name: "MPEG4-GENERIC".to_string(),
            clock_rate: 48000,
            encoding_param: "2".to_string(),
        };
        let result = rtpmap.marshal();
        assert!(result.contains("97"));
        assert!(result.contains("MPEG4-GENERIC"));
        assert!(result.contains("48000"));
        assert!(result.contains("/2"));
    }

    #[test]
    fn test_marshal_empty_encoding_param() {
        let rtpmap = RtpMap {
            payload_type: 96,
            encoding_name: "H264".to_string(),
            clock_rate: 90000,
            encoding_param: "".to_string(),
        };
        let result = rtpmap.marshal();
        // Should not include extra slash when encoding_param is empty
        assert!(!result.contains("90000/"));
    }

    // ============================================
    // Round-trip Tests (Unmarshal -> Marshal)
    // ============================================

    #[test]
    fn test_rtpmap_roundtrip_h264() {
        let original_str = "96 H264/90000";
        let rtpmap = RtpMap::unmarshal(original_str).unwrap();
        let marshaled = rtpmap.marshal();

        // Parse marshaled result (remove \r\n)
        let marshaled_clean = marshaled.trim_end();
        let roundtrip = RtpMap::unmarshal(marshaled_clean).unwrap();

        assert_eq!(rtpmap.payload_type, roundtrip.payload_type);
        assert_eq!(rtpmap.encoding_name, roundtrip.encoding_name);
        assert_eq!(rtpmap.clock_rate, roundtrip.clock_rate);
    }

    #[test]
    fn test_rtpmap_roundtrip_aac() {
        let original_str = "97 MPEG4-GENERIC/48000/2";
        let rtpmap = RtpMap::unmarshal(original_str).unwrap();
        let marshaled = rtpmap.marshal();

        let marshaled_clean = marshaled.trim_end();
        let roundtrip = RtpMap::unmarshal(marshaled_clean).unwrap();

        assert_eq!(rtpmap.payload_type, roundtrip.payload_type);
        assert_eq!(rtpmap.encoding_name, roundtrip.encoding_name);
        assert_eq!(rtpmap.clock_rate, roundtrip.clock_rate);
        assert_eq!(rtpmap.encoding_param, roundtrip.encoding_param);
    }

    // ============================================
    // Clock Rate Variations
    // ============================================

    #[test]
    fn test_unmarshal_various_clock_rates() {
        let test_cases = vec![
            (8000, "8 PCMU/8000"),
            (16000, "97 MPEG4-GENERIC/16000/1"),
            (32000, "97 MPEG4-GENERIC/32000/2"),
            (44100, "97 MPEG4-GENERIC/44100/2"),
            (48000, "97 MPEG4-GENERIC/48000/2"),
            (90000, "96 H264/90000"),
        ];

        for (expected_rate, input) in test_cases {
            let rtpmap = RtpMap::unmarshal(input).unwrap();
            assert_eq!(
                rtpmap.clock_rate, expected_rate,
                "Failed for input: {}",
                input
            );
        }
    }

    // ============================================
    // Payload Type Variations
    // ============================================

    #[test]
    fn test_unmarshal_various_payload_types() {
        for payload_type in 0..=127u16 {
            let input = format!("{} H264/90000", payload_type);
            let rtpmap = RtpMap::unmarshal(&input).unwrap();
            assert_eq!(rtpmap.payload_type, payload_type);
        }
    }
}
