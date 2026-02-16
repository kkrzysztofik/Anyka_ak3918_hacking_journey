//! Validation platform implementation for H.264 playback testing.
//!
//! This module provides a `ValidationPlatform` that wraps `MockVideoPublisher`
//! and implements the `Platform` trait for comprehensive H.264 playback validation.

use super::traits::{
    AudioEncoder, AudioEncoderConfig, AudioInput, AudioSourceConfig, DeviceInfo, ImagingControl,
    NetworkInfo, PTZControl, Platform, PlatformResult, Resolution, VideoEncoder,
    VideoEncoderConfig, VideoEncoderOptions, VideoEncoding, VideoInput, VideoSourceConfig,
};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Stub implementation for VideoInput trait
pub struct StubVideoInput;

#[async_trait]
impl VideoInput for StubVideoInput {
    async fn open(&self) -> PlatformResult<()> {
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        Ok(())
    }

    async fn get_resolution(&self) -> PlatformResult<Resolution> {
        Ok(Resolution::new(1280, 720))
    }

    async fn get_sources(&self) -> PlatformResult<Vec<VideoSourceConfig>> {
        Ok(vec![VideoSourceConfig {
            token: "main".to_string(),
            name: "Main Stream".to_string(),
            resolution: Resolution::new(1280, 720),
            max_framerate: 25.0,
        }])
    }
}

/// Stub implementation for VideoEncoder trait
pub struct StubVideoEncoder;

