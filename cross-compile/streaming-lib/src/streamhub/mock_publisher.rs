use crate::codec::h264_file_reader::{H264FileError, H264FileReader, NalUnitType};
use crate::streamhub::define::{
    DataSender, FrameData, FrameDataSender, Information, InformationSender, MediaInfo,
    SubscribeType, TStreamHandler, VideoCodecType,
};
use crate::streamhub::{StatisticsStream, StreamHubError};
use async_trait::async_trait;
use bytes::BytesMut;
use std::sync::Arc;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::Mutex;

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
    is_running: Arc<Mutex<bool>>,
    loop_playback: bool,
}

impl MockVideoPublisher {
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
        let mut reader = H264FileReader::new(file_path, frame_rate)?;
        let (sps, pps) = reader.extract_sps_pps()?;

        Ok(Self {
            stream_name,
            reader: Arc::new(Mutex::new(reader)),
            sps,
            pps,
            is_running: Arc::new(Mutex::new(false)),
            loop_playback,
        })
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
        let is_running = Arc::clone(&self.is_running);
        let _stream_name = self.stream_name.clone();
        let _sps = self.sps.clone();
        let _pps = self.pps.clone();
        let loop_playback = self.loop_playback;

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

                // Read and send NAL units
                let mut reader = reader.lock().await;
                let mut frame_count: u32 = 0;
                let frame_duration_ms = reader.frame_duration_ms();
                let loop_start = Instant::now();

                while let Ok(Some(nal)) = reader.read_next_nal() {
                    match nal.unit_type {
                        NalUnitType::SequenceParameterSet => {
                            let data = BytesMut::from(nal.data.as_slice());
                            let frame = FrameData::Video {
                                timestamp: timestamp_offset,
                                data,
                            };
                            let _ = sender.send(frame);
                        }
                        NalUnitType::PictureParameterSet => {
                            let data = BytesMut::from(nal.data.as_slice());
                            let frame = FrameData::Video {
                                timestamp: timestamp_offset,
                                data,
                            };
                            let _ = sender.send(frame);
                        }
                        NalUnitType::IdrSlice | NalUnitType::NonIdrSlice => {
                            let timestamp = timestamp_offset
                                .saturating_add(frame_count.saturating_mul(frame_duration_ms));
                            let data = BytesMut::from(nal.data.as_slice());

                            let frame = FrameData::Video { timestamp, data };

                            if sender.send(frame).is_err() {
                                return;
                            }

                            frame_count = frame_count.saturating_add(1);
                            frames_since_report += 1;

                            let elapsed = last_report.elapsed();
                            if elapsed >= Duration::from_secs(1) {
                                let fps = compute_fps(frames_since_report, elapsed);
                                log::info!(
                                    "mock_publisher: sent {} frames in {:.2}s ({:.2} fps)",
                                    frames_since_report,
                                    elapsed.as_secs_f64(),
                                    fps
                                );
                                frames_since_report = 0;
                                last_report = Instant::now();
                            }

                            // Frame rate control: align to target schedule to avoid drift.
                            let target_elapsed_ms =
                                frame_count.saturating_mul(frame_duration_ms) as u64;
                            let target_elapsed = Duration::from_millis(target_elapsed_ms);
                            let elapsed = loop_start.elapsed();
                            if let Some(remaining) = target_elapsed.checked_sub(elapsed) {
                                tokio::time::sleep(remaining).await;
                            }
                        }
                        _ => {}
                    }
                }

                if !loop_playback {
                    // Stop playback if looping is disabled
                    break;
                }

                // Update timestamp offset for next loop iteration
                // Total time for this loop = frame_count * frame_duration_ms
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

    /// Get SPS data
    pub fn sps(&self) -> &[u8] {
        &self.sps
    }

    /// Get PPS data
    pub fn pps(&self) -> &[u8] {
        &self.pps
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
                timestamp: 0,
                data: sps_data,
            });

            // Send PPS as video frame
            let pps_data = BytesMut::from(self.pps.as_slice());
            let _ = frame_sender.send(FrameData::Video {
                timestamp: 0,
                data: pps_data,
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
    use super::{compute_fps, generate_sdp_from_sps_pps};
    use std::time::Duration;

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
    async fn test_mock_publisher_creation() {
        // This test verifies the publisher structure can be created
        // In a real test, we would need a valid H264 file
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
}
