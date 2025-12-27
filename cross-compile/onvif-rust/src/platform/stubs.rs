//! Stub implementations of platform traits for testing.
//!
//! This module provides configurable stub implementations that can be used
//! for testing without actual hardware. The stubs maintain in-memory state
//! and can be configured to succeed or fail specific operations.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use parking_lot::RwLock;

use super::traits::{
    AudioEncoder, AudioEncoderConfig, AudioEncoding, AudioInput, AudioSourceConfig, BitrateMode,
    DeviceInfo, DnsInfo, ImagingControl, ImagingOptions, ImagingSettings, NetworkInfo,
    NetworkInterfaceInfo, NetworkProtocolInfo, NtpInfo, PTZControl, Platform, PlatformError,
    PlatformResult, PtzLimits, PtzPosition, PtzPreset, PtzVelocity, Resolution, VideoEncoder,
    VideoEncoderConfig, VideoEncoderOptions, VideoEncoding, VideoInput, VideoSourceConfig,
};

/// Builder for configuring stub platform behavior.
#[derive(Debug, Clone, Default)]
pub struct StubPlatformBuilder {
    /// Device information to return.
    device_info: Option<DeviceInfo>,
    /// Video sources to return.
    video_sources: Vec<VideoSourceConfig>,
    /// Video encoder configurations.
    video_encoders: Vec<VideoEncoderConfig>,
    /// Audio sources to return.
    audio_sources: Vec<AudioSourceConfig>,
    /// Audio encoder configurations.
    audio_encoders: Vec<AudioEncoderConfig>,
    /// Initial PTZ position.
    ptz_position: Option<PtzPosition>,
    /// PTZ presets.
    ptz_presets: Vec<PtzPreset>,
    /// PTZ limits.
    ptz_limits: Option<PtzLimits>,
    /// Imaging settings.
    imaging_settings: Option<ImagingSettings>,
    /// Whether PTZ is supported.
    ptz_supported: bool,
    /// Whether imaging control is supported.
    imaging_supported: bool,
    /// Whether network info is supported (default: true).
    network_info_supported: bool,
    /// MAC address for stub network info.
    mac_address: Option<String>,
    /// IP address for stub network info (None = auto-detect).
    ip_address: Option<String>,
    /// Force initialization failure.
    fail_init: bool,
}

