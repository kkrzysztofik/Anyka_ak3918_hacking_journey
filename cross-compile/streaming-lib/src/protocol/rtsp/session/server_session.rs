use crate::config::StreamingConfig;
use crate::protocol::rtsp::global_trait::Marshal;
use crate::protocol::rtsp::global_trait::Unmarshal;
use crate::protocol::rtsp::rtsp_codec;
use bytes::{BufMut, Bytes, BytesMut};
use chrono::Utc;

use crate::protocol::rtsp::rtp::define::ANNEXB_NALU_START_CODE;
use crate::protocol::rtsp::rtp::utils::Marshal as RtpMarshal;

use crate::common::auth::SecretCarrier;
use crate::common::http::HttpRequest as RtspRequest;
use crate::common::http::HttpResponse as RtspResponse;
use crate::common::http::Marshal as RtspMarshal;
use crate::common::http::Unmarshal as RtspUnmarshal;
use crate::common::http::try_get_complete_message_len;
use crate::common::utils::LogThrottle;

use crate::protocol::rtsp::rtp::RtpPacket;
use crate::protocol::rtsp::rtsp_range::RtspRange;

use crate::protocol::rtsp::sdp::fmtp::Fmtp;

use crate::protocol::rtsp::rtsp_codec::RtspCodecInfo;
use crate::protocol::rtsp::rtsp_track::RtspTrack;
use crate::protocol::rtsp::rtsp_track::TrackType;
use crate::protocol::rtsp::rtsp_transport::ProtocolType;
use crate::protocol::rtsp::rtsp_transport::RtspTransport;

use crate::io::bytes_reader::BytesReader;
use crate::io::bytes_writer::AsyncBytesWriter;

use super::errors::SessionError;
use super::errors::SessionErrorValue;
use super::rtp_counters::{
    InterleavedBinaryData, RtpCountersHandle, RtpPacketObservation, RtpTrackCounters,
};
use crate::hub::define::DataSender;
use crate::hub::define::MediaInfo;
use crate::hub::define::VideoCodecType;
use crate::io::UdpIO;
use crate::io::bytes_writer::BytesWriter;
use http::StatusCode;
use tokio::sync::oneshot;

use crate::protocol::rtsp::rtp::errors::{PackerError, PackerErrorValue, UnPackerError};
use crate::protocol::rtsp::rtp::utils::OnRtpPacketFn;
use crate::protocol::rtsp::sdp::Sdp;

use super::define;
use super::define::rtsp_method_name;
use crate::io::TNetIO;
use crate::io::{TcpReadIO, TcpWriteIO};
use async_trait::async_trait;
use tokio::time::{Duration, timeout};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::common::auth::Auth;
use crate::hub::{
    define::{
        FrameData, FrameDataSender, Information, InformationSender, NotifyInfo, PublishType,
        PublisherInfo, StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo,
        TStreamHandler,
    },
    errors::{StreamHubError, StreamHubErrorValue},
    statistics::StatisticsStream,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, Notify};
use tracing::{debug, error, info, warn};

pub use super::playback::LagRecoveryMode;
use super::playback::{PlaybackLatencyPolicy, run_playback_loop};

const SESSION_ID_RANDOM_DIGITS: RandomDigitCount = RandomDigitCount::Four;
const DEFAULT_PLAY_READY_TIMEOUT_MS: u64 = 1500;

/// RFC 2326 §12.36 — Server header value.
const SERVER_HEADER: &str = "streaming-lib/0.1";

/// Log threshold for slow TCP/UDP RTP writes (wall-clock flush time, milliseconds).
const SLOW_WRITE_THRESHOLD_MS: u128 = 10;

/// How often a session may report slow writes. Crossing the threshold is a steady state on a
/// constrained transmit path, so it is reported as a rate rather than once per frame.
const SLOW_WRITE_REPORT_PERIOD: Duration = Duration::from_secs(30);

/// Default UDP inter-batch sleep when `udp_pace_sleep_micros` is `0` (embedded TX pacing).
const DEFAULT_UDP_PACE_SLEEP_MICROS: u64 = 300;

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

    /// Optional shared shutdown flag for graceful shutdown from server
    shutdown_flag: Option<Arc<AtomicBool>>,

    /// Packet-level RTP logging configuration and counters.
    ///
    /// `rtp_sample_interval` stores the packet sampling interval configured
    /// at session creation time (every N packets to log, 0 = disabled).
    rtp_sample_interval: u32,

    /// Streaming configuration for session setup and playback.
    #[allow(dead_code)]
    config: StreamingConfig,

    rtp_counters: HashMap<TrackType, RtpCountersHandle>,
    playback_cancel: Option<Arc<Notify>>,
    playback_task: Option<JoinHandle<()>>,
}

/// Log RTP sequence/timestamp anomalies and periodic packet samples for a track.
///
/// Shared by the UDP and TCP-interleaved send paths, which differ only in `protocol`.
#[allow(clippy::too_many_arguments)]
fn log_rtp_packet_stats(
    protocol: &str,
    stats: &RtpPacketObservation,
    packet: &RtpPacket,
    payload_len: usize,
    sample_interval: u32,
    track_label: &str,
    session_id: &str,
    remote_for_rtp: SocketAddr,
    stream_path: &str,
) {
    if stats.seq_gap || stats.seq_regressed || stats.timestamp_regressed {
        warn!(
            protocol = protocol,
            track = %track_label,
            session_id = %session_id,
            remote_addr = %remote_for_rtp,
            stream_path = %stream_path,
            prev_seq = ?stats.prev_seq,
            seq = packet.header.seq_number,
            seq_delta = ?stats.seq_delta,
            prev_timestamp = ?stats.prev_timestamp,
            timestamp = packet.header.timestamp,
            timestamp_delta = ?stats.timestamp_delta,
            seq_gap = stats.seq_gap,
            seq_regressed = stats.seq_regressed,
            timestamp_regressed = stats.timestamp_regressed,
            "rtp_packet_anomaly"
        );
    }

    if sample_interval > 0 && stats.packets_sent.is_multiple_of(sample_interval as u64) {
        debug!(
            protocol = protocol,
            track = %track_label,
            session_id = %session_id,
            remote_addr = %remote_for_rtp,
            stream_path = %stream_path,
            seq = packet.header.seq_number,
            timestamp = packet.header.timestamp,
            marker = packet.header.marker,
            size_bytes = payload_len,
            packets_sent = stats.packets_sent,
            bytes_sent = stats.bytes_sent,
            "rtp_packet_sample"
        );
    }
}

