//! Anyka platform implementation.
//!
//! This module provides the actual platform implementation using
//! the Anyka SDK for the AK3918 camera hardware.
//!
//! This implementation is only compiled when cross-compiling for ARM
//! (i.e., when `use_stubs` is not defined).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use parking_lot::RwLock;

use super::traits::{
    AudioEncoder, AudioEncoderConfig, AudioInput, AudioSourceConfig, DeviceInfo, DnsInfo,
    ImagingControl, ImagingOptions, ImagingSettings, NetworkInfo, NetworkInterfaceInfo,
    NetworkProtocolInfo, NtpInfo, PTZControl, Platform, PlatformError, PlatformResult, PtzLimits,
    PtzPosition, PtzPreset, PtzVelocity, Resolution, VideoEncoder, VideoEncoderConfig,
    VideoEncoderOptions, VideoEncoding, VideoInput, VideoSourceConfig,
};

/// Anyka platform implementation using the actual SDK.
///
/// This implementation wraps the Anyka SDK FFI calls and provides
/// a safe Rust interface to the hardware.
///
/// # Note
///
/// Currently this is a minimal stub. The actual SDK integration will be
/// implemented in Phase 5 when we connect to the real FFI bindings.
pub struct AnykaPlatform {
    initialized: AtomicBool,
    device_info: DeviceInfo,
    video_input: Arc<AnykaVideoInput>,
    video_encoder: Arc<AnykaVideoEncoder>,
    audio_input: Arc<AnykaAudioInput>,
    audio_encoder: Arc<AnykaAudioEncoder>,
    ptz_control: Option<Arc<dyn PTZControl>>,
    imaging_control: Option<Arc<dyn ImagingControl>>,
    network_info: Option<Arc<dyn NetworkInfo>>,
}

impl AnykaPlatform {
    /// Create a new Anyka platform instance.
    pub fn new() -> PlatformResult<Self> {
        let device_info = DeviceInfo {
            manufacturer: "Anyka".to_string(),
            model: "AK3918".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "AK3918-001".to_string(),
            hardware_id: "ak3918-hw".to_string(),
        };

        let video_input = Arc::new(AnykaVideoInput::new());
        let video_encoder = Arc::new(AnykaVideoEncoder::new());
        let audio_input = Arc::new(AnykaAudioInput::new());
        let audio_encoder = Arc::new(AnykaAudioEncoder::new());
        let ptz_control = Some(Arc::new(AnykaPTZControl::new()) as Arc<dyn PTZControl>);
        let imaging_control = Some(Arc::new(AnykaImagingControl::new()) as Arc<dyn ImagingControl>);
        let network_info = Some(Arc::new(AnykaNetworkInfo::new()) as Arc<dyn NetworkInfo>);

        Ok(Self {
            initialized: AtomicBool::new(false),
            device_info,
            video_input,
            video_encoder,
            audio_input,
            audio_encoder,
            ptz_control,
            imaging_control,
            network_info,
        })
    }
}

impl Default for AnykaPlatform {
    fn default() -> Self {
        Self::new().expect("Failed to create AnykaPlatform")
    }
}

