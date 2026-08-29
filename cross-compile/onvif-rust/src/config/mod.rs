//! Configuration system for the ONVIF application.
//!
//! This module provides a robust configuration system that:
//!
//! - Loads configuration from TOML files using serde
//! - Validates configuration against typed constraints
//! - Provides thread-safe runtime access via read/write guards
//! - Supports atomic configuration updates
//!
//! # Configuration Sections
//!
//! - `[onvif]` - ONVIF protocol settings
//! - `[network]` - Network configuration
//! - `[device]` - Device information
//! - `[server]` - HTTP server settings
//! - `[logging]` - Logging configuration
//! - `[media]` - Media stream settings
//! - `[ptz]` - PTZ control settings
//! - `[imaging]` - Imaging settings
//! - `[discovery]` - WS-Discovery settings
//! - `[memory]` - Memory management settings
//! - `[stream_profile_N]` - Stream profile configurations (1-4)
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::config::{ConfigRuntime, ConfigStorage, AppConfig};
//!
//! let config = ConfigStorage::load_or_default("/etc/onvif/config.toml")?;
//! let runtime = ConfigRuntime::new(config);
//!
//! let port = runtime.read().server.port;
//! runtime.write().server.port = 8080;
//! ```

pub(crate) mod file_ops;
pub mod netoverlay;
mod persistence;
pub mod profiles;
mod runtime;
pub mod snmp;
pub mod sound;
mod storage;
pub mod types;
pub mod users;

pub use persistence::*;
pub use profiles::{ProfileError, ProfileStorage, ProfilesFile};
pub use runtime::*;
pub use storage::*;
pub use types::AppConfig;
pub use users::{
    MAX_USERS, PasswordError, PasswordManager, UserAccount, UserError, UserLevel, UserLoadStatus,
    UserStorage,
};
