use bytes::BytesMut;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub enum SoundFormat {
    #[default]
    AAC = 10,
    OPUS = 13,
}

pub mod aac_packet_type {
    pub const AAC_SEQHDR: u8 = 0;
    pub const AAC_RAW: u8 = 1;
}

pub mod avc_packet_type {
    pub const AVC_SEQHDR: u8 = 0;
    pub const AVC_NALU: u8 = 1;
    pub const AVC_EOS: u8 = 2;
}

pub mod frame_type {
    /*
        1: keyframe (for AVC, a seekable frame)
        2: inter frame (for AVC, a non- seekable frame)
        3: disposable inter frame (H.263 only)
        4: generated keyframe (reserved for server use only)
        5: video info/command frame
    */
    pub const KEY_FRAME: u8 = 1;
    pub const INTER_FRAME: u8 = 2;
}

#[derive(Debug, Clone, Serialize, Default)]
pub enum AvcCodecId {
    #[default]
    UNKNOWN = 0,
    H264 = 7,
    HEVC = 12,
}

pub fn u8_2_avc_codec_id(codec_id: u8) -> AvcCodecId {
    match codec_id {
        7_u8 => AvcCodecId::H264,
        12_u8 => AvcCodecId::HEVC,
        _ => AvcCodecId::UNKNOWN,
    }
}

pub mod tag_type {
    pub const AUDIO: u8 = 8;
    pub const VIDEO: u8 = 9;
    pub const SCRIPT_DATA_AMF: u8 = 18;
}

pub mod h264_nal_type {
    pub const H264_NAL_IDR: u8 = 5;
    pub const H264_NAL_SPS: u8 = 7;
    pub const H264_NAL_PPS: u8 = 8;
    pub const H264_NAL_AUD: u8 = 9;
}
#[derive(Debug, Clone, Serialize, Default)]
pub enum AacProfile {
    // @see @see ISO_IEC_14496-3-AAC-2001.pdf, page 23
    #[default]
    UNKNOWN = -1,
    LC = 2,
    SSR = 3,
    // AAC HE = LC+SBR
    HE = 5,
    // AAC HEv2 = LC+SBR+PS
    HEV2 = 29,
}