#[async_trait]
impl Platform for AnykaPlatform {
    async fn get_device_info(&self) -> PlatformResult<DeviceInfo> {
        // TODO: Read actual device info from Anyka SDK
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
        // TODO: Call Anyka SDK initialization functions via FFI
        // - ak_vi_open()
        // - ak_venc_open()
        // - ak_ai_open()
        // - ak_aenc_open()
        // - ak_drv_ptz_open()
        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn shutdown(&self) -> PlatformResult<()> {
        // TODO: Call Anyka SDK cleanup functions via FFI
        // - ak_vi_close()
        // - ak_venc_close()
        // - ak_ai_close()
        // - ak_aenc_close()
        // - ak_drv_ptz_close()
        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }
}

// =============================================================================
// Video Input Implementation
// =============================================================================

/// Anyka video input implementation.
struct AnykaVideoInput {
    opened: AtomicBool,
}

impl AnykaVideoInput {
    fn new() -> Self {
        Self {
            opened: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl VideoInput for AnykaVideoInput {
    async fn open(&self) -> PlatformResult<()> {
        // TODO: Call ak_vi_open() via FFI
        self.opened.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        // TODO: Call ak_vi_close() via FFI
        self.opened.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn get_resolution(&self) -> PlatformResult<Resolution> {
        // TODO: Get actual resolution from ak_vi_get_sensor_resolution()
        Ok(Resolution::new(1920, 1080))
    }

    async fn get_sources(&self) -> PlatformResult<Vec<VideoSourceConfig>> {
        // TODO: Query actual video sources from Anyka SDK
        Ok(vec![VideoSourceConfig {
            token: "VideoSource_1".to_string(),
            name: "Main Camera".to_string(),
            resolution: Resolution::new(1920, 1080),
            max_framerate: 30.0,
        }])
    }
}

// =============================================================================
// Video Encoder Implementation
// =============================================================================

/// Anyka video encoder implementation.
struct AnykaVideoEncoder {
    configurations: RwLock<Vec<VideoEncoderConfig>>,
}

impl AnykaVideoEncoder {
    fn new() -> Self {
        Self {
            configurations: RwLock::new(vec![
                VideoEncoderConfig {
                    token: "VideoEncoder_1".to_string(),
                    name: "Main Stream".to_string(),
                    resolution: Resolution::new(1920, 1080),
                    framerate: 25,
                    bitrate: 4000,
                    encoding: VideoEncoding::H264,
                    ..Default::default()
                },
                VideoEncoderConfig {
                    token: "VideoEncoder_2".to_string(),
                    name: "Sub Stream".to_string(),
                    resolution: Resolution::new(640, 480),
                    framerate: 15,
                    bitrate: 512,
                    encoding: VideoEncoding::H264,
                    ..Default::default()
                },
            ]),
        }
    }
}

#[async_trait]
impl VideoEncoder for AnykaVideoEncoder {
    async fn init(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        // TODO: Call ak_venc_open() with actual config via FFI
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
            .ok_or_else(|| PlatformError::HardwareUnavailable("No encoder configured".to_string()))
    }

    async fn set_configuration(&self, config: &VideoEncoderConfig) -> PlatformResult<()> {
        // TODO: Call ak_venc_set_rc() or similar via FFI
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown encoder token: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<VideoEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }

    async fn get_options(&self) -> PlatformResult<VideoEncoderOptions> {
        // TODO: Query actual hardware capabilities
        Ok(VideoEncoderOptions {
            resolutions: vec![
                Resolution::new(1920, 1080),
                Resolution::new(1280, 720),
                Resolution::new(640, 480),
            ],
            encodings: vec![VideoEncoding::H264],
            framerate_range: (1, 30),
            bitrate_range: (128, 8000),
            gop_range: (1, 300),
            quality_range: (0, 100),
        })
    }
}

// =============================================================================
// Audio Input Implementation
// =============================================================================

/// Anyka audio input implementation.
struct AnykaAudioInput {
    opened: AtomicBool,
}

impl AnykaAudioInput {
    fn new() -> Self {
        Self {
            opened: AtomicBool::new(false),
        }
    }
}

#[async_trait]
impl AudioInput for AnykaAudioInput {
    async fn open(&self) -> PlatformResult<()> {
        // TODO: Call ak_ai_open() via FFI
        self.opened.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn close(&self) -> PlatformResult<()> {
        // TODO: Call ak_ai_close() via FFI
        self.opened.store(false, Ordering::SeqCst);
        Ok(())
    }

    async fn get_configuration(&self) -> PlatformResult<AudioSourceConfig> {
        // TODO: Get actual audio config from Anyka SDK
        Ok(AudioSourceConfig {
            token: "AudioSource_1".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        })
    }

    async fn get_sources(&self) -> PlatformResult<Vec<AudioSourceConfig>> {
        // TODO: Query actual audio sources
        Ok(vec![AudioSourceConfig {
            token: "AudioSource_1".to_string(),
            name: "Microphone".to_string(),
            channels: 1,
        }])
    }
}

// =============================================================================
// Audio Encoder Implementation
// =============================================================================

/// Anyka audio encoder implementation.
struct AnykaAudioEncoder {
    configurations: RwLock<Vec<AudioEncoderConfig>>,
}

impl AnykaAudioEncoder {
    fn new() -> Self {
        Self {
            configurations: RwLock::new(vec![AudioEncoderConfig {
                token: "AudioEncoder_1".to_string(),
                name: "Audio Stream".to_string(),
                sample_rate: 8000,
                channels: 1,
                ..Default::default()
            }]),
        }
    }
}

#[async_trait]
impl AudioEncoder for AnykaAudioEncoder {
    async fn init(&self, config: &AudioEncoderConfig) -> PlatformResult<()> {
        // TODO: Call ak_aenc_open() with actual config via FFI
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
        // TODO: Call ak_aenc_set_config() or similar via FFI
        let mut configs = self.configurations.write();
        if let Some(cfg) = configs.iter_mut().find(|c| c.token == config.token) {
            *cfg = config.clone();
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown audio encoder token: {}",
                config.token
            )))
        }
    }

    async fn get_configurations(&self) -> PlatformResult<Vec<AudioEncoderConfig>> {
        Ok(self.configurations.read().clone())
    }
}

// =============================================================================
// PTZ Control Implementation
// =============================================================================

/// Anyka PTZ control implementation.
struct AnykaPTZControl {
    position: RwLock<PtzPosition>,
    velocity: RwLock<PtzVelocity>,
    presets: RwLock<std::collections::HashMap<String, PtzPreset>>,
    next_preset_id: RwLock<u32>,
}

impl AnykaPTZControl {
    fn new() -> Self {
        Self {
            position: RwLock::new(PtzPosition::HOME),
            velocity: RwLock::new(PtzVelocity::STOP),
            presets: RwLock::new(std::collections::HashMap::new()),
            next_preset_id: RwLock::new(1),
        }
    }
}

#[async_trait]
impl PTZControl for AnykaPTZControl {
    async fn move_to_position(&self, position: PtzPosition) -> PlatformResult<()> {
        // TODO: Call ak_drv_ptz_turn() via FFI with appropriate direction
        *self.position.write() = position;
        *self.velocity.write() = PtzVelocity::STOP;
        Ok(())
    }

    async fn get_position(&self) -> PlatformResult<PtzPosition> {
        // TODO: Call ak_drv_ptz_get_step_pos() via FFI
        Ok(*self.position.read())
    }

    async fn continuous_move(&self, velocity: PtzVelocity) -> PlatformResult<()> {
        // TODO: Call ak_drv_ptz_turn() via FFI with continuous mode
        *self.velocity.write() = velocity;
        Ok(())
    }

    async fn stop(&self) -> PlatformResult<()> {
        // TODO: Call ak_drv_ptz_stop() via FFI
        *self.velocity.write() = PtzVelocity::STOP;
        Ok(())
    }

    async fn get_presets(&self) -> PlatformResult<Vec<PtzPreset>> {
        Ok(self.presets.read().values().cloned().collect())
    }

    async fn set_preset(&self, name: &str) -> PlatformResult<String> {
        let mut presets = self.presets.write();
        let mut next_id = self.next_preset_id.write();
        let token = format!("preset_{}", *next_id);
        *next_id += 1;

        let position = *self.position.read();
        presets.insert(
            token.clone(),
            PtzPreset {
                token: token.clone(),
                name: name.to_string(),
                position,
            },
        );

        Ok(token)
    }

    async fn goto_preset(&self, token: &str) -> PlatformResult<()> {
        let presets = self.presets.read();
        if let Some(preset) = presets.get(token) {
            *self.position.write() = preset.position;
            Ok(())
        } else {
            Err(PlatformError::InvalidParameter(format!(
                "Unknown preset: {}",
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
                "Unknown preset: {}",
                token
            )))
        }
    }

    async fn get_limits(&self) -> PlatformResult<PtzLimits> {
        // TODO: Query actual PTZ limits from hardware
        Ok(PtzLimits::DEFAULT)
    }
}

// =============================================================================
// Imaging Control Implementation
// =============================================================================

/// Anyka imaging control implementation.
struct AnykaImagingControl {
    settings: RwLock<ImagingSettings>,
}

impl AnykaImagingControl {
    fn new() -> Self {
        Self {
            settings: RwLock::new(ImagingSettings {
                brightness: 50.0,
                contrast: 50.0,
                saturation: 50.0,
                sharpness: 50.0,
                ir_cut_filter: true,
                ir_led: false,
                wdr: false,
                backlight_compensation: false,
            }),
        }
    }
}

#[async_trait]
impl ImagingControl for AnykaImagingControl {
    async fn get_settings(&self) -> PlatformResult<ImagingSettings> {
        // TODO: Read actual settings from Anyka imaging SDK
        Ok(self.settings.read().clone())
    }

    async fn set_settings(&self, settings: &ImagingSettings) -> PlatformResult<()> {
        // TODO: Apply settings via Anyka imaging SDK
        *self.settings.write() = settings.clone();
        Ok(())
    }

    async fn get_options(&self) -> PlatformResult<ImagingOptions> {
        // TODO: Query actual hardware capabilities
        Ok(ImagingOptions::default_options())
    }

    async fn set_brightness(&self, value: f32) -> PlatformResult<()> {
        // TODO: Call Anyka imaging SDK
        self.settings.write().brightness = value;
        Ok(())
    }

    async fn set_contrast(&self, value: f32) -> PlatformResult<()> {
        // TODO: Call Anyka imaging SDK
        self.settings.write().contrast = value;
        Ok(())
    }

    async fn set_saturation(&self, value: f32) -> PlatformResult<()> {
        // TODO: Call Anyka imaging SDK
        self.settings.write().saturation = value;
        Ok(())
    }

    async fn set_sharpness(&self, value: f32) -> PlatformResult<()> {
        // TODO: Call Anyka imaging SDK
        self.settings.write().sharpness = value;
        Ok(())
    }
}

// =============================================================================
// Network Info Implementation
// =============================================================================

/// Anyka network information implementation.
///
/// Reads network configuration from the Linux system. Falls back to empty
/// values if system files cannot be read.
struct AnykaNetworkInfo;

impl AnykaNetworkInfo {
    fn new() -> Self {
        Self
    }

    /// Read network interfaces from /sys/class/net and /proc/net/route.
    fn read_interfaces() -> Vec<NetworkInterfaceInfo> {
        use std::fs;
        use std::path::Path;

        let net_dir = Path::new("/sys/class/net");
        let mut interfaces = Vec::new();

        // Try to read available interfaces
        if let Ok(entries) = fs::read_dir(net_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip loopback
                if name == "lo" {
                    continue;
                }

                // Read MAC address
                let mac_path = entry.path().join("address");
                let mac_address = fs::read_to_string(&mac_path)
                    .ok()
                    .map(|s| s.trim().to_uppercase());

                // Read operational state
                let operstate_path = entry.path().join("operstate");
                let enabled = fs::read_to_string(&operstate_path)
                    .map(|s| s.trim() == "up")
                    .unwrap_or(false);

                // Read link speed (in Mbps)
                let speed_path = entry.path().join("speed");
                let link_speed = fs::read_to_string(&speed_path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());

                // Try to get IP address via ip command output parsing
                // This is a simplified approach - real implementation might use netlink
                let (ipv4_address, ipv4_prefix_length, ipv4_dhcp) = Self::read_interface_ip(&name);

                interfaces.push(NetworkInterfaceInfo {
                    token: name.clone(),
                    name,
                    enabled,
                    ipv4_address,
                    ipv4_prefix_length,
                    ipv4_dhcp,
                    mac_address,
                    link_speed,
                });
            }
        }

        interfaces
    }

    /// Read IP address for an interface.
    fn read_interface_ip(interface: &str) -> (Option<String>, Option<u8>, bool) {
        use std::fs;

        // Try to read from /etc/network/interfaces or similar
        // This is a simplified check - in real embedded Linux, DHCP state
        // might be determined differently

        // Check if DHCP is used (look for dhclient lease)
        let dhcp_lease_path = format!("/var/lib/dhcp/dhclient.{}.leases", interface);
        let from_dhcp = std::path::Path::new(&dhcp_lease_path).exists();

        // Try reading from /proc/net/fib_trie or parsing ip addr output
        // For now, try a simple approach via /proc/net/route
        if let Ok(route_content) = fs::read_to_string("/proc/net/route") {
            for line in route_content.lines().skip(1) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 8 && fields[0] == interface {
                    // Parse gateway destination to find interface IP
                    // This is a simplified approach
                    if fields[1] == "00000000" {
                        // Default route - interface has connectivity
                        // Would need more sophisticated parsing for actual IP
                    }
                }
            }
        }

        // For a more complete implementation, we'd use netlink or parse
        // /proc/net/fib_trie, but for now return None (empty will be reported)
        (None, None, from_dhcp)
    }