impl StubPlatformBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            network_info_supported: true, // Enable by default
            ..Self::default()
        }
    }

    /// Set device information.
    pub fn device_info(mut self, info: DeviceInfo) -> Self {
        self.device_info = Some(info);
        self
    }

    /// Add a video source.
    pub fn video_source(mut self, source: VideoSourceConfig) -> Self {
        self.video_sources.push(source);
        self
    }

    /// Add a video encoder configuration.
    pub fn video_encoder(mut self, encoder: VideoEncoderConfig) -> Self {
        self.video_encoders.push(encoder);
        self
    }

    /// Add an audio source.
    pub fn audio_source(mut self, source: AudioSourceConfig) -> Self {
        self.audio_sources.push(source);
        self
    }

    /// Add an audio encoder configuration.
    pub fn audio_encoder(mut self, encoder: AudioEncoderConfig) -> Self {
        self.audio_encoders.push(encoder);
        self
    }

    /// Set initial PTZ position.
    pub fn ptz_position(mut self, position: PtzPosition) -> Self {
        self.ptz_position = Some(position);
        self
    }

    /// Add a PTZ preset.
    pub fn ptz_preset(mut self, preset: PtzPreset) -> Self {
        self.ptz_presets.push(preset);
        self
    }

    /// Set PTZ limits.
    pub fn ptz_limits(mut self, limits: PtzLimits) -> Self {
        self.ptz_limits = Some(limits);
        self
    }

    /// Set imaging settings.
    pub fn imaging_settings(mut self, settings: ImagingSettings) -> Self {
        self.imaging_settings = Some(settings);
        self
    }

    /// Enable or disable PTZ support.
    pub fn ptz_supported(mut self, supported: bool) -> Self {
        self.ptz_supported = supported;
        self
    }

    /// Enable or disable imaging control support.
    pub fn imaging_supported(mut self, supported: bool) -> Self {
        self.imaging_supported = supported;
        self
    }

    /// Enable or disable network info support.
    pub fn network_info_supported(mut self, supported: bool) -> Self {
        self.network_info_supported = supported;
        self
    }

    /// Set the MAC address for stub network info.
    pub fn mac_address(mut self, mac: impl Into<String>) -> Self {
        self.mac_address = Some(mac.into());
        self
    }

    /// Set the IP address for stub network info (None = auto-detect).
    pub fn ip_address(mut self, ip: Option<String>) -> Self {
        self.ip_address = ip;
        self
    }

    /// Force initialization to fail.
    pub fn fail_init(mut self, fail: bool) -> Self {
        self.fail_init = fail;
        self
    }

    /// Build the stub platform with default video/audio if none specified.
    pub fn build(self) -> StubPlatform {
        let video_sources = Self::get_video_sources(self.video_sources);
        let video_encoders = Self::get_video_encoders(self.video_encoders);
        let audio_sources = Self::get_audio_sources(self.audio_sources);
        let audio_encoders = Self::get_audio_encoders(self.audio_encoders);
        let device_info = self.device_info.unwrap_or_else(Self::default_device_info);
        let ptz_position = self.ptz_position.unwrap_or(PtzPosition::HOME);
        let ptz_limits = self.ptz_limits.unwrap_or(PtzLimits::DEFAULT);
        let imaging_settings = self
            .imaging_settings
            .unwrap_or_else(Self::default_imaging_settings);
        let presets: HashMap<String, PtzPreset> = self
            .ptz_presets
            .into_iter()
            .map(|p| (p.token.clone(), p))
            .collect();

        let video_input = Arc::new(StubVideoInput {
            opened: AtomicBool::new(false),
            sources: video_sources.clone(),
        });

        let video_encoder = Arc::new(StubVideoEncoder {
            configurations: RwLock::new(video_encoders),
        });

        let audio_input = Arc::new(StubAudioInput {
            opened: AtomicBool::new(false),
            sources: audio_sources.clone(),
        });

        let audio_encoder = Arc::new(StubAudioEncoder {
            configurations: RwLock::new(audio_encoders),
        });

        let ptz_control =
            Self::create_ptz_control(self.ptz_supported, ptz_position, ptz_limits, presets);
        let imaging_control =
            Self::create_imaging_control(self.imaging_supported, imaging_settings);
        let network_info = Self::create_network_info(
            self.network_info_supported,
            self.mac_address,
            self.ip_address,
        );

        StubPlatform {
            initialized: AtomicBool::new(false),
            fail_init: self.fail_init,
            device_info,
            video_input,
            video_encoder,
            audio_input,
            audio_encoder,
            ptz_control,
            imaging_control,
            network_info,
        }
    }

    /// Get video sources, using defaults if empty.
    fn get_video_sources(sources: Vec<VideoSourceConfig>) -> Vec<VideoSourceConfig> {
        if sources.is_empty() {
            vec![VideoSourceConfig {
                token: "VideoSource_1".to_string(),
                name: "Main Camera".to_string(),
                resolution: Resolution::new(1920, 1080),
                max_framerate: 30.0,
            }]
        } else {
            sources
        }
    }

    /// Get video encoders, using defaults if empty.
    fn get_video_encoders(encoders: Vec<VideoEncoderConfig>) -> Vec<VideoEncoderConfig> {
        if encoders.is_empty() {
            vec![
                VideoEncoderConfig {
                    token: "VideoEncoder_1".to_string(),
                    name: "Main Stream".to_string(),
                    resolution: Resolution::new(1920, 1080),
                    framerate: 25,
                    bitrate: 4000,
                    encoding: VideoEncoding::H264,
                    bitrate_mode: BitrateMode::Vbr,
                    gop_length: 50,
                    quality: 80,
                },
                VideoEncoderConfig {
                    token: "VideoEncoder_2".to_string(),
                    name: "Sub Stream".to_string(),
                    resolution: Resolution::new(640, 480),
                    framerate: 15,
                    bitrate: 512,
                    encoding: VideoEncoding::H264,
                    bitrate_mode: BitrateMode::Vbr,
                    gop_length: 30,
                    quality: 60,
                },
            ]
        } else {
            encoders
        }
    }

    /// Get audio sources, using defaults if empty.
    fn get_audio_sources(sources: Vec<AudioSourceConfig>) -> Vec<AudioSourceConfig> {
        if sources.is_empty() {
            vec![AudioSourceConfig {
                token: "AudioSource_1".to_string(),
                name: "Microphone".to_string(),
                channels: 1,
            }]
        } else {
            sources
        }
    }

    /// Get audio encoders, using defaults if empty.
    fn get_audio_encoders(encoders: Vec<AudioEncoderConfig>) -> Vec<AudioEncoderConfig> {
        if encoders.is_empty() {
            vec![AudioEncoderConfig {
                token: "AudioEncoder_1".to_string(),
                name: "Audio Stream".to_string(),
                sample_rate: 8000,
                channels: 1,
                encoding: AudioEncoding::G711U,
                bitrate: 64,
            }]
        } else {
            encoders
        }
    }

    /// Create default device info.
    fn default_device_info() -> DeviceInfo {
        DeviceInfo {
            manufacturer: "Anyka".to_string(),
            model: "AK3918".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "STUB-001".to_string(),
            hardware_id: "ak3918-stub".to_string(),
        }
    }

    /// Create default imaging settings.
    fn default_imaging_settings() -> ImagingSettings {
        ImagingSettings {
            brightness: 50.0,
            contrast: 50.0,
            saturation: 50.0,
            sharpness: 50.0,
            ir_cut_filter: true,
            ir_led: false,
            wdr: false,
            backlight_compensation: false,
        }
    }

    /// Create PTZ control if supported.
    fn create_ptz_control(
        ptz_supported: bool,
        ptz_position: PtzPosition,
        ptz_limits: PtzLimits,
        presets: HashMap<String, PtzPreset>,
    ) -> Option<Arc<dyn PTZControl>> {
        if ptz_supported {
            Some(Arc::new(StubPTZControl {
                position: RwLock::new(ptz_position),
                velocity: RwLock::new(PtzVelocity::STOP),
                presets: RwLock::new(presets),
                limits: ptz_limits,
                next_preset_id: RwLock::new(1),
            }) as Arc<dyn PTZControl>)
        } else {
            None
        }
    }

    /// Create imaging control if supported.
    fn create_imaging_control(
        imaging_supported: bool,
        imaging_settings: ImagingSettings,
    ) -> Option<Arc<dyn ImagingControl>> {
        if imaging_supported {
            Some(Arc::new(StubImagingControl {
                settings: RwLock::new(imaging_settings),
                options: ImagingOptions::default_options(),
            }) as Arc<dyn ImagingControl>)
        } else {
            None
        }
    }

    /// Create network info if supported.
    fn create_network_info(
        network_info_supported: bool,
        mac_address: Option<String>,
        ip_address: Option<String>,
    ) -> Option<Arc<dyn NetworkInfo>> {
        if network_info_supported {
            let stub = match (mac_address, ip_address) {
                (Some(mac), ip) => StubNetworkInfo::with_mac_and_ip(mac, ip),
                (None, Some(ip)) => {
                    StubNetworkInfo::with_mac_and_ip("AA:BB:CC:DD:EE:FF".to_string(), Some(ip))
                }
                (None, None) => StubNetworkInfo::new(),
            };
            Some(Arc::new(stub) as Arc<dyn NetworkInfo>)
        } else {
            None
        }
    }
}

