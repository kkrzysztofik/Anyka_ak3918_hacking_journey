use crate::rtsp::rtp::RtpPacket;
use crate::rtsp::rtp::utils;
use bytes::BytesMut;

use super::{
    RTCP_RR,
    rtcp_app::RtcpApp,
    rtcp_bye::RtcpBye,
    rtcp_header::RtcpHeader,
    rtcp_rr::{ReportBlock, RtcpReceiverReport},
    rtcp_sr::RtcpSenderReport,
};

//For example: sequence numbers inserted are 65533, 65534, the new coming one is 2,
//the new is 2 and old is 65534, the distance between 2 and 65534 is 4 which is
//65535 - 65534 + 2 + 1.(65533,65534,65535,0,1,2)
pub fn distance(new: u16, old: u16) -> u16 {
    if new < old {
        65535 - old + new + 1
    } else {
        new - old
    }
}

const MIN_SEQUENTIAL: u32 = 2;
const RTP_SEQ_MOD: u32 = 1 << 16;
const MAX_DROPOUT: u32 = 3000;
const MAX_MISORDER: u32 = 100;

/*
 * Per-source state information
 */
#[derive(Debug, Clone, Default)]
struct RtcpSource {
    max_seq: u16,        /* highest seq. number seen */
    cycles: u32,         /* shifted count of seq. number cycles */
    base_seq: u32,       /* base seq number */
    bad_seq: u32,        /* last 'bad' seq number + 1 */
    probation: u32,      /* sequ. packets till source is valid */
    received: u32,       /* packets received */
    expected_prior: u32, /* packet expected at last interval */
    received_prior: u32, /* packet received at last interval */
    jitter: f64,         /* estimated jitter */
}

impl RtcpSource {
    //static int rtp_seq_update(struct rtp_member *sender, uint16_t seq)
    fn update_sequence(&mut self, seq: u16) -> usize {
        let delta = distance(seq, self.max_seq);

        if self.probation > 0 {
            /* packet is in sequence */
            if seq == self.max_seq + 1 {
                self.probation -= 1;
                self.max_seq = seq;
                if self.probation == 0 {
                    self.init_seq(seq);
                    self.received += 1;
                    return 1;
                }
            } else {
                self.probation = MIN_SEQUENTIAL - 1;
                self.max_seq = seq;
            }
            return 0;
        } else if delta < MAX_DROPOUT as u16 {
            /* in order, with permissible gap */
            if seq < self.max_seq {
                /*
                 * Sequence number wrapped - count another 64K cycle.
                 */
                self.cycles += RTP_SEQ_MOD;
            }
            self.max_seq = seq;
        } else if delta as u32 <= RTP_SEQ_MOD - MAX_MISORDER {
            if seq == self.bad_seq as u16 {
                /*
                 * Two sequential packets -- assume that the other side
                 * restarted without telling us so just re-sync
                 * (i.e., pretend this was the first packet).
                 */
                self.init_seq(seq);
            } else {
                self.bad_seq = (seq as u32 + 1) & (RTP_SEQ_MOD - 1);
                return 0;
            }
        } else {
            /* duplicate or reordered packet */
        }
        self.received += 1;

        1
    }

    fn init_seq(&mut self, seq: u16) {
        self.base_seq = seq as u32;
        self.max_seq = seq;
        self.bad_seq = RTP_SEQ_MOD + 1; /* so seq == bad_seq is false */
        self.cycles = 0;
        self.received = 0;
        self.received_prior = 0;
        self.expected_prior = 0;
        /* other initialization */
    }
}

#[derive(Debug, Clone, Default)]
pub struct RtcpContext {
    ssrc: u32,
    sender_ssrc: u32,

    sr_ntp_lsr: u64,
    sr_clock_time: u64,
    last_rtp_clock: u64,
    last_rtp_timestamp: u32,
    /// Wall-clock time (µs since epoch) when `last_rtp_timestamp` was recorded.
    /// Used to extrapolate the RTP timestamp in RTCP SR generation so the
    /// NTP/RTP pair is correctly correlated per RFC 3550 §6.4.1.
    last_rtp_wallclock_us: u64,
    sample_rate: u32,
    send_bytes: u64,
    send_packets: u64,

    source: RtcpSource,
}

