use super::RtpPacket;
use super::define;
use super::errors::PackerError;
use super::errors::UnPackerError;
use crate::bytesio::TNetIO;
use crate::bytesio::bytes_reader::BytesReader;
use crate::streamhub::define::FrameData;
use async_trait::async_trait;
use bytes::BytesMut;
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

/// Adjusts the position backwards to exclude trailing zeros before a start code.
/// This handles the case of 4-byte start codes (00 00 00 01) where we need to
/// find the actual end of the previous NALU.
fn adjust_for_trailing_zeros(nalus: &[u8], mut pos: usize) -> usize {
    while pos > 0 && nalus[pos - 1] == 0 {
        pos -= 1;
    }
    pos
}

/// Extracts the next NALU from the buffer, returning the NALU data without start code.
/// Returns None if no start code is found (raw NALU case).
fn extract_next_nalu(nalus: &mut BytesMut) -> Option<BytesMut> {
    let first_pos = find_start_code(&nalus[..])?;

    let mut nalu_with_start_code = match find_start_code(&nalus[first_pos + 3..]) {
        Some(distance_to_first_pos) => {
            let second_pos =
                adjust_for_trailing_zeros(nalus, first_pos + 3 + distance_to_first_pos);
            nalus.split_to(second_pos)
        }
        None => nalus.split_to(nalus.len()),
    };

    Some(nalu_with_start_code.split_off(first_pos + 3))
}

pub async fn split_annexb_and_process<T: TVideoPacker>(
    nalus: &mut BytesMut,
    packer: &mut T,
) -> Result<(), PackerError> {
    while !nalus.is_empty() {
        if let Some(nalu) = extract_next_nalu(nalus) {
            packer.pack_nalu(nalu).await?;
        } else {
            // If the buffer contains a single raw NAL unit (no Annex-B start code),
            // treat the entire buffer as one NAL for robustness.
            let raw_nalu = nalus.split_to(nalus.len());
            if !raw_nalu.is_empty() {
                packer.pack_nalu(raw_nalu).await?;
            }
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
            tracing::error!(error = %err, "current_time_error");
            0
        }
    }
}

/// Generate an NTP timestamp (RFC 5905 format: 64-bit value) from a SystemTime.
/// Upper 32 bits: seconds since January 1, 1900 00:00 UTC
/// Lower 32 bits: fractional seconds (2^-32 second resolution)
/// Returns None if the system time is before the Unix epoch.
pub fn ntp_timestamp_from_system_time(st: SystemTime) -> Option<u64> {
    const NTP_UNIX_EPOCH_DIFF: u64 = 2208988800;

    let duration = st.duration_since(SystemTime::UNIX_EPOCH).ok()?;

    let unix_secs = duration.as_secs();
    let unix_nanos = duration.subsec_nanos() as u64;

    let ntp_secs = unix_secs + NTP_UNIX_EPOCH_DIFF;
    let ntp_frac = (unix_nanos << 32) / 1_000_000_000;

    Some((ntp_secs << 32) | ntp_frac)
}

