use super::RtpPacket;
use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use crate::bytesio::TNetIO;
use crate::bytesio::bytes_reader::BytesReader;
use crate::streamhub::define::FrameData;
use async_trait::async_trait;
use bytes::BytesMut;
use log::error;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

pub trait Unmarshal<T1, T2> {
    fn unmarshal(data: T1) -> T2
    where
        Self: Sized;
}

pub trait Marshal<T> {
    fn marshal(&self) -> T;
}

pub type OnFrameFn = Box<dyn Fn(FrameData) -> Result<(), UnPackerError> + Send + Sync>;

//Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> : The network connection used by packer to send a/v data
//BytesMut: The Rtp packet data that will be sent using the TNetIO
pub type OnRtpPacketFn = Box<
    dyn Fn(
            Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
            RtpPacket,
        ) -> Pin<Box<dyn Future<Output = Result<(), PackerError>> + Send + 'static>>
        + Send
        + Sync,
>;

pub type OnRtpPacketFn2 =
    Box<dyn Fn(RtpPacket) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> + Send + Sync>;
// pub type OnPacketFn2 = Box<dyn Fn(&RtpPacket) + Send + Sync>;

pub trait TRtpReceiverForRtcp {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2);
}

#[async_trait]
pub trait TPacker: TRtpReceiverForRtcp + Send + Sync {
    /*Split frame to rtp packets and send out*/
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError>;
    /*Call back function used for processing a rtp packet.*/
    fn on_packet_handler(&mut self, f: OnRtpPacketFn);
}

#[async_trait]
pub trait TVideoPacker: TPacker {
    /*pack one nalu to rtp packets*/
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError>;
}

#[async_trait]
pub trait TUnPacker: TRtpReceiverForRtcp + Send + Sync {
    /*Assemble rtp fragments into complete frame and send to stream hub*/
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError>;
    /*Call back function used for processing a frame.*/
    fn on_frame_handler(&mut self, f: OnFrameFn);
}

pub(super) fn is_fu_start(fu_header: u8) -> bool {
    fu_header & define::FU_START > 0
}

pub(super) fn is_fu_end(fu_header: u8) -> bool {
    fu_header & define::FU_END > 0
}

pub fn find_start_code(nalus: &[u8]) -> Option<usize> {
    let pattern = [0x00, 0x00, 0x01];
    nalus.windows(pattern.len()).position(|w| w == pattern)
}

pub async fn split_annexb_and_process<T: TVideoPacker>(
    nalus: &mut BytesMut,
    packer: &mut T,
) -> Result<(), PackerError> {
    while !nalus.is_empty() {
        /* 0x02,...,0x00,0x00,0x01,0x02..,0x00,0x00,0x01  */
        /*  |         |              |      |             */
        /*  -----------              --------             */
        /*   first_pos         distance_to_first_pos      */
        if let Some(first_pos) = find_start_code(&nalus[..]) {
            let mut nalu_with_start_code =
                if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                    let mut second_pos = first_pos + 3 + distance_to_first_pos;
                    while second_pos > 0 && nalus[second_pos - 1] == 0 {
                        second_pos -= 1;
                    }
                    nalus.split_to(second_pos)
                } else {
                    nalus.split_to(nalus.len())
                };

            let nalu = nalu_with_start_code.split_off(first_pos + 3);
            packer.pack_nalu(nalu).await?;
        } else {
            break;
        }
    }
    Ok(())
}