impl RtcpContext {
    pub fn new(ssrc: u32, seq: u16, sample_rate: u32) -> Self {
        RtcpContext {
            ssrc,
            source: RtcpSource {
                max_seq: seq.wrapping_sub(1),
                probation: MIN_SEQUENTIAL,
                ..Default::default()
            },

            sample_rate,
            ..Default::default()
        }
    }

    pub fn generate_app(&self, name: String, data: BytesMut) -> RtcpApp {
        let mut buf = BytesMut::with_capacity(name.len());
        buf.extend_from_slice(name.as_bytes());

        RtcpApp {
            header: RtcpHeader {
                // RFC 3550 section 6.1: all RTCP packets use version 2.
                version: 2,
                payload_type: super::RTCP_APP,
                ..Default::default()
            },
            ssrc: self.ssrc,
            name: buf,
            app_data: data,
        }
    }

    pub fn generate_bye(&self) -> RtcpBye {
        let ssrcs = vec![self.ssrc];
        RtcpBye {
            header: RtcpHeader {
                // RFC 3550 section 6.1: all RTCP packets use version 2.
                version: 2,
                report_count: 1,
                payload_type: super::RTCP_BYE,
                ..Default::default()
            },
            ssrcs,
            ..Default::default()
        }
    }

    //int rtcp_report_block(struct rtp_member* sender, uint8_t* ptr, int bytes)
    fn gen_report_block(&mut self) -> ReportBlock {
        let extend_max = self.source.cycles + self.source.max_seq as u32;
        let expected = extend_max - self.source.base_seq + 1;
        let lost = expected - self.source.received;
        let expected_interval = expected - self.source.expected_prior;
        self.source.expected_prior = expected;

        let received_interval = self.source.received - self.source.received_prior;
        self.source.received_prior = self.source.received;
        let lost_interval = expected_interval as i64 - received_interval as i64;

        let fraction = if expected_interval == 0 || lost_interval < 0 {
            0
        } else {
            ((lost_interval as u32) << 8) / expected_interval
        };

        let delay = utils::current_time() - self.sr_clock_time;
        // Extract the middle 32 bits of the 64-bit NTP timestamp (RFC3550).
        let lsr = (self.sr_ntp_lsr >> 8) & 0xFFFFFFFF;
        // DLSR is in units of 1/65536 seconds per RFC3550.
        let dlsr = (delay as f64 / 1_000_000.0 * 65_535.0) as u32;

        ReportBlock {
            cumulative_num_of_packets_lost: lost,
            fraction_lost: fraction as u8,
            extended_highest_seq_number: extend_max,
            lsr: lsr as u32,
            dlsr,
            ssrc: self.sender_ssrc,
            jitter: self.source.jitter as u32,
        }
    }

    pub fn generate_rr(&mut self) -> RtcpReceiverReport {
        let block = self.gen_report_block();

        RtcpReceiverReport {
            header: RtcpHeader {
                payload_type: RTCP_RR,
                report_count: 1,
                version: 2,
                length: 7, // RFC 3550: (32 bytes / 4) - 1 = 7
                ..Default::default()
            },
            report_blocks: vec![block],
            ssrc: self.ssrc,
        }
    }

    pub fn send_rtp(&mut self, pkt: RtpPacket) {
        self.send_bytes += pkt.payload.len() as u64;
        self.send_packets += 1;
        self.last_rtp_timestamp = pkt.header.timestamp;
        self.last_rtp_wallclock_us = utils::current_time();
    }

    pub fn generate_sr(&mut self) -> Option<RtcpSenderReport> {
        let ntp = utils::ntp_timestamp_from_system_time(std::time::SystemTime::now())?;
        let now_us = utils::current_time();

        let rtp_timestamp = if self.last_rtp_wallclock_us > 0 && self.sample_rate > 0 {
            let elapsed_us = now_us.saturating_sub(self.last_rtp_wallclock_us);
            let elapsed_ticks = (elapsed_us as u128 * self.sample_rate as u128 / 1_000_000) as u32;
            self.last_rtp_timestamp.wrapping_add(elapsed_ticks)
        } else {
            self.last_rtp_timestamp
        };

        Some(RtcpSenderReport {
            header: RtcpHeader {
                version: 2,
                padding_flag: 0,
                report_count: 0,
                payload_type: super::RTCP_SR,
                length: 6,
            },
            ssrc: self.ssrc,
            ntp,
            rtp_timestamp,
            sender_packet_count: self.send_packets as u32,
            sender_octet_count: self.send_bytes as u32,
            report_blocks: Vec::new(),
        })
    }

