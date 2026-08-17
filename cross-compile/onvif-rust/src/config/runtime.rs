//! Runtime configuration manager with thread-safe access.
//!
//! Wraps `AppConfig` in an `Arc<RwLock<_>>` and exposes read/write guards
//! for direct field access, plus a generation counter for change detection
//! by the persistence service.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use portable_atomic::AtomicU64;

use super::types::AppConfig;

/// Thread-safe runtime configuration manager.
///
/// Consumers access fields through `read()` / `write()` guards:
///
/// ```ignore
/// let port = runtime.read().server.port;
/// runtime.write().device.hostname = "my-cam".to_string();
/// ```
///
/// Every `write()` call increments the generation counter so the
/// persistence service knows when to flush to disk.
pub struct ConfigRuntime {
    /// Configuration data protected by RwLock.
    config: Arc<RwLock<AppConfig>>,
    /// Generation counter for change detection.
    generation: AtomicU64,
}

impl ConfigRuntime {
    /// Create a new runtime configuration manager.
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            generation: AtomicU64::new(0),
        }
    }

    /// Acquire a read guard for the configuration.
    ///
    /// Provides direct, typed access to all configuration fields.
    #[inline]
    pub fn read(&self) -> RwLockReadGuard<'_, AppConfig> {
        self.config.read()
    }

    /// Acquire a write guard and bump the generation counter.
    ///
    /// The generation bump signals the persistence service that a save is needed.
    #[inline]
    pub fn write(&self) -> RwLockWriteGuard<'_, AppConfig> {
        self.bump_generation();
        self.config.write()
    }

    /// Get the current configuration generation.
    ///
    /// Compare saved generation with current to detect changes.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Increment the generation counter.
    fn bump_generation(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    /// Get a cloned snapshot of all configuration values.
    pub fn snapshot(&self) -> AppConfig {
        self.config.read().clone()
    }

    /// Replace the entire configuration atomically.
    pub fn replace(&self, config: AppConfig) {
        *self.config.write() = config;
        self.bump_generation();
    }

    /// Serialize current configuration to a TOML string (for backup/restore).
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(&*self.config.read())
    }

    /// Replace configuration from a TOML string (for restore).
    pub fn load_from_toml_string(&self, toml_str: &str) -> Result<(), toml::de::Error> {
        let config: AppConfig = toml::from_str(toml_str)?;
        self.replace(config);
        tracing::info!("Configuration restored from TOML string");
        Ok(())
    }

    /// Log the loaded configuration values at info level.
    pub fn log_loaded_config(&self) {
        let c = self.config.read();
        tracing::info!("=== Loaded Configuration ===");
        tracing::info!(
            "  [server] port={}, bind={}, auth={}",
            c.server.port,
            c.server.bind_address,
            c.server.auth_enabled
        );
        tracing::info!(
            "  [device] {} {} fw={}",
            c.device.manufacturer,
            c.device.model,
            c.device.firmware_version
        );
        tracing::info!(
            "  [network] iface={}, dhcp={}",
            c.network.interface,
            c.network.dhcp_enabled
        );
        tracing::info!(
            "  [media] rtsp_port={}, max_streams={}",
            c.media.rtsp_port,
            c.media.max_streams
        );
        tracing::info!(
            "  [ptz] enabled={}, pan={}, tilt={}",
            c.ptz.enabled,
            c.ptz.pan_speed,
            c.ptz.tilt_speed
        );
        tracing::info!(
            "  [discovery] enabled={}, interval={}s",
            c.discovery.enabled,
            c.discovery.hello_interval
        );
        tracing::info!("  [logging] level={}", c.logging.level);
        tracing::info!(
            "  [memory] hard={}MB, soft={}MB",
            c.memory.hard_limit_mb,
            c.memory.soft_limit_mb
        );
        for (i, profile) in [
            &c.stream_profile_1,
            &c.stream_profile_2,
            &c.stream_profile_3,
            &c.stream_profile_4,
        ]
        .iter()
        .enumerate()
        {
            if profile.enabled {
                tracing::info!(
                    "  [stream_profile_{}] {}x{}@{}fps {}",
                    i + 1,
                    profile.width,
                    profile.height,
                    profile.framerate,
                    profile.encoding
                );
            }
        }
        tracing::info!("=== End Configuration ===");
    }
}

impl Clone for ConfigRuntime {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            generation: AtomicU64::new(self.generation.load(Ordering::SeqCst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_runtime_read_defaults() {
        let runtime = ConfigRuntime::new(AppConfig::default());
        let c = runtime.read();
        assert_eq!(c.server.port, 80);
        assert_eq!(c.device.manufacturer, "");
        assert!(c.server.auth_enabled);
        assert!((c.ptz.pan_speed - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_runtime_write() {
        let runtime = ConfigRuntime::new(AppConfig::default());

        runtime.write().server.port = 8080;
        assert_eq!(runtime.read().server.port, 8080);

        runtime.write().device.manufacturer = "Custom".to_string();
        assert_eq!(runtime.read().device.manufacturer, "Custom");

        runtime.write().server.auth_enabled = false;
        assert!(!runtime.read().server.auth_enabled);

        runtime.write().ptz.pan_speed = 0.75;
        assert!((runtime.read().ptz.pan_speed - 0.75).abs() < f64::EPSILON);
    }

    #[test]
    fn test_config_runtime_generation() {
        let runtime = ConfigRuntime::new(AppConfig::default());

        let gen1 = runtime.generation();
        runtime.write().server.port = 8080;
        let gen2 = runtime.generation();

        assert!(gen2 > gen1);
    }

    #[test]
    fn test_config_runtime_snapshot() {
        let runtime = ConfigRuntime::new(AppConfig::default());
        runtime.write().server.port = 9090;

        let snap = runtime.snapshot();
        assert_eq!(snap.server.port, 9090);
    }

    #[test]
    fn test_config_runtime_replace() {
        let runtime = ConfigRuntime::new(AppConfig::default());

        let mut new_config = AppConfig::default();
        new_config.server.port = 3000;

        let gen1 = runtime.generation();
        runtime.replace(new_config);
        let gen2 = runtime.generation();

        assert!(gen2 > gen1);
        assert_eq!(runtime.read().server.port, 3000);
    }

    #[test]
    fn test_config_runtime_clone_shares_state() {
        let runtime1 = ConfigRuntime::new(AppConfig::default());
        let runtime2 = runtime1.clone();

        runtime1.write().server.port = 4000;
        assert_eq!(runtime2.read().server.port, 4000);
    }

    #[test]
    fn test_config_runtime_to_toml_string() {
        let runtime = ConfigRuntime::new(AppConfig::default());
        let toml_str = runtime.to_toml_string().unwrap();

        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("port = 80"));
        assert!(toml_str.contains("[device]"));
    }

    #[test]
    fn test_config_runtime_load_from_toml_string() {
        let runtime = ConfigRuntime::new(AppConfig::default());

        let toml_str = r#"
[server]
port = 9999

[device]
manufacturer = "Restored"
"#;

        runtime.load_from_toml_string(toml_str).unwrap();
        assert_eq!(runtime.read().server.port, 9999);
        assert_eq!(runtime.read().device.manufacturer, "Restored");
    }
}