/// Stub platform implementation for testing.
pub struct StubPlatform {
    initialized: AtomicBool,
    fail_init: bool,
    device_info: DeviceInfo,
    video_input: Arc<StubVideoInput>,
    video_encoder: Arc<StubVideoEncoder>,
    audio_input: Arc<StubAudioInput>,
    audio_encoder: Arc<StubAudioEncoder>,
    ptz_control: Option<Arc<dyn PTZControl>>,
    imaging_control: Option<Arc<dyn ImagingControl>>,
    network_info: Option<Arc<dyn NetworkInfo>>,
}

impl StubPlatform {
    /// Create a new stub platform with default settings.
    pub fn new() -> Self {
        StubPlatformBuilder::new()
            .ptz_supported(true)
            .imaging_supported(true)
            .build()
    }
}

impl Default for StubPlatform {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Platform for StubPlatform {
    async fn get_device_info(&self) -> PlatformResult<DeviceInfo> {
        Ok(self.device_info.clone())
    }

    fn video_input(&self) -> Arc<dyn VideoInput> {
        self.video_input.clone()
    }

    fn video_encoder(&self) -> Arc<dyn VideoEncoder> {
        self.video_encoder.clone()
    }

    fn audio_input(&self) -> Arc<dyn AudioInput> {
        self.audio_input.clone()
    }

    fn audio_encoder(&self) -> Arc<dyn AudioEncoder> {
        self.audio_encoder.clone()
    }

    fn ptz_control(&self) -> Option<Arc<dyn PTZControl>> {
        self.ptz_control.clone()
    }

    fn imaging_control(&self) -> Option<Arc<dyn ImagingControl>> {
        self.imaging_control.clone()
    }

    fn network_info(&self) -> Option<Arc<dyn NetworkInfo>> {
        self.network_info.clone()
    }

    fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    async fn initialize(&self) -> PlatformResult<()> {
        if self.fail_init {
            return Err(PlatformError::InitializationFailed(
                "Forced failure for testing".to_string(),
            ));
        }
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }
}

/// Stub video input implementation.
pub struct StubVideoInput {
    opened: AtomicBool,
    sources: Vec<VideoSourceConfig>,
}

#[async_trait]
impl VideoInput for StubVideoInput {
    async fn open(&self) -> PlatformResult<()> {
        self.opened.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        self.opened.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn get_resolution(&self) -> PlatformResult<Resolution> {
        if let Some(source) = self.sources.first() {
            Ok(source.resolution)
        } else {
            Err(PlatformError::HardwareUnavailable(
                "No video source".to_string(),
            ))
        }
    }

    async fn get_sources(&self) -> PlatformResult<Vec<VideoSourceConfig>> {
        Ok(self.sources.clone())
    }
}

/// Stub video encoder implementation.
pub struct StubVideoEncoder {
    configurations: RwLock<Vec<VideoEncoderConfig>>,
}

#[async_trait]
impl VideoEncoder for StubVideoEncoder {
    async fn init(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
        } else {
            configs.push(config.clone());
        }
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<VideoEncoderConfig> {
        let configs = self.configurations.read();
        configs
            .first()
            .cloned()
            .ok_or_else(|| PlatformError::HardwareUnavailable("No encoder config".to_string()))
    }

    async fn set_configuration(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<VideoEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }

    async fn get_options(&self) -> PlatformResult<VideoEncoderOptions> {
        Ok(VideoEncoderOptions {
            resolutions: vec![
                Resolution::new(1920, 1080),
                Resolution::new(1280, 720),
                Resolution::new(640, 480),
                Resolution::new(320, 240),
            ],
            encodings: vec![
                VideoEncoding::H264,
                VideoEncoding::H265,
                VideoEncoding::Mjpeg,
            ],
            framerate_range: (1, 30),
            bitrate_range: (64, 8192),
            gop_range: (1, 300),
            quality_range: (1, 100),
        })
    }
}

/// Stub audio input implementation.
pub struct StubAudioInput {
    opened: AtomicBool,
    sources: Vec<AudioSourceConfig>,
}

#[async_trait]
impl AudioInput for StubAudioInput {
    async fn open(&self) -> PlatformResult<()> {
        self.opened.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        self.opened.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioSourceConfig> {
        self.sources
            .first()
            .cloned()
            .ok_or_else(|| PlatformError::HardwareUnavailable("No audio source".to_string()))
    }

    async fn get_sources(&self) -> PlatformResult<Vec<AudioSourceConfig>> {
        Ok(self.sources.clone())
    }
}

/// Stub audio encoder implementation.
pub struct StubAudioEncoder {
    configurations: RwLock<Vec<AudioEncoderConfig>>,
}

#[async_trait]
impl AudioEncoder for StubAudioEncoder {
    async fn init(&self, config: &AudioEncoderConfig) -> PlatformResult<()> {
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
        } else {
            configs.push(config.clone());
        }
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioEncoderConfig> {
        let configs = self.configurations.read();
        configs
            .first()
            .cloned()
            .ok_or_else(|| PlatformError::HardwareUnavailable("No audio encoder".to_string()))
    }

    async fn set_configuration(&self, config: &AudioEncoderConfig) -> PlatformResult<()> {
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<AudioEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }
}

/// Stub PTZ control implementation.
pub struct StubPTZControl {
    position: RwLock<PtzPosition>,
    velocity: RwLock<PtzVelocity>,
    presets: RwLock<HashMap<String, PtzPreset>>,
    limits: PtzLimits,
    next_preset_id: RwLock<u32>,
}

#[async_trait]
impl PTZControl for StubPTZControl {
    async fn move_to_position(&self, position: PtzPosition) -> PlatformResult<()> {
        // Clamp position to limits
        let clamped = PtzPosition {
            pan: position.pan.clamp(self.limits.min_pan, self.limits.max_pan),
            tilt: position
                .tilt
                .clamp(self.limits.min_tilt, self.limits.max_tilt),
            zoom: position
                .zoom
                .clamp(self.limits.min_zoom, self.limits.max_zoom),
        };
        *self.position.write() = clamped;
        Ok(())
    }

    async fn get_position(&self) -> PlatformResult<PtzPosition> {
        Ok(*self.position.read())
    }

    async fn continuous_move(&self, velocity: PtzVelocity) -> PlatformResult<()> {
        *self.velocity.write() = velocity;
        Ok(())
    }

    async fn stop(&self) -> PlatformResult<()> {
        *self.velocity.write() = PtzVelocity::STOP;
        Ok(())
    }

    async fn get_presets(&self) -> PlatformResult<Vec<PtzPreset>> {
        Ok(self.presets.read().values().cloned().collect())
    }

