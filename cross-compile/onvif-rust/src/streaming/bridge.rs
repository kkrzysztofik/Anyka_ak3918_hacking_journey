//! Bridge between platform frame callbacks and streaming-lib channels.
//!
//! `StreamingBridge` implements [`FrameCallback`] to receive raw encoded frames
//! from the Anyka SDK and converts them into [`FrameData`] for the streaming
//! pipeline. It routes frames by [`StreamId`]:
//!
//! - `VideoMain` → main stream channel
//! - `VideoSub` → sub stream channel
//! - `Audio` → both main and sub stream channels

use bytes::BytesMut;
use parking_lot::RwLock;
use portable_atomic::AtomicU32;
use std::sync::Arc;
use streaming_lib::FrameData;
use tokio::sync::mpsc;

use crate::platform::frame::{Frame, FrameCallback, FrameType, StreamId};

/// Per-stream state maintained by the bridge.
pub struct StreamState {
    /// Channel for sending frames to the fanout task.
    pub frame_tx: mpsc::UnboundedSender<FrameData>,
    /// Cached SPS NAL unit (extracted from IDR frames).
    pub sps: RwLock<Option<Vec<u8>>>,
    /// Cached PPS NAL unit (extracted from IDR frames).
    pub pps: RwLock<Option<Vec<u8>>>,
    /// Last video timestamp in milliseconds (for late-joining subscribers).
    pub last_timestamp_ms: Arc<AtomicU32>,
    /// Cached latest IDR frame for late-joining subscribers (updated on every I-frame).
    pub bootstrap_idr: RwLock<Option<Vec<u8>>>,
}

/// Bridge that receives raw SDK frames and routes them to streaming channels.
///
/// Implements [`FrameCallback`] for integration with the platform abstraction.
/// The `on_frame` method must complete within 2ms — it performs only a memcpy
/// into `BytesMut` and an unbounded channel send.
pub struct StreamingBridge {
    /// State for the main video stream.
    pub main_stream: StreamState,
    /// State for the sub video stream.
    pub sub_stream: StreamState,
    /// Cached audio config bytes (AAC AudioSpecificConfig).
    pub audio_config: RwLock<Option<Vec<u8>>>,
    /// Audio sample rate in Hz.
    pub audio_sample_rate: u32,
}

impl StreamingBridge {
    /// Create a new bridge with the given frame channels.
    pub fn new(
        main_tx: mpsc::UnboundedSender<FrameData>,
        sub_tx: mpsc::UnboundedSender<FrameData>,
        audio_sample_rate: u32,
    ) -> Self {
        Self {
            main_stream: StreamState {
                frame_tx: main_tx,
                sps: RwLock::new(None),
                pps: RwLock::new(None),
                last_timestamp_ms: Arc::new(AtomicU32::new(0)),
                bootstrap_idr: RwLock::new(None),
            },
            sub_stream: StreamState {
                frame_tx: sub_tx,
                sps: RwLock::new(None),
                pps: RwLock::new(None),
                last_timestamp_ms: Arc::new(AtomicU32::new(0)),
                bootstrap_idr: RwLock::new(None),
            },
            audio_config: RwLock::new(None),
            audio_sample_rate,
        }
    }

    /// Convert a raw SDK frame to `FrameData` and route to the appropriate channel(s).
    fn route_frame(&self, frame: &Frame) {
        // SAFETY: frame.data is valid for frame.size bytes during this callback.
        let data = BytesMut::from(unsafe { std::slice::from_raw_parts(frame.data, frame.size) });
        let timestamp_ms = (frame.timestamp / 1000) as u32;

        match frame.stream_id {
            StreamId::VideoMain => {
                self.process_video_frame(&self.main_stream, frame, data, timestamp_ms);
            }
            StreamId::VideoSub => {
                self.process_video_frame(&self.sub_stream, frame, data, timestamp_ms);
            }
            StreamId::Audio => {
                let frame_data = FrameData::Audio {
                    timestamp: timestamp_ms,
                    data: data.clone(),
                };
                // Audio goes to both streams.
                let _ = self.main_stream.frame_tx.send(frame_data.clone());
                let _ = self.sub_stream.frame_tx.send(frame_data);
            }
        }
    }