    /// Read DNS configuration from /etc/resolv.conf.
    fn read_dns_config() -> DnsInfo {
        use std::fs;

        let mut dns_info = DnsInfo::default();

        if let Ok(content) = fs::read_to_string("/etc/resolv.conf") {
            for line in content.lines() {
                let line = line.trim();

                // Skip comments
                if line.starts_with('#') {
                    continue;
                }

                if let Some(domain) = line.strip_prefix("search ") {
                    dns_info
                        .search_domains
                        .extend(domain.split_whitespace().map(String::from));
                } else if let Some(domain) = line.strip_prefix("domain ") {
                    dns_info.search_domains.push(domain.trim().to_string());
                } else if let Some(nameserver) = line.strip_prefix("nameserver ") {
                    let ns = nameserver.trim().to_string();
                    // Assume manual unless we detect DHCP
                    dns_info.dns_manual.push(ns);
                }
            }
        }

        // Check if DNS was obtained via DHCP
        // Simple heuristic: if /etc/resolv.conf was modified by dhclient
        if std::path::Path::new("/var/lib/dhcp/dhclient.leases").exists() {
            dns_info.from_dhcp = true;
            // Move servers to dhcp list
            dns_info.dns_from_dhcp = std::mem::take(&mut dns_info.dns_manual);
        }

        dns_info
    }

