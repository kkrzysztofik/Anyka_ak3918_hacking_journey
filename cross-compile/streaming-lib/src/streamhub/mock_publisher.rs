use crate::codec::h264_file_reader::{H264FileError, H264FileReader, NalUnitType};
use crate::streamhub::define::{
    DataSender, FrameData, FrameDataSender, Information, InformationSender, MediaInfo,
    SubscribeType, TStreamHandler, VideoCodecType,
};
use crate::streamhub::{StatisticsStream, StreamHubError};
use async_trait::async_trait;
use bytes::BytesMut;
use portable_atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::{self, Interval, MissedTickBehavior};

/// Errors that can occur during mock video publishing
#[derive(Error, Debug)]
pub enum PublisherError {
    #[error("H264 file error: {0}")]
    H264Error(#[from] H264FileError),

    #[error("Channel send error")]
    ChannelError,

    #[error("Stream already stopped")]
    StreamStopped,
}

/// Mock video publisher for H264 file playback
pub struct MockVideoPublisher {
    stream_name: String,
    reader: Arc<Mutex<H264FileReader>>,
    sps: Vec<u8>,
    pps: Vec<u8>,
    bootstrap_idr: Option<Vec<u8>>,
    access_units_cache: Option<Arc<Vec<Vec<u8>>>>,
    frame_duration_ms: u32,
    is_running: Arc<Mutex<bool>>,
    loop_playback: bool,
    last_timestamp_ms: Arc<AtomicU32>,
    /// Callback to get current subscriber count for on-demand publishing
    subscriber_count_fn: Option<Arc<dyn Fn() -> usize + Send + Sync>>,
}

impl MockVideoPublisher {
    const DEFAULT_BOOTSTRAP_IDR_SCAN_MAX_BYTES: u64 = 4 * 1024 * 1024;
    const BOOTSTRAP_IDR_SCAN_MAX_BYTES_ENV: &'static str = "ONVIF_BOOTSTRAP_IDR_SCAN_MAX_BYTES";
    const DEFAULT_ACCESS_UNITS_CACHE_MAX_BYTES: u64 = 8 * 1024 * 1024;
    const ACCESS_UNITS_CACHE_MAX_BYTES_ENV: &'static str = "ONVIF_ACCESS_UNITS_CACHE_MAX_BYTES";

    /// Create a new mock video publisher from H264 file
    ///
    /// # Arguments
    ///
    /// * `stream_name` - Name of the stream
    /// * `file_path` - Path to H264 file in Annex-B format
    /// * `frame_rate` - Frame rate in fps (default 25fps)
    /// * `loop_playback` - If true, loop the file when reaching EOF; if false, stop
    ///
    /// # Returns
    ///
    /// Result with MockVideoPublisher or PublisherError
    pub async fn new(
        stream_name: String,
        file_path: &str,
        frame_rate: u32,
        loop_playback: bool,
    ) -> Result<Self, PublisherError> {
        let mut reader = H264FileReader::new(file_path, frame_rate).await?;
        let frame_duration_ms = reader.frame_duration_ms();
        let (sps, pps) = reader.extract_sps_pps().await?;
        let bootstrap_idr = Self::extract_first_idr(&mut reader).await?;
        let access_units_cache = Self::try_build_access_units_cache(file_path, &mut reader).await?;

        Ok(Self {
            stream_name,
            reader: Arc::new(Mutex::new(reader)),
            sps,
            pps,
            bootstrap_idr,
            access_units_cache,
            frame_duration_ms,
            is_running: Arc::new(Mutex::new(false)),
            loop_playback,
            last_timestamp_ms: Arc::new(AtomicU32::new(0)),
            subscriber_count_fn: None,
        })
    }

    /// Set subscriber count callback for on-demand publishing.
    ///
    /// When set, the publisher will pause (sleep) when subscriber count is 0,
    /// reducing CPU usage. Publishing resumes when subscribers connect.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function that returns the current subscriber count
    pub fn set_subscriber_count_callback(
        &mut self,
        callback: Arc<dyn Fn() -> usize + Send + Sync>,
    ) {
        self.subscriber_count_fn = Some(callback);
    }