/// Marshal a packet, account it against the track counters, and emit the shared
/// anomaly/sample logs.
///
/// Shared preamble of the UDP and TCP-interleaved send paths; returns the
/// marshalled bytes and their length for the transport-specific framing that
/// follows.
#[allow(clippy::too_many_arguments)]
fn marshal_and_log_rtp_packet(
    protocol: &str,
    packet: &RtpPacket,
    counters: &RtpTrackCounters,
    stream_identifier: Option<&StreamIdentifier>,
    sample_interval: u32,
    track_label: &str,
    session_id: &str,
    remote_for_rtp: SocketAddr,
) -> Result<(BytesMut, usize), PackerError> {
    let msg = packet.marshal()?;
    let payload_len = msg.len();
    let stream_path = stream_identifier
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let stats = counters.on_packet_sent(
        payload_len,
        packet.header.seq_number,
        packet.header.timestamp,
    );

    log_rtp_packet_stats(
        protocol,
        &stats,
        packet,
        payload_len,
        sample_interval,
        track_label,
        session_id,
        remote_for_rtp,
        &stream_path,
    );

    Ok((msg, payload_len))
}

/// Write one accumulated frame's worth of RTP packets to the UDP socket.
///
/// Pace UDP writes: yield every N packets to let the kernel drain the socket
/// buffer and the NIC transmit queued packets, preventing client-side receive
/// buffer overflow.
#[allow(clippy::too_many_arguments)] // matches the other RTP write/handler helpers here
async fn write_udp_frame(
    io: &Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    packets: Vec<BytesMut>,
    udp_pace_batch: usize,
    udp_pace_sleep_micros: u32,
    track_label: &str,
    session_id: &str,
    remote_for_rtp: SocketAddr,
    slow_write_throttle: &LogThrottle,
) -> Result<(), PackerError> {
    let start = std::time::Instant::now();
    let packet_count = packets.len();
    let mut io = io.lock().await;
    let pace_batch = udp_pace_batch.max(1);
    let sleep_micros = if udp_pace_sleep_micros == 0 {
        DEFAULT_UDP_PACE_SLEEP_MICROS
    } else {
        u64::from(udp_pace_sleep_micros)
    };
    let pace_sleep = Duration::from_micros(sleep_micros.max(1));
    // Hand each pacing batch to the transport in one call rather than one await per datagram:
    // the per-datagram cost dominates a frame's send time, and the pacing contract (yield every
    // `pace_batch` packets so the kernel and NIC can drain) is unchanged by how a batch is issued.
    let mut written = 0;
    for chunk in packets.chunks(pace_batch) {
        let batch: Vec<Bytes> = chunk.iter().map(|pkt| pkt.clone().freeze()).collect();
        io.write_batch(batch).await?;
        written += chunk.len();
        if written < packet_count {
            tokio::time::sleep(pace_sleep).await;
        }
    }
    let elapsed = start.elapsed();

    // Log slow writes (see SLOW_WRITE_THRESHOLD_MS). On a constrained transmit path every
    // frame can cross the threshold, so report a rate rather than a line per frame.
    if elapsed.as_millis() >= SLOW_WRITE_THRESHOLD_MS
        && let Some(burst) = slow_write_throttle.record(elapsed.as_millis() as u64)
    {
        tracing::warn!(
            protocol = "UDP",
            track = %track_label,
            session_id = %session_id,
            remote_addr = %remote_for_rtp,
            elapsed_ms = elapsed.as_millis(),
            occurrences = burst.occurrences,
            peak_ms = burst.peak,
            window_secs = SLOW_WRITE_REPORT_PERIOD.as_secs(),
            "slow_udp_write"
        );
    }

    Ok(())
}