    async fn set_preset(&self, name: &str) -> PlatformResult<String> {
        let position = *self.position.read();
        let mut next_id = self.next_preset_id.write();
        let token = format!("Preset_{}", *next_id);
        *next_id += 1;

        let preset = PtzPreset {
            token: token.clone(),
            name: name.to_string(),
            position,
        };
        self.presets.write().insert(token.clone(), preset);
        Ok(token)
    }

    async fn goto_preset(&self, token: &str) -> PlatformResult<()> {
        let presets = self.presets.read();
        if let Some(preset) = presets.get(token) {
            *self.position.write() = preset.position;
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Preset not found: {}",
                token
            )))
        }
    }

    async fn remove_preset(&self, token: &str) -> PlatformResult<()> {
        let mut presets = self.presets.write();
        if presets.remove(token).is_some() {
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Preset not found: {}",
                token
            )))
        }
    }

    async fn get_limits(&self) -> PlatformResult<PtzLimits> {
        Ok(self.limits)
    }
}

/// Stub imaging control implementation.
pub struct StubImagingControl {
    settings: RwLock<ImagingSettings>,
    options: ImagingOptions,
}

#[async_trait]
impl ImagingControl for StubImagingControl {
    async fn get_settings(&self) -> PlatformResult<ImagingSettings> {
        Ok(self.settings.read().clone())
    }

    async fn set_settings(&self, settings: &ImagingSettings) -> PlatformResult<()> {
        *self.settings.write() = settings.clone();
        Ok(())
    }

    async fn get_options(&self) -> PlatformResult<ImagingOptions> {
        Ok(self.options.clone())
    }

    async fn set_brightness(&self, value: f32) -> PlatformResult<()> {
        let (min, max) = self.options.brightness_range;
        if value < min || value > max {
            return Err(PlatformError::InvalidParameter(format!(
                "Brightness {} out of range [{}, {}]",
                value, min, max
            )));
        }
        self.settings.write().brightness = value;
        Ok(())
    }

    async fn set_contrast(&self, value: f32) -> PlatformResult<()> {
        let (min, max) = self.options.contrast_range;
        if value < min || value > max {
            return Err(PlatformError::InvalidParameter(format!(
                "Contrast {} out of range [{}, {}]",
                value, min, max
            )));
        }
        self.settings.write().contrast = value;
        Ok(())
    }

    async fn set_saturation(&self, value: f32) -> PlatformResult<()> {
        let (min, max) = self.options.saturation_range;
        if value < min || value > max {
            return Err(PlatformError::InvalidParameter(format!(
                "Saturation {} out of range [{}, {}]",
                value, min, max
            )));
        }
        self.settings.write().saturation = value;
        Ok(())
    }

    async fn set_sharpness(&self, value: f32) -> PlatformResult<()> {
        let (min, max) = self.options.sharpness_range;
        if value < min || value > max {
            return Err(PlatformError::InvalidParameter(format!(
                "Sharpness {} out of range [{}, {}]",
                value, min, max
            )));
        }
        self.settings.write().sharpness = value;
        Ok(())
    }
}

/// Stub network information implementation.
///
/// Returns configurable network data for testing. By default returns
/// a mock MAC address and detects real IP using UDP socket trick.
pub struct StubNetworkInfo {
    /// MAC address to return (default: "AA:BB:CC:DD:EE:FF").
    mac_address: String,
    /// IP address override (None = auto-detect).
    ip_address: Option<String>,
    /// Network settings storage (for stub set operations).
    settings: parking_lot::RwLock<NetworkSettings>,
}

/// Internal storage for network settings stubs.
#[derive(Debug, Clone, Default)]
struct NetworkSettings {
    dns_servers: Vec<String>,
    search_domains: Vec<String>,
    gateway: Option<String>,
}

impl StubNetworkInfo {
    /// Create a new stub network info with defaults.
    pub fn new() -> Self {
        Self {
            mac_address: "AA:BB:CC:DD:EE:FF".to_string(),
            ip_address: None,
            settings: parking_lot::RwLock::new(NetworkSettings::default()),
        }
    }

    /// Create with custom MAC address.
    pub fn with_mac(mac_address: String) -> Self {
        Self {
            mac_address,
            ip_address: None,
            settings: parking_lot::RwLock::new(NetworkSettings::default()),
        }
    }

    /// Create with custom MAC and IP addresses.
    pub fn with_mac_and_ip(mac_address: String, ip_address: Option<String>) -> Self {
        Self {
            mac_address,
            ip_address,
            settings: parking_lot::RwLock::new(NetworkSettings::default()),
        }
    }
}

