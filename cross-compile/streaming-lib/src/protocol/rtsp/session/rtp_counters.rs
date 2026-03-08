use byteorder::BigEndian;
use portable_atomic::AtomicU64;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use crate::io::bytes_reader::BytesReader;
use super::errors::SessionError;

/// Half the 32-bit RTP timestamp space.
///
/// Used to distinguish a legitimate forward wrap from a backwards regression:
/// a delta greater than this value is treated as regression rather than
/// a normal forward advance.
pub(super) const RTP_TIMESTAMP_WRAP_THRESHOLD: u32 = 0x8000_0000;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lightweight per-track RTP counters for logging.
///
/// These use atomics so they can be safely updated from async contexts
/// without introducing additional locking on the hot path.
pub(super) struct RtpTrackCounters {
    pub(super) packet_count: AtomicU64,
    pub(super) byte_count: AtomicU64,
    pub(super) first_send_ms: AtomicU64,
    pub(super) last_send_ms: AtomicU64,
    pub(super) last_seq: AtomicU32,
    pub(super) last_timestamp: AtomicU32,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RtpPacketObservation {
    pub(super) packets_sent: u64,
    pub(super) bytes_sent: u64,
    pub(super) prev_seq: Option<u16>,
    pub(super) seq_delta: Option<u16>,
    pub(super) prev_timestamp: Option<u32>,
    pub(super) timestamp_delta: Option<u32>,
    pub(super) seq_gap: bool,
    pub(super) seq_regressed: bool,
    pub(super) timestamp_regressed: bool,
}

impl RtpTrackCounters {
    pub(super) fn new() -> Self {
        Self {
            packet_count: AtomicU64::new(0),
            byte_count: AtomicU64::new(0),
            first_send_ms: AtomicU64::new(0),
            last_send_ms: AtomicU64::new(0),
            last_seq: AtomicU32::new(u32::MAX),
            last_timestamp: AtomicU32::new(u32::MAX),
        }
    }

    /// Record a sent RTP packet and return counters plus monotonicity checks.
    pub(super) fn on_packet_sent(&self, payload_len: usize, seq: u16, timestamp: u32) -> RtpPacketObservation {
        let now = now_millis();

        // First-send timestamp (best-effort, race-safe).
        let _ = self
            .first_send_ms
            .compare_exchange(0, now, Ordering::Relaxed, Ordering::Relaxed);
        self.last_send_ms.store(now, Ordering::Relaxed);

        let packets = self.packet_count.fetch_add(1, Ordering::Relaxed) + 1;
        let bytes = self
            .byte_count
            .fetch_add(payload_len as u64, Ordering::Relaxed)
            + payload_len as u64;

        let prev_seq_raw = self.last_seq.swap(seq as u32, Ordering::Relaxed);
        let prev_timestamp_raw = self.last_timestamp.swap(timestamp, Ordering::Relaxed);

        let prev_seq = if prev_seq_raw == u32::MAX {
            None
        } else {
            Some(prev_seq_raw as u16)
        };
        let prev_timestamp = if prev_timestamp_raw == u32::MAX {
            None
        } else {
            Some(prev_timestamp_raw)
        };

        let seq_delta = prev_seq.map(|prev| seq.wrapping_sub(prev));
        let seq_gap = matches!(seq_delta, Some(delta) if delta > 1 && delta < 0x8000);
        let seq_regressed = matches!(seq_delta, Some(delta) if delta >= 0x8000);

        let timestamp_delta = prev_timestamp.map(|prev| timestamp.wrapping_sub(prev));
        let timestamp_regressed =
            matches!(timestamp_delta, Some(delta) if delta > RTP_TIMESTAMP_WRAP_THRESHOLD);

        RtpPacketObservation {
            packets_sent: packets,
            bytes_sent: bytes,
            prev_seq,
            seq_delta,
            prev_timestamp,
            timestamp_delta,
            seq_gap,
            seq_regressed,
            timestamp_regressed,
        }
    }

    pub(super) fn snapshot(&self) -> (u64, u64, Option<u64>) {
        let packets = self.packet_count.load(Ordering::Relaxed);
        let bytes = self.byte_count.load(Ordering::Relaxed);
        let first = self.first_send_ms.load(Ordering::Relaxed);
        let last = self.last_send_ms.load(Ordering::Relaxed);
        let duration_ms = if first > 0 && last >= first {
            Some(last - first)
        } else {
            None
        };
        (packets, bytes, duration_ms)
    }
}

pub(super) type RtpCountersHandle = Arc<RtpTrackCounters>;

pub struct InterleavedBinaryData {
    pub channel_identifier: u8,
    pub length: u16,
}

impl InterleavedBinaryData {
    // 10.12 Embedded (Interleaved) Binary Data
    // Stream data such as RTP packets is encapsulated by an ASCII dollar
    // sign (24 hexadecimal), followed by a one-byte channel identifier,
    // followed by the length of the encapsulated binary data as a binary,
    // two-byte integer in network byte order
    pub fn new(reader: &mut BytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8()? == 0x24;
        if crate::stream_frame_debug_logging_enabled() {
            debug!(is_dollar_sign, "interleaved_parse");
        }
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            if crate::stream_frame_debug_logging_enabled() {
                debug!(channel_identifier = channel_identifier, "channel_id_parse");
            }
            let length = reader.read_u16::<BigEndian>()?;
            if crate::stream_frame_debug_logging_enabled() {
                debug!(length = length, "interleaved_length");
            }
            // RFC 2326 §10.12: validate interleaved payload length
            if length == 0 {
                warn!(
                    channel = channel_identifier,
                    "zero_length_interleaved_payload"
                );
            }
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

#[cfg(test)]
#[path = "rtp_counters_tests.rs"]
mod tests;