    fn bootstrap_idr_scan_max_bytes() -> u64 {
        std::env::var(Self::BOOTSTRAP_IDR_SCAN_MAX_BYTES_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(Self::DEFAULT_BOOTSTRAP_IDR_SCAN_MAX_BYTES)
    }

    fn access_units_cache_max_bytes() -> u64 {
        std::env::var(Self::ACCESS_UNITS_CACHE_MAX_BYTES_ENV)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .filter(|value| *value > 0)
            .unwrap_or(Self::DEFAULT_ACCESS_UNITS_CACHE_MAX_BYTES)
    }

    async fn try_build_access_units_cache(
        file_path: &str,
        reader: &mut H264FileReader,
    ) -> Result<Option<Arc<Vec<Vec<u8>>>>, PublisherError> {
        let file_size = std::fs::metadata(file_path)
            .map(|meta| meta.len())
            .unwrap_or(u64::MAX);
        let max_cache_bytes = Self::access_units_cache_max_bytes();
        if file_size > max_cache_bytes {
            reader.reset().await?;
            return Ok(None);
        }

        let access_units = Self::collect_access_units(reader).await?;
        if access_units.is_empty() {
            return Ok(None);
        }

        let total_bytes: usize = access_units.iter().map(Vec::len).sum();
        log::info!(
            "mock_publisher: access-unit cache enabled (frames={} bytes={} source_bytes={})",
            access_units.len(),
            total_bytes,
            file_size
        );
        Ok(Some(Arc::new(access_units)))
    }

    async fn collect_access_units(
        reader: &mut H264FileReader,
    ) -> Result<Vec<Vec<u8>>, PublisherError> {
        let mut access_units = Vec::new();
        let mut pending_access_unit = BytesMut::new();

        while let Ok(Some(nal)) = reader.read_next_nal().await {
            match nal.unit_type {
                NalUnitType::SequenceParameterSet | NalUnitType::PictureParameterSet => {
                    if !pending_access_unit.is_empty() {
                        access_units.push(std::mem::take(&mut pending_access_unit).to_vec());
                    }
                }
                NalUnitType::IdrSlice | NalUnitType::NonIdrSlice => {
                    let first_slice_of_picture = is_first_vcl_slice(&nal.data);
                    if first_slice_of_picture && !pending_access_unit.is_empty() {
                        access_units.push(std::mem::take(&mut pending_access_unit).to_vec());
                    }
                    append_annexb_nal(&mut pending_access_unit, &nal.data);
                }
                _ => {}
            }
        }

        if !pending_access_unit.is_empty() {
            access_units.push(std::mem::take(&mut pending_access_unit).to_vec());
        }

        reader.reset().await?;
        Ok(access_units)
    }

    async fn extract_first_idr(
        reader: &mut H264FileReader,
    ) -> Result<Option<Vec<u8>>, PublisherError> {
        let max_scan_bytes = Self::bootstrap_idr_scan_max_bytes();
        Self::extract_first_idr_with_limit(reader, max_scan_bytes).await
    }

    async fn extract_first_idr_with_limit(
        reader: &mut H264FileReader,
        max_scan_bytes: u64,
    ) -> Result<Option<Vec<u8>>, PublisherError> {
        let mut first_idr = None;
        let mut reached_scan_limit = false;
        while reader.current_position() < max_scan_bytes {
            let before = reader.current_position();
            match reader.read_next_nal().await? {
                Some(nal) => {
                    if nal.unit_type == NalUnitType::IdrSlice {
                        first_idr = Some(nal.data);
                        break;
                    }
                }
                None => break,
            }
            if reader.current_position() <= before {
                break;
            }
        }
        if first_idr.is_none() && reader.current_position() >= max_scan_bytes {
            reached_scan_limit = true;
        }
        let scanned_bytes = reader.current_position();
        reader.reset().await?;
        if reached_scan_limit {
            log::warn!(
                "mock_publisher: bootstrap IDR scan reached {} bytes (scanned={}), continuing without bootstrap IDR",
                max_scan_bytes,
                scanned_bytes
            );
        }
        Ok(first_idr)
    }

    /// Start publishing frames from the H264 file
    ///
    /// # Arguments
    ///
    /// * `sender` - Channel sender for frame data (unbounded)
    ///
    /// # Returns
    ///
    /// Tokio join handle for the publishing task
    pub fn start_publishing(&self, sender: FrameDataSender) -> tokio::task::JoinHandle<()> {
        let reader = Arc::clone(&self.reader);
        let access_units_cache = self.access_units_cache.clone();
        let is_running = Arc::clone(&self.is_running);
        let _stream_name = self.stream_name.clone();
        let _sps = self.sps.clone();
        let _pps = self.pps.clone();
        let loop_playback = self.loop_playback;
        let frame_duration_ms = self.frame_duration_ms;
        let last_timestamp_ms = Arc::clone(&self.last_timestamp_ms);
        let subscriber_count_fn = self.subscriber_count_fn.clone();

        tokio::spawn(async move {
            {
                let mut running = is_running.lock().await;
                *running = true;
            }

            let mut timestamp_offset = 0u32;
            let mut frames_since_report: u64 = 0;
            let mut last_report = Instant::now();

            loop {
                let is_running_check = is_running.lock().await;
                if !*is_running_check {
                    break;
                }
                drop(is_running_check);

                if has_no_subscribers(&subscriber_count_fn) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }

                if let Some(access_units) = access_units_cache.as_ref() {
                    let result = publish_from_cache(
                        &sender,
                        access_units,
                        frame_duration_ms,
                        timestamp_offset,
                        &last_timestamp_ms,
                        &subscriber_count_fn,
                        &mut frames_since_report,
                        &mut last_report,
                    )
                    .await;
                    match result {
                        CachePublishResult::Interrupted => continue,
                        CachePublishResult::ChannelClosed => {
                            set_not_running(&is_running).await;
                            return;
                        }
                        CachePublishResult::Completed(frame_count) => {
                            if !loop_playback {
                                break;
                            }
                            timestamp_offset = timestamp_offset
                                .saturating_add(frame_count.saturating_mul(frame_duration_ms));
                        }
                    }
                    continue;
                }

                let result = publish_from_reader(
                    &reader,
                    &sender,
                    frame_duration_ms,
                    timestamp_offset,
                    loop_playback,
                    &last_timestamp_ms,
                    &subscriber_count_fn,
                    &mut frames_since_report,
                    &mut last_report,
                )
                .await;

                match result {
                    ReaderPublishResult::Interrupted => continue,
                    ReaderPublishResult::ChannelClosed => {
                        set_not_running(&is_running).await;
                        return;
                    }
                    ReaderPublishResult::Completed(new_offset) => {
                        if !loop_playback {
                            break;
                        }
                        timestamp_offset = new_offset;
                    }
                }
            }

            set_not_running(&is_running).await;
        })
    }