impl Default for StubNetworkInfo {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NetworkInfo for StubNetworkInfo {
    async fn get_network_interfaces(&self) -> PlatformResult<Vec<NetworkInterfaceInfo>> {
        // Use configured IP or detect real IP
        let ip = self.ip_address.clone().or_else(|| self.detect_local_ip());

        Ok(vec![NetworkInterfaceInfo {
            token: "eth0".to_string(),
            name: "eth0".to_string(),
            enabled: true,
            ipv4_address: ip,
            ipv4_prefix_length: Some(24),
            ipv4_dhcp: true,
            mac_address: Some(self.mac_address.clone()),
            link_speed: Some(100),
        }])
    }

    async fn get_dns_info(&self) -> PlatformResult<DnsInfo> {
        let settings = self.settings.read();
        Ok(DnsInfo {
            from_dhcp: settings.dns_servers.is_empty(),
            search_domains: settings.search_domains.clone(),
            dns_from_dhcp: if settings.dns_servers.is_empty() {
                vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()]
            } else {
                vec![]
            },
            dns_manual: settings.dns_servers.clone(),
        })
    }

    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo> {
        Ok(NtpInfo {
            from_dhcp: true,
            ntp_from_dhcp: vec!["pool.ntp.org".to_string()],
            ntp_manual: vec![],
        })
    }

    async fn get_network_protocols(&self) -> PlatformResult<Vec<NetworkProtocolInfo>> {
        Ok(vec![
            NetworkProtocolInfo {
                name: "HTTP".to_string(),
                enabled: true,
                ports: vec![80],
            },
            NetworkProtocolInfo {
                name: "RTSP".to_string(),
                enabled: true,
                ports: vec![554],
            },
        ])
    }

    async fn set_network_interface(
        &self,
        _token: &str,
        _ipv4_address: Option<String>,
        _ipv4_prefix_length: Option<u8>,
        _ipv4_dhcp: bool,
    ) -> PlatformResult<()> {
        // Stub: accept but don't persist (in-memory only for testing)
        Ok(())
    }

    async fn set_dns(
        &self,
        dns_servers: &[String],
        search_domains: &[String],
    ) -> PlatformResult<()> {
        let mut settings = self.settings.write();
        settings.dns_servers = dns_servers.to_vec();
        settings.search_domains = search_domains.to_vec();
        Ok(())
    }

    async fn set_gateway(&self, gateway: &str) -> PlatformResult<()> {
        let mut settings = self.settings.write();
        settings.gateway = Some(gateway.to_string());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_platform_default() {
        let platform = StubPlatform::new();
        assert!(!platform.is_initialized());

        platform.initialize().await.unwrap();
        assert!(platform.is_initialized());

        let info = platform.get_device_info().await.unwrap();
        assert_eq!(info.manufacturer, "Anyka");
        assert_eq!(info.model, "AK3918");
    }

    #[tokio::test]
    async fn test_stub_platform_builder() {
        let platform = StubPlatformBuilder::new()
            .device_info(DeviceInfo {
                manufacturer: "Test".to_string(),
                model: "TestModel".to_string(),
                firmware_version: "2.0.0".to_string(),
                serial_number: "TEST-123".to_string(),
                hardware_id: "test-hw".to_string(),
            })
            .ptz_supported(false)
            .build();

        let info = platform.get_device_info().await.unwrap();
        assert_eq!(info.manufacturer, "Test");
        assert!(platform.ptz_control().is_none());
    }

    #[tokio::test]
    async fn test_stub_platform_fail_init() {
        let platform = StubPlatformBuilder::new().fail_init(true).build();

        let result = platform.initialize().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stub_video_input() {
        let platform = StubPlatform::new();
        let video = platform.video_input();

        video.open().await.unwrap();
        let resolution = video.get_resolution().await.unwrap();
        assert_eq!(resolution.width, 1920);
        assert_eq!(resolution.height, 1080);

        let sources = video.get_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
    }

    #[tokio::test]
    async fn test_stub_video_encoder() {
        let platform = StubPlatform::new();
        let encoder = platform.video_encoder();

        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 2);

        let options = encoder.get_options().await.unwrap();
        assert!(options.resolutions.len() >= 4);
    }

    #[tokio::test]
    async fn test_stub_ptz_control() {
        let platform = StubPlatform::new();
        let ptz = platform.ptz_control().unwrap();

        // Check initial position
        let pos = ptz.get_position().await.unwrap();
        assert_eq!(pos, PtzPosition::HOME);

        // Move to new position
        let new_pos = PtzPosition::new(45.0, -30.0, 2.0);
        ptz.move_to_position(new_pos).await.unwrap();

        let pos = ptz.get_position().await.unwrap();
        assert_eq!(pos.pan, 45.0);
        assert_eq!(pos.tilt, -30.0);
        assert_eq!(pos.zoom, 2.0);
    }

    #[tokio::test]
    async fn test_stub_ptz_presets() {
        let platform = StubPlatform::new();
        let ptz = platform.ptz_control().unwrap();

        // Set preset
        ptz.move_to_position(PtzPosition::new(90.0, 45.0, 1.5))
            .await
            .unwrap();
        let token = ptz.set_preset("Corner View").await.unwrap();

        // Get presets
        let presets = ptz.get_presets().await.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].name, "Corner View");

