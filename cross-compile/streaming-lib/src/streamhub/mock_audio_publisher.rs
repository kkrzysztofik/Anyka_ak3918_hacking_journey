use crate::codec::aac_file_reader::{AacFileError, AacFileReader};
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
        let mut reader = AacFileReader::new(file_path, sample_rate)?;
        let audio_config = reader.extract_audio_config()?;

        Ok(Self {
            stream_name,
            reader: Arc::new(Mutex::new(reader)),
            audio_config,
            is_running: Arc::new(Mutex::new(false)),
            loop_playback,
            sample_rate,
            last_timestamp_ms: Arc::new(AtomicU32::new(0)),
        })
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
        let is_running = Arc::clone(&self.is_running);
        let _stream_name = self.stream_name.clone();
        let _audio_config = self.audio_config.clone();
        let loop_playback = self.loop_playback;
        let last_timestamp_ms = Arc::clone(&self.last_timestamp_ms);

        tokio::spawn(async move {
            {
                let mut running = is_running.lock().await;
                *running = true;
            }

            let mut timestamp_offset = 0u32;

            loop {
                let is_running_check = is_running.lock().await;
                if !*is_running_check {
                    break;
                }
                drop(is_running_check);

                // Read and send AAC frames
                let mut reader = reader.lock().await;
                let mut frame_count: u32 = 0;
                let frame_duration_ms = reader.frame_duration_ms();
                let loop_start = Instant::now();

                while let Ok(Some(frame)) = reader.read_next_frame() {
                    let timestamp = timestamp_offset
                        .saturating_add(frame_count.saturating_mul(frame_duration_ms));
                    let data = BytesMut::from(frame.data.as_slice());

                    let frame_data = FrameData::Audio { timestamp, data };

                    if sender.send(frame_data).is_err() {
                        return;
                    }
                    last_timestamp_ms.store(timestamp, Ordering::Relaxed);

                    frame_count = frame_count.saturating_add(1);

                    // Frame rate control: align to target schedule to avoid drift.
                    let target_elapsed_ms = frame_count.saturating_mul(frame_duration_ms) as u64;
                    let target_elapsed = Duration::from_millis(target_elapsed_ms);
                    let elapsed = loop_start.elapsed();
                    if let Some(remaining) = target_elapsed.checked_sub(elapsed) {
                        tokio::time::sleep(remaining).await;
                    }
                }

                if !loop_playback {
                    // Stop playback if looping is disabled
                    break;
                }

                // Update timestamp offset for next loop iteration
                let total_time_this_loop = frame_count.saturating_mul(frame_duration_ms);
                timestamp_offset = timestamp_offset.saturating_add(total_time_this_loop);

                // Reset file for next loop
                if let Err(_e) = reader.reset() {
                    break;
                }
            }

            let mut running = is_running.lock().await;
            *running = false;
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
                audio_clock_rate: self.sample_rate,
                video_clock_rate: 90000,      // Default video clock rate
                vcodec: VideoCodecType::H264, // Placeholder, audio-only stream
            };

            let _ = frame_sender.send(FrameData::MediaInfo { media_info });

            // Send AudioSpecificConfig as first audio frame
            let config_data = BytesMut::from(self.audio_config.as_slice());
            let _ = frame_sender.send(FrameData::Audio {
                timestamp: ts,
                data: config_data,
            });
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

    format!(
        "v=0\r\n\
         o=- 0 0 IN IP4 0.0.0.0\r\n\
         s=Mock AAC Stream\r\n\
         c=IN IP4 0.0.0.0\r\n\
         t=0 0\r\n\
         a=tool:streaming-lib-mock\r\n\
         m=audio 0 RTP/AVP 97\r\n\
         a=rtpmap:97 MPEG4-GENERIC/{}/{}\r\n\
         a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3;config={}\r\n",
        sample_rate, channels, config_hex
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamhub::define::{DataSender, FrameData, SubscribeType, TStreamHandler};
    use portable_atomic::Ordering;
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
        let _ = rx.recv().await.expect("media_info");
        let config = rx.recv().await.expect("config");

        match config {
            FrameData::Audio { timestamp, data } => {
                assert_eq!(timestamp, 12_345);
                assert_eq!(data.as_ref(), publisher.audio_config.as_slice());
            }
            other => panic!("expected AudioSpecificConfig audio frame, got {other:?}"),
        }

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
    }

    #[test]
    fn test_sdp_generation_44k() {
        // AAC-LC, 44.1kHz, stereo = 0x1210
        let audio_config = vec![0x12, 0x10];
        let sdp = generate_sdp_from_audio_config(&audio_config, 44100);

        assert!(sdp.contains("a=rtpmap:97 MPEG4-GENERIC/44100/2"));
        assert!(sdp.contains("config=1210"));
    }
}