impl RtspServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        config: StreamingConfig,
    ) -> Self {
        let remote_addr = stream
            .peer_addr()
            .or_else(|_| stream.local_addr())
            .unwrap_or_else(|_| SocketAddr::from(([0, 0, 0, 0], 0)));

        // Enable TCP_NODELAY to reduce latency on small RTP packets
        if let Err(err) = stream.set_nodelay(true) {
            tracing::warn!(error = %err, remote_addr = %remote_addr, "failed_to_set_tcp_nodelay");
        }

        let (read_half, write_half) = stream.into_split();
        let read_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpReadIO::new(read_half));
        let write_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpWriteIO::new(write_half));

        Self::new_with_io_pair(read_io, write_io, event_producer, auth, remote_addr, config)
    }

    pub fn new_with_io(
        io: Box<dyn TNetIO + Send + Sync>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        // Tests and in-memory sessions can share one IO object for both read and write.
        let io = Arc::new(Mutex::new(io));
        Self::new_with_shared_io(io, event_producer, auth, remote_addr, config)
    }

    pub fn new_with_io_pair(
        read_io: Box<dyn TNetIO + Send + Sync>,
        write_io: Box<dyn TNetIO + Send + Sync>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        let read_io = Arc::new(Mutex::new(read_io));
        let write_io = Arc::new(Mutex::new(write_io));
        Self::new_with_reader_writer_io(
            read_io,
            write_io,
            event_producer,
            auth,
            remote_addr,
            config,
        )
    }

    fn new_with_shared_io(
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        Self::new_with_reader_writer_io(io.clone(), io, event_producer, auth, remote_addr, config)
    }

    fn new_with_reader_writer_io(
        io_reader: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
        remote_addr: SocketAddr,
        config: StreamingConfig,
    ) -> Self {
        let sample_interval = config.rtp_sample_interval;
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
            shutdown_flag: None,
            rtp_sample_interval: sample_interval,
            config,
            rtp_counters: HashMap::new(),
            playback_cancel: None,
            playback_task: None,
        }
    }

    /// Set the shutdown flag for graceful shutdown from server
    pub fn set_shutdown_flag(&mut self, flag: Arc<AtomicBool>) {
        self.shutdown_flag = Some(flag);
    }

    /// RFC 2326 §12.37: default session timeout in seconds.
    const SESSION_TIMEOUT_SECS: u64 = 60;

    /// Read from the IO reader with session timeout. Returns error on timeout.
    async fn read_with_session_timeout(
        io_reader: &Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        timeout_secs: u64,
        remote_addr: &SocketAddr,
    ) -> Result<BytesMut, SessionError> {
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            io_reader.lock().await.read(),
        )
        .await;
        match result {
            Ok(Ok(data)) => Ok(data),
            Ok(Err(e)) => Err(e.into()),
            Err(_) => {
                info!(
                    timeout_secs = timeout_secs,
                    remote_addr = %remote_addr,
                    "rtsp_session_timeout"
                );
                Err(SessionError {
                    value: SessionErrorValue::SessionTimeout(timeout_secs),
                })
            }
        }
    }

    async fn read_rtsp_data(&mut self) -> Result<(), SessionError> {
        while self.reader.len() < 4 {
            let data = Self::read_with_session_timeout(
                &self.io_reader,
                Self::SESSION_TIMEOUT_SECS,
                &self.remote_addr,
            )
            .await?;
            self.reader.extend_from_slice(&data[..]);
        }
        Ok(())
    }

    async fn handle_interleaved_data(&mut self) -> Result<(), SessionError> {
        if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
            match data {
                Some(a) => {
                    const INTERLEAVED_READ_TIMEOUT_SECS: u64 = 30;

                    while self.reader.len() < a.length as usize {
                        let read_result = timeout(
                            Duration::from_secs(INTERLEAVED_READ_TIMEOUT_SECS),
                            self.io_reader.lock().await.read(),
                        )
                        .await;

                        match read_result {
                            Ok(Ok(data)) => {
                                self.reader.extend_from_slice(&data[..]);
                            }
                            Ok(Err(e)) => {
                                return Err(SessionError::from(e));
                            }
                            Err(_) => {
                                // Timeout - close connection
                                return Err(SessionError::from(std::io::Error::new(
                                    std::io::ErrorKind::TimedOut,
                                    format!(
                                        "Interleaved read timeout after {}s",
                                        INTERLEAVED_READ_TIMEOUT_SECS
                                    ),
                                )));
                            }
                        }
                    }
                    self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                        .await?;
                }
                None => {
                    self.on_rtsp_message().await?;
                }
            }
        }
        Ok(())
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        // Clone shutdown flag for the run loop
        let shutdown_flag = self.shutdown_flag.clone();

        let run_result: Result<(), SessionError> = async {
            loop {
                if self.shutdown.load(Ordering::Acquire) {
                    break;
                }

                // Check shared shutdown flag if provided
                if let Some(ref flag) = shutdown_flag
                    && flag.load(Ordering::Acquire)
                {
                    break;
                }

                self.read_rtsp_data().await?;

                self.handle_interleaved_data().await?;
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
            cancel.notify_waiters();
        }
        if let Some(handle) = self.playback_task.take() {
            handle.abort();
        }
    }

    async fn stop_playback_task(&mut self) {
        if let Some(cancel) = self.playback_cancel.take() {
            cancel.notify_waiters();
        }
        if let Some(handle) = self.playback_task.take() {
            handle.abort();
            match handle.await {
                Ok(()) => {}
                Err(err) if err.is_cancelled() => {}
                Err(err) => {
                    warn!(error = %err, "playback_task_join_error");
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

    async fn parse_rtsp_request(
        &mut self,
        message_len: usize,
    ) -> Result<RtspRequest, SessionError> {
        let message_bytes = self.reader.read_bytes(message_len)?;
        let message_str = std::str::from_utf8(&message_bytes)?;

        let Some(rtsp_request) = RtspRequest::unmarshal(message_str) else {
            return Err(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("request parse failed".to_string()),
            });
        };
        Ok(rtsp_request)
    }

    /// Read until we have a complete RTSP message; returns the message length.
    async fn read_until_complete_message_len(&mut self) -> Result<usize, SessionError> {
        const MAX_RETRIES: u32 = 16;
        let mut retry_count = 0;
        loop {
            let data = self.reader.get_remaining_bytes();
            match try_get_complete_message_len(&data) {
                Ok(Some(len)) => return Ok(len),
                Ok(None) => {
                    if retry_count >= MAX_RETRIES {
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
        }
    }

    /// If the request fails version or Content-Length validation, returns the error response to send.
    fn validate_rtsp_request_headers(
        rtsp_request: &RtspRequest,
        remote_addr: &SocketAddr,
    ) -> Option<RtspResponse> {
        if rtsp_request.version != "RTSP/1.0" {
            warn!(
                version = %rtsp_request.version,
                remote_addr = %remote_addr,
                "rtsp_unsupported_version"
            );
            return Some(Self::gen_response(
                http::StatusCode::HTTP_VERSION_NOT_SUPPORTED,
                rtsp_request,
            ));
        }
        if let Some(cl_str) = rtsp_request.get_header("Content-Length") {
            let claimed: usize = cl_str.trim().parse().unwrap_or(0);
            let actual = rtsp_request.body.as_ref().map_or(0, |b| b.len());
            if claimed != actual {
                warn!(
                    claimed = claimed,
                    actual = actual,
                    remote_addr = %remote_addr,
                    "rtsp_content_length_mismatch"
                );
                return Some(Self::gen_response(
                    http::StatusCode::BAD_REQUEST,
                    rtsp_request,
                ));
            }
        }
        None
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let message_len = self.read_until_complete_message_len().await?;
        let rtsp_request = self.parse_rtsp_request(message_len).await?;

        if let Some(response) =
            Self::validate_rtsp_request_headers(&rtsp_request, &self.remote_addr)
        {
            self.send_response(&response).await?;
            return Ok(());
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
                    info!(error = %err, "handle_play_error");
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
                warn!(
                    method = %rtsp_request.method,
                    remote_addr = %self.remote_addr,
                    "rtsp_unknown_method"
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
        info!(
            method = "OPTIONS",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_request"
        );

        Ok(())
    }

    async fn handle_describe(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if self.auth.is_none() {
            let has_authorization_header = rtsp_request.get_header("Authorization").is_some();
            let has_userinfo_in_uri = rtsp_request.uri.host.contains('@');
            if has_authorization_header || has_userinfo_in_uri {
                self.send_unauthorized_response(rtsp_request).await?;
                return Ok(());
            }
        }

        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            let auth_result = auth.authenticate_request(
                &stream_name,
                &rtsp_request.uri.query,
                rtsp_request
                    .get_header("Authorization")
                    .map(std::string::String::as_str),
                true,
            );
            if auth_result.is_err() {
                self.send_unauthorized_response(rtsp_request).await?;
                return Ok(());
            }
        }

        // Request the stream from the hub and wait for its SDP (populates
        // self.sdp and self.tracks); shared with `ensure_tracks_from_streamhub`.
        self.ensure_tracks_from_streamhub(rtsp_request).await?;

        // M-02: RFC 2326 §12.1: honour Accept header in DESCRIBE
        if let Some(accept) = rtsp_request.get_header("Accept") {
            let dominated = accept.contains("application/sdp") || accept.contains("*/*");
            if !dominated {
                warn!(
                    accept = %accept,
                    remote_addr = %self.remote_addr,
                    "rtsp_not_acceptable"
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
        info!(
            method = "DESCRIBE",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            sdp_media_count = media_count,
            "rtsp_request"
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
                    if let Err(err) = sender_out.try_send(msg) {
                        error!(error = %err, "send_frame_error");
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

    fn ensure_stream_identifier(&mut self, request_path: &str) {
        if self.stream_identifier.is_none() {
            let normalized_path = self.normalize_rtsp_stream_path(request_path);
            if !normalized_path.is_empty() {
                self.stream_identifier = Some(StreamIdentifier::Rtsp {
                    stream_path: normalized_path,
                });
            }
        }
    }

    fn find_track_for_uri(&self, uri: &crate::common::http::Uri) -> Option<TrackType> {
        let request_uri = uri.marshal();
        for (track_type, track) in &self.tracks {
            if !track.media_control.is_empty() && request_uri.contains(&track.media_control) {
                return Some(track_type.clone());
            }
        }
        if self.tracks.contains_key(&TrackType::Video) {
            return Some(TrackType::Video);
        }
        self.tracks
            .iter()
            .next()
            .map(|(track_type, _)| track_type.clone())
    }

    async fn setup_tcp_transport(
        io_writer: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
        track: &mut RtspTrack,
        transport: &RtspTransport,
    ) {
        // set_transport MUST happen before rtcp_send_loop: the loop's first
        // RTCP SR fires immediately (tokio::time::interval first-tick semantics)
        // and needs the correct interleaved channel_identifier already assigned.
        track.set_transport(transport.clone()).await;
        track.create_packer(io_writer.clone()).await;
        track.rtcp_send_loop(io_writer).await;
    }

    async fn setup_udp_transport(
        remote_addr: SocketAddr,
        track: &mut RtspTrack,
        trans: &RtspTransport,
    ) -> Result<(Option<u16>, Option<u16>), SessionError> {
        let (rtp_port, rtcp_port) = trans
            .client_port
            .ok_or(SessionError {
                value: SessionErrorValue::MissingClientPort,
            })?
            .into();

        let address = remote_addr.ip().to_string();
        let mut rtp_server_port: Option<u16> = None;
        let mut rtcp_server_port: Option<u16> = None;

        if let Some(rtp_io) = UdpIO::new(address.clone(), rtp_port, 0).await {
            rtp_server_port = rtp_io.get_local_port();

            let box_udp_io: Box<dyn TNetIO + Send + Sync> = Box::new(rtp_io);
            let is_record = matches!(trans.transport_mod.as_deref(), Some("record"));
            if !is_record {
                track.create_packer(Arc::new(Mutex::new(box_udp_io))).await;
            } else {
                track.rtp_receive_loop(box_udp_io).await;
            }
        }

        let rtp_port = rtp_server_port.ok_or(SessionError {
            value: SessionErrorValue::MissingClientPort,
        })?;

        if let Some(rtcp_io) = UdpIO::new(address.clone(), rtcp_port, rtp_port + 1).await {
            rtcp_server_port = rtcp_io.get_local_port();
            let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                Arc::new(Mutex::new(Box::new(rtcp_io)));
            track.rtcp_receive_loop(box_rtcp_io.clone()).await;
            track.rtcp_send_loop(box_rtcp_io).await;
        }

        Ok((rtp_server_port, rtcp_server_port))
    }

    fn log_setup_request(&self, rtsp_request: &RtspRequest, track_type: &TrackType) {
        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        let transport_hdr = rtsp_request
            .get_header("Transport")
            .cloned()
            .unwrap_or_default();
        info!(
            method = "SETUP",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            track_type = ?track_type,
            transport_req = %transport_hdr,
            "rtsp_request"
        );
    }

    async fn handle_setup(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        self.ensure_stream_identifier(&rtsp_request.uri.path);
        self.ensure_tracks_from_streamhub(rtsp_request).await?;

        let Some(track_type) = self.find_track_for_uri(&rtsp_request.uri) else {
            let response = Self::gen_response(http::StatusCode::NOT_FOUND, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        };

        let Some(transport_data) = rtsp_request.get_header("Transport") else {
            self.send_response(&Self::gen_response(http::StatusCode::OK, rtsp_request))
                .await?;
            return Ok(());
        };

        if self.session_id.is_none() {
            self.session_id = Some(Uuid::new(SESSION_ID_RANDOM_DIGITS));
        }

        let transport = RtspTransport::unmarshal(transport_data);
        if let Err(ref err) = transport {
            warn!(
                error = %err,
                remote_addr = %self.remote_addr,
                "rtsp_unsupported_transport"
            );
            let response = Self::gen_response(
                http::StatusCode::from_u16(461).unwrap_or(http::StatusCode::BAD_REQUEST),
                rtsp_request,
            );
            self.send_response(&response).await?;
            return Ok(());
        }

        let Ok(mut trans) = transport else {
            unreachable!("transport error already handled");
        };

        let io_writer = self.io_writer.clone();
        let remote_addr = self.remote_addr;
        let (rtp_server_port, rtcp_server_port) = {
            let track = self.tracks.get_mut(&track_type).ok_or(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("track missing".to_string()),
            })?;

            match trans.protocol_type {
                ProtocolType::TCP => {
                    Self::setup_tcp_transport(io_writer, track, &trans).await;
                    (None, None)
                }
                ProtocolType::UDP => Self::setup_udp_transport(remote_addr, track, &trans).await?,
            }
        };

        let mut server_ports: [u16; 2] = [0, 0];
        if let Some(rtp_port) = rtp_server_port {
            server_ports[0] = rtp_port;
        }
        if let Some(rtcp_port) = rtcp_server_port {
            server_ports[1] = rtcp_port;
            trans.server_port = Some(server_ports);
        }

        let mut response = Self::gen_response(http::StatusCode::OK, rtsp_request);
        response
            .headers
            .insert("Transport".to_string(), trans.marshal());
        response.headers.insert(
            "Session".to_string(),
            self.session_id
                .ok_or(SessionError {
                    value: SessionErrorValue::MissingSessionId,
                })?
                .to_string(),
        );

        // For UDP, set_transport after server_port is assigned.
        // TCP already calls set_transport inside setup_tcp_transport (before rtcp_send_loop).
        if trans.protocol_type == ProtocolType::UDP {
            let track = self.tracks.get_mut(&track_type).ok_or(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("track missing".to_string()),
            })?;
            track.set_transport(trans).await;
        }

        self.send_response(&response).await?;
        self.log_setup_request(rtsp_request, &track_type);

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

    async fn wait_for_tracks(
        &mut self,
        rtsp_request: &RtspRequest,
        timeout: Duration,
    ) -> Result<bool, SessionError> {
        if !self.tracks.is_empty() {
            return Ok(true);
        }

        let deadline = Instant::now() + timeout;
        while Instant::now() <= deadline {
            self.ensure_tracks_from_streamhub(rtsp_request).await?;
            if !self.tracks.is_empty() {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        Ok(false)
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

    async fn build_rtp_info_header(&self, rtsp_request: &RtspRequest) -> Option<String> {
        let content_base = self.build_content_base(rtsp_request)?;
        if self.tracks.is_empty() {
            return None;
        }

        let mut rtp_info_parts = Vec::new();
        for track_type in [TrackType::Video, TrackType::Audio, TrackType::Application] {
            if let Some(track) = self.tracks.get(&track_type) {
                let rtp_channel = track.rtp_channel.lock().await;
                let seq = rtp_channel.initial_sequence();
                // RFC 3550 §5.1 — use random initial timestamp, not hardcoded 0
                let rtptime = rtp_channel.initial_timestamp();
                let track_url = format!("{}{}", content_base, track.media_control);
                rtp_info_parts.push(format!("url={};seq={};rtptime={}", track_url, seq, rtptime));
            }
        }

        if rtp_info_parts.is_empty() {
            None
        } else {
            Some(rtp_info_parts.join(", "))
        }
    }

    async fn apply_range_header(
        &mut self,
        rtsp_request: &RtspRequest,
        response: &mut RtspResponse,
    ) -> Result<bool, SessionError> {
        if let Some(range_str) = rtsp_request.get_header("Range") {
            match RtspRange::unmarshal(range_str) {
                Ok(range) => {
                    response
                        .headers
                        .insert(String::from("Range"), range.marshal());
                    Ok(true)
                }
                Err(err) => {
                    // RFC 2326 §11.3.7 — invalid Range returns 457
                    warn!(error = %err, "invalid_range_header");
                    let err_response = Self::gen_rtsp_response(457, "Invalid Range", rtsp_request);
                    self.send_response(&err_response).await?;
                    Ok(false)
                }
            }
        } else {
            Ok(true)
        }
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

    #[allow(clippy::too_many_arguments)]
    fn setup_tcp_play_packet_handler(
        channel_identifier: u8,
        counters: Arc<RtpTrackCounters>,
        stream_identifier: Option<StreamIdentifier>,
        track_label: String,
        session_id: String,
        remote_for_rtp: SocketAddr,
        sample_interval: u32,
        max_tcp_interleaved_frame_bytes: usize,
    ) -> OnRtpPacketFn {
        // Initial capacity tracks [`StreamingConfig::tcp_interleaved_buffer_max`] so embedded
        // deployments can lower per-connection RAM (default 1 MiB in config).
        let frame_buffer: Arc<Mutex<BytesMut>> = Arc::new(Mutex::new(BytesMut::with_capacity(
            max_tcp_interleaved_frame_bytes,
        )));

        Box::new(
            move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                let counters = counters.clone();
                let stream_identifier = stream_identifier.clone();
                let track_label = track_label.clone();
                let session_id = session_id.clone();
                let frame_buffer = frame_buffer.clone();
                Box::pin(async move {
                    let (msg, payload_len) = marshal_and_log_rtp_packet(
                        "TCP",
                        &packet,
                        &counters,
                        stream_identifier.as_ref(),
                        sample_interval,
                        &track_label,
                        &session_id,
                        remote_for_rtp,
                    )?;

                    // Build interleaved RTP packet: 0x24 + channel + length + payload
                    let interleaved_chunk = 4usize.saturating_add(payload_len);
                    if payload_len > u16::MAX as usize {
                        error!(
                            protocol = "TCP",
                            track = %track_label,
                            session_id = %session_id,
                            payload_len = payload_len,
                            "interleaved_rtsp_payload_exceeds_u16"
                        );
                        return Err(PackerError {
                            value: PackerErrorValue::InterleavedFraming(format!(
                                "RTP payload length {} exceeds {}",
                                payload_len,
                                u16::MAX
                            )),
                        });
                    }
                    let mut buffer = frame_buffer.lock().await;
                    let new_total = buffer.len().saturating_add(interleaved_chunk);
                    if new_total > max_tcp_interleaved_frame_bytes {
                        warn!(
                            protocol = "TCP",
                            session_id = %session_id,
                            track = %track_label,
                            channel_identifier = channel_identifier,
                            current_len = buffer.len(),
                            incoming = interleaved_chunk,
                            max = max_tcp_interleaved_frame_bytes,
                            "tcp_interleaved_frame_buffer_overflow"
                        );
                        buffer.clear();
                        return Err(PackerError {
                            value: PackerErrorValue::InterleavedFraming(
                                "tcp interleaved frame buffer overflow".to_string(),
                            ),
                        });
                    }
                    buffer.reserve(interleaved_chunk);
                    buffer.put_u8(0x24);
                    buffer.put_u8(channel_identifier);
                    let len_bytes = (payload_len as u16).to_be_bytes();
                    buffer.extend_from_slice(&len_bytes);
                    buffer.extend_from_slice(&msg);

                    // Flush only when marker bit is set (end of frame)
                    if packet.header.marker == 1 {
                        let start = std::time::Instant::now();
                        let data = buffer.split().freeze();
                        drop(buffer);
                        io.lock().await.write(data).await?;
                        let elapsed = start.elapsed();

                        // Log slow writes (see SLOW_WRITE_THRESHOLD_MS)
                        if elapsed.as_millis() >= SLOW_WRITE_THRESHOLD_MS {
                            tracing::warn!(
                                protocol = "TCP",
                                track = %track_label,
                                session_id = %session_id,
                                remote_addr = %remote_for_rtp,
                                elapsed_ms = elapsed.as_millis(),
                                "slow_tcp_write"
                            );
                        }
                    }

                    Ok(())
                })
            },
        )
    }

    /// Set up the per-packet callback for a UDP `PLAY` track.
    #[allow(clippy::too_many_arguments)]
    fn setup_udp_play_packet_handler(
        counters: Arc<RtpTrackCounters>,
        stream_identifier: Option<StreamIdentifier>,
        track_label: String,
        session_id: String,
        remote_for_rtp: SocketAddr,
        sample_interval: u32,
        udp_pace_batch: usize,
        udp_pace_sleep_micros: u32,
        max_udp_accumulated_frame_bytes: usize,
    ) -> OnRtpPacketFn {
        // Packet buffer: accumulate marshalled packets (150 packets for large I-frames).
        // Second element is running total of `buffer` payload bytes (avoids O(n) sum per packet).
        let udp_accum: Arc<Mutex<(Vec<BytesMut>, usize)>> =
            Arc::new(Mutex::new((Vec::with_capacity(150), 0)));
        // Per-session, so one struggling client cannot mask another's slow writes.
        let slow_write_throttle = Arc::new(LogThrottle::new(SLOW_WRITE_REPORT_PERIOD));

        Box::new(
            move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                let counters = counters.clone();
                let stream_identifier = stream_identifier.clone();
                let track_label = track_label.clone();
                let session_id = session_id.clone();
                let udp_accum = udp_accum.clone();
                let slow_write_throttle = slow_write_throttle.clone();
                Box::pin(async move {
                    let (msg, _payload_len) = marshal_and_log_rtp_packet(
                        "UDP",
                        &packet,
                        &counters,
                        stream_identifier.as_ref(),
                        sample_interval,
                        &track_label,
                        &session_id,
                        remote_for_rtp,
                    )?;

                    // Accumulate packet into buffer (bounded; same cap as TCP interleaved framing).
                    let mut guard = udp_accum.lock().await;
                    let (buffer, accumulated) = &mut *guard;
                    let incoming = msg.len();
                    let current = *accumulated;
                    let new_total = current.saturating_add(incoming);
                    if new_total > max_udp_accumulated_frame_bytes {
                        error!(
                            protocol = "UDP",
                            session_id = %session_id,
                            track = %track_label,
                            remote_addr = %remote_for_rtp,
                            current_accumulated = current,
                            incoming = incoming,
                            max = max_udp_accumulated_frame_bytes,
                            "udp_playback_packet_buffer_overflow"
                        );
                        buffer.clear();
                        *accumulated = 0;
                        return Err(PackerError {
                            value: PackerErrorValue::InterleavedFraming(format!(
                                "udp playback packet buffer overflow: {} + {} > {}",
                                current, incoming, max_udp_accumulated_frame_bytes
                            )),
                        });
                    }
                    buffer.push(msg);
                    *accumulated = new_total;

                    // Write all accumulated packets when marker bit is set (end of frame)
                    if packet.header.marker == 1 {
                        let packets: Vec<BytesMut> =
                            std::mem::replace(buffer, Vec::with_capacity(150));
                        *accumulated = 0;
                        drop(guard);
                        write_udp_frame(
                            &io,
                            packets,
                            udp_pace_batch,
                            udp_pace_sleep_micros,
                            &track_label,
                            &session_id,
                            remote_for_rtp,
                            &slow_write_throttle,
                        )
                        .await?;
                    }

                    Ok(())
                })
            },
        )
    }

    async fn handle_play(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            let auth_result = auth.authenticate_request(
                &stream_name,
                &rtsp_request.uri.query,
                rtsp_request
                    .get_header("Authorization")
                    .map(std::string::String::as_str),
                true,
            );
            if auth_result.is_err() {
                self.send_unauthorized_response(rtsp_request).await?;
                return Ok(());
            }
        }

        let cseq = rtsp_request.get_header("CSeq").cloned().unwrap_or_default();
        let session_id = self
            .session_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".to_string());
        info!(
            method = "PLAY",
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_request"
        );

        if let Some(range_str) = rtsp_request.get_header("Range")
            && RtspRange::unmarshal(range_str).is_err()
        {
            warn!(
                session_id = %session_id,
                remote_addr = %self.remote_addr,
                stream_path = %rtsp_request.uri.path,
                range = %range_str,
                "play_rejected_invalid_range"
            );
            let response = Self::gen_rtsp_response(457, "Invalid Range", rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        let play_gate_timeout_ms = if self.config.play_ready_timeout_ms > 0 {
            self.config.play_ready_timeout_ms
        } else {
            DEFAULT_PLAY_READY_TIMEOUT_MS
        };
        debug!(
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            timeout_ms = play_gate_timeout_ms,
            "play_waiting_for_tracks"
        );
        if !self
            .wait_for_tracks(
                rtsp_request,
                Duration::from_millis(play_gate_timeout_ms.max(50)),
            )
            .await?
        {
            warn!(
                session_id = %session_id,
                remote_addr = %self.remote_addr,
                stream_path = %rtsp_request.uri.path,
                timeout_ms = play_gate_timeout_ms,
                "play_rejected_waiting_for_sps_pps"
            );
            let response = Self::gen_response(http::StatusCode::SERVICE_UNAVAILABLE, rtsp_request);
            self.send_response(&response).await?;
            return Ok(());
        }

        let sample_interval = self.rtp_sample_interval;
        let udp_pace_batch = self.config.udp_pace_batch;
        let udp_pace_sleep_micros = self.config.udp_pace_sleep_micros;
        let tcp_interleaved_buffer_max = self.config.tcp_interleaved_buffer_max;
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

            let track_label = format!("{:?}", track_type);
            let session_id_clone = session_id_for_rtp.clone();
            let stream_id_clone = stream_identifier.clone();

            match protocol_type {
                ProtocolType::TCP => {
                    let channel_identifier = track
                        .transport
                        .interleaved
                        .map(|i| i[0])
                        .unwrap_or_else(|| {
                            error!("unexpected_state");
                            0
                        });
                    let handler = Self::setup_tcp_play_packet_handler(
                        channel_identifier,
                        counters,
                        stream_id_clone,
                        track_label,
                        session_id_clone,
                        remote_for_rtp,
                        sample_interval,
                        tcp_interleaved_buffer_max,
                    );
                    track.rtp_channel.lock().await.on_packet_handler(handler);
                }
                ProtocolType::UDP => {
                    // `tcp_interleaved_buffer_max` is the shared per-session cap on buffered bytes
                    // for one logical RTP frame (until marker); UDP uses the same field to bound
                    // accumulated packet payloads before flush (not TCP interleaving).
                    let handler = Self::setup_udp_play_packet_handler(
                        counters,
                        stream_id_clone,
                        track_label,
                        session_id_clone,
                        remote_for_rtp,
                        sample_interval,
                        udp_pace_batch,
                        udp_pace_sleep_micros,
                        tcp_interleaved_buffer_max,
                    );
                    track.rtp_channel.lock().await.on_packet_handler(handler);
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

        if let Some(rtp_info) = self.build_rtp_info_header(rtsp_request).await {
            response.headers.insert("RTP-Info".to_string(), rtp_info);
        }

        if !self.apply_range_header(rtsp_request, &mut response).await? {
            return Ok(());
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

        let receiver = event_result_receiver
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
        let latency_policy = PlaybackLatencyPolicy::from_config(&self.config);

        info!(
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            max_frame_age_ms = latency_policy.max_frame_age_ms,
            lag_recovery_mode = ?latency_policy.lag_recovery_mode,
            lag_recovery_threshold_ms = latency_policy.lag_recovery_threshold_ms,
            sustained_lag_frames = latency_policy.sustained_lag_frames,
            "playback_latency_policy"
        );

        self.stop_playback_task().await;
        let playback_cancel = Arc::new(Notify::new());
        let playback_cancel_for_task = playback_cancel.clone();
        self.playback_cancel = Some(playback_cancel);

        self.playback_task = Some(tokio::spawn(run_playback_loop(
            receiver,
            audio_rtp_channel,
            video_rtp_channel,
            playback_cancel_for_task,
            shutdown,
            session_id_for_task,
            remote_addr,
            request_path,
            latency_policy,
        )));

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
                    warn!(error = %err, "invalid_range_header_ignored");
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
        info!(
            method = %method,
            cseq = %cseq,
            session_id = %session_id,
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_request"
        );

        Ok(())
    }

    fn handle_teardown(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let identifier = self.resolve_stream_identifier(&rtsp_request.uri.path);
        info!(
            session_id = %self.session_id
                .map(|id| id.to_string())
                .unwrap_or_else(|| "none".to_string()),
            remote_addr = %self.remote_addr,
            stream_path = %rtsp_request.uri.path,
            session_type = ?self.session_type,
            "rtsp_teardown"
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
            info!(
                track = ?track_type,
                session_id = %self.session_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "none".to_string()),
                remote_addr = %self.remote_addr,
                stream_path = %stream_path,
                packets = packets,
                bytes = bytes,
                duration_ms = duration_ms.unwrap_or(0),
                bitrate_kbps = bitrate_kbps,
                "rtp_session_summary"
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
                info!("session_exit_no_pubsub");
                return Ok(());
            }
        };
        let event_json_str =
            serde_json::to_string(&event).unwrap_or_else(|_| "<serialize failed>".to_string());

        let rv = self.event_producer.send(event);
        match rv {
            Err(err) => {
                error!(error = %err, event_json = %event_json_str, "session_exit_send_error");
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                self.is_normal_exit = true;
                info!(event_json = %event_json_str, "session_exit_success");
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
            info!(media_name = %media_name, "media_track_info");
            match media_name.as_str() {
                "audio" => {
                    let codec_name = media.rtpmap.encoding_name.to_lowercase();
                    let codec_id = rtsp_codec::RtspCodecId::from_name(codec_name.as_str()).ok_or(
                        SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(format!(
                                "unsupported audio codec: {}",
                                codec_name
                            )),
                        },
                    )?;
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

                    info!(codec_info = ?codec_info, "audio_codec_info");

                    let track = RtspTrack::new(TrackType::Audio, codec_info, media_control);
                    self.tracks.insert(TrackType::Audio, track);
                    self.rtp_counters
                        .entry(TrackType::Audio)
                        .or_insert_with(|| Arc::new(RtpTrackCounters::new()));
                }
                "video" => {
                    let codec_name = media.rtpmap.encoding_name.to_lowercase();
                    let codec_id = rtsp_codec::RtspCodecId::from_name(codec_name.as_str()).ok_or(
                        SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(format!(
                                "unsupported video codec: {}",
                                codec_name
                            )),
                        },
                    )?;
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

    async fn send_unauthorized_response(
        &mut self,
        rtsp_request: &RtspRequest,
    ) -> Result<(), SessionError> {
        let mut response = Self::gen_response(http::StatusCode::UNAUTHORIZED, rtsp_request);
        if let Some(auth) = &self.auth {
            response
                .headers
                .insert("WWW-Authenticate".to_string(), auth.basic_challenge());
        }
        self.send_response(&response).await
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
                warn!(
                    remote_addr = %self.remote_addr,
                    requested = ?requested,
                    current = ?current.to_string(),
                    "session_id_mismatch"
                );
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
            sub_data_type: crate::hub::define::SubDataType::Frame,
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
            pub_data_type: crate::hub::define::PubDataType::Frame,
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

    #[allow(dead_code)]
    async fn send_prior_data_rtsp_remux_rtmp(
        &self,
        sender: &FrameDataSender,
    ) -> Result<(), StreamHubError> {
        let sdp_info = self.sdp.lock().await;
        let mut video_clock_rate: u32 = 0;
        let mut audio_clock_rate: u32 = 0;
        let mut vcodec: VideoCodecType = VideoCodecType::H264;

        for media in &sdp_info.medias {
            Self::send_prior_fmtp_frames(
                sender,
                media,
                &mut video_clock_rate,
                &mut audio_clock_rate,
                &mut vcodec,
            )?;
        }

        if let Err(err) = sender.try_send(FrameData::MediaInfo {
            media_info: MediaInfo {
                audio_clock_rate,
                video_clock_rate,
                vcodec,
            },
        }) {
            error!(error = %err, "send_media_info_error");
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn send_prior_fmtp_frames(
        sender: &FrameDataSender,
        media: &crate::protocol::rtsp::sdp::SdpMediaInfo,
        video_clock_rate: &mut u32,
        audio_clock_rate: &mut u32,
        vcodec: &mut VideoCodecType,
    ) -> Result<(), StreamHubError> {
        let Some(fmtp) = &media.fmtp else {
            return Ok(());
        };
        match fmtp {
            Fmtp::H264(data) => {
                let mut bytes_writer = BytesWriter::new();
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.sps)?;
                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                bytes_writer.write(&data.pps)?;
                let frame_data = FrameData::Video {
                    timestamp: 0,
                    data: bytes_writer.extract_current_bytes(),
                };
                if let Err(err) = sender.try_send(frame_data) {
                    error!(error = %err, "send_sps_pps_error");
                }
                *video_clock_rate = media.rtpmap.clock_rate;
            }
            Fmtp::H265(data) => {
                let mut bytes_writer = BytesWriter::new();
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
                if let Err(err) = sender.try_send(frame_data) {
                    error!(error = %err, "send_sps_pps_vps_error");
                }
                *vcodec = VideoCodecType::H265;
                *video_clock_rate = media.rtpmap.clock_rate;
            }
            Fmtp::Mpeg4(data) => {
                let frame_data = FrameData::Audio {
                    timestamp: 0,
                    data: data.asc.clone(),
                };
                if let Err(err) = sender.try_send(frame_data) {
                    error!(error = %err, "send_asc_error");
                }
                *audio_clock_rate = media.rtpmap.clock_rate;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl TStreamHandler for RtspStreamHandler {
    async fn send_prior_data(
        &self,
        data_sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        let _sender = match data_sender {
            DataSender::Frame { sender } => sender,
            DataSender::Packet { sender: _ } => {
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NotCorrectDataSenderType,
                });
            }
        };
        // No specific handling needed for remaining SubscribeType variants
        let _ = sub_type;
        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        if let Err(err) = sender.send(Information::Sdp {
            data: self.sdp.lock().await.marshal(),
        }) {
            error!(error = %err, "send_information_error");
        }
    }
}

#[cfg(test)]
#[path = "server_session_tests.rs"]
mod tests;