        // Go home
        ptz.move_to_position(PtzPosition::HOME).await.unwrap();

        // Goto preset
        ptz.goto_preset(&token).await.unwrap();
        let pos = ptz.get_position().await.unwrap();
        assert_eq!(pos.pan, 90.0);

        // Remove preset
        ptz.remove_preset(&token).await.unwrap();
        let presets = ptz.get_presets().await.unwrap();
        assert!(presets.is_empty());
    }

    #[tokio::test]
    async fn test_stub_imaging_control() {
        let platform = StubPlatform::new();
        let imaging = platform.imaging_control().unwrap();

        let settings = imaging.get_settings().await.unwrap();
        assert_eq!(settings.brightness, 50.0);

        imaging.set_brightness(75.0).await.unwrap();
        let settings = imaging.get_settings().await.unwrap();
        assert_eq!(settings.brightness, 75.0);

        // Test out of range
        let result = imaging.set_brightness(150.0).await;
        assert!(result.is_err());
    }

    // Tests for StubPlatformBuilder methods
    #[tokio::test]
    async fn test_stub_platform_builder_device_info() {
        let custom_info = DeviceInfo {
            manufacturer: "Custom".to_string(),
            model: "Custom Model".to_string(),
            firmware_version: "2.0.0".to_string(),
            serial_number: "CUSTOM-001".to_string(),
            hardware_id: "custom-hw".to_string(),
        };

        let platform = StubPlatformBuilder::new()
            .device_info(custom_info.clone())
            .build();

        let info = platform.get_device_info().await.unwrap();
        assert_eq!(info.manufacturer, "Custom");
        assert_eq!(info.model, "Custom Model");
        assert_eq!(info.firmware_version, "2.0.0");
        assert_eq!(info.serial_number, "CUSTOM-001");
    }

    #[tokio::test]
    async fn test_stub_platform_builder_video_source() {
        let custom_source = VideoSourceConfig {
            token: "CustomVideoSource".to_string(),
            name: "Custom Camera".to_string(),
            resolution: Resolution::new(3840, 2160),
            max_framerate: 60.0,
        };

        let platform = StubPlatformBuilder::new()
            .video_source(custom_source)
            .build();

        let video = platform.video_input();
        let sources = video.get_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].token, "CustomVideoSource");
        assert_eq!(sources[0].resolution.width, 3840);
    }

    #[tokio::test]
    async fn test_stub_platform_builder_video_encoder() {
        let custom_encoder = VideoEncoderConfig {
            token: "CustomEncoder".to_string(),
            name: "4K Stream".to_string(),
            resolution: Resolution::new(3840, 2160),
            framerate: 30,
            bitrate: 8000,
            encoding: VideoEncoding::H264,
            bitrate_mode: BitrateMode::Cbr,
            gop_length: 60,
            quality: 90,
        };

        let platform = StubPlatformBuilder::new()
            .video_encoder(custom_encoder)
            .build();

        let encoder = platform.video_encoder();
        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].token, "CustomEncoder");
        assert_eq!(configs[0].resolution.width, 3840);
    }

    #[tokio::test]
    async fn test_stub_platform_builder_audio_source() {
        let custom_source = AudioSourceConfig {
            token: "CustomAudioSource".to_string(),
            name: "Stereo Mic".to_string(),
            channels: 2,
        };

        let platform = StubPlatformBuilder::new()
            .audio_source(custom_source)
            .build();

        let audio = platform.audio_input();
        let sources = audio.get_sources().await.unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].token, "CustomAudioSource");
        assert_eq!(sources[0].channels, 2);
    }

    #[tokio::test]
    async fn test_stub_platform_builder_audio_encoder() {
        let custom_encoder = AudioEncoderConfig {
            token: "CustomAudioEncoder".to_string(),
            name: "High Quality Audio".to_string(),
            sample_rate: 48000,
            channels: 2,
            encoding: AudioEncoding::Aac,
            bitrate: 256,
        };

        let platform = StubPlatformBuilder::new()
            .audio_encoder(custom_encoder)
            .build();

        let encoder = platform.audio_encoder();
        let configs = encoder.get_configurations().await.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].token, "CustomAudioEncoder");
        assert_eq!(configs[0].sample_rate, 48000);
    }

    #[tokio::test]
    async fn test_stub_platform_builder_ptz_position() {
        let custom_position = PtzPosition::new(45.0, -20.0, 2.0);

        let platform = StubPlatformBuilder::new()
            .ptz_supported(true) // Need to enable PTZ support
            .ptz_position(custom_position)
            .build();

        let ptz = platform.ptz_control().unwrap();
        let pos = ptz.get_position().await.unwrap();
        assert_eq!(pos.pan, 45.0);
        assert_eq!(pos.tilt, -20.0);
        assert_eq!(pos.zoom, 2.0);
    }

    #[tokio::test]
    async fn test_stub_platform_builder_ptz_preset() {
        let preset1 = PtzPreset {
            token: "preset1".to_string(),
            name: "Entrance".to_string(),
            position: PtzPosition::new(0.0, 0.0, 1.0),
        };
        let preset2 = PtzPreset {
            token: "preset2".to_string(),
            name: "Parking Lot".to_string(),
            position: PtzPosition::new(90.0, -10.0, 1.5),
        };

        let platform = StubPlatformBuilder::new()
            .ptz_supported(true) // Need to enable PTZ support
            .ptz_preset(preset1)
            .ptz_preset(preset2)
            .build();

        let ptz = platform.ptz_control().unwrap();
        let presets = ptz.get_presets().await.unwrap();
        assert_eq!(presets.len(), 2);
        // Presets are stored in a HashMap, so order is not guaranteed
        let preset_names: Vec<&str> = presets.iter().map(|p| p.name.as_str()).collect();
        assert!(preset_names.contains(&"Entrance"));
        assert!(preset_names.contains(&"Parking Lot"));
    }

    #[tokio::test]
    async fn test_stub_platform_builder_ptz_limits() {
        let custom_limits = PtzLimits {
            min_pan: -90.0,
            max_pan: 90.0,
            min_tilt: -45.0,
            max_tilt: 45.0,
            min_zoom: 1.0,
            max_zoom: 10.0,
        };

        let platform = StubPlatformBuilder::new()
            .ptz_supported(true) // Need to enable PTZ support
            .ptz_limits(custom_limits)
            .build();

        let ptz = platform.ptz_control().unwrap();
        let limits = ptz.get_limits().await.unwrap();
        assert_eq!(limits.max_pan, 90.0);
        assert_eq!(limits.max_zoom, 10.0);
    }

    #[tokio::test]
    async fn test_stub_platform_builder_imaging_settings() {
        let custom_settings = ImagingSettings {
            brightness: 75.0,
            contrast: 60.0,
            saturation: 55.0,
            sharpness: 50.0,
            ir_cut_filter: true,
            ir_led: false,
            wdr: true,
            backlight_compensation: false,
        };

        let platform = StubPlatformBuilder::new()
            .imaging_supported(true) // Need to enable imaging support
            .imaging_settings(custom_settings)
            .build();

        let imaging = platform.imaging_control().unwrap();
        let settings = imaging.get_settings().await.unwrap();
        assert_eq!(settings.brightness, 75.0);
        assert_eq!(settings.contrast, 60.0);
    }

    #[test]
    fn test_stub_platform_builder_ptz_supported() {
        let platform = StubPlatformBuilder::new().ptz_supported(false).build();

        assert!(platform.ptz_control().is_none());
    }

    #[test]
    fn test_stub_platform_builder_imaging_supported() {
        let platform = StubPlatformBuilder::new().imaging_supported(false).build();

        assert!(platform.imaging_control().is_none());
    }

    #[tokio::test]
    async fn test_stub_platform_builder_fail_init() {
        let platform = StubPlatformBuilder::new().fail_init(true).build();

        let result = platform.initialize().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_stub_platform_builder_combined() {
        // Test building a platform with multiple customizations
        let platform = StubPlatformBuilder::new()
            .device_info(DeviceInfo {
                manufacturer: "Test".to_string(),
                model: "TestModel".to_string(),
                firmware_version: "1.0.0".to_string(),
                serial_number: "TEST-001".to_string(),
                hardware_id: "test-hw".to_string(),
            })
            .ptz_supported(true)
            .imaging_supported(true)
            .ptz_position(PtzPosition::new(10.0, 20.0, 1.0))
            .build();

        let info = platform.get_device_info().await.unwrap();
        assert_eq!(info.manufacturer, "Test");

        assert!(platform.ptz_control().is_some());
        assert!(platform.imaging_control().is_some());
    }
}
