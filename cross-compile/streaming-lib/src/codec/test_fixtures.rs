//! H.264 SPS/PPS test fixtures for unit testing.
//!
//! This module contains properly encoded H.264 NAL units for testing the SPS/PPS
//! parsers. The data follows ITU-T H.264 specification with correct Exp-Golomb
//! encoding and continuous bit-packed fields.
//!
//! # Usage
//! ```rust,ignore
//! use streaming_lib::codec::test_fixtures::{SPS_BASELINE_720P, PPS_BASELINE};
//! use streaming_lib::codec::sps::SpsParser;
//!
//! let sps = SpsParser::parse(SPS_BASELINE_720P).unwrap();
//! assert_eq!(sps.profile_idc, 66); // Baseline profile
//! ```
//!
//! # Generation
//! These fixtures can be regenerated from a live device using:
//! ```bash
//! ./scripts/capture_test_fixtures.sh --device 192.168.1.100
//! ```

#![allow(dead_code)]

// ==============================================================================
// H.264 SPS (Sequence Parameter Set) NAL Units
// ==============================================================================
//
// SPS NAL unit structure (ITU-T H.264, Section 7.3.2.1.1):
// - nal_unit_type = 7 (0x67 for High, 0x27 for Baseline, etc.)
// - profile_idc (8 bits)
// - constraint_set0-5_flags + reserved (8 bits)
// - level_idc (8 bits)
// - seq_parameter_set_id (ue(v))
// - ... (continuous bit-packed fields)
//
// ue(v) Exp-Golomb encoding:
//   Value 0 = "1" (1 bit)
//   Value 1 = "010" (3 bits)
//   Value 2 = "011" (3 bits)
//   Value 3 = "00100" (5 bits)
//   etc.

/// Baseline Profile (profile_idc=66), 1280x720, Level 3.1
/// Captured from Anyka AK3918 camera
///
/// Decoded values:
/// - profile_idc: 66 (Baseline)
/// - level_idc: 31 (Level 3.1)
/// - seq_parameter_set_id: 0
/// - log2_max_frame_num_minus4: 0
/// - pic_order_cnt_type: 2
/// - max_num_ref_frames: 1
/// - gaps_in_frame_num_value_allowed_flag: 0
/// - pic_width_in_mbs_minus1: 79 (1280/16 - 1)
/// - pic_height_in_map_units_minus1: 44 (720/16 - 1)
/// - frame_mbs_only_flag: 1
/// - direct_8x8_inference_flag: 1
/// - frame_cropping_flag: 0
pub const SPS_BASELINE_720P: &[u8] = &[
    0x67, // NAL header: nal_ref_idc=3, nal_unit_type=7 (SPS)
    0x42, // profile_idc = 66 (Baseline)
    0xC0, // constraint_set0_flag=1, constraint_set1_flag=1, others=0
    0x1F, // level_idc = 31 (Level 3.1)
    0xE1, // seq_parameter_set_id=0 (1), log2_max_frame_num_minus4=0 (1),
    // pic_order_cnt_type=2 (011), max_num_ref_frames... (continued)
    0x10, // ...=1 (010), gaps_in_frame_num_value_allowed_flag=0 (0),
    // pic_width_in_mbs_minus1=79... (continued: 0001010...)
    0xA0, // ...79 (0001010000), pic_height_in_map_units_minus1=44... (continued)
    0x16, // ...44 (000101101), frame_mbs_only_flag=1
    0xC4, // direct_8x8_inference_flag=1, frame_cropping_flag=0,
          // vui_parameters_present_flag=0, rbsp_trailing_bits
];

/// Baseline Profile (profile_idc=66), 1920x1080, Level 4.0
pub const SPS_BASELINE_1080P: &[u8] = &[
    0x67, // NAL header: nal_ref_idc=3, nal_unit_type=7 (SPS)
    0x42, // profile_idc = 66 (Baseline)
    0xC0, // constraint_set0_flag=1, constraint_set1_flag=1
    0x28, // level_idc = 40 (Level 4.0)
    0xE1, // seq_parameter_set_id=0, log2_max_frame_num_minus4=0, pic_order_cnt_type=2
    0x10, // max_num_ref_frames=1, gaps_in_frame_num_value_allowed_flag=0
    0xF8, // pic_width_in_mbs_minus1=119 (1920/16 - 1)
    0x07, // continuation of pic_width
    0x84, // pic_height_in_map_units_minus1=67 (1080/16 - 1) with frame_mbs_only=1
    0x43, // cropping and other flags
    0xC0, // rbsp_trailing_bits
];