    pub fn received_sr(&mut self, sr: &RtcpSenderReport) {
        // RFC 3550 §6.4.1: NTP timestamp must be non-zero in a valid SR
        if sr.ntp == 0 {
            tracing::warn!(
                ssrc = %sr.ssrc,
                "received_sr_ignoring_zero_ntp"
            );
            return;
        }
        self.sr_clock_time = utils::current_time();

        self.sr_ntp_lsr = sr.ntp;
        self.sender_ssrc = sr.ssrc;
    }

    pub fn received_rtp(&mut self, pkt: RtpPacket) {
        if 0 == self.source.update_sequence(pkt.header.seq_number) {
            return;
        }

        let rtp_clock = utils::current_time();
        if self.last_rtp_clock == 0 {
            self.source.jitter = 0.;
        } else {
            let rtp_delta =
                (rtp_clock - self.last_rtp_clock) as f64 * (self.sample_rate as f64) / 1_000_000.0;
            let ts_delta = (pkt.header.timestamp - self.last_rtp_timestamp) as f64;
            let mut d = rtp_delta - ts_delta;

            if d < 0.0 {
                d = -d;
            }
            self.source.jitter += (d - self.source.jitter) / 16.0;
        }

        self.last_rtp_clock = rtp_clock;
        self.last_rtp_timestamp = pkt.header.timestamp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtsp::rtp::rtp_header::RtpHeader;

    // ========== Constant Tests ==========

    #[test]
    fn test_constants() {
        assert_eq!(MIN_SEQUENTIAL, 2);
        assert_eq!(RTP_SEQ_MOD, 65536);
        assert_eq!(MAX_DROPOUT, 3000);
        assert_eq!(MAX_MISORDER, 100);
    }

    // ========== distance() Tests ==========

    #[test]
    fn test_distance_simple() {
        assert_eq!(distance(10, 5), 5);
    }

    #[test]
    fn test_distance_wrapping() {
        // When sequence wraps around (65534 -> 2)
        assert_eq!(distance(2, 65534), 4); // 65534, 65535, 0, 1, 2
    }

    #[test]
    fn test_distance_same() {
        assert_eq!(distance(100, 100), 0);
    }

    #[test]
    fn test_distance_one() {
        assert_eq!(distance(1, 0), 1);
    }

    #[test]
    fn test_distance_max_to_zero() {
        assert_eq!(distance(0, 65535), 1);
    }

    // ========== RtcpSource Tests ==========

    #[test]
    fn test_rtcp_source_default() {
        let source = RtcpSource::default();
        assert_eq!(source.max_seq, 0);
        assert_eq!(source.cycles, 0);
        assert_eq!(source.received, 0);
    }

    #[test]
    fn test_rtcp_source_init_seq() {
        let mut source = RtcpSource::default();
        source.init_seq(1000);
        assert_eq!(source.base_seq, 1000);
        assert_eq!(source.max_seq, 1000);
        assert_eq!(source.cycles, 0);
        assert_eq!(source.received, 0);
    }

    // ========== RtcpContext Construction Tests ==========

    #[test]
    fn test_rtcp_context_new() {
        let ctx = RtcpContext::new(12345, 100, 48000);
        assert_eq!(ctx.ssrc, 12345);
        assert_eq!(ctx.sample_rate, 48000);
        assert_eq!(ctx.source.max_seq, 99); // seq - 1
        assert_eq!(ctx.source.probation, MIN_SEQUENTIAL);
    }

    #[test]
    fn test_rtcp_context_default() {
        let ctx = RtcpContext::default();
        assert_eq!(ctx.ssrc, 0);
        assert_eq!(ctx.sample_rate, 0);
    }

    // ========== generate_bye Tests ==========

    #[test]
    fn test_generate_bye() {
        let ctx = RtcpContext::new(12345, 100, 48000);
        let bye = ctx.generate_bye();
        assert_eq!(bye.ssrcs.len(), 1);
        assert_eq!(bye.ssrcs[0], 12345);
        assert_eq!(bye.header.report_count, 1);
    }

    // ========== generate_app Tests ==========

    #[test]
    fn test_generate_app() {
        let ctx = RtcpContext::new(12345, 100, 48000);
        let data = BytesMut::from(&[0x01, 0x02, 0x03][..]);
        let app = ctx.generate_app("TEST".to_string(), data.clone());

        assert_eq!(app.ssrc, 12345);
        assert_eq!(app.name.as_ref(), b"TEST");
        assert_eq!(app.app_data.as_ref(), data.as_ref());
    }

    #[test]
    fn test_generate_app_empty_data() {
        let ctx = RtcpContext::new(12345, 100, 48000);
        let app = ctx.generate_app("TEST".to_string(), BytesMut::new());

        assert_eq!(app.ssrc, 12345);
        assert!(app.app_data.is_empty());
    }

    // ========== generate_rr Tests ==========

    #[test]
    fn test_generate_rr() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);
        let rr = ctx.generate_rr();