    /// Process a video frame: extract SPS/PPS from IDR, cache bootstrap, and send.
    fn process_video_frame(
        &self,
        stream: &StreamState,
        frame: &Frame,
        data: BytesMut,
        timestamp_ms: u32,
    ) {
        stream
            .last_timestamp_ms
            .store(timestamp_ms, portable_atomic::Ordering::Relaxed);

        if frame.frame_type == FrameType::VideoIFrame {
            // Parse Annex-B NAL units to extract SPS and PPS.
            self.extract_parameter_sets(stream, &data);
            // Cache the full IDR for late-joining subscribers.
            *stream.bootstrap_idr.write() = Some(data.to_vec());
        }

        let frame_data = FrameData::Video {
            timestamp: timestamp_ms,
            data,
        };
        let _ = stream.frame_tx.send(frame_data);
    }

    /// Scan Annex-B NAL units in an IDR frame to extract and cache SPS/PPS.
    fn extract_parameter_sets(&self, stream: &StreamState, data: &[u8]) {
        for nal in AnnexBIterator::new(data) {
            if nal.is_empty() {
                continue;
            }
            let nal_type = nal[0] & 0x1F;
            match nal_type {
                7 => {
                    // SPS
                    *stream.sps.write() = Some(nal.to_vec());
                }
                8 => {
                    // PPS
                    *stream.pps.write() = Some(nal.to_vec());
                }
                _ => {}
            }
        }
    }
}

impl FrameCallback for StreamingBridge {
    fn on_frame(&self, frame: &Frame) {
        self.route_frame(frame);
    }
}

/// Iterator over Annex-B delimited NAL units in a byte slice.
///
/// Finds `00 00 01` or `00 00 00 01` start codes and yields the data between them.
struct AnnexBIterator<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> AnnexBIterator<'a> {
    fn new(data: &'a [u8]) -> Self {
        let mut iter = Self { data, pos: 0 };
        // Skip to the first start code.
        iter.skip_to_first_start_code();
        iter
    }

    fn skip_to_first_start_code(&mut self) {
        while self.pos < self.data.len() {
            if let Some(sc_len) = self.start_code_at(self.pos) {
                self.pos += sc_len;
                return;
            }
            self.pos += 1;
        }
    }

    fn start_code_at(&self, pos: usize) -> Option<usize> {
        let remaining = &self.data[pos..];
        if remaining.len() >= 4
            && remaining[0] == 0
            && remaining[1] == 0
            && remaining[2] == 0
            && remaining[3] == 1
        {
            Some(4)
        } else if remaining.len() >= 3
            && remaining[0] == 0
            && remaining[1] == 0
            && remaining[2] == 1
        {
            Some(3)
        } else {
            None
        }
    }
}