    /// Stop publishing frames
    pub async fn stop_publishing(&self) {
        let mut running = self.is_running.lock().await;
        *running = false;
    }

    /// Check if currently publishing
    pub async fn is_publishing(&self) -> bool {
        *self.is_running.lock().await
    }

    /// Get stream name
    pub fn stream_name(&self) -> &str {
        &self.stream_name
    }

    /// Get SPS data
    pub fn sps(&self) -> &[u8] {
        &self.sps
    }

    /// Get PPS data
    pub fn pps(&self) -> &[u8] {
        &self.pps
    }

    /// Get a shared handle to the publisher's most recently emitted VCL access unit timestamp.
    pub fn last_timestamp_handle(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.last_timestamp_ms)
    }

    /// Get a bootstrap IDR NAL (if available) for fast decoder synchronization.
    pub fn bootstrap_idr(&self) -> Option<Vec<u8>> {
        self.bootstrap_idr.clone()
    }
}

/// TStreamHandler implementation for MockVideoPublisher
/// Provides SDP and media info to StreamHub subscribers
#[async_trait]
impl TStreamHandler for MockVideoPublisher {
    /// Send prior metadata and SPS/PPS data to new subscribers
    async fn send_prior_data(
        &self,
        sender: DataSender,
        _sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        // Extract frame sender from DataSender
        if let DataSender::Frame {
            sender: frame_sender,
        } = sender
        {
            // Use last known timestamp to avoid sending regressions (e.g. 0) mid-stream.
            let ts = self.last_timestamp_ms.load(Ordering::Relaxed);

            // Send MediaInfo first
            let media_info = MediaInfo {
                audio_clock_rate: 48000,
                video_clock_rate: 90000,
                vcodec: VideoCodecType::H264,
            };

            let _ = frame_sender.send(FrameData::MediaInfo { media_info });

            // Send SPS as video frame
            let sps_data = BytesMut::from(self.sps.as_slice());
            let _ = frame_sender.send(FrameData::Video {
                timestamp: ts,
                data: sps_data,
            });

            // Send PPS as video frame
            let pps_data = BytesMut::from(self.pps.as_slice());
            let _ = frame_sender.send(FrameData::Video {
                timestamp: ts,
                data: pps_data,
            });

            // Ensure new subscribers can decode immediately even when joining mid-GOP.
            if let Some(idr) = self.bootstrap_idr.as_ref() {
                let idr_data = BytesMut::from(idr.as_slice());
                let _ = frame_sender.send(FrameData::Video {
                    timestamp: ts,
                    data: idr_data,
                });
            }
        }

        Ok(())
    }

    /// Get statistics data (not implemented for mock publisher)
    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    /// Send SDP information to subscribers
    async fn send_information(&self, sender: InformationSender) {
        // Generate SDP from SPS/PPS
        let sdp = generate_sdp_from_sps_pps(&self.sps, &self.pps);

        let _ = sender.send(Information::Sdp { data: sdp });
    }
}

/// Generate SDP string from H264 SPS and PPS parameters
fn generate_sdp_from_sps_pps(sps: &[u8], pps: &[u8]) -> String {
    // Extract profile-level-id from SPS RBSP bytes
    // SPS format: [NAL header(0x67)][profile_idc][constraint_flags][level_idc]...
    // We extract bytes [1][2][3] which are the RBSP bytes
    let profile_level_id = if sps.len() >= 4 {
        format!("{:02x}{:02x}{:02x}", sps[1], sps[2], sps[3])
    } else {
        "42e01e".to_string()
    };

    let sps_b64 = base64_encode(sps);
    let pps_b64 = base64_encode(pps);

    let mut sdp = String::new();
    sdp.push_str("v=0\r\n");
    sdp.push_str("o=- 0 0 IN IP4 0.0.0.0\r\n");
    sdp.push_str("s=Mock H264 Stream\r\n");
    sdp.push_str("c=IN IP4 0.0.0.0\r\n");
    sdp.push_str("t=0 0\r\n");
    sdp.push_str("a=tool:streaming-lib-mock\r\n");
    sdp.push_str("a=control:*\r\n");
    sdp.push_str("m=video 0 RTP/AVP 96\r\n");
    sdp.push_str("a=rtpmap:96 H264/90000\r\n");
    sdp.push_str(&format!(
        "a=fmtp:96 profile-level-id={};sprop-parameter-sets={},{}\r\n",
        profile_level_id, sps_b64, pps_b64
    ));
    sdp.push_str("a=control:trackID=0\r\n");
    sdp.push_str("a=sendonly\r\n");
    sdp
}