/// Generate an NTP timestamp (RFC 5905 format: 64-bit value) from the current system time.
/// Upper 32 bits: seconds since January 1, 1900 00:00 UTC
/// Lower 32 bits: fractional seconds (2^-32 second resolution)
/// Returns None if system time is before Unix epoch.
pub fn ntp_timestamp() -> Option<u64> {
    ntp_timestamp_from_system_time(SystemTime::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};
    use crate::rtsp::rtp::errors::PackerErrorValue;
    use bytes::BytesMut;
    use std::sync::{Arc, Mutex};

    struct CapturePacker {
        nalus: Arc<Mutex<Vec<BytesMut>>>,
    }

    struct ErrorPacker;

    #[async_trait]
    impl TRtpReceiverForRtcp for CapturePacker {
        fn on_packet_for_rtcp_handler(&mut self, _f: OnRtpPacketFn2) {}
    }

    #[async_trait]
    impl TPacker for CapturePacker {
        async fn pack(
            &mut self,
            _nalus: &mut BytesMut,
            _timestamp: u32,
        ) -> Result<(), PackerError> {
            Ok(())
        }

        fn on_packet_handler(&mut self, _f: OnRtpPacketFn) {}
    }

    #[async_trait]
    impl TVideoPacker for CapturePacker {
        async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError> {
            self.nalus.lock().unwrap().push(nalu);
            Ok(())
        }
    }

    #[async_trait]
    impl TRtpReceiverForRtcp for ErrorPacker {
        fn on_packet_for_rtcp_handler(&mut self, _f: OnRtpPacketFn2) {}
    }

    #[async_trait]
    impl TPacker for ErrorPacker {
        async fn pack(
            &mut self,
            _nalus: &mut BytesMut,
            _timestamp: u32,
        ) -> Result<(), PackerError> {
            Ok(())
        }

        fn on_packet_handler(&mut self, _f: OnRtpPacketFn) {}
    }

    #[async_trait]
    impl TVideoPacker for ErrorPacker {
        async fn pack_nalu(&mut self, _nalu: BytesMut) -> Result<(), PackerError> {
            Err(PackerError {
                value: PackerErrorValue::BytesWriteError(BytesWriteError {
                    value: BytesWriteErrorValue::Timeout,
                }),
            })
        }
    }

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

    #[tokio::test]
    async fn test_split_annexb_raw_nalu_fallback() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let mut packer = CapturePacker {
            nalus: Arc::clone(&captured),
        };

        let mut nalus = BytesMut::from(&[0x65, 0x88, 0x84][..]);
        let result = split_annexb_and_process(&mut nalus, &mut packer).await;
        assert!(result.is_ok());
        let stored = captured.lock().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0], BytesMut::from(&[0x65, 0x88, 0x84][..]));
    }

    #[tokio::test]
    async fn test_split_annexb_and_process_mixed_start_codes_extracts_all_nalus() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let mut packer = CapturePacker {
            nalus: Arc::clone(&captured),
        };

        // Mixes 3-byte and 4-byte start codes and verifies boundary handling.
        let mut nalus = BytesMut::from(
            &[
                0x00, 0x00, 0x01, 0x65, 0xAA, 0xBB, 0x00, 0x00, 0x00, 0x01, 0x41, 0xCC, 0x00, 0x00,
                0x01, 0x06, 0xDD,
            ][..],
        );

        let result = split_annexb_and_process(&mut nalus, &mut packer).await;

        assert!(result.is_ok());
        assert!(nalus.is_empty());
        let stored = captured.lock().unwrap();
        assert_eq!(stored.len(), 3);
        assert_eq!(stored[0], BytesMut::from(&[0x65, 0xAA, 0xBB][..]));
        assert_eq!(stored[1], BytesMut::from(&[0x41, 0xCC][..]));
        assert_eq!(stored[2], BytesMut::from(&[0x06, 0xDD][..]));
    }

    // ========== ntp_timestamp_from_system_time Tests ==========

    #[test]
    fn test_ntp_timestamp_from_system_time_invalid_before_unix_epoch() {
        use std::time::Duration;
        let invalid_time = SystemTime::UNIX_EPOCH - Duration::from_secs(1);
        let result = super::ntp_timestamp_from_system_time(invalid_time);
        assert!(result.is_none(), "Expected None for time before Unix epoch");
    }

    #[test]
    fn test_ntp_timestamp_from_system_time_valid_after_unix_epoch() {
        use std::time::Duration;
        let valid_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1);
        let result = super::ntp_timestamp_from_system_time(valid_time);
        assert!(
            result.is_some(),
            "Expected Some for valid time after Unix epoch"
        );
        let ntp = result.unwrap();
        let ntp_secs = ntp >> 32;
        assert_eq!(ntp_secs, 2208988801, "NTP seconds should be epoch diff + 1");
    }

    #[test]
    fn test_ntp_timestamp_from_system_time_unix_epoch_zero() {
        let result = super::ntp_timestamp_from_system_time(SystemTime::UNIX_EPOCH);
        assert!(result.is_some());
        let ntp = result.unwrap();
        let ntp_secs = ntp >> 32;
        assert_eq!(
            ntp_secs, 2208988800,
            "NTP seconds at Unix epoch should be epoch diff"
        );
    }

    // ========== ntp_timestamp Tests ==========

    #[test]
    fn test_ntp_timestamp_returns_some() {
        let ts = ntp_timestamp();
        assert!(ts.is_some());
    }

    #[test]
    fn test_ntp_timestamp_returns_nonzero() {
        let ts = ntp_timestamp().expect("System time should be valid");
        assert!(ts > 0);
    }

    #[test]
    fn test_ntp_timestamp_upper_bits_reasonable() {
        let ts = ntp_timestamp().expect("System time should be valid");
        let ntp_secs = ts >> 32;
        // NTP epoch offset from Unix epoch is ~2.2 billion seconds
        // Current time in NTP seconds should be > 3.9 billion (year 2023+)
        assert!(ntp_secs > 3_900_000_000);
    }

    // ========== adjust_for_trailing_zeros Tests ==========

    #[test]
    fn test_adjust_for_trailing_zeros_no_zeros() {
        let data = [0x65, 0x88, 0x84, 0x21];
        assert_eq!(adjust_for_trailing_zeros(&data, 4), 4);
    }

    #[test]
    fn test_adjust_for_trailing_zeros_one_zero() {
        // pos=3, nalus[2]=0x00 → skip, nalus[1]=0x88 → stop → returns 2
        let data = [0x65, 0x88, 0x00, 0x01];
        assert_eq!(adjust_for_trailing_zeros(&data, 3), 2);
    }

    #[test]
    fn test_adjust_for_trailing_zeros_multiple_zeros() {
        let data = [0x65, 0x00, 0x00, 0x00, 0x01];
        assert_eq!(adjust_for_trailing_zeros(&data, 3), 1);
    }

    #[test]
    fn test_adjust_for_trailing_zeros_at_zero() {
        let data = [0x00, 0x00, 0x01];
        assert_eq!(adjust_for_trailing_zeros(&data, 0), 0);
    }

    // ========== extract_next_nalu Tests ==========

    #[test]
    fn test_extract_next_nalu_single() {
        let mut nalus = BytesMut::from(&[0x00, 0x00, 0x01, 0x65, 0x88][..]);
        let nalu = extract_next_nalu(&mut nalus);
        assert!(nalu.is_some());
        assert_eq!(nalu.unwrap(), BytesMut::from(&[0x65, 0x88][..]));
        assert!(nalus.is_empty());
    }

    #[test]
    fn test_extract_next_nalu_two_nalus() {
        let mut nalus = BytesMut::from(&[0x00, 0x00, 0x01, 0x65, 0x88, 0x00, 0x00, 0x01, 0x41][..]);
        let nalu1 = extract_next_nalu(&mut nalus);
        assert!(nalu1.is_some());
        assert_eq!(nalu1.unwrap(), BytesMut::from(&[0x65, 0x88][..]));

        let nalu2 = extract_next_nalu(&mut nalus);
        assert!(nalu2.is_some());
        assert_eq!(nalu2.unwrap(), BytesMut::from(&[0x41][..]));
    }

    #[test]
    fn test_extract_next_nalu_four_byte_start_code() {
        let mut nalus = BytesMut::from(
            &[
                0x00, 0x00, 0x00, 0x01, 0x65, 0xAA, 0x00, 0x00, 0x00, 0x01, 0x41,
            ][..],
        );
        let nalu1 = extract_next_nalu(&mut nalus);
        assert!(nalu1.is_some());
        assert_eq!(nalu1.unwrap(), BytesMut::from(&[0x65, 0xAA][..]));
    }

    #[test]
    fn test_extract_next_nalu_no_start_code() {
        let mut nalus = BytesMut::from(&[0x65, 0x88, 0x84][..]);
        let nalu = extract_next_nalu(&mut nalus);
        assert!(nalu.is_none());
    }

    #[test]
    fn test_extract_next_nalu_empty() {
        let mut nalus = BytesMut::new();
        let nalu = extract_next_nalu(&mut nalus);
        assert!(nalu.is_none());
    }

    #[tokio::test]
    async fn test_split_annexb_and_process_packer_error_is_propagated() {
        let mut packer = ErrorPacker;
        let mut nalus = BytesMut::from(&[0x00, 0x00, 0x01, 0x65, 0xAA][..]);

        let result = split_annexb_and_process(&mut nalus, &mut packer).await;

        assert!(matches!(
            result,
            Err(PackerError {
                value: PackerErrorValue::BytesWriteError(_)
            })
        ));
    }
}
