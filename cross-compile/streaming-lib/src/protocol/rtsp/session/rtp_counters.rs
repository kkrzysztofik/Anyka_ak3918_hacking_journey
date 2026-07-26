use byteorder::BigEndian;
use portable_atomic::AtomicU64;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

use super::errors::{SessionError, SessionErrorValue};
use crate::io::bytes_reader::BytesReader;

/// Half the 32-bit RTP timestamp space.
///
/// Used to distinguish a legitimate forward wrap from a backwards regression:
/// a delta greater than this value is treated as regression rather than
/// a normal forward advance.
pub(super) const RTP_TIMESTAMP_WRAP_THRESHOLD: u32 = 0x8000_0000;

/// Half the 16-bit RTP sequence number space.
///
/// Sequence deltas at or above this value are treated as backwards wrap/regression rather than
/// a small forward gap (see [`RtpTrackCounters::on_packet_sent`]).
pub(super) const RTP_SEQUENCE_WRAP_THRESHOLD: u16 = 0x8000;

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Lightweight per-track RTP send counters for diagnostics and anomaly logging.
///
/// Values are stored in atomics so producers can update metrics from async tasks
/// without a mutex on the RTP send path. Counts are best-effort (relaxed ordering):
/// suitable for logging, not billing.
///
/// # Fields
///
/// * `packet_count` — Total RTP packets recorded for this track.
/// * `byte_count` — Sum of payload sizes passed to [`RtpTrackCounters::on_packet_sent`].
/// * `first_send_ms` / `last_send_ms` — Wall-clock milliseconds since UNIX epoch when the
///   first/last packet was sent (`0` means “not yet sent” for `first_send_ms`).
/// * `last_seq` / `last_timestamp` — Previous RTP header values (`u32::MAX` means “none yet”).
pub(super) struct RtpTrackCounters {
    packet_count: AtomicU64,
    byte_count: AtomicU64,
    first_send_ms: AtomicU64,
    last_send_ms: AtomicU64,
    last_seq: AtomicU32,
    last_timestamp: AtomicU32,
}

/// Observations returned when recording one RTP packet on a track.
///
/// Deltas use RTP’s 16-bit sequence and 32-bit timestamp semantics (wrapping).
#[derive(Debug, Clone, Copy)]
pub(super) struct RtpPacketObservation {
    /// Running totals after this packet.
    pub(super) packets_sent: u64,
    pub(super) bytes_sent: u64,
    pub(super) prev_seq: Option<u16>,
    /// `seq - prev_seq` in modular arithmetic (`None` on first packet).
    pub(super) seq_delta: Option<u16>,
    pub(super) prev_timestamp: Option<u32>,
    pub(super) timestamp_delta: Option<u32>,
    /// `true` if sequence jumped by more than 1 (loss / reorder), excluding wrap.
    pub(super) seq_gap: bool,
    /// `true` if sequence moved “backwards” in modular space (often wrap).
    pub(super) seq_regressed: bool,
    /// `true` if timestamp delta suggests a backward move vs wrap threshold.
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

