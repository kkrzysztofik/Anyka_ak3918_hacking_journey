use crate::rtsp::global_trait::Marshal;
use crate::rtsp::global_trait::Unmarshal;
use crate::rtsp::rtsp_codec;
use chrono::Utc;

use crate::rtsp::rtp::define::ANNEXB_NALU_START_CODE;
use crate::rtsp::rtp::utils::Marshal as RtpMarshal;

use crate::common::auth::SecretCarrier;
use crate::common::http::HttpRequest as RtspRequest;
use crate::common::http::HttpResponse as RtspResponse;
use crate::common::http::Marshal as RtspMarshal;
use crate::common::http::Unmarshal as RtspUnmarshal;
use crate::common::http::try_get_complete_message_len;

use crate::rtsp::rtp::RtpPacket;
use crate::rtsp::rtsp_range::RtspRange;

use crate::rtsp::sdp::fmtp::Fmtp;

use crate::rtsp::rtsp_channel::RtpChannel;
use crate::rtsp::rtsp_codec::RtspCodecInfo;
use crate::rtsp::rtsp_track::RtspTrack;
use crate::rtsp::rtsp_track::TrackType;
use crate::rtsp::rtsp_transport::ProtocolType;
use crate::rtsp::rtsp_transport::RtspTransport;

use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::AsyncBytesWriter;
use byteorder::BigEndian;
use bytes::BytesMut;

use super::errors::SessionError;
use super::errors::SessionErrorValue;
use crate::bytesio::UdpIO;
use crate::bytesio::bytes_writer::BytesWriter;
use crate::streamhub::define::DataSender;
use crate::streamhub::define::MediaInfo;
use crate::streamhub::define::VideoCodecType;
use http::StatusCode;
use tokio::sync::oneshot;

use crate::rtsp::rtp::errors::UnPackerError;
use crate::rtsp::sdp::Sdp;

use super::define;
use super::define::rtsp_method_name;
use crate::bytesio::TNetIO;
use crate::bytesio::{TcpReadIO, TcpWriteIO};
use async_trait::async_trait;
use portable_atomic::AtomicU64;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::common::auth::Auth;
use crate::streamhub::{
    define::{
        FrameData, Information, InformationSender, NotifyInfo, PublishType, PublisherInfo,
        StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    },
    errors::{StreamHubError, StreamHubErrorValue},
    statistics::StatisticsStream,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

/// Global RTP sample interval (in packets) used for validation-style logging.
///
/// When set to a non-zero value, every Nth RTP packet per track will emit a
/// `event=rtp_packet_sample` debug log with sequence number, timestamp and size.
/// This defaults to 0 (disabled) for normal operation and can be enabled by
/// callers such as the onvif-rust validation mode.
static RTP_SAMPLE_INTERVAL: AtomicU32 = AtomicU32::new(0);

/// Configure the RTP packet sampling interval for RTSP sessions.
///
/// A value of 0 disables sampling (default). A small value like 10 will log
/// very frequently, while larger values (e.g. 100 or 1000) are better suited
/// for validation runs.
pub fn set_rtp_sample_interval(interval: u32) {
    RTP_SAMPLE_INTERVAL.store(interval, Ordering::Relaxed);
}

fn rtp_sample_interval() -> u32 {
    RTP_SAMPLE_INTERVAL.load(Ordering::Relaxed)
}

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
struct RtpTrackCounters {
    packet_count: AtomicU64,
    byte_count: AtomicU64,
    first_send_ms: AtomicU64,
    last_send_ms: AtomicU64,
    last_seq: AtomicU32,
    last_timestamp: AtomicU32,
}

const RTP_TIMESTAMP_WRAP_THRESHOLD: u32 = 0x8000_0000;
const SESSION_ID_RANDOM_DIGITS: RandomDigitCount = RandomDigitCount::Four;

/// RFC 2326 §12.36 — Server header value.
const SERVER_HEADER: &str = "streaming-lib/0.1";

#[derive(Debug, Clone, Copy)]
struct RtpTimestampSample {
    output_timestamp: u32,
    scaled_timestamp: u32,
    previous_scaled_timestamp: Option<u32>,
    non_wrap_regressed: bool,
    non_wrap_regression_count: u64,
}

/// Keeps RTP timestamps monotonic for source-side resets while preserving true RTP wrap.
#[derive(Debug, Default)]
struct RtpTimestampNormalizer {
    previous_scaled_timestamp: Option<u32>,
    correction: u32,
    non_wrap_regression_count: u64,
}

impl RtpTimestampNormalizer {
    /// Normalize RTP timestamps for audio or video.
    ///
    /// # Arguments
    ///
    /// * `source_timestamp` - For audio: timestamp in sample units (per RFC 3640).
    ///   For video: timestamp in milliseconds.
    /// * `clock_rate` - RTP clock rate (e.g., 48000 for AAC, 90000 for H264)
    /// * `track_type` - Audio or Video track type
    ///
    /// # Timestamp Units
    ///
    /// - **Audio (AAC)**: Timestamps are already in sample units (1024 samples/frame)
    ///   and match the clock_rate. No scaling is performed.
    /// - **Video (H264/H265)**: Timestamps are in milliseconds and must be scaled
    ///   to 90kHz RTP clock rate.
    fn normalize(
        &mut self,
        source_timestamp: u32,
        clock_rate: u32,
        track_type: TrackType,
    ) -> RtpTimestampSample {
        // Audio timestamps are already in sample units (RFC 3640), don't scale.
        // Video timestamps are in milliseconds, scale to 90kHz RTP clock.
        let scaled_timestamp = match track_type {
            TrackType::Audio => source_timestamp,
            TrackType::Video => {
                RtspServerSession::scale_rtp_timestamp(source_timestamp, clock_rate)
            }
            TrackType::Application => {
                RtspServerSession::scale_rtp_timestamp(source_timestamp, clock_rate)
            }
        };
        let previous_scaled_timestamp = self.previous_scaled_timestamp;

        let mut non_wrap_regressed = false;
        if let Some(previous) = previous_scaled_timestamp
            && scaled_timestamp <= previous
            && previous.wrapping_sub(scaled_timestamp) <= RTP_TIMESTAMP_WRAP_THRESHOLD
        {
            // RFC 3550 section 5.1 requires RTP timestamps to reflect sampling instant.
            // In our validation streams (single access unit cadence, no B-frames), equal or
            // regressed source timestamps usually indicate source-side resets/chunking artifacts.
            // Shift by a correction offset so emitted access units remain strictly monotonic.
            let corrected_current = scaled_timestamp.wrapping_add(self.correction);
            let corrected_previous = previous.wrapping_add(self.correction);
            let target = corrected_previous.wrapping_add(1);
            let adjustment = target.wrapping_sub(corrected_current);
            self.correction = self.correction.wrapping_add(adjustment);
            self.non_wrap_regression_count += 1;
            non_wrap_regressed = true;
        }

        self.previous_scaled_timestamp = Some(scaled_timestamp);
        RtpTimestampSample {
            output_timestamp: scaled_timestamp.wrapping_add(self.correction),
            scaled_timestamp,
            previous_scaled_timestamp,
            non_wrap_regressed,
            non_wrap_regression_count: self.non_wrap_regression_count,
        }
    }
}

/// Coalesce H.264 access-units that arrive as multiple same-timestamp chunks.
///
/// Some publishers emit one `FrameData::Video` per NAL unit while keeping the
/// same timestamp for all NALs belonging to an access unit. RTP packetizers
/// expect a full access unit per `on_frame` call; sending NALs individually
/// causes multiple marker-terminated access units at the same RTP timestamp,
/// which strict demuxers flag as "same timestamp as previous access unit".
#[derive(Debug, Default)]
struct VideoAccessUnitAssembler {
    pending_timestamp: Option<u32>,
    pending_bytes: BytesMut,
}

impl VideoAccessUnitAssembler {
    fn push(&mut self, timestamp: u32, chunk: BytesMut) -> Option<(u32, BytesMut)> {
        if chunk.is_empty() {
            return None;
        }

        match self.pending_timestamp {
            None => {
                self.pending_timestamp = Some(timestamp);
                self.append_as_annexb(&chunk[..]);
                None
            }
            Some(pending_ts) if pending_ts == timestamp => {
                self.append_as_annexb(&chunk[..]);
                None
            }
            Some(_pending_ts) => {
                let flushed = self.flush();
                self.pending_timestamp = Some(timestamp);
                self.append_as_annexb(&chunk[..]);
                flushed
            }
        }
    }

    fn flush(&mut self) -> Option<(u32, BytesMut)> {
        let timestamp = self.pending_timestamp.take()?;
        if self.pending_bytes.is_empty() {
            return None;
        }
        let bytes = std::mem::take(&mut self.pending_bytes);
        Some((timestamp, bytes))
    }

    fn append_as_annexb(&mut self, chunk: &[u8]) {
        if chunk.is_empty() {
            return;
        }
        if has_annexb_start_code(chunk) {
            self.pending_bytes.extend_from_slice(chunk);
            return;
        }
        self.pending_bytes
            .extend_from_slice(&ANNEXB_NALU_START_CODE[..]);
        self.pending_bytes.extend_from_slice(chunk);
    }
}

fn has_annexb_start_code(data: &[u8]) -> bool {
    data.starts_with(&[0x00, 0x00, 0x01]) || data.starts_with(&ANNEXB_NALU_START_CODE[..])
}

struct PlaybackVideoSendContext<'a> {
    session_id: &'a str,
    remote_addr: std::net::SocketAddr,
    request_path: &'a str,
    shutdown: &'a Arc<AtomicBool>,
}

async fn send_video_access_unit(
    video_channel: &Arc<Mutex<RtpChannel>>,
    timestamp_normalizers: &mut HashMap<TrackType, RtpTimestampNormalizer>,
    ctx: &PlaybackVideoSendContext<'_>,
    timestamp: u32,
    data: &mut BytesMut,
) {
    let mut channel = video_channel.lock().await;
    let normalized = timestamp_normalizers
        .entry(TrackType::Video)
        .or_default()
        .normalize(timestamp, channel.clock_rate(), TrackType::Video);
    if normalized.non_wrap_regressed {
        log::warn!(
            "event=rtp_timestamp_non_wrap_regression track=Video session_id={} remote_addr={} stream_path={} prev_scaled={} current_scaled={} corrected_timestamp={} regression_count={}",
            ctx.session_id,
            ctx.remote_addr,
            ctx.request_path,
            normalized.previous_scaled_timestamp.unwrap_or_default(),
            normalized.scaled_timestamp,
            normalized.output_timestamp,
            normalized.non_wrap_regression_count,
        );
    }
    if let Err(err) = channel.on_frame(data, normalized.output_timestamp).await {
        log::info!("handle_play error: {err}");
        ctx.shutdown.store(true, Ordering::Release);
    }
}

#[derive(Debug, Clone, Copy)]
struct RtpPacketObservation {
    packets_sent: u64,
    bytes_sent: u64,
    prev_seq: Option<u16>,
    seq_delta: Option<u16>,
    prev_timestamp: Option<u32>,
    timestamp_delta: Option<u32>,
    seq_gap: bool,
    seq_regressed: bool,
    timestamp_regressed: bool,
}