impl<'a> Iterator for AnnexBIterator<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.data.len() {
            return None;
        }

        let start = self.pos;

        // Scan for the next start code.
        while self.pos < self.data.len() {
            if let Some(sc_len) = self.start_code_at(self.pos) {
                let nal = &self.data[start..self.pos];
                self.pos += sc_len;
                return Some(nal);
            }
            self.pos += 1;
        }

        // Last NAL unit (no trailing start code).
        let nal = &self.data[start..self.data.len()];
        Some(nal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(data: &[u8], frame_type: FrameType, stream_id: StreamId) -> Frame {
        Frame {
            data: data.as_ptr(),
            size: data.len(),
            timestamp: 2_000_000, // 2 seconds in microseconds
            frame_type,
            stream_id,
        }
    }

    #[test]
    fn test_bridge_routes_video_main_to_main_channel() {
        let (main_tx, mut main_rx) = mpsc::unbounded_channel();
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let payload = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24];
        let frame = make_frame(&payload, FrameType::VideoPFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        assert!(main_rx.try_recv().is_ok());
        assert!(sub_rx.try_recv().is_err());
    }

    #[test]
    fn test_bridge_routes_video_sub_to_sub_channel() {
        let (main_tx, mut main_rx) = mpsc::unbounded_channel();
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let payload = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24];
        let frame = make_frame(&payload, FrameType::VideoPFrame, StreamId::VideoSub);
        bridge.on_frame(&frame);

        assert!(main_rx.try_recv().is_err());
        assert!(sub_rx.try_recv().is_ok());
    }

    #[test]
    fn test_bridge_routes_audio_to_both_channels() {
        let (main_tx, mut main_rx) = mpsc::unbounded_channel();
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let payload = vec![0xFF, 0xF1, 0x50, 0x80]; // Fake AAC frame
        let frame = make_frame(&payload, FrameType::AudioPacket, StreamId::Audio);
        bridge.on_frame(&frame);

        assert!(main_rx.try_recv().is_ok());
        assert!(sub_rx.try_recv().is_ok());
    }

    #[test]
    fn test_bridge_extracts_sps_from_annex_b_idr() {
        let (main_tx, _main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        // SPS NAL (type 7) + PPS NAL (type 8) + IDR (type 5)
        let idr_frame = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xce, 0x06, 0xe2, // PPS
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, // IDR slice
        ];
        let frame = make_frame(&idr_frame, FrameType::VideoIFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let sps = bridge.main_stream.sps.read();
        assert!(sps.is_some());
        let sps = sps.as_ref().unwrap();
        assert_eq!(sps[0] & 0x1F, 7); // NAL type SPS
        assert_eq!(sps, &[0x67, 0x42, 0x00, 0x1e]);
    }

    #[test]
    fn test_bridge_extracts_pps_from_annex_b_idr() {
        let (main_tx, _main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let idr_frame = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0x00, 0x1e, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xce, 0x06, 0xe2, // PPS
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, // IDR slice
        ];
        let frame = make_frame(&idr_frame, FrameType::VideoIFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let pps = bridge.main_stream.pps.read();
        assert!(pps.is_some());
        assert_eq!(pps.as_ref().unwrap(), &[0x68, 0xce, 0x06, 0xe2]);
    }

    #[test]
    fn test_bridge_caches_bootstrap_idr() {
        let (main_tx, _main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let idr_frame = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21, 0xA0];
        let frame = make_frame(&idr_frame, FrameType::VideoIFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let cached = bridge.main_stream.bootstrap_idr.read();
        assert!(cached.is_some());
        assert_eq!(cached.as_ref().unwrap(), &idr_frame);
    }

    #[test]
    fn test_bridge_timestamp_conversion_us_to_ms() {
        let (main_tx, mut main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let payload = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a];
        let frame = Frame {
            data: payload.as_ptr(),
            size: payload.len(),
            timestamp: 3_500_000, // 3500ms in microseconds
            frame_type: FrameType::VideoPFrame,
            stream_id: StreamId::VideoMain,
        };
        bridge.on_frame(&frame);

        let received = main_rx.try_recv().unwrap();
        match received {
            FrameData::Video { timestamp, .. } => {
                assert_eq!(timestamp, 3500); // microseconds → milliseconds
            }
            _ => panic!("expected video frame"),
        }
    }

    #[test]
    fn test_bridge_iframe_produces_correct_frame_data() {
        let (main_tx, mut main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let idr_frame = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84];
        let frame = make_frame(&idr_frame, FrameType::VideoIFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let received = main_rx.try_recv().unwrap();
        assert!(matches!(received, FrameData::Video { .. }));
    }

    #[test]
    fn test_bridge_pframe_produces_correct_frame_data() {
        let (main_tx, mut main_rx) = mpsc::unbounded_channel();
        let (sub_tx, _sub_rx) = mpsc::unbounded_channel();
        let bridge = StreamingBridge::new(main_tx, sub_tx, 48000);

        let p_frame = vec![0x00, 0x00, 0x00, 0x01, 0x41, 0x9a, 0x24];
        let frame = make_frame(&p_frame, FrameType::VideoPFrame, StreamId::VideoMain);
        bridge.on_frame(&frame);

        let received = main_rx.try_recv().unwrap();
        assert!(matches!(received, FrameData::Video { .. }));
    }

    #[test]
    fn test_annex_b_iterator_parses_three_code_start_codes() {
        let data = vec![
            0x00, 0x00, 0x01, 0xAA, 0xBB, // NAL 1
            0x00, 0x00, 0x01, 0xCC, // NAL 2
        ];
        let nals: Vec<&[u8]> = AnnexBIterator::new(&data).collect();
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0], &[0xAA, 0xBB]);
        assert_eq!(nals[1], &[0xCC]);
    }

    #[test]
    fn test_annex_b_iterator_parses_four_byte_start_codes() {
        let data = vec![
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, // SPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, // PPS
        ];
        let nals: Vec<&[u8]> = AnnexBIterator::new(&data).collect();
        assert_eq!(nals.len(), 2);
        assert_eq!(nals[0], &[0x67, 0x42]);
        assert_eq!(nals[1], &[0x68, 0xCE]);
    }

    #[test]
    fn test_annex_b_iterator_empty_data() {
        let data: Vec<u8> = vec![];
        let nals: Vec<&[u8]> = AnnexBIterator::new(&data).collect();
        assert!(nals.is_empty());
    }
}