    // Used by `include!("rtp_counters_tests.rs")` unit tests; not referenced from production paths.
    #[allow(dead_code)]
    pub(super) fn packet_count(&self) -> u64 {
        self.packet_count.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(super) fn byte_count(&self) -> u64 {
        self.byte_count.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(super) fn first_send_ms(&self) -> u64 {
        self.first_send_ms.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(super) fn last_send_ms(&self) -> u64 {
        self.last_send_ms.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(super) fn last_seq_raw(&self) -> u32 {
        self.last_seq.load(Ordering::Relaxed)
    }

    #[allow(dead_code)]
    pub(super) fn last_timestamp_raw(&self) -> u32 {
        self.last_timestamp.load(Ordering::Relaxed)
    }

    /// Record a sent RTP packet and return counters plus monotonicity checks.
    ///
    /// Updates per-track totals and compares the new RTP sequence number and timestamp to the
    /// previous values to detect gaps, reordering, and timestamp regressions (vs legitimate wrap).
    ///
    /// # Arguments
    ///
    /// * `payload_len` — RTP payload size in bytes (excludes the 12-byte fixed header).
    /// * `seq` — RTP sequence number (`u16`, host byte order).
    /// * `timestamp` — RTP timestamp field (`u32`, media clock units).
    ///
    /// # Returns
    ///
    /// An [`RtpPacketObservation`] with running totals after this packet (`packets_sent`,
    /// `bytes_sent`) and flags such as [`RtpPacketObservation::seq_gap`] /
    /// [`RtpPacketObservation::seq_regressed`].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let c = RtpTrackCounters::new();
    /// let obs = c.on_packet_sent(1200, 501, 90_000);
    /// assert_eq!(obs.packets_sent, 1);
    /// assert_eq!(obs.bytes_sent, 1200);
    /// ```
    pub(super) fn on_packet_sent(
        &self,
        payload_len: usize,
        seq: u16,
        timestamp: u32,
    ) -> RtpPacketObservation {
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
        let seq_gap = matches!(
            seq_delta,
            Some(delta) if delta > 1 && delta < RTP_SEQUENCE_WRAP_THRESHOLD
        );
        let seq_regressed =
            matches!(seq_delta, Some(delta) if delta >= RTP_SEQUENCE_WRAP_THRESHOLD);

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

    /// Snapshot totals and an approximate send duration.
    ///
    /// Returns `(packet_count, byte_count, duration_ms)` where `duration_ms` is
    /// `last_send_ms - first_send_ms` when both timestamps are set and ordered;
    /// otherwise [`None`] (no packets yet, or clock went backwards).
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

/// Shared handle to [`RtpTrackCounters`] for use across async tasks (`Arc` clone is cheap).
pub(super) type RtpCountersHandle = Arc<RtpTrackCounters>;

/// Parsed RTSP TCP interleaved frame header (RFC 2326 §10.12).
///
/// Binary data (e.g. RTP) is prefixed with `$`, a one-byte channel id, and a
/// big-endian 16-bit length. The caller reads `length` bytes of payload after this header.
#[derive(Debug)]
pub struct InterleavedBinaryData {
    pub channel_identifier: u8,
    pub length: u16,
}

impl InterleavedBinaryData {
    /// Parse an interleaved header if the reader is positioned at `$` (`0x24`).
    ///
    /// # Arguments
    ///
    /// * `reader` — Byte reader. Uses [`BytesReader::advance_u8`], which **peeks** the next byte
    ///   (copies one byte without advancing the reader). If it is not `$`, returns `Ok(None)` and
    ///   leaves the cursor unchanged; if it is `$`, consumes `$` via `read_u8` before channel/length.
    ///
    /// # Errors
    ///
    /// * Underflow / malformed data while reading header fields.
    /// * Zero `length` (invalid for RTP payload framing).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut reader = BytesReader::new(buf);
    /// if let Some(h) = InterleavedBinaryData::new(&mut reader)? {
    ///     let payload = reader.read_bytes(h.length as usize)?;
    /// }
    /// ```
    pub fn new(reader: &mut BytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8()? == 0x24;
        if crate::stream_frame_debug_logging_enabled() {
            debug!(is_dollar_sign, "interleaved_parse");
        }
        if is_dollar_sign {
            // `advance_u8` already peeked '$'; consume it before channel/length.
            let _ = reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            if crate::stream_frame_debug_logging_enabled() {
                debug!(channel_identifier = channel_identifier, "channel_id_parse");
            }
            let length = reader.read_u16::<BigEndian>()?;
            if crate::stream_frame_debug_logging_enabled() {
                debug!(length = length, "interleaved_length");
            }
            if length == 0 {
                warn!(
                    channel = channel_identifier,
                    "zero_length_interleaved_payload"
                );
                return Err(SessionErrorValue::ZeroLengthInterleavedPayload.into());
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
/// Unit tests for RTP counters and interleaved framing (see `rtp_counters_tests.rs`).
mod tests {
    include!("rtp_counters_tests.rs");
}