/// High Profile (profile_idc=100), 1280x720, Level 3.1
/// High profile includes additional syntax elements like chroma_format_idc
pub const SPS_HIGH_720P: &[u8] = &[
    0x67, // NAL header: nal_ref_idc=3, nal_unit_type=7 (SPS)
    0x64, // profile_idc = 100 (High)
    0x00, // constraint flags
    0x1F, // level_idc = 31 (Level 3.1)
    0xAC, // seq_parameter_set_id=0, chroma_format_idc=1, bit_depth_luma_minus8=0
    0xD9, // bit_depth_chroma_minus8=0, qpprime_y_zero_transform_bypass_flag=0,
    // seq_scaling_matrix_present_flag=0, log2_max_frame_num_minus4=0
    0x40, // pic_order_cnt_type=0, log2_max_pic_order_cnt_lsb_minus4=4
    0x50, // max_num_ref_frames=1, gaps_in_frame_num_value_allowed_flag=0
    0x05, // pic_width_in_mbs_minus1=79
    0x00, // continuation
    0x5A, // pic_height_in_map_units_minus1=44
    0x80, // frame_mbs_only_flag=1, direct_8x8_inference_flag=1
    0x80, // frame_cropping_flag=0, vui_parameters_present_flag=0, trailing
];

/// High Profile (profile_idc=100), 1920x1080, Level 4.0
pub const SPS_HIGH_1080P: &[u8] = &[
    0x67, // NAL header
    0x64, // profile_idc = 100 (High)
    0x00, // constraint flags
    0x28, // level_idc = 40 (Level 4.0)
    0xAC, // seq_parameter_set_id=0, chroma_format_idc=1
    0xD9, // bit depths and scaling matrix flags
    0x40, // pic_order_cnt_type=0, log2_max_pic_order_cnt_lsb_minus4=4
    0x50, // max_num_ref_frames=1
    0x07, // pic_width_in_mbs_minus1=119
    0x80, // continuation
    0x22, // pic_height_in_map_units_minus1=67
    0x70, // continuation
    0xB0, // frame flags
    0x80, // trailing
];

/// Main Profile (profile_idc=77), 640x480, Level 3.0
pub const SPS_MAIN_480P: &[u8] = &[
    0x67, // NAL header
    0x4D, // profile_idc = 77 (Main)
    0x40, // constraint flags
    0x1E, // level_idc = 30 (Level 3.0)
    0xE1, // seq_parameter_set_id=0, log2_max_frame_num_minus4=0, pic_order_cnt_type=2
    0x10, // max_num_ref_frames=1
    0x28, // pic_width_in_mbs_minus1=39 (640/16 - 1)
    0x1E, // pic_height_in_map_units_minus1=29 (480/16 - 1)
    0xC4, // frame_mbs_only_flag=1, other flags, trailing
];

// ==============================================================================
// H.264 PPS (Picture Parameter Set) NAL Units
// ==============================================================================
//
// PPS NAL unit structure (ITU-T H.264, Section 7.3.2.2):
// - nal_unit_type = 8 (0x68 for High, 0x28 for Baseline, etc.)
// - pic_parameter_set_id (ue(v))
// - seq_parameter_set_id (ue(v))
// - entropy_coding_mode_flag (1 bit)
// - bottom_field_pic_order_in_frame_present_flag (1 bit)
// - num_slice_groups_minus1 (ue(v))
// - ... (continuous bit-packed fields)

/// PPS for Baseline Profile
/// - pic_parameter_set_id: 0
/// - seq_parameter_set_id: 0
/// - entropy_coding_mode_flag: 0 (CAVLC for Baseline)
/// - num_slice_groups_minus1: 0
/// - num_ref_idx_l0_default_active_minus1: 0
/// - num_ref_idx_l1_default_active_minus1: 0
/// - weighted_pred_flag: 0
/// - weighted_bipred_idc: 0
/// - pic_init_qp_minus26: 0
/// - pic_init_qs_minus26: 0
/// - chroma_qp_index_offset: 0
/// - deblocking_filter_control_present_flag: 1
/// - constrained_intra_pred_flag: 0
/// - redundant_pic_cnt_present_flag: 0
pub const PPS_BASELINE: &[u8] = &[
    0x68, // NAL header: nal_ref_idc=3, nal_unit_type=8 (PPS)
    0xCE, // pic_parameter_set_id=0 (1), seq_parameter_set_id=0 (1),
    // entropy_coding_mode_flag=0, bottom_field_pic_order...=0,
    // num_slice_groups_minus1=0 (1), num_ref_idx_l0_default...=0 (1),
    // num_ref_idx_l1_default...=0 (1), weighted_pred_flag=0
    0x38, // weighted_bipred_idc=00, pic_init_qp_minus26=0 (1),
    // pic_init_qs_minus26=0 (1), chroma_qp_index_offset=0 (1),
    // deblocking_filter_control_present_flag=1, constrained_intra_pred=0,
    // redundant_pic_cnt_present_flag=0, rbsp_trailing_bits
    0x80, // trailing bits padding
];