impl RtpTrackCounters {
    fn new() -> Self {
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
    fn on_packet_sent(&self, payload_len: usize, seq: u16, timestamp: u32) -> RtpPacketObservation {
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

    fn snapshot(&self) -> (u64, u64, Option<u64>) {
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

type RtpCountersHandle = Arc<RtpTrackCounters>;

pub struct RtspServerSession {
    io_reader: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<Uuid>,
    pub session_type: define::ServerSessionType,
    has_published: bool,
    has_subscribed: bool,

    stream_handler: Arc<RtspStreamHandler>,
    event_producer: StreamHubEventSender,

    auth: Option<Auth>,

    pub stream_identifier: Option<StreamIdentifier>,
    pub is_normal_exit: bool,
    remote_addr: SocketAddr,
    shutdown: Arc<AtomicBool>,

    /// Packet-level RTP logging configuration and counters.
    ///
    /// `rtp_sample_interval` is copied from the global `RTP_SAMPLE_INTERVAL`
    /// at session creation time so that each session has a consistent view
    /// even if the global is changed later.
    rtp_sample_interval: u32,
    rtp_counters: HashMap<TrackType, RtpCountersHandle>,
    playback_cancel: Option<Arc<AtomicBool>>,
    playback_task: Option<JoinHandle<()>>,
}

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
            log::debug!("dollar sign: {}", is_dollar_sign);
        }
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            if crate::stream_frame_debug_logging_enabled() {
                log::debug!("channel_identifier: {}", channel_identifier);
            }
            let length = reader.read_u16::<BigEndian>()?;
            if crate::stream_frame_debug_logging_enabled() {
                log::debug!("length: {}", length);
            }
            // RFC 2326 §10.12: validate interleaved payload length
            if length == 0 {
                log::warn!(
                    "interleaved: zero-length payload on channel {}",
                    channel_identifier
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

impl RtspServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
    ) -> Self {
        let remote_addr = stream
            .peer_addr()
            .or_else(|_| stream.local_addr())
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 0)));
        let (read_half, write_half) = stream.into_split();
        let read_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpReadIO::new(read_half));
        let write_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpWriteIO::new(write_half));

        Self::new_with_io_pair(read_io, write_io, event_producer, auth, remote_addr)
    }

    pub fn new_with_io(
        io: Box<dyn TNetIO + Send + Sync>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
    ) -> Self {
        // Tests and in-memory sessions can share one IO object for both read and write.
        let io = Arc::new(Mutex::new(io));
        Self::new_with_shared_io(io, event_producer, auth, remote_addr)
    }

    pub fn new_with_io_pair(
        read_io: Box<dyn TNetIO + Send + Sync>,
        write_io: Box<dyn TNetIO + Send + Sync>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
    ) -> Self {
        let read_io = Arc::new(Mutex::new(read_io));
        let write_io = Arc::new(Mutex::new(write_io));
        Self::new_with_reader_writer_io(read_io, write_io, event_producer, auth, remote_addr)
    }

    fn new_with_shared_io(
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
    ) -> Self {
        Self::new_with_reader_writer_io(io.clone(), io, event_producer, auth, remote_addr)
    }

    fn new_with_reader_writer_io(
        io_reader: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
    ) -> Self {
        let sample_interval = rtp_sample_interval();
        Self {
            io_reader,
            io_writer: io_writer.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io_writer),
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            session_type: define::ServerSessionType::Push,
            has_published: false,
            has_subscribed: false,
            event_producer,
            stream_handler: Arc::new(RtspStreamHandler::new()),
            auth,
            stream_identifier: None,
            is_normal_exit: false,
            remote_addr,
            shutdown: Arc::new(AtomicBool::new(false)),
            rtp_sample_interval: sample_interval,
            rtp_counters: HashMap::new(),
            playback_cancel: None,
            playback_task: None,
        }
    }

    /// RFC 2326 §12.37: default session timeout in seconds.
    const SESSION_TIMEOUT_SECS: u64 = 60;

    pub async fn run(&mut self) -> Result<(), SessionError> {
        let run_result: Result<(), SessionError> = async {
            loop {
                if self.shutdown.load(Ordering::Acquire) {
                    break;
                }
                while self.reader.len() < 4 {
                    let data = match tokio::time::timeout(
                        std::time::Duration::from_secs(Self::SESSION_TIMEOUT_SECS),
                        self.io_reader.lock().await.read(),
                    )
                    .await
                    {
                        Ok(result) => result?,
                        Err(_) => {
                            log::info!(
                                "event=rtsp_session_timeout timeout_secs={} remote_addr={}",
                                Self::SESSION_TIMEOUT_SECS,
                                self.remote_addr,
                            );
                            return Err(SessionError {
                                value: SessionErrorValue::SessionTimeout(
                                    Self::SESSION_TIMEOUT_SECS,
                                ),
                            });
                        }
                    };
                    self.reader.extend_from_slice(&data[..]);
                }
                // If delivering media data using RTP over RTSP(TCP), then it should use InterleavedBinaryData
                // to distinguish RTP from RTSP messges; If delivering media data over UDP, it will establish
                // separate udp channels for audio RTP data and video RTP data.

                // TODO: Here, some optimizations can be made since it's not necessary to use InterleavedBinaryData
                // in all cases.
                if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
                    match data {
                        Some(a) => {
                            while self.reader.len() < a.length as usize {
                                let data = self.io_reader.lock().await.read().await?;
                                self.reader.extend_from_slice(&data[..]);
                            }
                            self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                                .await?;
                        }
                        None => {
                            self.on_rtsp_message().await?;
                        }
                    }
                }
            }
            Ok(())
        }
        .await;

        self.stop_playback_task().await;

        // Ensure StreamHub receives unsubscribe/unpublish even when the run loop exits
        // without an explicit TEARDOWN/PAUSE (e.g. remote disconnect or internal shutdown).
        // This prevents stale subscriber counts from keeping on-demand publishers active.
        if !self.is_normal_exit
            && let Some(identifier) = self.stream_identifier.clone()
        {
            match self.exit(identifier) {
                Ok(()) => {}
                Err(cleanup_err) => {
                    // Preserve the original run error when present, but surface cleanup
                    // failures when run itself was otherwise successful.
                    if run_result.is_ok() {
                        return Err(cleanup_err);
                    }
                }
            }
        }

        run_result
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }

    fn abort_playback_task(&mut self) {
        if let Some(cancel) = self.playback_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(handle) = self.playback_task.take() {
            handle.abort();
        }
    }

    async fn stop_playback_task(&mut self) {
        if let Some(cancel) = self.playback_cancel.take() {
            cancel.store(true, Ordering::Release);
        }
        if let Some(handle) = self.playback_task.take() {
            handle.abort();
            match handle.await {
                Ok(()) => {}
                Err(err) if err.is_cancelled() => {}
                Err(err) => {
                    log::warn!("playback task join error: {err}");
                }
            }
        }
    }

    async fn on_rtp_over_rtsp_message(
        &mut self,
        channel_identifier: u8,
        length: usize,
    ) -> Result<(), SessionError> {
        let mut cur_reader = BytesReader::new(self.reader.read_bytes(length)?);

        for track in self.tracks.values_mut() {
            if let Some(interleaveds) = track.transport.interleaved {
                let rtp_identifier = interleaveds[0];
                let rtcp_identifier = interleaveds[1];

                if channel_identifier == rtp_identifier {
                    track.on_rtp(&mut cur_reader).await?;
                } else if channel_identifier == rtcp_identifier {
                    track.on_rtcp(&mut cur_reader, self.io_writer.clone()).await;
                }
            }
        }
        Ok(())
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let mut retry_count = 0;
        let message_len = loop {
            let data = self.reader.get_remaining_bytes();
            match try_get_complete_message_len(&data) {
                Ok(Some(len)) => break len,
                Ok(None) => {
                    if retry_count >= 16 {
                        return Err(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(
                                "max read retries exceeded".to_string(),
                            ),
                        });
                    }
                    retry_count += 1;
                    let data_recv = self.io_reader.lock().await.read().await?;
                    self.reader.extend_from_slice(&data_recv[..]);
                }
                Err(err) => {
                    return Err(SessionError {
                        value: SessionErrorValue::RtspMessageCorrupted(err),
                    });
                }
            }
        };

        let message_bytes = self.reader.read_bytes(message_len)?;
        let message_str = std::str::from_utf8(&message_bytes)?;

        let Some(rtsp_request) = RtspRequest::unmarshal(message_str) else {
            return Err(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("request parse failed".to_string()),
            });
        };

        // H-07: RFC 2326 §6: reject requests with unsupported RTSP version
        if rtsp_request.version != "RTSP/1.0" {
            log::warn!(
                "event=rtsp_unsupported_version version={} remote_addr={}",
                rtsp_request.version,
                self.remote_addr,
            );
            let response =
                Self::gen_response(http::StatusCode::HTTP_VERSION_NOT_SUPPORTED, &rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        // M-01: RFC 2326 §4.4: validate Content-Length against actual body
        if let Some(cl_str) = rtsp_request.get_header("Content-Length") {
            let claimed: usize = cl_str.trim().parse().unwrap_or(0);
            let actual = rtsp_request.body.as_ref().map_or(0, |b| b.len());
            if claimed != actual {
                log::warn!(
                    "event=rtsp_content_length_mismatch claimed={} actual={} remote_addr={}",
                    claimed,
                    actual,
                    self.remote_addr,
                );
                let response = Self::gen_response(http::StatusCode::BAD_REQUEST, &rtsp_request);
                self.send_response(&response).await?;
                return Ok(());
            }
        }

        match rtsp_request.method.as_str() {
            rtsp_method_name::OPTIONS => {
                self.handle_options(&rtsp_request).await?;
            }
            rtsp_method_name::DESCRIBE => {
                self.handle_describe(&rtsp_request).await?;
            }
            rtsp_method_name::ANNOUNCE => {
                self.handle_announce(&rtsp_request).await?;
            }
            rtsp_method_name::SETUP => {
                self.handle_setup(&rtsp_request).await?;
            }
            rtsp_method_name::PLAY => {
                if let Err(err) = self.handle_play(&rtsp_request).await {
                    log::info!("handle_play error: {}", err);
                }
            }
            rtsp_method_name::RECORD => {
                self.handle_record(&rtsp_request).await?;
            }
            rtsp_method_name::TEARDOWN => {
                self.stop_playback_task().await;
                if let Some(response) = self.validate_session_id(&rtsp_request) {
                    self.send_response(&response).await?;
                } else {
                    self.handle_teardown(&rtsp_request)?;
                    let mut response = Self::gen_response(http::StatusCode::OK, &rtsp_request);
                    if let Some(session_id) = self.session_id {
                        response
                            .headers
                            .insert("Session".to_string(), session_id.to_string());
                    }
                    self.send_response(&response).await?;
                }
            }
            rtsp_method_name::PAUSE => {
                self.handle_pause(&rtsp_request).await?;
            }
            rtsp_method_name::GET_PARAMETER => {
                self.handle_get_parameter(&rtsp_request).await?;
            }
            rtsp_method_name::SET_PARAMETER => {
                self.handle_set_parameter(&rtsp_request).await?;
            }
            rtsp_method_name::REDIRECT => {
                self.handle_redirect(&rtsp_request).await?;
            }

            _ => {
                log::warn!(
                    "event=rtsp_unknown_method method={} remote_addr={}",
                    rtsp_request.method,
                    self.remote_addr,
                );
                let response = Self::gen_response(http::StatusCode::NOT_IMPLEMENTED, &rtsp_request);
                self.send_response(&response).await?;
            }
        }
        Ok(())
    }

    async fn handle_options(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);
        let public_str = rtsp_method_name::PUBLIC_METHODS.join(", ");
        response.headers.insert("Public".to_string(), public_str);
        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        log::info!(
            "event=rtsp_request method=OPTIONS cseq={} session_id={} remote_addr={} stream_path={} session_type={}",
            cseq,
            session_id,
            self.remote_addr,
            rtsp_request.uri.path,
            self.session_type,
        );

        Ok(())
    }

    async fn handle_describe(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if self.auth.is_none() {
            let has_authorization_header = rtsp_request.get_header("Authorization").is_some();
            let has_userinfo_in_uri = rtsp_request.uri.host.contains('@');
            if has_authorization_header || has_userinfo_in_uri {
                let response = Self::gen_response(http::StatusCode::UNAUTHORIZED, rtsp_request);
                self.send_response(&response).await?;
                return Ok(());
            }
        }

        // The sender is used for sending sdp information from the server session to client session
        // receiver is used to receive the sdp information
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let stream_path = self.normalize_rtsp_stream_path(&rtsp_request.uri.path);
        let identifier = StreamIdentifier::Rtsp { stream_path };
        self.stream_identifier = Some(identifier.clone());

        let request_event = StreamHubEvent::Request { identifier, sender };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await
            && let Ok(sdp) = Sdp::unmarshal(&data)
        {
            self.sdp = sdp;
            //it can new tracks when get the sdp information;
            self.new_tracks()?;
        }

        // M-02: RFC 2326 §12.1: honour Accept header in DESCRIBE
        if let Some(accept) = rtsp_request.get_header("Accept") {
            let dominated = accept.contains("application/sdp") || accept.contains("*/*");
            if !dominated {
                log::warn!(
                    "event=rtsp_not_acceptable accept=\"{}\" remote_addr={}",
                    accept,
                    self.remote_addr,
                );
                let response = Self::gen_response(http::StatusCode::NOT_ACCEPTABLE, rtsp_request);
                self.send_response(&response).await?;
                return Ok(());
            }
        }

        if self.sdp.medias.is_empty() {
            let response = Self::gen_response(http::StatusCode::NOT_FOUND, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        let sdp = self.sdp.marshal();
        response.body = Some(sdp);
        response
            .headers
            .insert("Content-Type".to_string(), "application/sdp".to_string());
        if let Some(content_base) = self.build_content_base(rtsp_request) {
            response
                .headers
                .insert("Content-Base".to_string(), content_base);
        }
        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        let media_count = self.sdp.medias.len();
        log::info!(
            "event=rtsp_request method=DESCRIBE cseq={} session_id={} remote_addr={} stream_path={} session_type={} sdp_media_count={}",
            cseq,
            session_id,
            self.remote_addr,
            rtsp_request.uri.path,
            self.session_type,
            media_count,
        );

        Ok(())
    }

    async fn handle_announce(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(
                &stream_name,
                &rtsp_request
                    .uri
                    .query
                    .as_ref()
                    .map(|q| SecretCarrier::Query(q.to_string())),
                false,
            )?;
        }

        if let Some(request_body) = &rtsp_request.body
            && let Ok(sdp) = Sdp::unmarshal(request_body)
        {
            self.sdp = sdp.clone();
            self.stream_handler.set_sdp(sdp).await;
        }

        //new tracks for publish session
        self.new_tracks()?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let identifier = StreamIdentifier::Rtsp {
            stream_path: rtsp_request.uri.path.clone(),
        };
        self.stream_identifier = Some(identifier.clone());

        let publish_event = StreamHubEvent::Publish {
            identifier,
            result_sender: event_result_sender,
            info: self.get_publisher_info(),
            stream_handler: self.stream_handler.clone(),
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let sender = event_result_receiver.await??.0.ok_or(SessionError {
            value: SessionErrorValue::MissingFrameSender,
        })?;

        for track in self.tracks.values_mut() {
            let sender_out = sender.clone();
            let mut rtp_channel_guard = track.rtp_channel.lock().await;

            rtp_channel_guard.on_frame_handler(Box::new(
                move |msg: FrameData| -> Result<(), UnPackerError> {
                    if let Err(err) = sender_out.send(msg) {
                        log::error!("send frame error: {}", err);
                    }
                    Ok(())
                },
            ));

            let rtcp_channel = Arc::clone(&track.rtcp_channel);
            rtp_channel_guard.on_packet_for_rtcp_handler(Box::new(move |packet: RtpPacket| {
                let rtcp_channel_in = Arc::clone(&rtcp_channel);
                Box::pin(async move {
                    rtcp_channel_in.lock().await.on_packet(packet);
                })
            }));
        }

        self.has_published = true;
        self.session_type = define::ServerSessionType::Push;

        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_setup(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);

        if self.stream_identifier.is_none() {
            let normalized_path = self.normalize_rtsp_stream_path(&rtsp_request.uri.path);
            if !normalized_path.is_empty() {
                self.stream_identifier = Some(StreamIdentifier::Rtsp {
                    stream_path: normalized_path,
                });
            }
        }

        self.ensure_tracks_from_streamhub(rtsp_request).await?;

        let request_uri = rtsp_request.uri.marshal();
        let mut selected_track_type: Option<TrackType> = None;
        for (track_type, track) in &self.tracks {
            if !track.media_control.is_empty() && request_uri.contains(&track.media_control) {
                selected_track_type = Some(track_type.clone());
                break;
            }
        }

        if selected_track_type.is_none() {
            if self.tracks.contains_key(&TrackType::Video) {
                selected_track_type = Some(TrackType::Video);
            } else if let Some((track_type, _)) = self.tracks.iter().next() {
                selected_track_type = Some(track_type.clone());
            }
        }

        let Some(track_type) = selected_track_type else {
            let response = Self::gen_response(http::StatusCode::NOT_FOUND, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        };

        let track = self.tracks.get_mut(&track_type).ok_or(SessionError {
            value: SessionErrorValue::RtspMessageCorrupted("track missing".to_string()),
        })?;

        if let Some(transport_data) = rtsp_request.get_header("Transport") {
            if self.session_id.is_none() {
                self.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));
            }

            let transport = RtspTransport::unmarshal(transport_data);

            if let Err(ref err) = transport {
                // M-04: RFC 2326 §12.39: respond 461 for invalid transport
                log::warn!(
                    "event=rtsp_unsupported_transport error=\"{}\" remote_addr={}",
                    err,
                    self.remote_addr,
                );
                let response = Self::gen_response(
                    http::StatusCode::from_u16(461).unwrap_or(http::StatusCode::BAD_REQUEST),
                    rtsp_request,
                );
                self.send_response(&response).await?;
                return Ok(());
            }

            if let Ok(mut trans) = transport {
                let mut rtp_server_port: Option<u16> = None;
                let mut rtcp_server_port: Option<u16> = None;

                match trans.protocol_type {
                    ProtocolType::TCP => {
                        track.create_packer(self.io_writer.clone()).await;
                        // Start RTCP SR transmission over TCP (interleaved mode)
                        track.rtcp_send_loop(self.io_writer.clone()).await;
                    }
                    ProtocolType::UDP => {
                        let (rtp_port, rtcp_port) = trans
                            .client_port
                            .ok_or(SessionError {
                                value: SessionErrorValue::MissingClientPort,
                            })?
                            .into();

                        let address = self.remote_addr.ip().to_string();
                        if let Some(rtp_io) = UdpIO::new(address.clone(), rtp_port, 0).await {
                            rtp_server_port = rtp_io.get_local_port();

                            let box_udp_io: Box<dyn TNetIO + Send + Sync> = Box::new(rtp_io);
                            let is_record =
                                matches!(trans.transport_mod.as_deref(), Some("record"));
                            if !is_record {
                                track.create_packer(Arc::new(Mutex::new(box_udp_io))).await;
                            } else {
                                track.rtp_receive_loop(box_udp_io).await;
                            }
                        }

                        let rtp_port = rtp_server_port.ok_or(SessionError {
                            value: SessionErrorValue::MissingClientPort,
                        })?;
                        if let Some(rtcp_io) =
                            UdpIO::new(address.clone(), rtcp_port, rtp_port + 1).await
                        {
                            rtcp_server_port = rtcp_io.get_local_port();
                            let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                Arc::new(Mutex::new(Box::new(rtcp_io)));
                            track.rtcp_receive_loop(box_rtcp_io.clone()).await;
                            track.rtcp_send_loop(box_rtcp_io).await;
                        }
                    }
                }

                //tell client the udp ports of server side
                let mut server_ports: [u16; 2] = [0, 0];
                if let Some(rtp_port) = rtp_server_port {
                    server_ports[0] = rtp_port;
                }
                if let Some(rtcp_server_port) = rtcp_server_port {
                    server_ports[1] = rtcp_server_port;
                    trans.server_port = Some(server_ports);
                }

                let new_transport_data = trans.marshal();
                response
                    .headers
                    .insert("Transport".to_string(), new_transport_data);
                response.headers.insert(
                    "Session".to_string(),
                    self.session_id
                        .ok_or(SessionError {
                            value: SessionErrorValue::MissingSessionId,
                        })?
                        .to_string(),
                );

                track.set_transport(trans).await;
            }
        }

        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        let transport_hdr = rtsp_request
            .get_header("Transport")
            .cloned()
            .unwrap_or_default();
        log::info!(
            "event=rtsp_request method=SETUP cseq={} session_id={} remote_addr={} stream_path={} session_type={} track_type={:?} transport_req=\"{}\"",
            cseq,
            session_id,
            self.remote_addr,
            rtsp_request.uri.path,
            self.session_type,
            track_type,
            transport_hdr,
        );

        Ok(())
    }

    async fn ensure_tracks_from_streamhub(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        if !self.tracks.is_empty() {
            return Ok(());
        }

        let stream_path = self.normalize_rtsp_stream_path(&rtsp_request.uri.path);
        if stream_path.is_empty() {
            return Ok(());
        }

        let (sender, mut receiver) = mpsc::unbounded_channel();
        let identifier = StreamIdentifier::Rtsp { stream_path };
        self.stream_identifier = Some(identifier.clone());

        let request_event = StreamHubEvent::Request { identifier, sender };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await
            && let Ok(sdp) = Sdp::unmarshal(&data)
        {
            self.sdp = sdp;
            self.new_tracks()?;
        }

        Ok(())
    }

    fn resolve_stream_identifier(&mut self, request_path: &str) -> StreamIdentifier {
        if let Some(identifier) = &self.stream_identifier {
            return identifier.clone();
        }

        let normalized_path = self.normalize_rtsp_stream_path(request_path);
        let identifier = StreamIdentifier::Rtsp {
            stream_path: normalized_path,
        };
        self.stream_identifier = Some(identifier.clone());
        identifier
    }

    fn build_content_base(&self, request: &RtspRequest) -> Option<String> {
        let uri = &request.uri;
        let mut host = uri.host.clone();
        let mut port = uri.port;

        if host.is_empty()
            && let Some(host_header) = request.get_header("Host")
        {
            if let Some((host_val, port_val)) = host_header.split_once(':') {
                host = host_val.to_string();
                if let Ok(parsed_port) = port_val.parse::<u16>() {
                    port = Some(parsed_port);
                }
            } else {
                host = host_header.to_string();
            }
        }

        let normalized_path = self.normalize_rtsp_stream_path(&uri.path);
        let path = normalized_path.trim_matches('/');
        let mut base = if !host.is_empty() {
            let port_str = port.map(|val| format!(":{val}")).unwrap_or_default();
            if path.is_empty() {
                format!("rtsp://{host}{port_str}")
            } else {
                format!("rtsp://{host}{port_str}/{path}")
            }
        } else if !uri.path.is_empty() {
            uri.path.clone()
        } else {
            return None;
        };

        if !base.ends_with('/') {
            base.push('/');
        }

        Some(base)
    }

    fn normalize_rtsp_stream_path(&self, request_path: &str) -> String {
        let trimmed = request_path.trim_matches('/');
        if let Some(last_slash) = trimmed.rfind('/') {
            let (base, last) = trimmed.split_at(last_slash);
            let last = &last[1..];
            let lower = last.to_ascii_lowercase();
            let is_track_segment = lower.starts_with("track")
                || lower.starts_with("streamid")
                || lower.contains("trackid");
            if is_track_segment && !base.is_empty() {
                return base.to_string();
            }
        }

        trimmed.to_string()
    }

    /// Scale timestamp from milliseconds to RTP clock rate units.
    ///
    /// Used for video timestamps only. Audio timestamps are already in sample units.
    ///
    /// # Arguments
    ///
    /// * `timestamp_ms` - Timestamp in milliseconds
    /// * `clock_rate` - Target RTP clock rate (typically 90000 for video)
    ///
    /// # Returns
    ///
    /// Timestamp scaled to clock_rate units
    fn scale_rtp_timestamp(timestamp_ms: u32, clock_rate: u32) -> u32 {
        if clock_rate == 0 {
            return timestamp_ms;
        }
        ((timestamp_ms as u64).saturating_mul(clock_rate as u64) / 1000) as u32
    }

    async fn handle_play(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(
                &stream_name,
                &rtsp_request
                    .uri
                    .query
                    .as_ref()
                    .map(|q| SecretCarrier::Query(q.to_string())),
                true,
            )?;
        }

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        log::info!(
            "event=rtsp_request method=PLAY cseq={} session_id={} remote_addr={} stream_path={} session_type={}",
            cseq,
            session_id,
            self.remote_addr,
            rtsp_request.uri.path,
            self.session_type,
        );

        let sample_interval = self.rtp_sample_interval;
        let session_id_for_rtp = session_id.clone();
        let remote_for_rtp = self.remote_addr;
        let stream_identifier = self.stream_identifier.clone();

        for (track_type, track) in self.tracks.iter_mut() {
            let protocol_type = track.transport.protocol_type.clone();
            let counters = self
                .rtp_counters
                .entry(track_type.clone())
                .or_insert_with(|| Arc::new(RtpTrackCounters::new()))
                .clone();

            match protocol_type {
                ProtocolType::TCP => {
                    let channel_identifer = if let Some(interleaveds) = track.transport.interleaved
                    {
                        interleaveds[0]
                    } else {
                        log::error!("handle_play:should not be here!!!");
                        0
                    };

                    let track_label = format!("{:?}", track_type);
                    let session_id_for_rtp = session_id_for_rtp.clone();
                    let stream_identifier = stream_identifier.clone();
                    track.rtp_channel.lock().await.on_packet_handler(Box::new(
                        move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                            let counters = counters.clone();
                            let stream_identifier = stream_identifier.clone();
                            let track_label = track_label.clone();
                            let session_id = session_id_for_rtp.clone();
                            Box::pin(async move {
                                let msg = packet.marshal()?;
                                let payload_len = msg.len();
                                let stream_path = stream_identifier
                                    .as_ref()
                                    .map(|id| id.to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                let stats = counters.on_packet_sent(
                                    payload_len,
                                    packet.header.seq_number,
                                    packet.header.timestamp,
                                );

                                if stats.seq_gap
                                    || stats.seq_regressed
                                    || stats.timestamp_regressed
                                {
                                    log::warn!(
                                        "event=rtp_packet_anomaly protocol=TCP track={} session_id={} remote_addr={} stream_path={} prev_seq={:?} seq={} seq_delta={:?} prev_timestamp={:?} timestamp={} timestamp_delta={:?} seq_gap={} seq_regressed={} timestamp_regressed={}",
                                        track_label,
                                        session_id,
                                        remote_for_rtp,
                                        stream_path,
                                        stats.prev_seq,
                                        packet.header.seq_number,
                                        stats.seq_delta,
                                        stats.prev_timestamp,
                                        packet.header.timestamp,
                                        stats.timestamp_delta,
                                        stats.seq_gap,
                                        stats.seq_regressed,
                                        stats.timestamp_regressed,
                                    );
                                }

                                if sample_interval > 0
                                    && stats
                                        .packets_sent
                                        .is_multiple_of(sample_interval as u64)
                                {
                                    log::debug!(
                                        "event=rtp_packet_sample protocol=TCP track={} session_id={} remote_addr={} stream_path={} seq={} timestamp={} marker={} size_bytes={} packets_sent={} bytes_sent={}",
                                        track_label,
                                        session_id,
                                        remote_for_rtp,
                                        stream_path,
                                        packet.header.seq_number,
                                        packet.header.timestamp,
                                        packet.header.marker,
                                        payload_len,
                                        stats.packets_sent,
                                        stats.bytes_sent,
                                    );
                                }

                                let mut bytes_writer = AsyncBytesWriter::new(io);
                                bytes_writer.write_u8(0x24)?;
                                bytes_writer.write_u8(channel_identifer)?;
                                bytes_writer.write_u16::<BigEndian>(msg.len() as u16)?;
                                bytes_writer.write(&msg)?;
                                bytes_writer.flush().await?;
                                Ok(())
                            })
                        },
                    ));
                }
                ProtocolType::UDP => {
                    let track_label = format!("{:?}", track_type);
                    let session_id_for_rtp = session_id_for_rtp.clone();
                    let stream_identifier = stream_identifier.clone();
                    track.rtp_channel.lock().await.on_packet_handler(Box::new(
                        move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                            let counters = counters.clone();
                            let stream_identifier = stream_identifier.clone();
                            let track_label = track_label.clone();
                            let session_id = session_id_for_rtp.clone();
                            Box::pin(async move {
                                let msg = packet.marshal()?;
                                let payload_len = msg.len();
                                let stream_path = stream_identifier
                                    .as_ref()
                                    .map(|id| id.to_string())
                                    .unwrap_or_else(|| "unknown".to_string());
                                let stats = counters.on_packet_sent(
                                    payload_len,
                                    packet.header.seq_number,
                                    packet.header.timestamp,
                                );

                                if stats.seq_gap
                                    || stats.seq_regressed
                                    || stats.timestamp_regressed
                                {
                                    log::warn!(
                                        "event=rtp_packet_anomaly protocol=UDP track={} session_id={} remote_addr={} stream_path={} prev_seq={:?} seq={} seq_delta={:?} prev_timestamp={:?} timestamp={} timestamp_delta={:?} seq_gap={} seq_regressed={} timestamp_regressed={}",
                                        track_label,
                                        session_id,
                                        remote_for_rtp,
                                        stream_path,
                                        stats.prev_seq,
                                        packet.header.seq_number,
                                        stats.seq_delta,
                                        stats.prev_timestamp,
                                        packet.header.timestamp,
                                        stats.timestamp_delta,
                                        stats.seq_gap,
                                        stats.seq_regressed,
                                        stats.timestamp_regressed,
                                    );
                                }

                                if sample_interval > 0
                                    && stats
                                        .packets_sent
                                        .is_multiple_of(sample_interval as u64)
                                {
                                    log::debug!(
                                        "event=rtp_packet_sample protocol=UDP track={} session_id={} remote_addr={} stream_path={} seq={} timestamp={} marker={} size_bytes={} packets_sent={} bytes_sent={}",
                                        track_label,
                                        session_id,
                                        remote_for_rtp,
                                        stream_path,
                                        packet.header.seq_number,
                                        packet.header.timestamp,
                                        packet.header.marker,
                                        payload_len,
                                        stats.packets_sent,
                                        stats.bytes_sent,
                                    );
                                }

                                let mut bytes_writer = AsyncBytesWriter::new(io);

                                bytes_writer.write(&msg)?;
                                bytes_writer.flush().await?;
                                Ok(())
                            })
                        },
                    ));
                }
            }
        }

        // RFC 2326 §10.5 — validate session ID if provided
        if let Some(response) = self.validate_session_id(rtsp_request) {
            self.send_response(&response).await?;
            return Ok(());
        }

        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        // RFC 2326 §12.37 — Session header MUST be in PLAY response
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }

        if let Some(content_base) = self.build_content_base(rtsp_request)
            && !self.tracks.is_empty()
        {
            let mut rtp_info_parts = Vec::new();
            for track_type in [TrackType::Video, TrackType::Audio, TrackType::Application] {
                if let Some(track) = self.tracks.get(&track_type) {
                    let rtp_channel = track.rtp_channel.lock().await;
                    let seq = rtp_channel.initial_sequence();
                    // RFC 3550 §5.1 — use random initial timestamp, not hardcoded 0
                    let rtptime = rtp_channel.initial_timestamp();
                    let track_url = format!("{}{}", content_base, track.media_control);
                    rtp_info_parts
                        .push(format!("url={};seq={};rtptime={}", track_url, seq, rtptime));
                }
            }
            if !rtp_info_parts.is_empty() {
                response
                    .headers
                    .insert("RTP-Info".to_string(), rtp_info_parts.join(", "));
            }
        }

        if let Some(range_str) = rtsp_request.get_header("Range") {
            match RtspRange::unmarshal(range_str) {
                Ok(range) => {
                    response
                        .headers
                        .insert(String::from("Range"), range.marshal());
                }
                Err(err) => {
                    // RFC 2326 §11.3.7 — invalid Range returns 457
                    log::warn!("handle_play: invalid Range header: {err}");
                    let err_response = Self::gen_rtsp_response(457, "Invalid Range", rtsp_request);
                    self.send_response(&err_response).await?;
                    return Ok(());
                }
            }
        }

        self.send_response(&response).await?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
        let subscribe_event = StreamHubEvent::Subscribe {
            identifier,
            info: self.get_subscriber_info(),
            result_sender: event_result_sender,
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let mut receiver = event_result_receiver
            .await??
            .0
            .frame_receiver
            .ok_or(SessionError {
                value: SessionErrorValue::MissingFrameReceiver,
            })?;

        self.has_subscribed = true;
        self.session_type = define::ServerSessionType::Pull;
        let audio_rtp_channel = self
            .tracks
            .get(&TrackType::Audio)
            .map(|track| track.rtp_channel.clone());
        let video_rtp_channel = self
            .tracks
            .get(&TrackType::Video)
            .map(|track| track.rtp_channel.clone());
        let remote_addr = self.remote_addr;
        let request_path = rtsp_request.uri.path.clone();
        let session_id_for_task = session_id.clone();
        let shutdown = self.shutdown.clone();

        self.stop_playback_task().await;
        let playback_cancel = Arc::new(AtomicBool::new(false));
        let playback_cancel_for_task = playback_cancel.clone();
        self.playback_cancel = Some(playback_cancel);

        self.playback_task = Some(tokio::spawn(async move {
            let mut timestamp_normalizers: HashMap<TrackType, RtpTimestampNormalizer> =
                HashMap::new();
            let mut retry_times = 0;
            let mut video_assembler = VideoAccessUnitAssembler::default();
            let video_send_ctx = PlaybackVideoSendContext {
                session_id: session_id_for_task.as_str(),
                remote_addr,
                request_path: request_path.as_str(),
                shutdown: &shutdown,
            };

            loop {
                if playback_cancel_for_task.load(Ordering::Acquire) {
                    if let (Some(video_channel), Some((timestamp, mut data))) =
                        (&video_rtp_channel, video_assembler.flush())
                    {
                        send_video_access_unit(
                            video_channel,
                            &mut timestamp_normalizers,
                            &video_send_ctx,
                            timestamp,
                            &mut data,
                        )
                        .await;
                    }
                    break;
                }

                if let Some(frame_data) = receiver.recv().await {
                    retry_times = 0;
                    match frame_data {
                        FrameData::Audio {
                            timestamp,
                            mut data,
                        } => {
                            if let Some(audio_channel) = &audio_rtp_channel {
                                let mut channel = audio_channel.lock().await;
                                let normalized = timestamp_normalizers
                                    .entry(TrackType::Audio)
                                    .or_default()
                                    .normalize(timestamp, channel.clock_rate(), TrackType::Audio);
                                if crate::stream_frame_debug_logging_enabled() {
                                    log::debug!(
                                        "server_session: Audio timestamp_in={} clock_rate={} scaled={} output={}",
                                        timestamp,
                                        channel.clock_rate(),
                                        normalized.scaled_timestamp,
                                        normalized.output_timestamp
                                    );
                                }
                                if normalized.non_wrap_regressed {
                                    log::warn!(
                                        "event=rtp_timestamp_non_wrap_regression track=Audio session_id={} remote_addr={} stream_path={} prev_scaled={} current_scaled={} corrected_timestamp={} regression_count={}",
                                        session_id_for_task,
                                        remote_addr,
                                        request_path,
                                        normalized.previous_scaled_timestamp.unwrap_or_default(),
                                        normalized.scaled_timestamp,
                                        normalized.output_timestamp,
                                        normalized.non_wrap_regression_count,
                                    );
                                }
                                if let Err(err) = channel
                                    .on_frame(&mut data, normalized.output_timestamp)
                                    .await
                                {
                                    log::info!("handle_play error: {err}");
                                    shutdown.store(true, Ordering::Release);
                                    break;
                                }
                            }
                        }
                        FrameData::Video { timestamp, data } => {
                            if let Some(video_channel) = &video_rtp_channel {
                                let Some((flush_ts, mut flush_data)) =
                                    video_assembler.push(timestamp, data)
                                else {
                                    continue;
                                };
                                send_video_access_unit(
                                    video_channel,
                                    &mut timestamp_normalizers,
                                    &video_send_ctx,
                                    flush_ts,
                                    &mut flush_data,
                                )
                                .await;
                            }
                        }
                        _ => {}
                    }
                } else {
                    if let (Some(video_channel), Some((timestamp, mut data))) =
                        (&video_rtp_channel, video_assembler.flush())
                    {
                        send_video_access_unit(
                            video_channel,
                            &mut timestamp_normalizers,
                            &video_send_ctx,
                            timestamp,
                            &mut data,
                        )
                        .await;
                    }
                    retry_times += 1;
                    log::info!(
                        "send_channel_data: no data receives ,retry {} times!",
                        retry_times
                    );

                    if retry_times > 10 {
                        shutdown.store(true, Ordering::Release);
                        break;
                    }
                }
            }
        }));

        Ok(())
    }

    async fn handle_record(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        //A stream published by gstreamer does not support the Range header
        //https://github.com/harlanc/xiu/issues/135
        if let Some(range_str) = rtsp_request.get_header("Range") {
            match RtspRange::unmarshal(range_str) {
                Ok(range) => {
                    response
                        .headers
                        .insert(String::from("Range"), range.marshal());
                }
                Err(err) => {
                    log::warn!("invalid range header ignored: {err}");
                }
            }
        }

        response.headers.insert(
            "Session".to_string(),
            self.session_id
                .ok_or(SessionError {
                    value: SessionErrorValue::MissingSessionId,
                })?
                .to_string(),
        );

        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_get_parameter(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        self.handle_keep_alive(rtsp_request, rtsp_method_name::GET_PARAMETER)
            .await
    }

    async fn handle_set_parameter(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        self.handle_keep_alive(rtsp_request, rtsp_method_name::SET_PARAMETER)
            .await
    }

    async fn handle_pause(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        self.stop_playback_task().await;

        if self.has_subscribed {
            let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
            let unsubscribe = StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            };

            if self.event_producer.send(unsubscribe).is_err() {
                return Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                });
            }

            self.has_subscribed = false;
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_redirect(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let mut response = Self::gen_rtsp_response(405, "Method Not Allowed", rtsp_request);
        response.headers.insert(
            "Allow".to_string(),
            rtsp_method_name::PUBLIC_METHODS.join(", "),
        );
        self.send_response(&response).await?;
        Ok(())
    }

    async fn handle_keep_alive(
        &mut self,
        rtsp_request: &RtspRequest,
        method: &str,
    ) -> Result<(), SessionError> {
        if let Some(session_hdr) = rtsp_request.get_header("Session")
            && let Some(current) = self.session_id
        {
            let requested = Self::parse_session_header(session_hdr);
            if !requested.is_empty() && requested != current.to_string() {
                let response = Self::gen_rtsp_response(454, "Session Not Found", rtsp_request);
                self.send_response(&response).await?;
                return Ok(());
            }
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        if let Some(session_id) = self.session_id {
            response
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }
        self.send_response(&response).await?;

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        log::info!(
            "event=rtsp_request method={} cseq={} session_id={} remote_addr={} stream_path={} session_type={}",
            method,
            cseq,
            session_id,
            self.remote_addr,
            rtsp_request.uri.path,
            self.session_type,
        );

        Ok(())
    }

    fn handle_teardown(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
        log::info!(
            "event=rtsp_teardown session_id={} remote_addr={} stream_path={} session_type={}",
            self.session_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".to_string()),
            self.remote_addr,
            rtsp_request.uri.path,
            self.session_type,
        );

        // Best-effort per-track RTP summary at session teardown.
        for (track_type, counters) in &self.rtp_counters {
            let (packets, bytes, duration_ms) = counters.snapshot();
            if packets == 0 {
                continue;
            }
            let bitrate_kbps = duration_ms
                .filter(|d| *d > 0)
                .map(|d| ((bytes.saturating_mul(8) as u128 * 1000) / d as u128 / 1000) as u64)
                .unwrap_or(0);
            let stream_path = self
                .stream_identifier
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            log::info!(
                "event=rtp_session_summary reason=teardown track={:?} session_id={} remote_addr={} stream_path={} packets={} bytes={} duration_ms={} bitrate_kbps={}",
                track_type,
                self.session_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                self.remote_addr,
                stream_path,
                packets,
                bytes,
                duration_ms.unwrap_or(0),
                bitrate_kbps,
            );
        }
        self.exit(identifier)
    }

    pub fn exit(&mut self, identifier: StreamIdentifier) -> Result<(), SessionError> {
        let event = if self.has_published {
            Some(StreamHubEvent::UnPublish {
                identifier,
                info: self.get_publisher_info(),
            })
        } else if self.has_subscribed {
            Some(StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            })
        } else {
            None
        };

        let event = match event {
            Some(event) => event,
            None => {
                self.is_normal_exit = true;
                log::info!(
                    "session exit: no publish/subscribe established; skipping streamhub event"
                );
                return Ok(());
            }
        };
        let event_json_str =
            serde_json::to_string(&event).unwrap_or_else(|_| "<serialize failed>".to_string());

        let rv = self.event_producer.send(event);
        match rv {
            Err(err) => {
                log::error!("session exit: send event error: {err} for event: {event_json_str}");
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                self.is_normal_exit = true;
                log::info!("session exit: send event success: {event_json_str}");
                Ok(())
            }
        }
    }

    fn new_tracks(&mut self) -> Result<(), SessionError> {
        for media in &self.sdp.medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                String::from("")
            };

            let media_name = &media.media_type;
            log::info!("media_name: {}", media_name);
            match media_name.as_str() {
                "audio" => {
                    let codec_name = media.rtpmap.encoding_name.to_lowercase();
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(codec_name.as_str())
                        .cloned()
                        .ok_or(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(format!(
                                "unsupported audio codec: {}",
                                codec_name
                            )),
                        })?;
                    let channel_count = if media.rtpmap.encoding_param.is_empty() {
                        1
                    } else {
                        media
                            .rtpmap
                            .encoding_param
                            .parse()
                            .map_err(|_| SessionError {
                                value: SessionErrorValue::RtspMessageCorrupted(
                                    "invalid audio channel count".to_string(),
                                ),
                            })?
                    };
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        channel_count,
                    };

                    log::info!("audio codec info: {:?}", codec_info);

                    let track = RtspTrack::new(TrackType::Audio, codec_info, media_control);
                    self.tracks.insert(TrackType::Audio, track);
                    self.rtp_counters
                        .entry(TrackType::Audio)
                        .or_insert_with(|| Arc::new(RtpTrackCounters::new()));
                }
                "video" => {
                    let codec_name = media.rtpmap.encoding_name.to_lowercase();
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(codec_name.as_str())
                        .cloned()
                        .ok_or(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(format!(
                                "unsupported video codec: {}",
                                codec_name
                            )),
                        })?;
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        ..Default::default()
                    };
                    let track = RtspTrack::new(TrackType::Video, codec_info, media_control);
                    self.tracks.insert(TrackType::Video, track);
                    self.rtp_counters
                        .entry(TrackType::Video)
                        .or_insert_with(|| Arc::new(RtpTrackCounters::new()));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// RFC 2326 §12.18 — Date header in HTTP-date format (RFC 7231 §7.1.1.1).
    fn http_date_now() -> String {
        Utc::now().format("%a, %d %b %Y %H:%M:%S GMT").to_string()
    }

    fn gen_response(status_code: StatusCode, rtsp_request: &RtspRequest) -> RtspResponse {
        let reason_phrase = if let Some(reason) = status_code.canonical_reason() {
            reason.to_string()
        } else {
            String::from("")
        };

        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: status_code.as_u16(),
            reason_phrase,
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.get_header("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        // RFC 2326 §12.18 — Date header SHOULD be included
        response
            .headers
            .insert("Date".to_string(), Self::http_date_now());
        // RFC 2326 §12.36 — Server header identifies the implementation
        response
            .headers
            .insert("Server".to_string(), SERVER_HEADER.to_string());

        response
    }

    fn gen_rtsp_response(
        code: u16,
        reason_phrase: &str,
        rtsp_request: &RtspRequest,
    ) -> RtspResponse {
        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: code,
            reason_phrase: reason_phrase.to_string(),
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.get_header("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        response
    }

    fn parse_session_header(session_hdr: &str) -> String {
        session_hdr
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_string()
    }

    /// Validate that the Session header in the request (if any) matches the current session ID.
    /// Returns `Some(response)` with a 454 "Session Not Found" if there is a mismatch,
    /// or `None` if the session ID is valid or no Session header is present.
    /// Per RFC 2326 §12.37.
    fn validate_session_id(&self, rtsp_request: &RtspRequest) -> Option<RtspResponse> {
        if let Some(session_hdr) = rtsp_request.get_header("Session")
            && let Some(current) = self.session_id
        {
            let requested = Self::parse_session_header(session_hdr);
            if !requested.is_empty() && requested != current.to_string() {
                return Some(Self::gen_rtsp_response(
                    454,
                    "Session Not Found",
                    rtsp_request,
                ));
            }
        }
        None
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(SESSION_ID_RANDOM_DIGITS)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::RtspPull,
            sub_data_type: crate::streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn get_publisher_info(&mut self) -> PublisherInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(SESSION_ID_RANDOM_DIGITS)
        };

        PublisherInfo {
            id,
            pub_type: PublishType::RtspPush,
            pub_data_type: crate::streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    async fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
    }
}

impl Drop for RtspServerSession {
    fn drop(&mut self) {
        self.abort_playback_task();
    }
}

#[derive(Default)]
pub struct RtspStreamHandler {
    sdp: Mutex<Sdp>,
}

impl RtspStreamHandler {
    pub fn new() -> Self {
        Self {
            sdp: Mutex::new(Sdp::default()),
        }
    }
    pub async fn set_sdp(&self, sdp: Sdp) {
        *self.sdp.lock().await = sdp;
    }
}

#[async_trait]
impl TStreamHandler for RtspStreamHandler {
    async fn send_prior_data(
        &self,
        data_sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        let sender = match data_sender {
            DataSender::Frame { sender } => sender,
            DataSender::Packet { sender: _ } => {
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NotCorrectDataSenderType,
                });
            }
        };
        match sub_type {
            SubscribeType::RtspRemux2Rtmp => {
                let sdp_info = self.sdp.lock().await;
                let mut video_clock_rate: u32 = 0;
                let mut audio_clock_rate: u32 = 0;

                let mut vcodec: VideoCodecType = VideoCodecType::H264;

                for media in &sdp_info.medias {
                    let mut bytes_writer = BytesWriter::new();
                    if let Some(fmtp) = &media.fmtp {
                        match fmtp {
                            Fmtp::H264(data) => {
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.sps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.pps)?;

                                let frame_data = FrameData::Video {
                                    timestamp: 0,
                                    data: bytes_writer.extract_current_bytes(),
                                };
                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send sps/pps error: {}", err);
                                }
                                video_clock_rate = media.rtpmap.clock_rate;
                            }
                            Fmtp::H265(data) => {
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.sps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.pps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.vps)?;

                                let frame_data = FrameData::Video {
                                    timestamp: 0,
                                    data: bytes_writer.extract_current_bytes(),
                                };
                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send sps/pps/vps error: {}", err);
                                }

                                vcodec = VideoCodecType::H265;
                            }
                            Fmtp::Mpeg4(data) => {
                                let frame_data = FrameData::Audio {
                                    timestamp: 0,
                                    data: data.asc.clone(),
                                };

                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send asc error: {}", err);
                                }

                                audio_clock_rate = media.rtpmap.clock_rate;
                            }
                        }
                    }
                }

                if let Err(err) = sender.send(FrameData::MediaInfo {
                    media_info: MediaInfo {
                        audio_clock_rate,
                        video_clock_rate,

                        vcodec,
                    },
                }) {
                    log::error!("send media info error: {}", err);
                }
            }
            SubscribeType::RtmpRemux2Hls => {}
            _ => {}
        }

        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        if let Err(err) = sender.send(Information::Sdp {
            data: self.sdp.lock().await.marshal(),
        }) {
            log::error!("send_information of rtsp error: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_reader::BytesReader;
    use crate::common::http::HttpRequest as RtspRequest;
    use bytes::BytesMut;
    use http::StatusCode;

    // ========================================================================
    // InterleavedBinaryData Tests
    // ========================================================================

    #[test]
    fn test_interleaved_binary_data_parse_valid() {
        // Dollar sign (0x24) + channel (0x00) + length (0x0004)
        let data: &[u8] = &[0x24, 0x00, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, 0x00);
        assert_eq!(interleaved.length, 4);
    }

    #[test]
    fn test_interleaved_binary_data_parse_channel_1() {
        // Dollar sign + channel 1 + length 10
        let data: &[u8] = &[0x24, 0x01, 0x00, 0x0A];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, 0x01);
        assert_eq!(interleaved.length, 10);
    }

    #[test]
    fn test_interleaved_binary_data_parse_large_length() {
        // Dollar sign + channel 2 + length 0xFFFF (65535)
        let data: &[u8] = &[0x24, 0x02, 0xFF, 0xFF];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, 0x02);
        assert_eq!(interleaved.length, 65535);
    }

    #[test]
    fn test_interleaved_binary_data_no_dollar_sign() {
        // Not starting with dollar sign - should return None
        let data: &[u8] = &[0x52, 0x54, 0x53, 0x50]; // "RTSP"
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_interleaved_binary_data_insufficient_data() {
        // Only dollar sign, not enough for full header
        let data: &[u8] = &[0x24];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader);
        // Should return an error due to insufficient bytes
        assert!(result.is_err());
    }

    #[test]
    fn test_interleaved_binary_data_empty() {
        let data: &[u8] = &[];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader);
        assert!(result.is_err());
    }

    // ========================================================================
    // gen_response Tests
    // ========================================================================

    /// Create a test RtspRequest with the given method and CSeq
    fn create_test_request(method: &str, cseq: Option<&str>) -> RtspRequest {
        let mut request = RtspRequest {
            method: method.to_string(),
            version: "RTSP/1.0".to_string(),
            ..Default::default()
        };
        if let Some(seq) = cseq {
            request.headers.insert("CSeq".to_string(), seq.to_string());
        }
        request
    }

    #[test]
    fn test_gen_response_ok_status() {
        let request = create_test_request("OPTIONS", Some("1"));

        let response = RtspServerSession::gen_response(StatusCode::OK, &request);
        assert_eq!(response.version, "RTSP/1.0");
        assert_eq!(response.status_code, 200);
        assert_eq!(response.reason_phrase, "OK");
        assert_eq!(response.headers.get("CSeq"), Some(&"1".to_string()));
    }

    #[test]
    fn test_gen_response_not_found_status() {
        let request = create_test_request("DESCRIBE", None);

        let response = RtspServerSession::gen_response(StatusCode::NOT_FOUND, &request);
        assert_eq!(response.status_code, 404);
        assert_eq!(response.reason_phrase, "Not Found");
    }

    #[test]
    fn test_gen_response_unauthorized_status() {
        let request = create_test_request("PLAY", None);

        let response = RtspServerSession::gen_response(StatusCode::UNAUTHORIZED, &request);
        assert_eq!(response.status_code, 401);
        assert_eq!(response.reason_phrase, "Unauthorized");
    }

    #[test]
    fn test_gen_response_with_cseq() {
        let request = create_test_request("SETUP", Some("42"));

        let response = RtspServerSession::gen_response(StatusCode::OK, &request);
        assert_eq!(response.headers.get("CSeq"), Some(&"42".to_string()));
    }

    #[test]
    fn test_gen_response_without_cseq() {
        let request = create_test_request("OPTIONS", None);

        let response = RtspServerSession::gen_response(StatusCode::OK, &request);
        assert!(response.headers.get("CSeq").is_none());
    }

    #[test]
    fn test_gen_response_bad_request() {
        let request = create_test_request("INVALID", None);

        let response = RtspServerSession::gen_response(StatusCode::BAD_REQUEST, &request);
        assert_eq!(response.status_code, 400);
        assert_eq!(response.reason_phrase, "Bad Request");
    }

    #[test]
    fn test_gen_response_internal_error() {
        let request = create_test_request("PLAY", None);

        let response = RtspServerSession::gen_response(StatusCode::INTERNAL_SERVER_ERROR, &request);
        assert_eq!(response.status_code, 500);
        assert_eq!(response.reason_phrase, "Internal Server Error");
    }

    // ========================================================================
    // RtspStreamHandler Tests
    // ========================================================================

    #[test]
    fn test_rtsp_stream_handler_new() {
        let handler = RtspStreamHandler::new();
        // Handler should be created successfully
        assert!(std::mem::size_of_val(&handler) > 0);
    }

    #[test]
    fn test_rtsp_stream_handler_default() {
        let handler = RtspStreamHandler::default();
        assert!(std::mem::size_of_val(&handler) > 0);
    }

    #[tokio::test]
    async fn test_rtsp_stream_handler_set_sdp() {
        let handler = RtspStreamHandler::new();
        let sdp = Sdp::default();
        handler.set_sdp(sdp).await;
        // Should not panic
    }

    #[tokio::test]
    async fn test_rtsp_stream_handler_send_information() {
        let handler = RtspStreamHandler::new();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        handler.send_information(sender).await;

        // Should receive SDP information
        if let Some(info) = receiver.recv().await {
            let Information::Sdp { data: _ } = info;
        } else {
            panic!("Expected to receive information");
        }
    }

    #[tokio::test]
    async fn test_rtsp_stream_handler_get_statistic_data() {
        let handler = RtspStreamHandler::new();
        let stats = handler.get_statistic_data().await;
        assert!(stats.is_none());
    }

    // ========================================================================
    // MockNetIO for Testing
    // ========================================================================
    use crate::bytesio::NetType;
    use crate::bytesio::TNetIO;
    use crate::bytesio::bytesio_errors::BytesIOError;
    use async_trait::async_trait;
    use bytes::Bytes;
    use mockall::mock;

    mock! {
        pub NetIO {}
        #[async_trait]
        impl TNetIO for NetIO {
            fn get_net_type(&self) -> NetType;
            async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
            async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
            async fn read_timeout(&mut self, duration: std::time::Duration) -> Result<BytesMut, BytesIOError>;
        }
    }

    #[tokio::test]
    async fn test_rtsp_server_session_options() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        // Expect get_net_type to be called
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Expect write logic for the response
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
                    && s.contains("Public: OPTIONS, DESCRIBE")
                    && !s.contains("REDIRECT")
                    && s.contains("Date:")
                    && s.contains("Server: streaming-lib")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let request = create_test_request("OPTIONS", Some("1"));
        let result = session.handle_options(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_on_rtsp_message_leaves_interleaved_binary_buffered() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(0);

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
                    && s.contains("Public: OPTIONS, DESCRIBE")
                    && !s.contains("REDIRECT")
                    && s.contains("Date:")
                    && s.contains("Server: streaming-lib")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let mut data =
            BytesMut::from("OPTIONS rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 1\r\n\r\n");
        data.extend_from_slice(&[0x24, 0x00, 0x00, 0x04, 0xff, 0xff, 0xff, 0xff]);
        session.reader.extend_from_slice(&data[..]);

        let result = session.on_rtsp_message().await;
        assert!(result.is_ok());

        let remaining = session.reader.get_remaining_bytes();
        assert_eq!(remaining.len(), 8);
        assert_eq!(remaining[0], 0x24);
    }

    #[tokio::test]
    async fn test_rtsp_server_session_get_parameter_keep_alive_ok_includes_session() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(0);

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK") && s.contains("Session:")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let session_id = Uuid::new(SESSION_ID_RANDOM_DIGITS);
        let session_id_str = session_id.to_string();
        session.session_id = Some(session_id);

        let mut request = create_test_request(rtsp_method_name::GET_PARAMETER, Some("1"));
        request
            .headers
            .insert("Session".to_string(), session_id_str);
        let result = session.handle_get_parameter(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_get_parameter_wrong_session_returns_454() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(0);

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 454 Session Not Found") && s.contains("CSeq: 1")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));

        let mut request = create_test_request(rtsp_method_name::GET_PARAMETER, Some("1"));
        request
            .headers
            .insert("Session".to_string(), "does-not-exist".to_string());
        let result = session.handle_get_parameter(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_pause_unsubscribes_and_responds_ok() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(0);

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK") && s.contains("CSeq: 1")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        session.has_subscribed = true;
        session.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));
        session.stream_identifier = Some(StreamIdentifier::Rtsp {
            stream_path: "live/stream1".to_string(),
        });

        let mut request = create_test_request(rtsp_method_name::PAUSE, Some("1"));
        request.uri.path = "live/stream1".to_string();

        let result = session.handle_pause(&request).await;
        assert!(result.is_ok());
        assert!(!session.has_subscribed);

        let event = event_receiver.recv().await.expect("expected unsubscribe");
        match event {
            StreamHubEvent::UnSubscribe { identifier, .. } => match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "live/stream1");
                }
                _ => panic!("unexpected identifier"),
            },
            _ => panic!("expected UnSubscribe event"),
        }
    }

    #[tokio::test]
    async fn test_rtsp_server_session_redirect_returns_405_with_allow() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(0);

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 405 Method Not Allowed") && s.contains("Allow:")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let request = create_test_request(rtsp_method_name::REDIRECT, Some("1"));
        let result = session.handle_redirect(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_describe() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
                    && s.contains("application/sdp")
                    && s.contains("Content-Base: rtsp://127.0.0.1:8554/live/test/")
            })
            .times(1)
            .returning(|_| Ok(()));

        // Start a mock StreamHub event loop to handle the Request event
        tokio::spawn(async move {
            if let Some(event) = event_receiver.recv().await {
                if let StreamHubEvent::Request {
                    identifier: _,
                    sender,
                } = event
                {
                    // Respond with a minimal valid SDP containing one media block
                    let dummy_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
                    let _ = sender.send(Information::Sdp {
                        data: dummy_sdp.to_string(),
                    });
                }
            }
        });

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let mut request = create_test_request("DESCRIBE", Some("2"));
        // Need to set a valid path for StreamIdentifier
        request.uri.schema = crate::common::http::Schema::RTSP;
        request.uri.host = "127.0.0.1".to_string();
        request.uri.port = Some(8554);
        request.uri.path = "live/test".to_string();

        let result = session.handle_describe(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_describe_normalizes_path() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
                    && s.contains("application/sdp")
                    && s.contains("Content-Base: rtsp://127.0.0.1:8554/live/test/")
            })
            .times(1)
            .returning(|_| Ok(()));

        tokio::spawn(async move {
            if let Some(event) = event_receiver.recv().await {
                if let StreamHubEvent::Request { identifier, sender } = event {
                    match identifier {
                        StreamIdentifier::Rtsp { stream_path } => {
                            assert_eq!(stream_path, "live/test");
                        }
                        _ => panic!("unexpected identifier type"),
                    }
                    let dummy_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
                    let _ = sender.send(Information::Sdp {
                        data: dummy_sdp.to_string(),
                    });
                }
            }
        });

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let mut request = create_test_request("DESCRIBE", Some("3"));
        request.uri.schema = crate::common::http::Schema::RTSP;
        request.uri.host = "127.0.0.1".to_string();
        request.uri.port = Some(8554);
        request.uri.path = "live/test/trackID=0".to_string();

        let result = session.handle_describe(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_describe_empty_sdp_returns_not_found() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 404 Not Found")
            })
            .times(1)
            .returning(|_| Ok(()));

        tokio::spawn(async move {
            if let Some(StreamHubEvent::Request { sender, .. }) = event_receiver.recv().await {
                // No media blocks -> server should treat as not-found stream.
                let empty_sdp = "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\n";
                let _ = sender.send(Information::Sdp {
                    data: empty_sdp.to_string(),
                });
            }
        });

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let mut request = create_test_request("DESCRIBE", Some("4"));
        request.uri.schema = crate::common::http::Schema::RTSP;
        request.uri.host = "127.0.0.1".to_string();
        request.uri.port = Some(8554);
        request.uri.path = "bogus_stream".to_string();

        let result = session.handle_describe(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_describe_auth_header_without_auth_returns_unauthorized() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 401 Unauthorized")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let mut request = create_test_request("DESCRIBE", Some("5"));
        request.uri.schema = crate::common::http::Schema::RTSP;
        request.uri.host = "127.0.0.1".to_string();
        request.uri.port = Some(8554);
        request.uri.path = "stream1".to_string();
        request.headers.insert(
            "Authorization".to_string(),
            "Basic aW52YWxpZDppbnZhbGlk".to_string(),
        );

        let result = session.handle_describe(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_setup() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK") && s.contains("Transport")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        // Pre-populate a track so setup acts on it
        // The track logic requires an existing track in self.tracks map matching the control URI
        let codec_info = RtspCodecInfo {
            codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            channel_count: 0,
        };
        let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
        session.tracks.insert(TrackType::Video, track);

        let content = "SETUP rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 3\r\nTransport: RTP/AVP/TCP;unicast;interleaved=0-1\r\n\r\n";
        let request = RtspRequest::unmarshal(content).unwrap();

        let result = session.handle_setup(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_server_session_setup_base_path_selects_video_track() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK") && s.contains("Transport") && s.contains("Session")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let codec_info = RtspCodecInfo {
            codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            channel_count: 0,
        };
        let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
        session.tracks.insert(TrackType::Video, track);

        let content = "SETUP rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 3\r\nTransport: RTP/AVP/TCP;unicast;interleaved=0-1\r\n\r\n";
        let request = RtspRequest::unmarshal(content).unwrap();

        let result = session.handle_setup(&request).await;
        assert!(result.is_ok());
        assert_eq!(
            session.stream_identifier,
            Some(StreamIdentifier::Rtsp {
                stream_path: "stream1".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn test_rtsp_server_session_teardown() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mock_io = MockNetIO::new();

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        // Mock event receiver to handle UnSubscribe/UnPublish
        tokio::spawn(async move {
            if let Some(_event) = event_receiver.recv().await {
                // Just consume the event
            }
        });

        let request = create_test_request("TEARDOWN", Some("4"));
        let result = session.handle_teardown(&request);
        assert!(result.is_ok());
        assert!(session.is_normal_exit);
    }

    /// Drives the TEARDOWN branch in on_rtsp_message via run() to assert the server sends RTSP 200 OK.
    #[tokio::test]
    async fn test_rtsp_server_session_teardown_sends_response() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const TEARDOWN_REQ: &str =
            "TEARDOWN rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 4\r\nSession: 1\r\n\r\n";

        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let read_count = AtomicUsize::new(0);
        let teardown_bytes = BytesMut::from(TEARDOWN_REQ);
        mock_io.expect_read().times(2).returning(move || {
            if read_count.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(teardown_bytes.clone())
            } else {
                Err(BytesIOError::from(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "eof",
                )))
            }
        });

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let run_handle = tokio::spawn(async move {
            let _ = session.run().await;
        });

        run_handle.await.expect("run task panicked");
    }

    #[tokio::test]
    async fn test_rtsp_server_session_play_then_teardown_sends_two_responses_and_unsubscribes_once()
    {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const PLAY_REQ: &str = "PLAY rtsp://localhost/stream1/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nSession: 1\r\nRange: npt=0.000-\r\n\r\n";
        const TEARDOWN_REQ: &str =
            "TEARDOWN rtsp://localhost/stream1 RTSP/1.0\r\nCSeq: 6\r\nSession: 1\r\n\r\n";

        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let read_count = AtomicUsize::new(0);
        let play_bytes = BytesMut::from(PLAY_REQ);
        let teardown_bytes = BytesMut::from(TEARDOWN_REQ);
        mock_io.expect_read().times(3).returning(move || {
            match read_count.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(play_bytes.clone()),
                1 => Ok(teardown_bytes.clone()),
                _ => Err(BytesIOError::from(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "eof",
                ))),
            }
        });

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
            })
            .times(2)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let unsubscribe_count = Arc::new(AtomicUsize::new(0));
        let unsubscribe_count_for_task = unsubscribe_count.clone();
        let event_handle = tokio::spawn(async move {
            use crate::streamhub::define::DataReceiver;

            let mut held_frame_sender = None;
            while let Some(event) = event_receiver.recv().await {
                match event {
                    StreamHubEvent::Subscribe { result_sender, .. } => {
                        let (frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
                        held_frame_sender = Some(frame_sender);
                        let data_receiver = DataReceiver {
                            frame_receiver: Some(frame_receiver),
                            packet_receiver: None,
                        };
                        let _ = result_sender.send(Ok((data_receiver, None)));
                    }
                    StreamHubEvent::UnSubscribe { .. } => {
                        unsubscribe_count_for_task.fetch_add(1, Ordering::SeqCst);
                        drop(held_frame_sender);
                        break;
                    }
                    _ => {}
                }
            }
        });

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let run_result = session.run().await;
        assert!(run_result.is_err());
        assert!(session.playback_task.is_none());
        assert!(session.playback_cancel.is_none());

        event_handle.await.expect("event task panicked");
        assert_eq!(unsubscribe_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_rtsp_server_session_teardown_trailing_slash_unsubscribes_normalized_stream() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const PLAY_REQ: &str = "PLAY rtsp://localhost/stream1/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nSession: 1\r\nRange: npt=0.000-\r\n\r\n";
        const TEARDOWN_REQ: &str =
            "TEARDOWN rtsp://localhost/stream1/ RTSP/1.0\r\nCSeq: 6\r\nSession: 1\r\n\r\n";

        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let read_count = AtomicUsize::new(0);
        let play_bytes = BytesMut::from(PLAY_REQ);
        let teardown_bytes = BytesMut::from(TEARDOWN_REQ);
        mock_io.expect_read().times(3).returning(move || {
            match read_count.fetch_add(1, Ordering::SeqCst) {
                0 => Ok(play_bytes.clone()),
                1 => Ok(teardown_bytes.clone()),
                _ => Err(BytesIOError::from(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "eof",
                ))),
            }
        });

        mock_io.expect_write().times(2).returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let event_handle = tokio::spawn(async move {
            use crate::streamhub::define::DataReceiver;

            let mut held_frame_sender = None;
            while let Some(event) = event_receiver.recv().await {
                match event {
                    StreamHubEvent::Subscribe { result_sender, .. } => {
                        let (frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
                        held_frame_sender = Some(frame_sender);
                        let data_receiver = DataReceiver {
                            frame_receiver: Some(frame_receiver),
                            packet_receiver: None,
                        };
                        let _ = result_sender.send(Ok((data_receiver, None)));
                    }
                    StreamHubEvent::UnSubscribe { identifier, .. } => {
                        match identifier {
                            StreamIdentifier::Rtsp { stream_path } => {
                                assert_eq!(stream_path, "stream1");
                            }
                            _ => panic!("Expected RTSP identifier"),
                        }
                        drop(held_frame_sender);
                        break;
                    }
                    _ => {}
                }
            }
        });

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let run_result = session.run().await;
        assert!(run_result.is_err());
        event_handle.await.expect("event task panicked");
    }

    #[tokio::test]
    async fn test_rtsp_server_session_run_eof_stops_playback_task() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        const PLAY_REQ: &str = "PLAY rtsp://localhost/stream1/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nSession: 1\r\nRange: npt=0.000-\r\n\r\n";

        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let read_count = AtomicUsize::new(0);
        let play_bytes = BytesMut::from(PLAY_REQ);
        mock_io.expect_read().times(2).returning(move || {
            if read_count.fetch_add(1, Ordering::SeqCst) == 0 {
                Ok(play_bytes.clone())
            } else {
                Err(BytesIOError::from(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "eof",
                )))
            }
        });

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
            })
            .times(1)
            .returning(|_| Ok(()));

        let event_handle = tokio::spawn(async move {
            use crate::streamhub::define::DataReceiver;
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
                let _hold_sender = frame_sender;
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_receiver),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));
            }
        });

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let run_result = session.run().await;
        assert!(run_result.is_err());
        assert!(session.playback_task.is_none());
        assert!(session.playback_cancel.is_none());

        event_handle.await.expect("event task panicked");
    }

    #[tokio::test]
    async fn test_rtsp_server_session_run_shutdown_sends_unsubscribe_cleanup() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(0);
        mock_io.expect_write().times(0);

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();
        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        session.has_subscribed = true;
        session.stream_identifier = Some(StreamIdentifier::Rtsp {
            stream_path: "stream1".to_string(),
        });
        session.shutdown();

        let run_result = session.run().await;
        assert!(run_result.is_ok());
        assert!(session.is_normal_exit);

        match event_receiver
            .try_recv()
            .expect("expected UnSubscribe event")
        {
            StreamHubEvent::UnSubscribe { identifier, .. } => match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "stream1");
                }
                _ => panic!("Expected RTSP identifier"),
            },
            _ => panic!("Expected UnSubscribe event"),
        }
    }

    #[tokio::test]
    async fn test_rtsp_server_session_play() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        // Expect response write (200 OK)
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        // Populate a track so PLAY knows what to play (though handle_play logic might iterate all tracks)
        // handle_play calls `self.tracks.values_mut()` which implies it needs tracks?
        // Actually handle_play primarily subscribes to the stream identifier.
        // But let's check: handle_play iterates tracks?
        // It iterates tracks to set `is_sending = true`.
        // It subscribes via `event_producer`.

        // Ensure we use the aggregate stream identifier for PLAY requests that include track IDs.
        session.stream_identifier = Some(StreamIdentifier::Rtsp {
            stream_path: "live/test".to_string(),
        });

        // Mock StreamHub handling Subscribe
        let subscribe_handle = tokio::spawn(async move {
            use crate::streamhub::define::DataReceiver;
            if let Some(event) = event_receiver.recv().await {
                if let StreamHubEvent::Subscribe {
                    identifier,
                    result_sender,
                    ..
                } = event
                {
                    match identifier {
                        StreamIdentifier::Rtsp { stream_path } => {
                            assert_eq!(stream_path, "live/test");
                        }
                        _ => panic!("Expected RTSP identifier"),
                    }
                    // Create a channel for frame data that we immediately close to simulate end/error
                    let (_frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();

                    let data_receiver = DataReceiver {
                        frame_receiver: Some(frame_receiver),
                        packet_receiver: None,
                    };

                    let _ = result_sender.send(Ok((data_receiver, None)));
                }
            }
        });

        let content = "PLAY rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nRange: npt=0.000-\r\n\r\n";
        let request = RtspRequest::unmarshal(content).unwrap();

        let result = session.handle_play(&request).await;
        assert!(result.is_ok());
        session.stop_playback_task().await;
        assert!(session.playback_task.is_none());

        subscribe_handle
            .await
            .expect("Subscribe handler task panicked");
    }

    #[tokio::test]
    async fn test_rtsp_server_session_play_normalizes_track_path() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let subscribe_handle = tokio::spawn(async move {
            use crate::streamhub::define::DataReceiver;
            if let Some(event) = event_receiver.recv().await {
                if let StreamHubEvent::Subscribe {
                    identifier,
                    result_sender,
                    ..
                } = event
                {
                    match identifier {
                        StreamIdentifier::Rtsp { stream_path } => {
                            assert_eq!(stream_path, "live/test");
                        }
                        _ => panic!("Expected RTSP identifier"),
                    }
                    let (_frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();

                    let data_receiver = DataReceiver {
                        frame_receiver: Some(frame_receiver),
                        packet_receiver: None,
                    };

                    let _ = result_sender.send(Ok((data_receiver, None)));
                }
            }
        });

        let content = "PLAY rtsp://localhost/live/test/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nRange: npt=0.000-\r\n\r\n";
        let request = RtspRequest::unmarshal(content).unwrap();

        let result = session.handle_play(&request).await;
        assert!(result.is_ok());
        session.stop_playback_task().await;
        assert!(session.playback_task.is_none());

        subscribe_handle
            .await
            .expect("Subscribe handler task panicked");
    }

    #[tokio::test]
    async fn test_rtsp_server_session_play_includes_rtp_info() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut mock_io = MockNetIO::new();

        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RTSP/1.0 200 OK")
                    && s.contains("RTP-Info")
                    && s.contains("rtptime=")
                    && s.contains("url=")
                    && s.contains("seq=")
            })
            .times(1)
            .returning(|_| Ok(()));

        let session_io: Box<dyn TNetIO + Send + Sync> = Box::new(mock_io);
        let remote_addr = "127.0.0.1:0".parse().unwrap();

        let mut session =
            RtspServerSession::new_with_io(session_io, event_sender, None, remote_addr);

        let codec_info = RtspCodecInfo {
            codec_id: crate::rtsp::rtsp_codec::RtspCodecId::H264,
            payload_type: 96,
            sample_rate: 90000,
            channel_count: 0,
        };
        let track = RtspTrack::new(TrackType::Video, codec_info, "trackID=0".to_string());
        session.tracks.insert(TrackType::Video, track);
        session.stream_identifier = Some(StreamIdentifier::Rtsp {
            stream_path: "live/test".to_string(),
        });

        let subscribe_handle = tokio::spawn(async move {
            use crate::streamhub::define::DataReceiver;
            if let Some(event) = event_receiver.recv().await {
                if let StreamHubEvent::Subscribe { result_sender, .. } = event {
                    let (_frame_sender, frame_receiver) = tokio::sync::mpsc::unbounded_channel();
                    drop(_frame_sender);
                    let data_receiver = DataReceiver {
                        frame_receiver: Some(frame_receiver),
                        packet_receiver: None,
                    };
                    let _ = result_sender.send(Ok((data_receiver, None)));
                }
            }
        });

        let content = "PLAY rtsp://127.0.0.1:8554/live/test/trackID=0 RTSP/1.0\r\nCSeq: 5\r\nRange: npt=0.000-\r\n\r\n";
        let request = RtspRequest::unmarshal(content).unwrap();

        let result = session.handle_play(&request).await;
        assert!(result.is_ok());
        session.stop_playback_task().await;
        assert!(session.playback_task.is_none());

        subscribe_handle
            .await
            .expect("Subscribe handler task panicked");
    }

    // ========================================================================
    // ServerSessionType Tests
    // ========================================================================

    #[test]
    fn test_server_session_type_push() {
        let session_type = define::ServerSessionType::Push;
        // Should be able to compare
        assert!(matches!(session_type, define::ServerSessionType::Push));
    }

    #[test]
    fn test_server_session_type_pull() {
        let session_type = define::ServerSessionType::Pull;
        assert!(matches!(session_type, define::ServerSessionType::Pull));
    }

    #[test]
    fn test_rtsp_server_session_exit_without_publish_or_subscribe() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mock_io = MockNetIO::new();
        let remote_addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let mut session =
            RtspServerSession::new_with_io(Box::new(mock_io), event_sender, None, remote_addr);
        let identifier = StreamIdentifier::Rtsp {
            stream_path: "stream1".to_string(),
        };

        let result = session.exit(identifier);
        assert!(result.is_ok());
        assert!(session.is_normal_exit);
        assert!(matches!(
            event_receiver.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
    }

    #[test]
    fn test_rtsp_server_session_exit_published_sends_unpublish() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mock_io = MockNetIO::new();
        let remote_addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let mut session =
            RtspServerSession::new_with_io(Box::new(mock_io), event_sender, None, remote_addr);
        session.has_published = true;

        let identifier = StreamIdentifier::Rtsp {
            stream_path: "stream1".to_string(),
        };
        let result = session.exit(identifier);
        assert!(result.is_ok());

        match event_receiver.try_recv().expect("expected UnPublish event") {
            StreamHubEvent::UnPublish { identifier, .. } => match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "stream1");
                }
                _ => panic!("Expected RTSP identifier"),
            },
            _ => panic!("Expected UnPublish event"),
        }
    }

    #[test]
    fn test_rtsp_server_session_exit_subscribed_sends_unsubscribe() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mock_io = MockNetIO::new();
        let remote_addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let mut session =
            RtspServerSession::new_with_io(Box::new(mock_io), event_sender, None, remote_addr);
        session.has_subscribed = true;

        let identifier = StreamIdentifier::Rtsp {
            stream_path: "stream1".to_string(),
        };
        let result = session.exit(identifier);
        assert!(result.is_ok());

        match event_receiver
            .try_recv()
            .expect("expected UnSubscribe event")
        {
            StreamHubEvent::UnSubscribe { identifier, .. } => match identifier {
                StreamIdentifier::Rtsp { stream_path } => {
                    assert_eq!(stream_path, "stream1");
                }
                _ => panic!("Expected RTSP identifier"),
            },
            _ => panic!("Expected UnSubscribe event"),
        }
    }

    // ========================================================================
    // Integration-style tests for parsing
    // ========================================================================

    #[test]
    fn test_interleaved_binary_data_all_channels() {
        // Test all common channel identifiers (0-3 for RTP/RTCP audio/video)
        for channel in 0..4u8 {
            let data: &[u8] = &[0x24, channel, 0x00, 0x10];
            let mut reader = BytesReader::new(BytesMut::from(data));

            let result = InterleavedBinaryData::new(&mut reader).unwrap();
            assert!(result.is_some());
            let interleaved = result.unwrap();
            assert_eq!(interleaved.channel_identifier, channel);
            assert_eq!(interleaved.length, 16);
        }
    }

    #[test]
    fn test_gen_response_service_unavailable() {
        let request = create_test_request("DESCRIBE", None);

        let response = RtspServerSession::gen_response(StatusCode::SERVICE_UNAVAILABLE, &request);
        assert_eq!(response.status_code, 503);
        assert_eq!(response.reason_phrase, "Service Unavailable");
    }

    #[test]
    fn test_gen_response_method_not_allowed() {
        let request = create_test_request("UNKNOWN", None);

        let response = RtspServerSession::gen_response(StatusCode::METHOD_NOT_ALLOWED, &request);
        assert_eq!(response.status_code, 405);
        assert_eq!(response.reason_phrase, "Method Not Allowed");
    }

    #[test]
    fn test_scale_rtp_timestamp_90000hz() {
        let ts = RtspServerSession::scale_rtp_timestamp(1000, 90_000);
        assert_eq!(ts, 90_000);
    }

    #[test]
    fn test_scale_rtp_timestamp_zero_clock() {
        let ts = RtspServerSession::scale_rtp_timestamp(1234, 0);
        assert_eq!(ts, 1234);
    }

    #[test]
    fn test_rtp_timestamp_normalizer_corrects_non_wrap_regression() {
        let mut normalizer = RtpTimestampNormalizer::default();

        let first = normalizer.normalize(1000, 90_000, TrackType::Video);
        let second = normalizer.normalize(1033, 90_000, TrackType::Video);
        let regressed = normalizer.normalize(0, 90_000, TrackType::Video);
        let next = normalizer.normalize(33, 90_000, TrackType::Video);

        assert_eq!(first.output_timestamp, 90_000);
        assert_eq!(second.output_timestamp, 92_970);
        assert!(regressed.non_wrap_regressed);
        assert_eq!(regressed.non_wrap_regression_count, 1);
        assert_eq!(
            regressed.output_timestamp,
            second.output_timestamp.wrapping_add(1)
        );
        assert!(next.output_timestamp > regressed.output_timestamp);
    }

    #[test]
    fn test_rtp_timestamp_normalizer_corrects_duplicate_timestamp() {
        let mut normalizer = RtpTimestampNormalizer::default();

        let first = normalizer.normalize(1_000, 90_000, TrackType::Video);
        let duplicate = normalizer.normalize(1_000, 90_000, TrackType::Video);
        let next = normalizer.normalize(1_040, 90_000, TrackType::Video);

        assert_eq!(first.output_timestamp, 90_000);
        assert!(duplicate.non_wrap_regressed);
        assert_eq!(duplicate.non_wrap_regression_count, 1);
        assert_eq!(
            duplicate.output_timestamp,
            first.output_timestamp.wrapping_add(1)
        );
        assert!(next.output_timestamp > duplicate.output_timestamp);
    }

    #[test]
    fn test_rtp_timestamp_normalizer_preserves_true_wrap() {
        let mut normalizer = RtpTimestampNormalizer::default();

        let first = normalizer.normalize(u32::MAX - 10, 0, TrackType::Video);
        let wrapped = normalizer.normalize(5, 0, TrackType::Video);

        assert_eq!(first.output_timestamp, u32::MAX - 10);
        assert!(!wrapped.non_wrap_regressed);
        assert_eq!(wrapped.non_wrap_regression_count, 0);
        assert_eq!(wrapped.output_timestamp, 5);
    }

    #[test]
    fn test_rtp_timestamp_normalizer_audio_no_scaling() {
        // Audio timestamps are already in sample units, should not be scaled
        let mut normalizer = RtpTimestampNormalizer::default();

        // AAC @ 48kHz: First frame at 0 samples
        let first = normalizer.normalize(0, 48_000, TrackType::Audio);
        assert_eq!(first.output_timestamp, 0);

        // Second frame at 1024 samples
        let second = normalizer.normalize(1024, 48_000, TrackType::Audio);
        assert_eq!(second.output_timestamp, 1024);

        // Third frame at 2048 samples
        let third = normalizer.normalize(2048, 48_000, TrackType::Audio);
        assert_eq!(third.output_timestamp, 2048);

        // No scaling should occur
        assert_eq!(second.scaled_timestamp, 1024);
        assert_eq!(third.scaled_timestamp, 2048);
    }

    #[test]
    fn test_rtp_timestamp_normalizer_video_scaling() {
        // Video timestamps are in milliseconds, should be scaled to 90kHz
        let mut normalizer = RtpTimestampNormalizer::default();

        // First frame at 0ms
        let first = normalizer.normalize(0, 90_000, TrackType::Video);
        assert_eq!(first.output_timestamp, 0);

        // Second frame at 33ms (typical for 30fps)
        let second = normalizer.normalize(33, 90_000, TrackType::Video);
        assert_eq!(second.output_timestamp, 2970); // 33 * 90000 / 1000

        // Third frame at 66ms
        let third = normalizer.normalize(66, 90_000, TrackType::Video);
        assert_eq!(third.output_timestamp, 5940); // 66 * 90000 / 1000
    }

    #[test]
    fn test_rtp_timestamp_audio_sequence_monotonic() {
        // Verify audio timestamps produce monotonic sequence without precision loss
        let mut normalizer = RtpTimestampNormalizer::default();

        // Simulate 100 AAC frames @ 48kHz (1024 samples/frame)
        for i in 0..100 {
            let timestamp = i * 1024;
            let result = normalizer.normalize(timestamp, 48_000, TrackType::Audio);

            // Timestamp should exactly match input (no scaling)
            assert_eq!(result.output_timestamp, timestamp);
            assert_eq!(result.scaled_timestamp, timestamp);
            assert!(!result.non_wrap_regressed);
        }
    }

    #[test]
    fn test_video_access_unit_assembler_coalesces_same_timestamp() {
        let mut assembler = VideoAccessUnitAssembler::default();

        let ts1 = 100u32;
        let ts2 = 200u32;

        // First chunk is a raw NAL (no Annex-B prefix).
        assert!(
            assembler
                .push(ts1, BytesMut::from(&b"\x67\x11\x22"[..]))
                .is_none()
        );

        // Second chunk already has a 3-byte Annex-B start code.
        assert!(
            assembler
                .push(ts1, BytesMut::from(&b"\x00\x00\x01\x68\x33"[..]))
                .is_none()
        );

        // Timestamp change flushes the previous access unit.
        let flushed = assembler
            .push(ts2, BytesMut::from(&b"\x65\x44"[..]))
            .expect("expected flush on timestamp change");

        assert_eq!(flushed.0, ts1);
        let mut expected = BytesMut::new();
        expected.extend_from_slice(&ANNEXB_NALU_START_CODE[..]);
        expected.extend_from_slice(&b"\x67\x11\x22"[..]);
        expected.extend_from_slice(&b"\x00\x00\x01\x68\x33"[..]);
        assert_eq!(flushed.1, expected);

        let (ts, bytes) = assembler.flush().expect("expected pending access unit");
        assert_eq!(ts, ts2);
        let mut expected2 = BytesMut::new();
        expected2.extend_from_slice(&ANNEXB_NALU_START_CODE[..]);
        expected2.extend_from_slice(&b"\x65\x44"[..]);
        assert_eq!(bytes, expected2);
    }

    #[test]
    fn test_video_access_unit_assembler_flush_empty_returns_none() {
        let mut assembler = VideoAccessUnitAssembler::default();
        assert!(assembler.flush().is_none());
        assert!(assembler.push(1, BytesMut::new()).is_none());
        assert!(assembler.flush().is_none());
    }
}
