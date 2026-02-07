use crate::platform::Platform;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

/// Errors for H264 playback validation
#[derive(Error, Debug)]
pub enum PlaybackError {
    #[error("Validation mode not enabled")]
    NotEnabled,

    #[error("Platform error: {0}")]
    PlatformError(String),

    #[error("Stream not found: {0}")]
    StreamNotFound(String),
}

/// H264 playback validation mode configuration
#[derive(Clone, Debug)]
pub struct H264PlaybackConfig {
    pub file_path: String,
    pub audio_file_path: Option<String>,
    pub frame_rate: u32,
    pub audio_sample_rate: u32,
    pub loop_playback: bool,
    pub rtsp_port: u16,
    pub httpflv_port: u16,
}

/// H264 playback validation mode manager
pub struct H264PlaybackMode {
    config: H264PlaybackConfig,
    platform: Arc<RwLock<Option<Arc<dyn Platform>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl H264PlaybackMode {
    /// Create a new H264 playback validation mode
    pub fn new(config: H264PlaybackConfig) -> Self {
        Self {
            config,
            platform: Arc::new(RwLock::new(None)),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Initialize the playback mode with a platform instance
    pub async fn initialize(&self, platform: Arc<dyn Platform>) -> Result<(), PlaybackError> {
        let mut p = self.platform.write().await;
        *p = Some(platform);

        info!(
            file = %self.config.file_path,
            audio_file = ?self.config.audio_file_path,
            frame_rate = self.config.frame_rate,
            audio_sample_rate = self.config.audio_sample_rate,
            rtsp_port = self.config.rtsp_port,
            httpflv_port = self.config.httpflv_port,
            "H264 playback validation mode initialized"
        );

        Ok(())
    }

    /// Start playback
    pub async fn start(&self) -> Result<(), PlaybackError> {
        // Check platform availability first (before modifying state)
        let platform = self.platform.read().await;
        if platform.is_none() {
            return Err(PlaybackError::NotEnabled);
        }
        drop(platform); // Explicitly release the read lock

        // Now that platform is confirmed available, update running state
        let mut running = self.is_running.write().await;
        *running = true;

        info!("H264 playback started");
        Ok(())
    }

    /// Stop playback
    pub async fn stop(&self) -> Result<(), PlaybackError> {
        let mut running = self.is_running.write().await;
        *running = false;

        info!("H264 playback stopped");
        Ok(())
    }

    /// Check if playback is running
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get validation configuration
    pub fn config(&self) -> &H264PlaybackConfig {
        &self.config
    }

    /// Generate SDP information from H264 parameters and optional audio config
    pub fn generate_sdp(&self, sps: &[u8], pps: &[u8], audio_config: Option<&[u8]>) -> String {
        // Extract profile-level-id from SPS RBSP bytes
        // The profile-level-id is 3 bytes after the NAL header
        // SPS format: [NAL header(0x67)][profile_idc][constraint_flags][level_idc]...
        // We extract bytes [1][2][3] which are the RBSP bytes
        let profile_level_id = if sps.len() >= 4 {
            format!("{:02x}{:02x}{:02x}", sps[1], sps[2], sps[3])
        } else if sps.len() >= 3 {
            // Shorter SPS, use default safe value
            "42e01e".to_string()
        } else {
            // SPS is too short, use safe default (Baseline Profile, Level 3.0)
            "42e01e".to_string()
        };

        let mut sdp = format!(
            "v=0\r\n\
             o=- 0 0 IN IP4 0.0.0.0\r\n\
             s=H264 Validation Stream\r\n\
             c=IN IP4 0.0.0.0\r\n\
             t=0 0\r\n\
             a=tool:onvif-validation\r\n\
             m=video 0 RTP/AVP 96\r\n\
             a=rtpmap:96 H264/90000\r\n\
             a=fmtp:96 packetization-mode=1; sprop-parameter-sets={},{}; profile-level-id={}\r\n",
            base64_encode(sps),
            base64_encode(pps),
            profile_level_id
        );

        // Add audio section if audio config is provided
        if let Some(aac_config) = audio_config {
            let config_hex = aac_config
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<String>();

            // Calculate channels from AudioSpecificConfig
            // Byte 1, bits 3-6 = channelConfiguration
            let channels = if aac_config.len() >= 2 {
                ((aac_config[1] >> 3) & 0x0F) as u32
            } else {
                2 // Default to stereo
            };

            let audio_section = format!(
                "m=audio 0 RTP/AVP 97\r\n\
                 a=rtpmap:97 MPEG4-GENERIC/{}/{}\r\n\
                 a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3;config={}\r\n",
                self.config.audio_sample_rate, channels, config_hex
            );

            sdp.push_str(&audio_section);
        }

        sdp
    }
}

/// Helper function to base64 encode data
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_playback_config_creation() {
        let config = H264PlaybackConfig {
            file_path: "/tmp/test.h264".to_string(),
            audio_file_path: None,
            frame_rate: 25,
            audio_sample_rate: 48000,
            loop_playback: true,
            rtsp_port: 8554,
            httpflv_port: 8080,
        };

        assert_eq!(config.frame_rate, 25);
        assert!(config.loop_playback);
        assert_eq!(config.rtsp_port, 8554);
    }

    #[tokio::test]
    async fn test_playback_mode_creation() {
        let config = H264PlaybackConfig {
            file_path: "/tmp/test.h264".to_string(),
            audio_file_path: None,
            frame_rate: 25,
            audio_sample_rate: 48000,
            loop_playback: true,
            rtsp_port: 8554,
            httpflv_port: 8080,
        };

        let mode = H264PlaybackMode::new(config);
        assert!(!mode.is_running().await);
    }

    #[test]
    fn test_base64_encoding() {
        let data = b"hello";
        let encoded = base64_encode(data);
        assert_eq!(encoded, "aGVsbG8=");
    }

    #[test]
    fn test_generate_sdp_profile_level_id_extraction() {
        let config = H264PlaybackConfig {
            file_path: "/tmp/test.h264".to_string(),
            audio_file_path: None,
            frame_rate: 25,
            audio_sample_rate: 48000,
            loop_playback: false,
            rtsp_port: 8554,
            httpflv_port: 8080,
        };

        let mode = H264PlaybackMode::new(config);

        // Test with standard H.264 SPS (NAL header 0x67 followed by profile/constraint/level)
        // Format: [0x67 (NAL header)][0x42 (Baseline)][0xe0 (constraint)][0x1e (Level 3.0)]
        let sps = vec![0x67, 0x42, 0xe0, 0x1e];
        let pps = vec![0x68, 0xce, 0x38, 0x80];

        let sdp = mode.generate_sdp(&sps, &pps, None);

        // Should extract bytes [1][2][3] = 0x42, 0xe0, 0x1e -> "42e01e"
        assert!(sdp.contains("packetization-mode=1"));
        assert!(sdp.contains("profile-level-id=42e01e"));
        assert!(sdp.contains("sprop-parameter-sets="));
        assert!(!sdp.contains("m=audio")); // No audio
    }

    #[test]
    fn test_generate_sdp_with_audio() {
        let config = H264PlaybackConfig {
            file_path: "/tmp/test.h264".to_string(),
            audio_file_path: Some("/tmp/test.aac".to_string()),
            frame_rate: 25,
            audio_sample_rate: 48000,
            loop_playback: false,
            rtsp_port: 8554,
            httpflv_port: 8080,
        };

        let mode = H264PlaybackMode::new(config);

        let sps = vec![0x67, 0x42, 0xe0, 0x1e];
        let pps = vec![0x68, 0xce, 0x38, 0x80];

        // AAC-LC, 48kHz, stereo
        let audio_config = vec![0x11, 0x90];

        let sdp = mode.generate_sdp(&sps, &pps, Some(&audio_config));

        // Should have both video and audio sections
        assert!(sdp.contains("m=video 0 RTP/AVP 96"));
        assert!(sdp.contains("m=audio 0 RTP/AVP 97"));
        assert!(sdp.contains("MPEG4-GENERIC/48000/2"));
        assert!(sdp.contains("config=1190"));
    }

    #[test]
    fn test_generate_sdp_profile_level_id_short_sps() {
        let config = H264PlaybackConfig {
            file_path: "/tmp/test.h264".to_string(),
            audio_file_path: None,
            frame_rate: 25,
            audio_sample_rate: 48000,
            loop_playback: false,
            rtsp_port: 8554,
            httpflv_port: 8080,
        };

        let mode = H264PlaybackMode::new(config);

        // Test with short SPS (less than 4 bytes)
        let sps = vec![0x67, 0x42, 0xe0];
        let pps = vec![0x68];

        let sdp = mode.generate_sdp(&sps, &pps, None);

        // Should use default safe value
        assert!(sdp.contains("profile-level-id=42e01e"));
    }
}
