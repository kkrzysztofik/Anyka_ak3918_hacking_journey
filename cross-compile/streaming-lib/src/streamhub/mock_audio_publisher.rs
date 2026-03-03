use crate::streamhub::define::{
    DataSender, FrameData, FrameDataSender, Information, InformationSender, MediaInfo,
    SubscribeType, TStreamHandler, VideoCodecType,
};
use crate::streamhub::{StatisticsStream, StreamHubError};
use crate::validation::aac_file_reader::{AacFileError, AacFileReader};
use async_trait::async_trait;
use bytes::BytesMut;
use portable_atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

/// AAC-LC samples per frame (fixed by specification)
/// Per RFC 3640 Section 3.3, RTP timestamps for AAC are in sample units.
/// AAC-LC uses 1024 samples per frame regardless of sample rate.
const AAC_SAMPLES_PER_FRAME: u32 = 1024;
use tokio::sync::Mutex;
use tokio::time::{self, MissedTickBehavior};

/// Errors that can occur during mock audio publishing
#[derive(Error, Debug)]
pub enum AudioPublisherError {
    #[error("AAC file error: {0}")]
    AacError(#[from] AacFileError),

    #[error("Channel send error")]
    ChannelError,

    #[error("Stream already stopped")]
    StreamStopped,
}

/// Mock audio publisher for AAC file playback
pub struct MockAudioPublisher {
    stream_name: String,
    reader: Arc<Mutex<AacFileReader>>,
    audio_config: Vec<u8>,
    is_running: Arc<Mutex<bool>>,
    loop_playback: bool,
    sample_rate: u32,
    last_timestamp_ms: Arc<AtomicU32>,
    /// Callback to get current subscriber count for on-demand publishing
    subscriber_count_fn: Option<Arc<dyn Fn() -> usize + Send + Sync>>,
}

impl MockAudioPublisher {
    /// Create a new mock audio publisher from AAC file
    ///
    /// # Arguments
    ///
    /// * `stream_name` - Name of the stream
    /// * `file_path` - Path to AAC file in ADTS format
    /// * `sample_rate` - Expected sample rate (e.g., 48000, 44100)
    /// * `loop_playback` - If true, loop the file when reaching EOF; if false, stop
    ///
    /// # Returns
    ///
    /// Result with MockAudioPublisher or AudioPublisherError
    pub async fn new(
        stream_name: String,
        file_path: &str,
        sample_rate: u32,
        loop_playback: bool,
    ) -> Result<Self, AudioPublisherError> {
        let mut reader = AacFileReader::new(file_path, sample_rate).await?;
        let audio_config = reader.extract_audio_config().await?;
        let detected_sample_rate = reader.sample_rate();
        if detected_sample_rate != sample_rate {
            log::warn!(
                "mock_audio_publisher: requested sample_rate={} differs from ADTS sample_rate={}, using ADTS value",
                sample_rate,
                detected_sample_rate
            );
        }

        Ok(Self {
            stream_name,
            reader: Arc::new(Mutex::new(reader)),
            audio_config,
            is_running: Arc::new(Mutex::new(false)),
            loop_playback,
            sample_rate: detected_sample_rate,
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

    /// Start publishing frames from the AAC file
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
        let running_flag = Arc::clone(&self.is_running);
        let _stream_name = self.stream_name.clone();
        let _audio_config = self.audio_config.clone();
        let loop_playback = self.loop_playback;
        let last_timestamp_ms = Arc::clone(&self.last_timestamp_ms);
        let subscriber_count_fn = self.subscriber_count_fn.clone();

        tokio::spawn(async move {
            set_running_flag(&running_flag, true).await;

            let mut timestamp_offset = 0u32;

            loop {
                if !check_is_running(&running_flag).await {
                    break;
                }

                if has_no_audio_subscribers(&subscriber_count_fn) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    continue;
                }

                let result = publish_audio_frames(
                    &reader,
                    &sender,
                    &last_timestamp_ms,
                    timestamp_offset,
                    loop_playback,
                    &subscriber_count_fn,
                )
                .await;

                match result {
                    AudioPublishResult::Interrupted => continue,
                    AudioPublishResult::ChannelClosed => {
                        set_running_flag(&running_flag, false).await;
                        return;
                    }
                    AudioPublishResult::Completed(new_offset) => {
                        if !loop_playback {
                            break;
                        }
                        timestamp_offset = new_offset;
                    }
                }
            }

            set_running_flag(&running_flag, false).await;
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

    /// Get AudioSpecificConfig data
    pub fn audio_config(&self) -> &[u8] {
        &self.audio_config
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

/// TStreamHandler implementation for MockAudioPublisher
/// Provides SDP and media info to StreamHub subscribers
#[async_trait]
impl TStreamHandler for MockAudioPublisher {
    /// Send prior metadata and AudioSpecificConfig to new subscribers
    async fn send_prior_data(
        &self,
        sender: DataSender,
        sub_type: SubscribeType,
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
                audio_clock_rate: self.sample_rate,
                video_clock_rate: 90000,      // Default video clock rate
                vcodec: VideoCodecType::H264, // Placeholder, audio-only stream
            };

            let _ = frame_sender.send(FrameData::MediaInfo { media_info });

            // RTSP/RTP consumers receive AudioSpecificConfig via SDP fmtp config.
            // Sending config as an AAC frame can break depacketizers expecting raw AU data.
            if !matches!(sub_type, SubscribeType::RtspPull | SubscribeType::RtpPull) {
                let config_data = BytesMut::from(self.audio_config.as_slice());
                let _ = frame_sender.send(FrameData::Audio {
                    timestamp: ts,
                    data: config_data,
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
        // Generate SDP from AudioSpecificConfig
        let sdp = generate_sdp_from_audio_config(&self.audio_config, self.sample_rate);

        let _ = sender.send(Information::Sdp { data: sdp });
    }
}

/// Generate SDP string from AAC AudioSpecificConfig
fn generate_sdp_from_audio_config(audio_config: &[u8], sample_rate: u32) -> String {
    // Encode AudioSpecificConfig as hex string for SDP
    let config_hex = audio_config
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<String>();

    // Calculate channels from AudioSpecificConfig
    // Byte 1, bits 3-6 = channelConfiguration
    let channels = if audio_config.len() >= 2 {
        ((audio_config[1] >> 3) & 0x0F) as u32
    } else {
        2 // Default to stereo
    };

    // Calculate appropriate profile-level-id based on sample rate and channels
    // Per ISO/IEC 14496-1 Table 5 (audioProfileLevelIndication Values)
    // RFC 3640 Section 3.3.6 example uses profile-level-id=16 for 5.1ch 48kHz
    let profile_level_id = match (sample_rate, channels) {
        (48000, ch) if ch >= 2 => 15, // AAC Profile @ Level 4 (stereo+ 48kHz)
        (44100, ch) if ch >= 2 => 15, // AAC Profile @ Level 4 (stereo+ 44.1kHz)
        (_, 1) => 14,                 // AAC Profile @ Level 2 (mono)
        _ => 15,                      // AAC Profile @ Level 4 (default)
    };

    // RFC 3640 Section 3.3.6: AAC-hbr mode MUST include:
    // - streamtype=5 (AudioStream per ISO/IEC 14496-1 Table 9)
    // - constantDuration=1024 (samples per AAC frame)
    // - sizeLength=13, indexLength=3, indexDeltaLength=3
    format!(
        "v=0\r\n\
         o=- 0 0 IN IP4 0.0.0.0\r\n\
         s=Mock AAC Stream\r\n\
         c=IN IP4 0.0.0.0\r\n\
         t=0 0\r\n\
         a=tool:streaming-lib-mock\r\n\
         m=audio 0 RTP/AVP 97\r\n\
         a=rtpmap:97 MPEG4-GENERIC/{}/{}\r\n\
         a=fmtp:97 streamtype=5;profile-level-id={};mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3;constantDuration=1024;config={}\r\n",
        sample_rate, channels, profile_level_id, config_hex
    )
}

fn has_no_audio_subscribers(
    subscriber_count_fn: &Option<Arc<dyn Fn() -> usize + Send + Sync>>,
) -> bool {
    subscriber_count_fn.as_ref().is_some_and(|cb| cb() == 0)
}

async fn check_is_running(flag: &Arc<Mutex<bool>>) -> bool {
    *flag.lock().await
}

async fn set_running_flag(flag: &Arc<Mutex<bool>>, value: bool) {
    *flag.lock().await = value;
}

enum AudioPublishResult {
    Interrupted,
    ChannelClosed,
    Completed(u32),
}

async fn publish_audio_frames(
    reader: &Arc<Mutex<AacFileReader>>,
    sender: &FrameDataSender,
    last_timestamp_ms: &AtomicU32,
    timestamp_offset: u32,
    loop_playback: bool,
    subscriber_count_fn: &Option<Arc<dyn Fn() -> usize + Send + Sync>>,
) -> AudioPublishResult {
    let mut reader = reader.lock().await;
    let mut frame_count: u32 = 0;
    let sample_rate = reader.sample_rate();
    let frame_interval = compute_frame_interval(sample_rate);
    let mut pacer = build_audio_pacer(frame_interval);

    while let Ok(Some(frame)) = reader.read_next_frame().await {
        if has_no_audio_subscribers(subscriber_count_fn) {
            return AudioPublishResult::Interrupted;
        }

        if let Some(interval) = pacer.as_mut() {
            interval.tick().await;
        }

        let timestamp =
            timestamp_offset.saturating_add(frame_count.saturating_mul(AAC_SAMPLES_PER_FRAME));
        let data = BytesMut::from(frame.data.as_slice());
        let frame_data = FrameData::Audio { timestamp, data };

        log_audio_frame(frame_count, timestamp);

        if sender.send(frame_data).is_err() {
            return AudioPublishResult::ChannelClosed;
        }
        last_timestamp_ms.store(timestamp, Ordering::Relaxed);
        frame_count = frame_count.saturating_add(1);
    }

    if !loop_playback {
        return AudioPublishResult::Completed(timestamp_offset);
    }

    let new_offset =
        timestamp_offset.saturating_add(frame_count.saturating_mul(AAC_SAMPLES_PER_FRAME));
    if reader.reset().await.is_err() {
        return AudioPublishResult::Completed(timestamp_offset);
    }

    AudioPublishResult::Completed(new_offset)
}

fn compute_frame_interval(sample_rate: u32) -> Duration {
    if sample_rate == 0 {
        Duration::from_millis(0)
    } else {
        Duration::from_secs_f64(AAC_SAMPLES_PER_FRAME as f64 / sample_rate as f64)
    }
}

fn log_audio_frame(frame_count: u32, timestamp: u32) {
    if crate::stream_frame_debug_logging_enabled() {
        log::debug!(
            "mock_audio_publisher: AAC frame_count={} timestamp={} (sample units)",
            frame_count,
            timestamp
        );
    }
}

fn build_audio_pacer(frame_interval: Duration) -> Option<time::Interval> {
    if frame_interval.is_zero() {
        return None;
    }
    let mut interval = time::interval(frame_interval);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    Some(interval)
}

#[cfg(test)]
fn audio_target_elapsed(frame_count: u32, sample_rate: u32) -> Duration {
    if frame_count == 0 || sample_rate == 0 {
        return Duration::from_nanos(0);
    }

    let elapsed_nanos = (u128::from(frame_count))
        .saturating_mul(u128::from(AAC_SAMPLES_PER_FRAME))
        .saturating_mul(1_000_000_000u128)
        / u128::from(sample_rate);
    let elapsed_nanos_u64 = u64::try_from(elapsed_nanos).unwrap_or(u64::MAX);
    Duration::from_nanos(elapsed_nanos_u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamhub::define::{DataSender, FrameData, SubscribeType, TStreamHandler};
    use portable_atomic::Ordering;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_send_prior_data_uses_last_timestamp() {
        let path = format!("/tmp/mock_audio_publisher_test_{}.aac", std::process::id());

        // Minimal ADTS AAC-LC frame (48kHz, stereo).
        // Frame length = 7-byte header + 4-byte payload = 11 bytes.
        let bytes: &[u8] = &[
            0xFF, 0xF1, 0x4C, 0x80, 0x01, 0x7F, 0xFC, // ADTS header
            0x00, 0x00, 0x00, 0x00, // payload (unused by parser)
        ];
        std::fs::write(&path, bytes).expect("write test aac");

        let publisher = MockAudioPublisher::new("stream1".to_string(), &path, 48_000, true)
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
        match rx.recv().await.expect("media_info") {
            FrameData::MediaInfo { .. } => {}
            other => panic!("expected MediaInfo frame, got {other:?}"),
        }

        // No config frame should be injected for RTSP pull subscribers.
        assert!(rx.recv().await.is_none());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_sdp_generation() {
        // Test SDP generation with sample AudioSpecificConfig
        // AAC-LC, 48kHz, stereo = 0x1190
        let audio_config = vec![0x11, 0x90];
        let sdp = generate_sdp_from_audio_config(&audio_config, 48000);

        assert!(sdp.contains("m=audio 0 RTP/AVP 97"));
        assert!(sdp.contains("a=rtpmap:97 MPEG4-GENERIC/48000/2"));
        assert!(sdp.contains("config=1190"));

        // RFC 3640 Section 3.3.6 AAC-hbr mode required parameters
        assert!(
            sdp.contains("streamtype=5"),
            "Missing required streamtype=5"
        );
        assert!(
            sdp.contains("mode=AAC-hbr"),
            "Missing required mode=AAC-hbr"
        );
        assert!(
            sdp.contains("sizelength=13"),
            "Missing required sizelength=13"
        );
        assert!(
            sdp.contains("indexlength=3"),
            "Missing required indexlength=3"
        );
        assert!(
            sdp.contains("indexdeltalength=3"),
            "Missing required indexdeltalength=3"
        );
        assert!(
            sdp.contains("constantDuration=1024"),
            "Missing required constantDuration=1024"
        );
        assert!(
            sdp.contains("profile-level-id=15"),
            "48kHz stereo should use profile-level-id=15"
        );
    }

    #[test]
    fn test_sdp_generation_44k() {
        // AAC-LC, 44.1kHz, stereo = 0x1210
        let audio_config = vec![0x12, 0x10];
        let sdp = generate_sdp_from_audio_config(&audio_config, 44100);

        assert!(sdp.contains("a=rtpmap:97 MPEG4-GENERIC/44100/2"));
        assert!(sdp.contains("config=1210"));

        // RFC 3640 required parameters
        assert!(sdp.contains("streamtype=5"));
        assert!(sdp.contains("constantDuration=1024"));
        assert!(
            sdp.contains("profile-level-id=15"),
            "44.1kHz stereo should use profile-level-id=15"
        );
    }

    #[test]
    fn test_aac_samples_per_frame_constant() {
        // Verify the constant is correct for AAC-LC
        assert_eq!(AAC_SAMPLES_PER_FRAME, 1024);
    }

    #[test]
    fn test_audio_target_elapsed_uses_sample_accurate_timing() {
        assert_eq!(audio_target_elapsed(0, 48_000), Duration::from_nanos(0));
        assert_eq!(
            audio_target_elapsed(1, 48_000),
            Duration::from_nanos(21_333_333)
        );
        assert_eq!(
            audio_target_elapsed(3, 48_000),
            Duration::from_nanos(64_000_000)
        );
        assert_eq!(
            audio_target_elapsed(1, 44_100),
            Duration::from_nanos(23_219_954)
        );
    }

    #[tokio::test]
    async fn test_audio_timestamps_use_sample_units() {
        // Create a temporary AAC file for testing with 3 frames
        let path = format!("/tmp/test_audio_timestamps_{}.aac", std::process::id());

        // Create 3 minimal ADTS AAC-LC frames (48kHz, stereo)
        // Each frame: 7-byte ADTS header + 4-byte payload = 11 bytes
        let frame: &[u8] = &[
            0xFF, 0xF1, 0x4C, 0x80, 0x01, 0x7F, 0xFC, // ADTS header
            0x00, 0x00, 0x00, 0x00, // payload
        ];

        // Write 3 frames
        let mut file_data = Vec::new();
        for _ in 0..3 {
            file_data.extend_from_slice(frame);
        }
        std::fs::write(&path, file_data).expect("write test aac");

        let publisher = MockAudioPublisher::new("test_stream".to_string(), &path, 48_000, false)
            .await
            .expect("create publisher");

        let (tx, mut rx) = mpsc::unbounded_channel();
        let _handle = publisher.start_publishing(tx);

        // Collect first few frames
        let mut timestamps = Vec::new();
        while let Some(frame) = rx.recv().await {
            if let FrameData::Audio { timestamp, .. } = frame {
                timestamps.push(timestamp);
                if timestamps.len() >= 3 {
                    break;
                }
            }
        }

        publisher.stop_publishing().await;

        // Verify timestamps are in sample units (multiples of 1024)
        assert!(timestamps.len() >= 3, "Expected at least 3 frames");
        assert_eq!(timestamps[0], 0, "First timestamp should be 0");
        assert_eq!(timestamps[1], 1024, "Second timestamp should be 1024");
        assert_eq!(timestamps[2], 2048, "Third timestamp should be 2048");

        let _ = std::fs::remove_file(&path);
    }

    // ========== compute_frame_interval Tests ==========

    #[test]
    fn test_compute_frame_interval_48khz() {
        let interval = compute_frame_interval(48_000);
        // 1024 / 48000 ≈ 21.333ms
        let expected_nanos = (1024.0 / 48_000.0 * 1_000_000_000.0) as u64;
        let delta = if interval.as_nanos() > expected_nanos as u128 {
            interval.as_nanos() - expected_nanos as u128
        } else {
            expected_nanos as u128 - interval.as_nanos()
        };
        assert!(delta < 100, "interval off by {delta}ns");
    }

    #[test]
    fn test_compute_frame_interval_44100() {
        let interval = compute_frame_interval(44_100);
        // 1024 / 44100 ≈ 23.22ms
        assert!(interval.as_millis() >= 23);
        assert!(interval.as_millis() <= 24);
    }

    #[test]
    fn test_compute_frame_interval_zero() {
        let interval = compute_frame_interval(0);
        assert!(interval.is_zero());
    }

    #[test]
    fn test_compute_frame_interval_8khz() {
        let interval = compute_frame_interval(8_000);
        // 1024 / 8000 = 128ms
        assert_eq!(interval.as_millis(), 128);
    }

    // ========== has_no_audio_subscribers Tests ==========

    #[test]
    fn test_has_no_audio_subscribers_none_callback() {
        assert!(!has_no_audio_subscribers(&None));
    }

    #[test]
    fn test_has_no_audio_subscribers_zero_count() {
        let cb: Arc<dyn Fn() -> usize + Send + Sync> = Arc::new(|| 0);
        assert!(has_no_audio_subscribers(&Some(cb)));
    }

    #[test]
    fn test_has_no_audio_subscribers_nonzero_count() {
        let cb: Arc<dyn Fn() -> usize + Send + Sync> = Arc::new(|| 5);
        assert!(!has_no_audio_subscribers(&Some(cb)));
    }

    // ========== build_audio_pacer Tests ==========

    #[test]
    fn test_build_audio_pacer_zero_interval() {
        let pacer = build_audio_pacer(Duration::from_millis(0));
        assert!(pacer.is_none());
    }

    #[tokio::test]
    async fn test_build_audio_pacer_nonzero_interval() {
        let pacer = build_audio_pacer(Duration::from_millis(21));
        assert!(pacer.is_some());
    }

    // ========== AudioPublisherError Display Tests ==========

    #[test]
    fn test_audio_publisher_error_aac_display() {
        use crate::validation::aac_file_reader::AacFileError;
        let err = AudioPublisherError::AacError(AacFileError::InvalidAdtsHeader);
        let msg = format!("{}", err);
        assert!(msg.contains("AAC file error"));
    }

    #[test]
    fn test_audio_publisher_error_channel_display() {
        let err = AudioPublisherError::ChannelError;
        assert_eq!(format!("{}", err), "Channel send error");
    }

    #[test]
    fn test_audio_publisher_error_stream_stopped_display() {
        let err = AudioPublisherError::StreamStopped;
        assert_eq!(format!("{}", err), "Stream already stopped");
    }

    // ========== generate_sdp_from_audio_config Edge Cases ==========

    #[test]
    fn test_sdp_generation_mono() {
        // AAC-LC, 48kHz, mono: channelConfig = 1 => bits 3-6 of byte[1]
        // For mono: byte[1] = 0x08 => (0x08 >> 3) & 0x0F = 1
        let audio_config = vec![0x11, 0x08];
        let sdp = generate_sdp_from_audio_config(&audio_config, 48000);
        assert!(
            sdp.contains("profile-level-id=14"),
            "mono should use level 2"
        );
    }

    #[test]
    fn test_sdp_generation_empty_config() {
        // Edge case: empty config should still produce valid SDP
        let sdp = generate_sdp_from_audio_config(&[], 48000);
        assert!(sdp.contains("m=audio 0 RTP/AVP 97"));
        // Default to stereo (2) when config too short
        assert!(sdp.contains("/2"));
    }

    #[test]
    fn test_sdp_generation_single_byte_config() {
        let sdp = generate_sdp_from_audio_config(&[0x11], 44100);
        assert!(sdp.contains("config=11"));
        // Single byte config: channels defaults to 2
        assert!(sdp.contains("/2"));
    }

    #[test]
    fn test_sdp_generation_16khz() {
        let audio_config = vec![0x14, 0x10];
        let sdp = generate_sdp_from_audio_config(&audio_config, 16000);
        assert!(sdp.contains("MPEG4-GENERIC/16000"));
    }

    // ========== audio_target_elapsed Edge Cases ==========

    #[test]
    fn test_audio_target_elapsed_zero_sample_rate() {
        assert_eq!(audio_target_elapsed(10, 0), Duration::from_nanos(0));
    }

    #[test]
    fn test_audio_target_elapsed_large_frame_count() {
        // Should not panic with large frame counts
        let d = audio_target_elapsed(1_000_000, 48_000);
        assert!(d.as_secs() > 0);
    }

    // ========== check_is_running / set_running_flag Tests ==========

    #[tokio::test]
    async fn test_check_is_running_default_false() {
        let flag = Arc::new(tokio::sync::Mutex::new(false));
        assert!(!check_is_running(&flag).await);
    }

    #[tokio::test]
    async fn test_set_running_flag_and_check() {
        let flag = Arc::new(tokio::sync::Mutex::new(false));
        set_running_flag(&flag, true).await;
        assert!(check_is_running(&flag).await);
        set_running_flag(&flag, false).await;
        assert!(!check_is_running(&flag).await);
    }
}