#[async_trait]
impl VideoEncoder for StubVideoEncoder {
    async fn init(&self, _config: &VideoEncoderConfig) -> PlatformResult<()> {
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<VideoEncoderConfig> {
        Ok(VideoEncoderConfig {
            token: "encoder_h264".to_string(),
            name: "H264 Encoder".to_string(),
            resolution: Resolution::new(1280, 720),
            framerate: 25,
            bitrate: 2048,
            encoding: VideoEncoding::H264,
            gop_length: 25,
            quality: 80,
            ..Default::default()
        })
    }

    async fn set_configuration(&self, _config: &VideoEncoderConfig) -> PlatformResult<()> {
        Ok(())
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<VideoEncoderConfig>> {
        Ok(vec![self.get_configuration().await?])
    }

    async fn get_options(&self) -> PlatformResult<VideoEncoderOptions> {
        Ok(VideoEncoderOptions {
            resolutions: vec![
                Resolution::new(1920, 1080),
                Resolution::new(1280, 720),
                Resolution::new(640, 480),
            ],
            encodings: vec![VideoEncoding::H264],
            framerate_range: (1, 30),
            bitrate_range: (256, 10240),
            gop_range: (1, 150),
            quality_range: (0, 100),
        })
    }
}

/// Stub implementation for AudioInput trait
pub struct StubAudioInput;

#[async_trait]
impl AudioInput for StubAudioInput {
    async fn open(&self) -> PlatformResult<()> {
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioSourceConfig> {
        Ok(AudioSourceConfig {
            token: "audio_in".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        })
    }

    async fn get_sources(&self) -> PlatformResult<Vec<AudioSourceConfig>> {
        Ok(vec![self.get_configuration().await?])
    }
}

/// Stub implementation for AudioEncoder trait
pub struct StubAudioEncoder;

#[async_trait]
impl AudioEncoder for StubAudioEncoder {
    async fn init(&self, _config: &AudioEncoderConfig) -> PlatformResult<()> {
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioEncoderConfig> {
        Ok(AudioEncoderConfig {
            token: "audio_encoder".to_string(),
            name: "G.711 μ-law Encoder".to_string(),
            sample_rate: 8000,
            channels: 1,
            ..Default::default()
        })
    }

    async fn set_configuration(&self, _config: &AudioEncoderConfig) -> PlatformResult<()> {
        Ok(())
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<AudioEncoderConfig>> {
        Ok(vec![self.get_configuration().await?])
    }
}

/// Validation platform for H.264 playback testing.
///
/// This platform wraps the MockVideoPublisher and provides standard platform
/// interfaces for comprehensive ONVIF validation testing.
pub struct ValidationPlatform {
    device_info: DeviceInfo,
    video_input: Arc<dyn VideoInput>,
    video_encoder: Arc<dyn VideoEncoder>,
    audio_input: Arc<dyn AudioInput>,
    audio_encoder: Arc<dyn AudioEncoder>,
    is_initialized: AtomicBool,
}

impl ValidationPlatform {
    /// Create a new validation platform.
    pub fn new() -> Self {
        Self {
            device_info: DeviceInfo {
                manufacturer: "Anyka".to_string(),
                model: "AK3918 (Validation)".to_string(),
                firmware_version: "24.12".to_string(),
                serial_number: "VALIDATION-TEST".to_string(),
                hardware_id: "ak3918-validation".to_string(),
            },
            video_input: Arc::new(StubVideoInput),
            video_encoder: Arc::new(StubVideoEncoder),
            audio_input: Arc::new(StubAudioInput),
            audio_encoder: Arc::new(StubAudioEncoder),
            is_initialized: AtomicBool::new(false),
        }
    }
}

impl Default for ValidationPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for ValidationPlatform {
    async fn get_device_info(&self) -> PlatformResult<DeviceInfo> {
        Ok(self.device_info.clone())
    }

    fn video_input(&self) -> Arc<dyn VideoInput> {
        Arc::clone(&self.video_input)
    }

    fn video_encoder(&self) -> Arc<dyn VideoEncoder> {
        Arc::clone(&self.video_encoder)
    }

    fn audio_input(&self) -> Arc<dyn AudioInput> {
        Arc::clone(&self.audio_input)
    }

    fn audio_encoder(&self) -> Arc<dyn AudioEncoder> {
        Arc::clone(&self.audio_encoder)
    }

    fn ptz_control(&self) -> Option<Arc<dyn PTZControl>> {
        None // PTZ not supported in validation mode
    }

    fn imaging_control(&self) -> Option<Arc<dyn ImagingControl>> {
        None // Imaging control not supported in validation mode
    }

    fn network_info(&self) -> Option<Arc<dyn NetworkInfo>> {
        None // Network info not supported in validation mode
    }

    fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::SeqCst)
    }

    async fn initialize(&self) -> PlatformResult<()> {
        // Update is_initialized using atomic operations
        self.is_initialized.store(true, Ordering::SeqCst);
        tracing::info!("ValidationPlatform initialized for H.264 playback testing");
        Ok(())
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        self.is_initialized.store(false, Ordering::SeqCst);
        tracing::info!("ValidationPlatform shutdown");
        Ok(())
    }

    fn max_sensor_resolution(&self) -> PlatformResult<Resolution> {
        // Return default resolution for testing validation platform
        Ok(Resolution::new(1920, 1080))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_platform_creation() {
        let platform = ValidationPlatform::new();
        assert!(!platform.is_initialized());

        let device_info = platform.get_device_info().await.unwrap();
        assert_eq!(device_info.manufacturer, "Anyka");
        assert!(device_info.model.contains("AK3918"));
    }

    #[tokio::test]
    async fn test_validation_platform_initialization() {
        let platform = ValidationPlatform::new();
        assert!(platform.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_video_encoder_configuration() {
        let platform = ValidationPlatform::new();
        let encoder = platform.video_encoder();

        let config = encoder.get_configuration().await.unwrap();
        assert_eq!(config.encoding, VideoEncoding::H264);
        assert_eq!(config.framerate, 25);
    }

    #[tokio::test]
    async fn test_video_input_sources() {
        let platform = ValidationPlatform::new();
        let input = platform.video_input();

        let sources = input.get_sources().await.unwrap();
        assert!(!sources.is_empty());
        assert_eq!(sources[0].token, "main");
    }

    #[tokio::test]
    async fn test_validation_platform_is_initialized_false_before_init() {
        let platform = ValidationPlatform::new();
        assert!(!platform.is_initialized());
    }

    #[tokio::test]
    async fn test_validation_platform_is_initialized_true_after_init() {
        let platform = ValidationPlatform::new();
        platform.initialize().await.unwrap();
        assert!(platform.is_initialized());
    }

    #[tokio::test]
    async fn test_validation_platform_shutdown_after_init_returns_ok() {
        let platform = ValidationPlatform::new();
        platform.initialize().await.unwrap();
        assert!(platform.is_initialized());
        let result = platform.shutdown().await;
        assert!(result.is_ok());
        assert!(!platform.is_initialized());
    }

    #[tokio::test]
    async fn test_validation_platform_audio_input_returns_config() {
        let platform = ValidationPlatform::new();
        let input = platform.audio_input();
        let config = input.get_configuration().await.unwrap();
        assert_eq!(config.token, "audio_in");
        assert_eq!(config.name, "Microphone");
        assert_eq!(config.channels, 1);
    }

    #[tokio::test]
    async fn test_validation_platform_audio_encoder_returns_config() {
        let platform = ValidationPlatform::new();
        let encoder = platform.audio_encoder();
        let config = encoder.get_configuration().await.unwrap();
        assert_eq!(config.token, "audio_encoder");
        assert_eq!(config.sample_rate, 8000);
    }

    #[tokio::test]
    async fn test_validation_platform_shutdown_before_init_returns_ok() {
        let platform = ValidationPlatform::new();
        assert!(!platform.is_initialized());
        let result = platform.shutdown().await;
        assert!(result.is_ok());
        assert!(!platform.is_initialized());
    }

    #[tokio::test]
    async fn test_validation_platform_ptz_control_returns_none() {
        let platform = ValidationPlatform::new();
        assert!(platform.ptz_control().is_none());
    }

    #[tokio::test]
    async fn test_validation_platform_imaging_control_returns_none() {
        let platform = ValidationPlatform::new();
        assert!(platform.imaging_control().is_none());
    }

    #[tokio::test]
    async fn test_validation_platform_network_info_returns_none() {
        let platform = ValidationPlatform::new();
        assert!(platform.network_info().is_none());
    }

    #[tokio::test]
    async fn test_validation_platform_default_constructs() {
        let platform = ValidationPlatform::default();
        assert!(!platform.is_initialized());
        let device_info = platform.get_device_info().await.unwrap();
        assert_eq!(device_info.manufacturer, "Anyka");
    }

    #[tokio::test]
    async fn test_validation_platform_is_initialized_false_after_shutdown() {
        let platform = ValidationPlatform::new();
        platform.initialize().await.unwrap();
        assert!(platform.is_initialized());
        platform.shutdown().await.unwrap();
        assert!(!platform.is_initialized());
    }

    #[tokio::test]
    async fn test_validation_platform_init_shutdown_init_cycle() {
        let platform = ValidationPlatform::new();
        platform.initialize().await.unwrap();
        assert!(platform.is_initialized());
        platform.shutdown().await.unwrap();
        assert!(!platform.is_initialized());
        platform.initialize().await.unwrap();
        assert!(platform.is_initialized());
    }

    #[tokio::test]
    async fn test_validation_platform_video_input_get_resolution() {
        let platform = ValidationPlatform::new();
        let input = platform.video_input();
        let resolution = input.get_resolution().await.unwrap();
        assert_eq!(resolution.width, 1280);
        assert_eq!(resolution.height, 720);
    }

    #[tokio::test]
    async fn test_validation_platform_video_encoder_get_options() {
        let platform = ValidationPlatform::new();
        let encoder = platform.video_encoder();
        let options = encoder.get_options().await.unwrap();
        assert!(!options.resolutions.is_empty());
        assert!(options.resolutions.contains(&Resolution::new(1280, 720)));
        assert_eq!(options.framerate_range, (1, 30));
    }

    #[tokio::test]
    async fn test_validation_platform_video_encoder_get_configurations() {
        let platform = ValidationPlatform::new();
        let encoder = platform.video_encoder();
        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].token, "encoder_h264");
    }

    #[tokio::test]
    async fn test_validation_platform_audio_input_get_sources() {
        let platform = ValidationPlatform::new();
        let input = platform.audio_input();
        let sources = input.get_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].token, "audio_in");
    }

    #[tokio::test]
    async fn test_validation_platform_audio_encoder_get_configurations() {
        let platform = ValidationPlatform::new();
        let encoder = platform.audio_encoder();
        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].token, "audio_encoder");
    }
}