fn has_no_subscribers(subscriber_count_fn: &Option<Arc<dyn Fn() -> usize + Send + Sync>>) -> bool {
    subscriber_count_fn.as_ref().is_some_and(|cb| cb() == 0)
}

async fn set_not_running(is_running: &Arc<Mutex<bool>>) {
    let mut running = is_running.lock().await;
    *running = false;
}

enum CachePublishResult {
    Interrupted,
    ChannelClosed,
    Completed(u32),
}

#[allow(clippy::too_many_arguments)]
async fn publish_from_cache(
    sender: &FrameDataSender,
    access_units: &Arc<Vec<Vec<u8>>>,
    frame_duration_ms: u32,
    timestamp_offset: u32,
    last_timestamp_ms: &AtomicU32,
    subscriber_count_fn: &Option<Arc<dyn Fn() -> usize + Send + Sync>>,
    frames_since_report: &mut u64,
    last_report: &mut Instant,
) -> CachePublishResult {
    let mut frame_count: u32 = 0;
    let frame_interval = Duration::from_millis(frame_duration_ms as u64);
    let mut pacer = build_pacer(frame_interval);

    for access_unit in access_units.iter() {
        if has_no_subscribers(subscriber_count_fn) {
            return CachePublishResult::Interrupted;
        }

        let timestamp =
            timestamp_offset.saturating_add(frame_count.saturating_mul(frame_duration_ms));
        if !send_access_unit(
            sender,
            BytesMut::from(access_unit.as_slice()),
            timestamp,
            &mut pacer,
            last_timestamp_ms,
            frame_count,
            frames_since_report,
            last_report,
        )
        .await
        {
            return CachePublishResult::ChannelClosed;
        }
        frame_count = frame_count.saturating_add(1);
    }

    CachePublishResult::Completed(frame_count)
}

enum ReaderPublishResult {
    Interrupted,
    ChannelClosed,
    Completed(u32),
}

#[allow(clippy::too_many_arguments)]
async fn publish_from_reader(
    reader: &Arc<Mutex<H264FileReader>>,
    sender: &FrameDataSender,
    frame_duration_ms: u32,
    timestamp_offset: u32,
    loop_playback: bool,
    last_timestamp_ms: &AtomicU32,
    subscriber_count_fn: &Option<Arc<dyn Fn() -> usize + Send + Sync>>,
    frames_since_report: &mut u64,
    last_report: &mut Instant,
) -> ReaderPublishResult {
    let mut reader = reader.lock().await;
    let mut frame_count: u32 = 0;
    let frame_interval = Duration::from_millis(frame_duration_ms as u64);
    let mut pacer = build_pacer(frame_interval);
    let mut pending_access_unit = BytesMut::new();
    let mut pending_access_unit_timestamp: Option<u32> = None;

    while let Ok(Some(nal)) = reader.read_next_nal().await {
        if has_no_subscribers(subscriber_count_fn) {
            return ReaderPublishResult::Interrupted;
        }

        match nal.unit_type {
            NalUnitType::SequenceParameterSet => {
                if let Some(ts) = process_pending_access_unit(
                    sender,
                    &mut pending_access_unit,
                    pending_access_unit_timestamp.take(),
                    &mut pacer,
                    last_timestamp_ms,
                    &mut frame_count,
                    frames_since_report,
                    last_report,
                )
                .await
                {
                    return ts;
                }

                send_param_set_frame(
                    sender,
                    &nal.data,
                    timestamp_offset,
                    frame_count,
                    frame_duration_ms,
                    "SPS",
                );
            }
            NalUnitType::PictureParameterSet => {
                if let Some(ts) = process_pending_access_unit(
                    sender,
                    &mut pending_access_unit,
                    pending_access_unit_timestamp.take(),
                    &mut pacer,
                    last_timestamp_ms,
                    &mut frame_count,
                    frames_since_report,
                    last_report,
                )
                .await
                {
                    return ts;
                }

                send_param_set_frame(
                    sender,
                    &nal.data,
                    timestamp_offset,
                    frame_count,
                    frame_duration_ms,
                    "PPS",
                );
            }
            NalUnitType::IdrSlice | NalUnitType::NonIdrSlice => {
                let first_slice = is_first_vcl_slice(&nal.data);
                if first_slice
                    && pending_access_unit_timestamp.is_some()
                    && let Some(ts) = process_pending_access_unit(
                        sender,
                        &mut pending_access_unit,
                        pending_access_unit_timestamp.take(),
                        &mut pacer,
                        last_timestamp_ms,
                        &mut frame_count,
                        frames_since_report,
                        last_report,
                    )
                    .await
                {
                    return ts;
                }

                let timestamp = pending_access_unit_timestamp.get_or_insert_with(|| {
                    timestamp_offset.saturating_add(frame_count.saturating_mul(frame_duration_ms))
                });
                append_annexb_nal(&mut pending_access_unit, &nal.data);
                log_vcl_frame(frame_count, *timestamp, first_slice, nal.unit_type);
            }
            _ => {}
        }
    }

    if let Some(ts) = process_pending_access_unit(
        sender,
        &mut pending_access_unit,
        pending_access_unit_timestamp.take(),
        &mut pacer,
        last_timestamp_ms,
        &mut frame_count,
        frames_since_report,
        last_report,
    )
    .await
    {
        return ts;
    }

    if !loop_playback {
        return ReaderPublishResult::Completed(timestamp_offset);
    }

    let new_offset = timestamp_offset.saturating_add(frame_count.saturating_mul(frame_duration_ms));
    if reader.reset().await.is_err() {
        return ReaderPublishResult::Completed(timestamp_offset);
    }

    ReaderPublishResult::Completed(new_offset)
}