pub fn current_time() -> u64 {
    let duration = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH);

    match duration {
        Ok(result) => (result.as_nanos() / 1000) as u64,
        Err(err) => {
            error!("current_time error: {err}");
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // ========== find_start_code Tests ==========

    #[test]
    fn test_find_start_code_at_beginning() {
        let nalus = [0x00, 0x00, 0x01, 0x65, 0x88, 0x84];
        assert_eq!(find_start_code(&nalus), Some(0));
    }

    #[test]
    fn test_find_start_code_in_middle() {
        let nalus = [0x88, 0x84, 0x00, 0x00, 0x01, 0x65, 0x88];
        assert_eq!(find_start_code(&nalus), Some(2));
    }

    #[test]
    fn test_find_start_code_at_end() {
        let nalus = [0x88, 0x84, 0x99, 0x00, 0x00, 0x01];
        assert_eq!(find_start_code(&nalus), Some(3));
    }

    #[test]
    fn test_find_start_code_not_found() {
        let nalus = [0x88, 0x84, 0x00, 0x00, 0x02, 0x65];
        assert_eq!(find_start_code(&nalus), None);
    }

    #[test]
    fn test_find_start_code_empty_slice() {
        let nalus: [u8; 0] = [];
        assert_eq!(find_start_code(&nalus), None);
    }

    #[test]
    fn test_find_start_code_too_short() {
        let nalus = [0x00, 0x00];
        assert_eq!(find_start_code(&nalus), None);
    }

    #[test]
    fn test_find_start_code_multiple_occurrences() {
        let nalus = [0x00, 0x00, 0x01, 0x65, 0x00, 0x00, 0x01, 0x41];
        // Should return the first occurrence
        assert_eq!(find_start_code(&nalus), Some(0));
    }

    #[test]
    fn test_find_start_code_with_four_byte_prefix() {
        // 4-byte start code: 00 00 00 01
        let nalus = [0x00, 0x00, 0x00, 0x01, 0x65];
        // Should find 00 00 01 at position 1
        assert_eq!(find_start_code(&nalus), Some(1));
    }

    // ========== is_fu_start Tests ==========

    #[test]
    fn test_is_fu_start_true() {
        // FU_START bit is set (bit 7 = 0x80)
        let fu_header = define::FU_START;
        assert!(is_fu_start(fu_header));
    }

    #[test]
    fn test_is_fu_start_false() {
        // FU_START bit is not set
        let fu_header = 0x00;
        assert!(!is_fu_start(fu_header));
    }

    #[test]
    fn test_is_fu_start_with_other_bits() {
        // FU_START bit set along with other bits
        let fu_header = define::FU_START | 0x1F;
        assert!(is_fu_start(fu_header));
    }

    #[test]
    fn test_is_fu_start_with_end_bit_only() {
        // Only FU_END bit set
        let fu_header = define::FU_END;
        assert!(!is_fu_start(fu_header));
    }

    // ========== is_fu_end Tests ==========

    #[test]
    fn test_is_fu_end_true() {
        // FU_END bit is set (bit 6 = 0x40)
        let fu_header = define::FU_END;
        assert!(is_fu_end(fu_header));
    }

    #[test]
    fn test_is_fu_end_false() {
        // FU_END bit is not set
        let fu_header = 0x00;
        assert!(!is_fu_end(fu_header));
    }

    #[test]
    fn test_is_fu_end_with_other_bits() {
        // FU_END bit set along with other bits
        let fu_header = define::FU_END | 0x1F;
        assert!(is_fu_end(fu_header));
    }

    #[test]
    fn test_is_fu_end_with_start_bit_only() {
        // Only FU_START bit set
        let fu_header = define::FU_START;
        assert!(!is_fu_end(fu_header));
    }

    #[test]
    fn test_is_fu_start_and_end_both_set() {
        // Both FU_START and FU_END bits set (unusual but valid)
        let fu_header = define::FU_START | define::FU_END;
        assert!(is_fu_start(fu_header));
        assert!(is_fu_end(fu_header));
    }

    // ========== current_time Tests ==========

    #[test]
    fn test_current_time_returns_nonzero() {
        let time = current_time();
        assert!(time > 0);
    }

    #[test]
    fn test_current_time_is_monotonic() {
        let time1 = current_time();
        std::thread::sleep(std::time::Duration::from_micros(10));
        let time2 = current_time();
        assert!(time2 >= time1);
    }

    #[test]
    fn test_current_time_is_in_microseconds() {
        let time = current_time();
        // Current time in microseconds since epoch should be a large number
        // (at least billions for years after 2000)
        assert!(time > 1_000_000_000_000u64);
    }

    // ========== Annexb Split Test ==========

    #[test]
    fn test_annexb_split() {
        let mut nalus = BytesMut::new();
        nalus.extend_from_slice(&[
            0x00, 0x00, 0x01, 0x02, 0x03, 0x05, 0x06, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04,
            0x00, 0x00, 0x01, 0x02, 0x03,
        ]);

        let mut nalu_count = 0;
        while !nalus.is_empty() {
            if let Some(first_pos) = find_start_code(&nalus[..]) {
                let mut nalu_with_start_code =
                    if let Some(distance_to_first_pos) = find_start_code(&nalus[first_pos + 3..]) {
                        let mut second_pos = first_pos + 3 + distance_to_first_pos;
                        while second_pos > 0 && nalus[second_pos - 1] == 0 {
                            second_pos -= 1;
                        }
                        nalus.split_to(second_pos)
                    } else {
                        nalus.split_to(nalus.len())
                    };

                let _nalu = nalu_with_start_code.split_off(first_pos + 3);
                nalu_count += 1;
            } else {
                break;
            }
        }
        assert_eq!(nalu_count, 3);
    }

    #[test]
    fn test_annexb_single_nalu() {
        let nalus = [0x00, 0x00, 0x01, 0x65, 0x88, 0x84];
        assert_eq!(find_start_code(&nalus), Some(0));
        // After first start code, no more start codes
        assert_eq!(find_start_code(&nalus[3..]), None);
    }

    #[test]
    fn test_annexb_back_to_back_start_codes() {
        let nalus = [0x00, 0x00, 0x01, 0x00, 0x00, 0x01, 0x65];
        assert_eq!(find_start_code(&nalus), Some(0));
        assert_eq!(find_start_code(&nalus[3..]), Some(0));
    }
}