pub fn u8_2_aac_profile(profile: u8) -> AacProfile {
    match profile {
        2_u8 => AacProfile::LC,
        3_u8 => AacProfile::SSR,
        5_u8 => AacProfile::HE,
        29_u8 => AacProfile::HEV2,
        _ => AacProfile::UNKNOWN,
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub enum AvcProfile {
    #[default]
    UNKNOWN = -1,
    // @see ffmpeg, libavcodec/avcodec.h:2713
    Baseline = 66,
    Main = 77,
    Extended = 88,
    High = 100,
}

pub fn u8_2_avc_profile(profile: u8) -> AvcProfile {
    match profile {
        66_u8 => AvcProfile::Baseline,
        77_u8 => AvcProfile::Main,
        88_u8 => AvcProfile::Extended,
        100_u8 => AvcProfile::High,
        _ => AvcProfile::UNKNOWN,
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub enum AvcLevel {
    #[default]
    UNKNOWN = -1,
    #[serde(rename = "1.0")]
    Level1 = 10,
    #[serde(rename = "1.1")]
    Level11 = 11,
    #[serde(rename = "1.2")]
    Level12 = 12,
    #[serde(rename = "1.3")]
    Level13 = 13,
    #[serde(rename = "2.0")]
    Level2 = 20,
    #[serde(rename = "2.1")]
    Level21 = 21,
    #[serde(rename = "2.2")]
    Level22 = 22,
    #[serde(rename = "3.0")]
    Level3 = 30,
    #[serde(rename = "3.1")]
    Level31 = 31,
    #[serde(rename = "3.2")]
    Level32 = 32,
    #[serde(rename = "4.0")]
    Level4 = 40,
    #[serde(rename = "4.1")]
    Level41 = 41,
    #[serde(rename = "5.0")]
    Level5 = 50,
    #[serde(rename = "5.1")]
    Level51 = 51,
}

pub fn u8_2_avc_level(profile: u8) -> AvcLevel {
    match profile {
        10_u8 => AvcLevel::Level1,
        11_u8 => AvcLevel::Level11,
        12_u8 => AvcLevel::Level12,
        13_u8 => AvcLevel::Level13,
        20_u8 => AvcLevel::Level2,
        21_u8 => AvcLevel::Level21,
        22_u8 => AvcLevel::Level22,
        30_u8 => AvcLevel::Level3,
        31_u8 => AvcLevel::Level31,
        32_u8 => AvcLevel::Level32,
        40_u8 => AvcLevel::Level4,
        41_u8 => AvcLevel::Level41,
        50_u8 => AvcLevel::Level5,
        51_u8 => AvcLevel::Level51,

        _ => AvcLevel::UNKNOWN,
    }
}

pub enum FlvData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { timestamp: u32, data: BytesMut },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== u8_2_avc_codec_id Tests ==========

    #[test]
    fn test_u8_2_avc_codec_id_h264() {
        let codec = u8_2_avc_codec_id(7);
        assert!(matches!(codec, AvcCodecId::H264));
    }

    #[test]
    fn test_u8_2_avc_codec_id_hevc() {
        let codec = u8_2_avc_codec_id(12);
        assert!(matches!(codec, AvcCodecId::HEVC));
    }

    #[test]
    fn test_u8_2_avc_codec_id_unknown() {
        let codec = u8_2_avc_codec_id(0);
        assert!(matches!(codec, AvcCodecId::UNKNOWN));
    }

    #[test]
    fn test_u8_2_avc_codec_id_invalid_value() {
        let codec = u8_2_avc_codec_id(255);
        assert!(matches!(codec, AvcCodecId::UNKNOWN));
    }

    // ========== u8_2_aac_profile Tests ==========

    #[test]
    fn test_u8_2_aac_profile_lc() {
        let profile = u8_2_aac_profile(2);
        assert!(matches!(profile, AacProfile::LC));
    }

    #[test]
    fn test_u8_2_aac_profile_ssr() {
        let profile = u8_2_aac_profile(3);
        assert!(matches!(profile, AacProfile::SSR));
    }

    #[test]
    fn test_u8_2_aac_profile_he() {
        let profile = u8_2_aac_profile(5);
        assert!(matches!(profile, AacProfile::HE));
    }

    #[test]
    fn test_u8_2_aac_profile_hev2() {
        let profile = u8_2_aac_profile(29);
        assert!(matches!(profile, AacProfile::HEV2));
    }

    #[test]
    fn test_u8_2_aac_profile_unknown() {
        let profile = u8_2_aac_profile(0);
        assert!(matches!(profile, AacProfile::UNKNOWN));
    }

    #[test]
    fn test_u8_2_aac_profile_invalid_value() {
        let profile = u8_2_aac_profile(100);
        assert!(matches!(profile, AacProfile::UNKNOWN));
    }

    // ========== u8_2_avc_profile Tests ==========

    #[test]
    fn test_u8_2_avc_profile_baseline() {
        let profile = u8_2_avc_profile(66);
        assert!(matches!(profile, AvcProfile::Baseline));
    }

    #[test]
    fn test_u8_2_avc_profile_main() {
        let profile = u8_2_avc_profile(77);
        assert!(matches!(profile, AvcProfile::Main));
    }

    #[test]
    fn test_u8_2_avc_profile_extended() {
        let profile = u8_2_avc_profile(88);
        assert!(matches!(profile, AvcProfile::Extended));
    }

    #[test]
    fn test_u8_2_avc_profile_high() {
        let profile = u8_2_avc_profile(100);
        assert!(matches!(profile, AvcProfile::High));
    }

    #[test]
    fn test_u8_2_avc_profile_unknown() {
        let profile = u8_2_avc_profile(0);
        assert!(matches!(profile, AvcProfile::UNKNOWN));
    }

    // ========== u8_2_avc_level Tests ==========

    #[test]
    fn test_u8_2_avc_level_1() {
        let level = u8_2_avc_level(10);
        assert!(matches!(level, AvcLevel::Level1));
    }

    #[test]
    fn test_u8_2_avc_level_11() {
        let level = u8_2_avc_level(11);
        assert!(matches!(level, AvcLevel::Level11));
    }

    #[test]
    fn test_u8_2_avc_level_21() {
        let level = u8_2_avc_level(21);
        assert!(matches!(level, AvcLevel::Level21));
    }

    #[test]
    fn test_u8_2_avc_level_31() {
        let level = u8_2_avc_level(31);
        assert!(matches!(level, AvcLevel::Level31));
    }

    #[test]
    fn test_u8_2_avc_level_41() {
        let level = u8_2_avc_level(41);
        assert!(matches!(level, AvcLevel::Level41));
    }

    #[test]
    fn test_u8_2_avc_level_51() {
        let level = u8_2_avc_level(51);
        assert!(matches!(level, AvcLevel::Level51));
    }

    #[test]
    fn test_u8_2_avc_level_unknown() {
        let level = u8_2_avc_level(0);
        assert!(matches!(level, AvcLevel::UNKNOWN));
    }

    #[test]
    fn test_u8_2_avc_level_invalid_value() {
        let level = u8_2_avc_level(255);
        assert!(matches!(level, AvcLevel::UNKNOWN));
    }

    // ========== Default Tests ==========

    #[test]
    fn test_sound_format_default() {
        let format: SoundFormat = Default::default();
        assert!(matches!(format, SoundFormat::AAC));
    }

    #[test]
    fn test_avc_codec_id_default() {
        let codec: AvcCodecId = Default::default();
        assert!(matches!(codec, AvcCodecId::UNKNOWN));
    }

    #[test]
    fn test_aac_profile_default() {
        let profile: AacProfile = Default::default();
        assert!(matches!(profile, AacProfile::UNKNOWN));
    }

    #[test]
    fn test_avc_profile_default() {
        let profile: AvcProfile = Default::default();
        assert!(matches!(profile, AvcProfile::UNKNOWN));
    }

    #[test]
    fn test_avc_level_default() {
        let level: AvcLevel = Default::default();
        assert!(matches!(level, AvcLevel::UNKNOWN));
    }

    // ========== Constants Tests ==========

    #[test]
    fn test_aac_packet_type_constants() {
        assert_eq!(aac_packet_type::AAC_SEQHDR, 0);
        assert_eq!(aac_packet_type::AAC_RAW, 1);
    }

    #[test]
    fn test_avc_packet_type_constants() {
        assert_eq!(avc_packet_type::AVC_SEQHDR, 0);
        assert_eq!(avc_packet_type::AVC_NALU, 1);
        assert_eq!(avc_packet_type::AVC_EOS, 2);
    }

    #[test]
    fn test_frame_type_constants() {
        assert_eq!(frame_type::KEY_FRAME, 1);
        assert_eq!(frame_type::INTER_FRAME, 2);
    }

    #[test]
    fn test_tag_type_constants() {
        assert_eq!(tag_type::AUDIO, 8);
        assert_eq!(tag_type::VIDEO, 9);
        assert_eq!(tag_type::SCRIPT_DATA_AMF, 18);
    }

    #[test]
    fn test_h264_nal_type_constants() {
        assert_eq!(h264_nal_type::H264_NAL_IDR, 5);
        assert_eq!(h264_nal_type::H264_NAL_SPS, 7);
        assert_eq!(h264_nal_type::H264_NAL_PPS, 8);
        assert_eq!(h264_nal_type::H264_NAL_AUD, 9);
    }

    // ========== Additional AVC Level Tests ==========

    #[test]
    fn test_u8_2_avc_level_12() {
        let level = u8_2_avc_level(12);
        assert!(matches!(level, AvcLevel::Level12));
    }

    #[test]
    fn test_u8_2_avc_level_13() {
        let level = u8_2_avc_level(13);
        assert!(matches!(level, AvcLevel::Level13));
    }

    #[test]
    fn test_u8_2_avc_level_2() {
        let level = u8_2_avc_level(20);
        assert!(matches!(level, AvcLevel::Level2));
    }

    #[test]
    fn test_u8_2_avc_level_22() {
        let level = u8_2_avc_level(22);
        assert!(matches!(level, AvcLevel::Level22));
    }

    #[test]
    fn test_u8_2_avc_level_3() {
        let level = u8_2_avc_level(30);
        assert!(matches!(level, AvcLevel::Level3));
    }

    #[test]
    fn test_u8_2_avc_level_32() {
        let level = u8_2_avc_level(32);
        assert!(matches!(level, AvcLevel::Level32));
    }

    #[test]
    fn test_u8_2_avc_level_4() {
        let level = u8_2_avc_level(40);
        assert!(matches!(level, AvcLevel::Level4));
    }

    #[test]
    fn test_u8_2_avc_level_5() {
        let level = u8_2_avc_level(50);
        assert!(matches!(level, AvcLevel::Level5));
    }

    // ========== FlvData Tests ==========

    #[test]
    fn test_flv_data_video() {
        let data = BytesMut::from(&b"video data"[..]);
        let flv = FlvData::Video {
            timestamp: 1000,
            data: data.clone(),
        };
        match flv {
            FlvData::Video { timestamp, data: d } => {
                assert_eq!(timestamp, 1000);
                assert_eq!(d, data);
            }
            _ => panic!("Expected FlvData::Video"),
        }
    }

    #[test]
    fn test_flv_data_audio() {
        let data = BytesMut::from(&b"audio data"[..]);
        let flv = FlvData::Audio {
            timestamp: 2000,
            data: data.clone(),
        };
        match flv {
            FlvData::Audio { timestamp, data: d } => {
                assert_eq!(timestamp, 2000);
                assert_eq!(d, data);
            }
            _ => panic!("Expected FlvData::Audio"),
        }
    }

    #[test]
    fn test_flv_data_metadata() {
        let data = BytesMut::from(&b"metadata"[..]);
        let flv = FlvData::MetaData {
            timestamp: 0,
            data: data.clone(),
        };
        match flv {
            FlvData::MetaData { timestamp, data: d } => {
                assert_eq!(timestamp, 0);
                assert_eq!(d, data);
            }
            _ => panic!("Expected FlvData::MetaData"),
        }
    }
}