fn send_param_set_frame(
    sender: &FrameDataSender,
    data: &[u8],
    timestamp_offset: u32,
    frame_count: u32,
    frame_duration_ms: u32,
    frame_type: &'static str,
) {
    let timestamp = timestamp_offset.saturating_add(frame_count.saturating_mul(frame_duration_ms));
    let frame = FrameData::Video {
        timestamp,
        data: BytesMut::from(data),
    };
    let _ = sender.send(frame);
    if crate::stream_frame_debug_logging_enabled() {
        log::debug!(
            "mock_publisher: {} frame_count={} timestamp={} ({}ms)",
            frame_type,
            frame_count,
            timestamp,
            timestamp
        );
    }
}

fn log_vcl_frame(frame_count: u32, timestamp: u32, first_slice: bool, unit_type: NalUnitType) {
    if crate::stream_frame_debug_logging_enabled() {
        let frame_type = if matches!(unit_type, NalUnitType::IdrSlice) {
            "IDR"
        } else {
            "NonIDR"
        };
        log::debug!(
            "mock_publisher: {} frame_count={} timestamp={} first_slice={}",
            frame_type,
            frame_count,
            timestamp,
            first_slice
        );
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_pending_access_unit(
    sender: &FrameDataSender,
    pending_access_unit: &mut BytesMut,
    timestamp: Option<u32>,
    pacer: &mut Option<Interval>,
    last_timestamp_ms: &AtomicU32,
    frame_count: &mut u32,
    frames_since_report: &mut u64,
    last_report: &mut Instant,
) -> Option<ReaderPublishResult> {
    let ts = timestamp?;
    if !send_access_unit(
        sender,
        std::mem::take(pending_access_unit),
        ts,
        pacer,
        last_timestamp_ms,
        *frame_count,
        frames_since_report,
        last_report,
    )
    .await
    {
        return Some(ReaderPublishResult::ChannelClosed);
    }
    *frame_count = frame_count.saturating_add(1);
    None
}

#[allow(clippy::too_many_arguments)]
async fn send_access_unit(
    sender: &FrameDataSender,
    data: BytesMut,
    timestamp: u32,
    pacer: &mut Option<Interval>,
    last_timestamp_ms: &AtomicU32,
    frame_count: u32,
    frames_since_report: &mut u64,
    last_report: &mut Instant,
) -> bool {
    if data.is_empty() {
        return true;
    }
    let access_unit_size = data.len();

    if let Some(interval) = pacer.as_mut() {
        interval.tick().await;
    }

    if sender.send(FrameData::Video { timestamp, data }).is_err() {
        return false;
    }

    last_timestamp_ms.store(timestamp, Ordering::Relaxed);
    *frames_since_report += 1;

    if crate::stream_frame_debug_logging_enabled() {
        log::debug!(
            "mock_publisher: access_unit frame_count={} timestamp={} bytes={}",
            frame_count,
            timestamp,
            access_unit_size
        );
    }

    if *frames_since_report >= 25 {
        let elapsed = last_report.elapsed();
        if elapsed < Duration::from_secs(1) {
            return true;
        }
        let fps = compute_fps(*frames_since_report, elapsed);
        log::debug!(
            "mock_publisher: sent {} frames in {:.2}s ({:.2} fps)",
            *frames_since_report,
            elapsed.as_secs_f64(),
            fps
        );
        *frames_since_report = 0;
        *last_report = Instant::now();
    }

    true
}

fn build_pacer(frame_interval: Duration) -> Option<Interval> {
    if frame_interval.is_zero() {
        return None;
    }
    let mut interval = time::interval(frame_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    Some(interval)
}

fn append_annexb_nal(out: &mut BytesMut, nal: &[u8]) {
    const START_CODE_4: &[u8] = &[0x00, 0x00, 0x00, 0x01];
    out.extend_from_slice(START_CODE_4);
    out.extend_from_slice(nal);
}

fn is_first_vcl_slice(nal: &[u8]) -> bool {
    if nal.len() < 2 {
        return true;
    }
    let nal_type = nal[0] & 0x1F;
    if !(nal_type == 1 || nal_type == 5) {
        return true;
    }

    let mut reader = RbspBitReader::new(&nal[1..]);
    match reader.read_ue() {
        Some(first_mb_in_slice) => first_mb_in_slice == 0,
        None => true,
    }
}

#[cfg(test)]
fn remove_emulation_prevention_bytes(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len());
    let mut zero_count = 0u8;
    for &byte in data {
        if zero_count >= 2 && byte == 0x03 {
            zero_count = 0;
            continue;
        }
        out.push(byte);
        if byte == 0x00 {
            zero_count = zero_count.saturating_add(1);
        } else {
            zero_count = 0;
        }
    }
    out
}

struct RbspBitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    current_byte: u8,
    bits_remaining: u8,
    zero_count: u8,
}

impl<'a> RbspBitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            current_byte: 0,
            bits_remaining: 0,
            zero_count: 0,
        }
    }

    fn next_rbsp_byte(&mut self) -> Option<u8> {
        while self.byte_pos < self.data.len() {
            let byte = self.data[self.byte_pos];
            self.byte_pos += 1;
            if self.zero_count >= 2 && byte == 0x03 {
                self.zero_count = 0;
                continue;
            }
            if byte == 0x00 {
                self.zero_count = self.zero_count.saturating_add(1);
            } else {
                self.zero_count = 0;
            }
            return Some(byte);
        }
        None
    }

    fn read_bit(&mut self) -> Option<u8> {
        if self.bits_remaining == 0 {
            self.current_byte = self.next_rbsp_byte()?;
            self.bits_remaining = 8;
        }
        let bit = (self.current_byte >> 7) & 1;
        self.current_byte <<= 1;
        self.bits_remaining -= 1;
        Some(bit)
    }

    fn read_bits(&mut self, count: usize) -> Option<u32> {
        let mut value = 0u32;
        for _ in 0..count {
            value = (value << 1) | u32::from(self.read_bit()?);
        }
        Some(value)
    }

    fn read_ue(&mut self) -> Option<u32> {
        let mut leading_zero_bits = 0usize;
        while self.read_bit()? == 0 {
            leading_zero_bits += 1;
            if leading_zero_bits > 31 {
                return None;
            }
        }

        if leading_zero_bits == 0 {
            return Some(0);
        }

        let suffix = self.read_bits(leading_zero_bits)?;
        Some(((1u32 << leading_zero_bits) - 1) + suffix)
    }
}

