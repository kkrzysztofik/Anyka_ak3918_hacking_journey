pub const ANNEXB_NALU_START_CODE: [u8; 4] = [0x00, 0x00, 0x00, 0x01];

pub type RtpNalType = u8;
//H264
pub const STAP_A: RtpNalType = 24;
pub const STAP_B: RtpNalType = 25;
pub const MTAP_16: RtpNalType = 26;
pub const MTAP_24: RtpNalType = 27;
pub const FU_A: RtpNalType = 28;
pub const FU_B: RtpNalType = 29;
// H265
pub const AP: RtpNalType = 48; //Aggregation Packets
pub const FU: RtpNalType = 49; //Fragmentation Units
pub const PACI: RtpNalType = 50;

pub const FU_START: u8 = 0x80;
pub const FU_END: u8 = 0x40;

pub const RTP_FIXED_HEADER_LEN: usize = 12;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annexb_nalu_start_code() {
        assert_eq!(ANNEXB_NALU_START_CODE, [0x00, 0x00, 0x00, 0x01]);
        assert_eq!(ANNEXB_NALU_START_CODE.len(), 4);
    }

    #[test]
    fn test_h264_nal_types() {
        assert_eq!(STAP_A, 24);
        assert_eq!(STAP_B, 25);
        assert_eq!(MTAP_16, 26);
        assert_eq!(MTAP_24, 27);
        assert_eq!(FU_A, 28);
        assert_eq!(FU_B, 29);
    }

    #[test]
    fn test_h265_nal_types() {
        assert_eq!(AP, 48);
        assert_eq!(FU, 49);
        assert_eq!(PACI, 50);
    }

    #[test]
    fn test_fu_flags() {
        assert_eq!(FU_START, 0x80);
        assert_eq!(FU_END, 0x40);
        // FU_START and FU_END should not overlap
        assert_eq!(FU_START & FU_END, 0);
    }

    #[test]
    fn test_rtp_fixed_header_len() {
        assert_eq!(RTP_FIXED_HEADER_LEN, 12);
    }

    #[test]
    fn test_h264_nal_types_are_contiguous() {
        // STAP-A through FU-B should be contiguous range 24-29
        assert_eq!(STAP_B, STAP_A + 1);
        assert_eq!(MTAP_16, STAP_A + 2);
        assert_eq!(MTAP_24, STAP_A + 3);
        assert_eq!(FU_A, STAP_A + 4);
        assert_eq!(FU_B, STAP_A + 5);
    }

    #[test]
    fn test_h265_nal_types_are_contiguous() {
        assert_eq!(FU, AP + 1);
        assert_eq!(PACI, AP + 2);
    }
}