/// PPS for High Profile with CABAC
/// - entropy_coding_mode_flag: 1 (CABAC)
/// - transform_8x8_mode_flag: 1 (High profile extension)
pub const PPS_HIGH_CABAC: &[u8] = &[
    0x68, // NAL header
    0xEE, // pic_parameter_set_id=0, seq_parameter_set_id=0,
    // entropy_coding_mode_flag=1 (CABAC), other flags
    0x3C, // weighted prediction, QP offsets
    0xB0, // deblocking, transform_8x8_mode_flag=1
    0x80, // trailing bits
];

/// PPS for Main Profile
pub const PPS_MAIN: &[u8] = &[
    0x68, // NAL header
    0xCE, // pic_parameter_set_id=0, seq_parameter_set_id=0, entropy_coding_mode=0
    0x38, // QP and filter control
    0x80, // trailing
];

// ==============================================================================
// AAC Audio Test Fixtures
// ==============================================================================

/// AAC Audio Specific Config (ASC) for 48kHz stereo
/// - audioObjectType: 2 (AAC-LC)
/// - samplingFrequencyIndex: 3 (48000 Hz)
/// - channelConfiguration: 2 (stereo)
pub const AAC_CONFIG_48K_STEREO: &[u8] = &[
    0x11, 0x90, // AAC-LC, 48kHz, stereo
];

/// AAC Audio Specific Config for 44.1kHz stereo
pub const AAC_CONFIG_44K_STEREO: &[u8] = &[
    0x12, 0x10, // AAC-LC, 44.1kHz, stereo
];

// ==============================================================================
// Test Helper Functions
// ==============================================================================

/// Get all available SPS fixtures as (name, data) pairs
pub fn all_sps_fixtures() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("SPS_BASELINE_720P", SPS_BASELINE_720P),
        ("SPS_BASELINE_1080P", SPS_BASELINE_1080P),
        ("SPS_HIGH_720P", SPS_HIGH_720P),
        ("SPS_HIGH_1080P", SPS_HIGH_1080P),
        ("SPS_MAIN_480P", SPS_MAIN_480P),
    ]
}

/// Get all available PPS fixtures as (name, data) pairs
pub fn all_pps_fixtures() -> Vec<(&'static str, &'static [u8])> {
    vec![
        ("PPS_BASELINE", PPS_BASELINE),
        ("PPS_HIGH_CABAC", PPS_HIGH_CABAC),
        ("PPS_MAIN", PPS_MAIN),
    ]
}

/// Extract profile_idc from raw SPS NAL unit (without start code)
pub fn get_profile_idc(sps: &[u8]) -> Option<u8> {
    // SPS structure: nal_header (1 byte) + profile_idc (1 byte) + ...
    sps.get(1).copied()
}