/// Base64 encode helper for SPS/PPS data
fn base64_encode(data: &[u8]) -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let mut buf = [0u8; 3];
        for (i, &b) in chunk.iter().enumerate() {
            buf[i] = b;
        }

        let b1 = (buf[0] >> 2) as usize;
        let b2 = (((buf[0] & 0x03) << 4) | (buf[1] >> 4)) as usize;
        let b3 = (((buf[1] & 0x0f) << 2) | (buf[2] >> 6)) as usize;
        let b4 = (buf[2] & 0x3f) as usize;

        result.push(BASE64_CHARS[b1] as char);
        result.push(BASE64_CHARS[b2] as char);

        if chunk.len() > 1 {
            result.push(BASE64_CHARS[b3] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(BASE64_CHARS[b4] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn compute_fps(frames: u64, elapsed: Duration) -> f64 {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 {
        0.0
    } else {
        frames as f64 / seconds
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MockVideoPublisher, compute_fps, generate_sdp_from_sps_pps, is_first_vcl_slice,
        remove_emulation_prevention_bytes,
    };
    use crate::codec::h264_file_reader::H264FileReader;
    use crate::streamhub::define::{DataSender, FrameData, SubscribeType, TStreamHandler};
    use portable_atomic::Ordering;
    use std::time::Duration;
    use tokio::sync::mpsc;

    fn annexb_nal(nal: &[u8]) -> Vec<u8> {
        let mut out = vec![0x00, 0x00, 0x00, 0x01];
        out.extend_from_slice(nal);
        out
    }

    fn temp_h264_path(label: &str) -> String {
        format!("/tmp/mock_publisher_{}_{}.h264", label, std::process::id())
    }

    #[test]
    fn test_generate_sdp_includes_controls() {
        let sps = [0x67, 0x42, 0xE0, 0x1E, 0x89];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let sdp = generate_sdp_from_sps_pps(&sps, &pps);

        assert!(sdp.contains("a=control:*"));
        assert!(sdp.contains("a=control:trackID=0"));
        assert!(sdp.contains("a=rtpmap:96 H264/90000"));
    }

    #[test]
    fn test_generate_sdp_profile_level_id_from_sps() {
        let sps = [0x67, 0x42, 0xE0, 0x1E];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let sdp = generate_sdp_from_sps_pps(&sps, &pps);

        assert!(sdp.contains("profile-level-id=42e01e"));
    }

    #[test]
    fn test_generate_sdp_has_no_indented_lines() {
        let sps = [0x67, 0x42, 0xE0, 0x1E];
        let pps = [0x68, 0xCE, 0x06, 0xE2];
        let sdp = generate_sdp_from_sps_pps(&sps, &pps);

        for line in sdp.lines() {
            assert!(!line.starts_with(' '));
            assert!(!line.starts_with('\t'));
        }
    }

    #[tokio::test]
    async fn test_send_prior_data_uses_last_timestamp() {
        let path = format!("/tmp/mock_publisher_test_{}.h264", std::process::id());
        // Minimal Annex-B stream with SPS + PPS so the reader can initialize.
        let bytes: &[u8] = &[
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00, //
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, 0x00, //
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21, 0xA0, 0x00, //
        ];
        std::fs::write(&path, bytes).expect("write test h264");

        let publisher = MockVideoPublisher::new("stream1".to_string(), &path, 25, true)
            .await
            .expect("publisher new");
        publisher.last_timestamp_ms.store(12_345, Ordering::Relaxed);

        let (tx, mut rx) = mpsc::unbounded_channel();
        let sender = DataSender::Frame { sender: tx };
        publisher
            .send_prior_data(sender, SubscribeType::RtspPull)
            .await
            .expect("send_prior_data");

        // MediaInfo first
        let _ = rx.recv().await.expect("media_info");
        let sps = rx.recv().await.expect("sps");
        let pps = rx.recv().await.expect("pps");
        let idr = rx.recv().await.expect("idr");

        match sps {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 12_345);
                assert!(!data.is_empty());
            }
            other => panic!("expected SPS video frame, got {other:?}"),
        }

        match pps {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 12_345);
                assert!(!data.is_empty());
            }
            other => panic!("expected PPS video frame, got {other:?}"),
        }

        match idr {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 12_345);
                assert!(!data.is_empty());
                assert_eq!(data[0] & 0x1f, 5); // IDR NAL type
            }
            other => panic!("expected IDR video frame, got {other:?}"),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_last_timestamp_handle_only_updates_on_vcl() {
        let path = format!(
            "/tmp/mock_publisher_test_last_ts_vcl_{}.h264",
            std::process::id()
        );
        // SPS + PPS only (no VCL). last_timestamp_handle() must remain at the default value.
        let bytes: &[u8] = &[
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00, //
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, 0x00, //
        ];
        std::fs::write(&path, bytes).expect("write test h264");

        let publisher = MockVideoPublisher::new("stream1".to_string(), &path, 25, false)
            .await
            .expect("publisher new");

        let (tx, mut rx) = mpsc::unbounded_channel();
        let _handle = publisher.start_publishing(tx);

        let mut saw_sps = false;
        let mut saw_pps = false;
        for _ in 0..4 {
            let item = tokio::time::timeout(Duration::from_secs(1), rx.recv())
                .await
                .expect("timeout waiting for video frame");
            let Some(item) = item else { break };
            if let FrameData::Video { data, .. } = item {
                let nal_type = data.first().copied().unwrap_or_default() & 0x1f;
                if nal_type == 7 {
                    saw_sps = true;
                } else if nal_type == 8 {
                    saw_pps = true;
                }
            }
            if saw_sps && saw_pps {
                break;
            }
        }

        assert!(saw_sps, "expected to see SPS");
        assert!(saw_pps, "expected to see PPS");
        assert_eq!(
            publisher.last_timestamp_handle().load(Ordering::Relaxed),
            0,
            "SPS/PPS emission must not update last_timestamp_handle()"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_start_publishing_sps_pps_timestamp_is_monotonic() {
        let path = format!(
            "/tmp/mock_publisher_test_monotonic_{}.h264",
            std::process::id()
        );
        // SPS/PPS followed by frames, then SPS/PPS again to ensure timestamps never regress.
        let bytes: &[u8] = &[
            // SPS
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00, //
            // PPS
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, 0x00, //
            // IDR
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21, 0xA0, 0x00, //
            // Non-IDR
            0x00, 0x00, 0x00, 0x01, 0x41, 0x9A, 0x22, 0x11, 0x00, //
            // SPS again
            0x00, 0x00, 0x00, 0x01, 0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00, //
            // PPS again
            0x00, 0x00, 0x00, 0x01, 0x68, 0xCE, 0x06, 0xE2, 0x00, //
            // IDR again
            0x00, 0x00, 0x00, 0x01, 0x65, 0x88, 0x84, 0x21, 0xA0, 0x00, //
        ];
        std::fs::write(&path, bytes).expect("write test h264");

        let publisher = MockVideoPublisher::new("stream1".to_string(), &path, 25, false)
            .await
            .expect("publisher new");
        let (tx, mut rx) = mpsc::unbounded_channel();
        let _handle = publisher.start_publishing(tx);

        let mut timestamps: Vec<u32> = Vec::new();
        loop {
            let item = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv())
                .await
                .expect("timeout waiting for video frame");
            let Some(item) = item else { break };

            match item {
                FrameData::Video { timestamp, .. } => timestamps.push(timestamp),
                _ => {}
            }
        }

        assert!(!timestamps.is_empty());
        for window in timestamps.windows(2) {
            assert!(
                window[0] <= window[1],
                "timestamps regressed: {timestamps:?}"
            );
        }

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_mock_video_publisher_new_no_idr_returns_without_timeout() {
        let path = temp_h264_path("no_idr");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&annexb_nal(&[0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00])); // SPS
        bytes.extend_from_slice(&annexb_nal(&[0x68, 0xCE, 0x06, 0xE2, 0x00])); // PPS
        for _ in 0..512 {
            bytes.extend_from_slice(&annexb_nal(&[0x41, 0x9A, 0x22, 0x11, 0x00])); // non-IDR
        }
        std::fs::write(&path, &bytes).expect("write test h264");

        let publisher = tokio::time::timeout(
            Duration::from_secs(2),
            MockVideoPublisher::new("stream1".to_string(), &path, 25, true),
        )
        .await
        .expect("publisher new timeout")
        .expect("publisher new error");

        assert!(
            publisher.bootstrap_idr().is_none(),
            "expected no bootstrap IDR for stream without IDR NAL units"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_mock_video_publisher_bootstrap_idr_found_within_scan_limit() {
        let path = temp_h264_path("idr_within_limit");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&annexb_nal(&[0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00])); // SPS
        bytes.extend_from_slice(&annexb_nal(&[0x68, 0xCE, 0x06, 0xE2, 0x00])); // PPS
        bytes.extend_from_slice(&annexb_nal(&[0x41, 0x9A, 0x22, 0x11, 0x00])); // non-IDR
        bytes.extend_from_slice(&annexb_nal(&[0x65, 0x88, 0x84, 0x21, 0xA0, 0x00])); // IDR
        std::fs::write(&path, &bytes).expect("write test h264");

        let publisher = MockVideoPublisher::new("stream1".to_string(), &path, 25, true)
            .await
            .expect("publisher new");
        assert!(
            publisher.bootstrap_idr().is_some(),
            "expected bootstrap IDR when IDR appears early in stream"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_mock_video_publisher_builds_access_unit_cache_for_small_file() {
        let path = temp_h264_path("cache_small");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&annexb_nal(&[0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00])); // SPS
        bytes.extend_from_slice(&annexb_nal(&[0x68, 0xCE, 0x06, 0xE2, 0x00])); // PPS
        bytes.extend_from_slice(&annexb_nal(&[0x65, 0x88, 0x84, 0x21, 0xA0, 0x00])); // IDR
        bytes.extend_from_slice(&annexb_nal(&[0x41, 0x9A, 0x22, 0x11, 0x00])); // non-IDR
        std::fs::write(&path, &bytes).expect("write test h264");

        let publisher = MockVideoPublisher::new("stream1".to_string(), &path, 25, true)
            .await
            .expect("publisher new");

        assert!(
            publisher.access_units_cache.is_some(),
            "expected access-unit cache for small source file"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn test_mock_video_publisher_bootstrap_idr_skipped_when_beyond_limit() {
        let path = temp_h264_path("idr_beyond_limit");
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&annexb_nal(&[0x67, 0x42, 0xE0, 0x1E, 0x89, 0x00])); // SPS
        bytes.extend_from_slice(&annexb_nal(&[0x68, 0xCE, 0x06, 0xE2, 0x00])); // PPS
        for _ in 0..512 {
            bytes.extend_from_slice(&annexb_nal(&[0x41, 0x9A, 0x22, 0x11, 0x00])); // non-IDR
        }
        bytes.extend_from_slice(&annexb_nal(&[0x65, 0x88, 0x84, 0x21, 0xA0, 0x00])); // IDR (late)
        std::fs::write(&path, &bytes).expect("write test h264");

        let mut reader = H264FileReader::new(&path, 25).await.expect("reader new");
        let _ = reader.extract_sps_pps().await.expect("extract_sps_pps");
        let bootstrap_idr = MockVideoPublisher::extract_first_idr_with_limit(&mut reader, 128)
            .await
            .expect("extract_first_idr_with_limit");
        assert!(
            bootstrap_idr.is_none(),
            "expected no bootstrap IDR when scan limit is too small"
        );

        let publisher = MockVideoPublisher::new("stream1".to_string(), &path, 25, true)
            .await
            .expect("publisher new");
        assert!(
            publisher.bootstrap_idr().is_some(),
            "sanity check: default scan limit should still find IDR in this fixture"
        );

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_compute_fps_basic() {
        let fps = compute_fps(30, Duration::from_secs(2));
        assert!((fps - 15.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_fps_zero_elapsed() {
        let fps = compute_fps(10, Duration::from_secs(0));
        assert_eq!(fps, 0.0);
    }

    #[test]
    fn test_is_first_vcl_slice_detects_first_mb_zero() {
        // nal_type=1 (non-IDR), slice payload starts with Exp-Golomb `1` => first_mb_in_slice=0
        assert!(is_first_vcl_slice(&[0x41, 0x80]));
    }

    #[test]
    fn test_is_first_vcl_slice_detects_non_zero_first_mb() {
        // nal_type=1 (non-IDR), slice payload starts with Exp-Golomb `010` => first_mb_in_slice=1
        assert!(!is_first_vcl_slice(&[0x41, 0x40]));
    }

    #[test]
    fn test_remove_emulation_prevention_bytes() {
        let rbsp = remove_emulation_prevention_bytes(&[0x00, 0x00, 0x03, 0x01, 0x22]);
        assert_eq!(rbsp, vec![0x00, 0x00, 0x01, 0x22]);
    }
}