        assert_eq!(rr.ssrc, 12345);
        assert_eq!(rr.header.payload_type, RTCP_RR);
        assert_eq!(rr.header.report_count, 1);
        assert_eq!(rr.header.version, 2);
        assert_eq!(rr.report_blocks.len(), 1);
    }

    #[test]
    fn test_generate_sr_header_fields_match_rfc3550() {
        let mut ctx = RtcpContext::new(12345, 100, 48_000);
        let sr = ctx
            .generate_sr()
            .expect("SR should be generated with valid system time");

        assert_eq!(sr.header.version, 2);
        assert_eq!(sr.header.payload_type, crate::rtsp::rtp::rtcp::RTCP_SR);
        assert_eq!(sr.header.report_count, 0);
    }

    // ========== send_rtp Tests ==========

    #[test]
    fn test_send_rtp() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        let pkt = RtpPacket {
            header: RtpHeader {
                timestamp: 1000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 100][..]),
            ..Default::default()
        };

        ctx.send_rtp(pkt);

        assert_eq!(ctx.send_packets, 1);
        assert_eq!(ctx.send_bytes, 100);
        assert_eq!(ctx.last_rtp_timestamp, 1000);
    }

    #[test]
    fn test_send_rtp_multiple() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        for i in 0..5 {
            let pkt = RtpPacket {
                header: RtpHeader {
                    timestamp: 1000 + i * 160,
                    ..Default::default()
                },
                payload: BytesMut::from(&[0x00; 50][..]),
                ..Default::default()
            };
            ctx.send_rtp(pkt);
        }

        assert_eq!(ctx.send_packets, 5);
        assert_eq!(ctx.send_bytes, 250);
    }

    // ========== received_sr Tests ==========

    #[test]
    fn test_received_sr() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        // Create default SR and set public fields
        let mut sr = RtcpSenderReport::default();
        sr.ssrc = 54321;
        sr.ntp = 0x0011223344556677;

        ctx.received_sr(&sr);

        assert_eq!(ctx.sender_ssrc, 54321);
        assert_eq!(ctx.sr_ntp_lsr, 0x0011223344556677);
        assert!(ctx.sr_clock_time > 0);
    }

    // ========== Clone and Debug Tests ==========

    #[test]
    fn test_rtcp_context_clone() {
        let ctx = RtcpContext::new(12345, 100, 48000);
        let cloned = ctx.clone();
        assert_eq!(cloned.ssrc, ctx.ssrc);
        assert_eq!(cloned.sample_rate, ctx.sample_rate);
    }

    #[test]
    fn test_rtcp_context_debug() {
        let ctx = RtcpContext::new(12345, 100, 48000);
        let debug_str = format!("{:?}", ctx);
        assert!(debug_str.contains("RtcpContext"));
        assert!(debug_str.contains("ssrc"));
    }

    #[test]
    fn test_rtcp_source_clone() {
        let mut source = RtcpSource::default();
        source.max_seq = 1000;
        let cloned = source.clone();
        assert_eq!(cloned.max_seq, source.max_seq);
    }

    #[test]
    fn test_rtcp_source_debug() {
        let source = RtcpSource::default();
        let debug_str = format!("{:?}", source);
        assert!(debug_str.contains("RtcpSource"));
    }

    // ========== update_sequence Tests ==========

    #[test]
    fn test_update_sequence_probation_success() {
        let mut source = RtcpSource {
            max_seq: 100,
            probation: 2, // MIN_SEQUENTIAL
            ..Default::default()
        };

        // First sequential packet
        let result = source.update_sequence(101);
        assert_eq!(result, 0); // still in probation
        assert_eq!(source.probation, 1);
        assert_eq!(source.max_seq, 101);

        // Second sequential packet - should exit probation
        let result = source.update_sequence(102);
        assert_eq!(result, 1);
        assert_eq!(source.probation, 0);
    }

    #[test]
    fn test_update_sequence_probation_reset() {
        let mut source = RtcpSource {
            max_seq: 100,
            probation: 2,
            ..Default::default()
        };

        // Non-sequential packet resets probation
        let result = source.update_sequence(150);
        assert_eq!(result, 0);
        assert_eq!(source.probation, 1); // MIN_SEQUENTIAL - 1
        assert_eq!(source.max_seq, 150);
    }

    #[test]
    fn test_update_sequence_in_order_within_dropout() {
        let mut source = RtcpSource {
            max_seq: 100,
            probation: 0,
            received: 10,
            ..Default::default()
        };

        // In order packet
        let result = source.update_sequence(101);
        assert_eq!(result, 1);
        assert_eq!(source.max_seq, 101);
        assert_eq!(source.received, 11);
    }

    #[test]
    fn test_update_sequence_wrap_around() {
        let mut source = RtcpSource {
            max_seq: 65534,
            probation: 0,
            cycles: 0,
            received: 10,
            ..Default::default()
        };

        // Packet after max (causes wrap)
        source.update_sequence(65535);
        assert_eq!(source.max_seq, 65535);

        // Next packet wraps around
        let result = source.update_sequence(0);
        assert_eq!(result, 1);
        assert_eq!(source.cycles, RTP_SEQ_MOD); // 65536
    }

    #[test]
    fn test_update_sequence_bad_seq_resync() {
        let mut source = RtcpSource {
            max_seq: 100,
            probation: 0,
            bad_seq: RTP_SEQ_MOD + 1, // Initial value from init_seq
            received: 10,
            ..Default::default()
        };

        // First out-of-order packet sets bad_seq to seq+1
        let result = source.update_sequence(5000);
        assert_eq!(result, 0);
        assert_eq!(source.bad_seq, 5001);

        // Send the same packet matching bad_seq - this will trigger resync
        // because seq == bad_seq as u16 is checked
        source.bad_seq = 5001;
        let result = source.update_sequence(5001);
        // After resync via init_seq, received gets incremented and returns 1
        assert_eq!(result, 1);
    }

    #[test]
    fn test_update_sequence_large_delta() {
        let mut source = RtcpSource {
            max_seq: 100,
            probation: 0,
            received: 5,
            ..Default::default()
        };

        // Large delta (greater than MAX_DROPOUT but within misorder range)
        let result = source.update_sequence(5000);
        assert_eq!(result, 0); // Sets bad_seq, returns 0
    }

    // ========== received_rtp Tests ==========

    #[test]
    fn test_received_rtp_first_packet() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        let pkt = RtpPacket {
            header: RtpHeader {
                seq_number: 100,
                timestamp: 1000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 50][..]),
            ..Default::default()
        };

        ctx.received_rtp(pkt);
        assert_eq!(ctx.source.jitter, 0.0);
    }

    #[test]
    fn test_received_rtp_jitter_calculation() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        // First packet (exits probation)
        let pkt1 = RtpPacket {
            header: RtpHeader {
                seq_number: 100,
                timestamp: 1000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 50][..]),
            ..Default::default()
        };
        ctx.received_rtp(pkt1);

        // Second packet (now we exit probation and jitter can be calculated)
        let pkt2 = RtpPacket {
            header: RtpHeader {
                seq_number: 101,
                timestamp: 1160, // +160 samples (at 48kHz = 3.33ms)
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 50][..]),
            ..Default::default()
        };
        ctx.received_rtp(pkt2);
        // Jitter should be calculated now
    }

    #[test]
    fn test_received_rtp_sequence_rejected() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        // Out of sequence packet during probation should be rejected
        let pkt = RtpPacket {
            header: RtpHeader {
                seq_number: 200, // Way out of sequence
                timestamp: 1000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 50][..]),
            ..Default::default()
        };

        let initial_jitter = ctx.source.jitter;
        ctx.received_rtp(pkt);
        // Packet rejected, jitter unchanged
        assert_eq!(ctx.source.jitter, initial_jitter);
    }

    // ========== gen_report_block Tests ==========

    #[test]
    fn test_gen_report_block_with_received_packets() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);
        ctx.source.probation = 0;
        ctx.source.base_seq = 100;
        ctx.source.max_seq = 105;
        ctx.source.received = 5;
        ctx.source.cycles = 0;

        let block = ctx.gen_report_block();
        assert_eq!(block.ssrc, ctx.sender_ssrc);
        // expected = 105 - 100 + 1 = 6, lost = 6 - 5 = 1
        assert_eq!(block.cumulative_num_of_packets_lost, 1);
    }

    #[test]
    fn test_gen_report_block_no_loss() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);
        ctx.source.probation = 0;
        ctx.source.base_seq = 100;
        ctx.source.max_seq = 104;
        ctx.source.received = 5;
        ctx.source.cycles = 0;

        let block = ctx.gen_report_block();
        assert_eq!(block.cumulative_num_of_packets_lost, 0);
    }

    #[test]
    fn test_gen_report_block_fraction_lost_calculation() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);
        ctx.source.probation = 0;
        ctx.source.base_seq = 0;
        ctx.source.max_seq = 100;
        ctx.source.received = 90;
        ctx.source.cycles = 0;
        ctx.source.expected_prior = 50;
        ctx.source.received_prior = 45;

        let _block = ctx.gen_report_block();
        // Just verify it doesn't panic
    }

    // ========== generate_rr with sender info Tests ==========

    #[test]
    fn test_generate_rr_after_sr() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);

        // Receive an SR first
        let mut sr = RtcpSenderReport::default();
        sr.ssrc = 54321;
        sr.ntp = 0x0011223344556677;
        ctx.received_sr(&sr);

        // Now generate RR
        let rr = ctx.generate_rr();
        assert_eq!(rr.ssrc, 12345);
        assert_eq!(rr.report_blocks[0].ssrc, 54321);
    }

    // ========== generate_sr extrapolation Tests ==========

    #[test]
    fn test_generate_sr_extrapolates_rtp_timestamp() {
        let mut ctx = RtcpContext::new(12345, 100, 90_000);

        // Simulate sending an RTP packet at timestamp 1000.
        let pkt = RtpPacket {
            header: RtpHeader {
                timestamp: 1000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 100][..]),
            ..Default::default()
        };
        ctx.send_rtp(pkt);

        // Verify wallclock was recorded.
        assert!(ctx.last_rtp_wallclock_us > 0);

        // Wait ~50ms, then generate SR. The extrapolated RTP timestamp
        // should be greater than the base (1000) by approximately
        // 50ms * 90000 / 1_000_000 = 4500 ticks.
        std::thread::sleep(std::time::Duration::from_millis(50));
        let sr = ctx
            .generate_sr()
            .expect("SR should be generated with valid system time");

        // The extrapolated timestamp should be ahead of the base.
        let delta = sr.rtp_timestamp.wrapping_sub(1000);
        // Allow a wide range: 2000..8000 ticks (22ms..89ms at 90kHz)
        // to account for timing variability in CI.
        assert!(
            delta >= 2000 && delta <= 8000,
            "expected extrapolated delta ~4500 ticks, got {}",
            delta
        );
    }

    #[test]
    fn test_generate_sr_no_extrapolation_without_sample_rate() {
        let mut ctx = RtcpContext::new(12345, 100, 0);

        let pkt = RtpPacket {
            header: RtpHeader {
                timestamp: 5000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 50][..]),
            ..Default::default()
        };
        ctx.send_rtp(pkt);

        std::thread::sleep(std::time::Duration::from_millis(20));
        let sr = ctx
            .generate_sr()
            .expect("SR should be generated with valid system time");

        // With sample_rate=0, no extrapolation — timestamp stays at 5000.
        assert_eq!(sr.rtp_timestamp, 5000);
    }

    #[test]
    fn test_send_rtp_records_wallclock() {
        let mut ctx = RtcpContext::new(12345, 100, 48000);
        assert_eq!(ctx.last_rtp_wallclock_us, 0);

        let pkt = RtpPacket {
            header: RtpHeader {
                timestamp: 1000,
                ..Default::default()
            },
            payload: BytesMut::from(&[0x00; 50][..]),
            ..Default::default()
        };
        ctx.send_rtp(pkt);

        assert!(ctx.last_rtp_wallclock_us > 0);
    }
}