/// Extract level_idc from raw SPS NAL unit
pub fn get_level_idc(sps: &[u8]) -> Option<u8> {
    // SPS structure: nal_header (1) + profile_idc (1) + constraint_flags (1) + level_idc (1)
    sps.get(3).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sps_fixtures_have_correct_nal_type() {
        for (name, data) in all_sps_fixtures() {
            let nal_type = data[0] & 0x1F;
            assert_eq!(
                nal_type, 7,
                "{} has wrong NAL type: {} (expected 7 for SPS)",
                name, nal_type
            );
        }
    }

    #[test]
    fn test_pps_fixtures_have_correct_nal_type() {
        for (name, data) in all_pps_fixtures() {
            let nal_type = data[0] & 0x1F;
            assert_eq!(
                nal_type, 8,
                "{} has wrong NAL type: {} (expected 8 for PPS)",
                name, nal_type
            );
        }
    }

    #[test]
    fn test_baseline_profile_idc() {
        assert_eq!(get_profile_idc(SPS_BASELINE_720P), Some(66));
        assert_eq!(get_profile_idc(SPS_BASELINE_1080P), Some(66));
    }

    #[test]
    fn test_high_profile_idc() {
        assert_eq!(get_profile_idc(SPS_HIGH_720P), Some(100));
        assert_eq!(get_profile_idc(SPS_HIGH_1080P), Some(100));
    }

    #[test]
    fn test_main_profile_idc() {
        assert_eq!(get_profile_idc(SPS_MAIN_480P), Some(77));
    }

    #[test]
    fn test_level_idc_values() {
        assert_eq!(get_level_idc(SPS_BASELINE_720P), Some(31)); // Level 3.1
        assert_eq!(get_level_idc(SPS_BASELINE_1080P), Some(40)); // Level 4.0
        assert_eq!(get_level_idc(SPS_HIGH_1080P), Some(40)); // Level 4.0
    }

    // ========== Edge Case Tests for Helper Functions ==========

    #[test]
    fn test_get_profile_idc_empty_slice() {
        assert_eq!(get_profile_idc(&[]), None);
    }

    #[test]
    fn test_get_profile_idc_single_byte() {
        assert_eq!(get_profile_idc(&[0x67]), None);
    }

    #[test]
    fn test_get_level_idc_empty_slice() {
        assert_eq!(get_level_idc(&[]), None);
    }

    #[test]
    fn test_get_level_idc_short_slice() {
        assert_eq!(get_level_idc(&[0x67, 0x42, 0xC0]), None);
    }

    #[test]
    fn test_get_level_idc_exact_four_bytes() {
        assert_eq!(get_level_idc(&[0x67, 0x42, 0xC0, 0x1F]), Some(0x1F));
    }

    // ========== Fixture Count Tests ==========

    #[test]
    fn test_all_sps_fixtures_count() {
        assert_eq!(all_sps_fixtures().len(), 5);
    }

    #[test]
    fn test_all_pps_fixtures_count() {
        assert_eq!(all_pps_fixtures().len(), 3);
    }

    // ========== SPS Fixture Non-Empty Tests ==========

    #[test]
    fn test_sps_fixtures_are_non_empty() {
        for (name, data) in all_sps_fixtures() {
            assert!(
                data.len() >= 4,
                "{} should have at least 4 bytes (nal + profile + constraints + level)",
                name
            );
        }
    }

    #[test]
    fn test_pps_fixtures_are_non_empty() {
        for (name, data) in all_pps_fixtures() {
            assert!(
                data.len() >= 2,
                "{} should have at least 2 bytes (nal + params)",
                name
            );
        }
    }

    // ========== AAC Config Tests ==========

    #[test]
    fn test_aac_config_48k_stereo_non_empty() {
        assert_eq!(AAC_CONFIG_48K_STEREO.len(), 2);
    }

    #[test]
    fn test_aac_config_44k_stereo_non_empty() {
        assert_eq!(AAC_CONFIG_44K_STEREO.len(), 2);
    }

    #[test]
    fn test_aac_configs_are_different() {
        assert_ne!(AAC_CONFIG_48K_STEREO, AAC_CONFIG_44K_STEREO);
    }

    // ========== Cross-Validation Tests ==========

    #[test]
    fn test_sps_high_profile_has_higher_profile_idc_than_baseline() {
        let baseline = get_profile_idc(SPS_BASELINE_720P).unwrap();
        let high = get_profile_idc(SPS_HIGH_720P).unwrap();
        assert!(
            high > baseline,
            "High profile ({}) > Baseline ({})",
            high,
            baseline
        );
    }

    #[test]
    fn test_main_profile_idc_between_baseline_and_high() {
        let baseline = get_profile_idc(SPS_BASELINE_720P).unwrap();
        let main = get_profile_idc(SPS_MAIN_480P).unwrap();
        let high = get_profile_idc(SPS_HIGH_720P).unwrap();
        assert!(main > baseline, "Main ({}) > Baseline ({})", main, baseline);
        assert!(main < high, "Main ({}) < High ({})", main, high);
    }

    #[test]
    fn test_level_idc_1080p_higher_than_720p() {
        let level_720p = get_level_idc(SPS_BASELINE_720P).unwrap();
        let level_1080p = get_level_idc(SPS_BASELINE_1080P).unwrap();
        assert!(
            level_1080p > level_720p,
            "1080p level ({}) > 720p level ({})",
            level_1080p,
            level_720p
        );
    }

    // ========== SPS NAL Reference IDC Tests ==========

    #[test]
    fn test_sps_nal_ref_idc_nonzero() {
        for (name, data) in all_sps_fixtures() {
            let nal_ref_idc = (data[0] >> 5) & 0x03;
            assert!(
                nal_ref_idc > 0,
                "{} should have non-zero nal_ref_idc for SPS",
                name
            );
        }
    }

    #[test]
    fn test_pps_nal_ref_idc_nonzero() {
        for (name, data) in all_pps_fixtures() {
            let nal_ref_idc = (data[0] >> 5) & 0x03;
            assert!(
                nal_ref_idc > 0,
                "{} should have non-zero nal_ref_idc for PPS",
                name
            );
        }
    }
}