    /// Read NTP configuration from /etc/ntp.conf or similar.
    fn read_ntp_config() -> NtpInfo {
        let mut ntp_info = NtpInfo::default();

        if let Some(servers) = Self::parse_ntp_conf() {
            ntp_info.ntp_manual = servers;
        } else if let Some(servers) = Self::parse_timesyncd_conf() {
            ntp_info.ntp_manual = servers;
        }

        ntp_info
    }

    /// Parse /etc/ntp.conf file.
    fn parse_ntp_conf() -> Option<Vec<String>> {
        use std::fs;

        let content = fs::read_to_string("/etc/ntp.conf").ok()?;
        let mut servers = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }

            if let Some(server) = line.strip_prefix("server ") {
                let server = server.split_whitespace().next()?.to_string();
                if !server.is_empty() {
                    servers.push(server);
                }
            }
        }

        if servers.is_empty() {
            None
        } else {
            Some(servers)
        }
    }

    /// Parse /etc/systemd/timesyncd.conf file.
    fn parse_timesyncd_conf() -> Option<Vec<String>> {
        use std::fs;

        let content = fs::read_to_string("/etc/systemd/timesyncd.conf").ok()?;
        let mut servers = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if let Some(servers_str) = line.strip_prefix("NTP=") {
                servers.extend(servers_str.split_whitespace().map(String::from));
            }
        }

        if servers.is_empty() {
            None
        } else {
            Some(servers)
        }
    }
}

#[async_trait]
impl NetworkInfo for AnykaNetworkInfo {
    async fn get_network_interfaces(&self) -> PlatformResult<Vec<NetworkInterfaceInfo>> {
        Ok(Self::read_interfaces())
    }

    async fn get_dns_info(&self) -> PlatformResult<DnsInfo> {
        Ok(Self::read_dns_config())
    }

    async fn get_ntp_info(&self) -> PlatformResult<NtpInfo> {
        Ok(Self::read_ntp_config())
    }

    async fn get_network_protocols(&self) -> PlatformResult<Vec<NetworkProtocolInfo>> {
        // Return the protocols this ONVIF server supports
        // These are typically configured at build/runtime, not read from system
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
}
