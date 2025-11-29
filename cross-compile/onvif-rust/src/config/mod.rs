//! Configuration system for the ONVIF application.
//!
//! This module provides a robust configuration system that:
//!
//! - Loads configuration from TOML files
//! - Validates configuration against a schema
//! - Provides thread-safe runtime access
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
//! - `[stream_profile_N]` - Stream profile configurations (1-4)
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::config::{ConfigRuntime, ConfigStorage};
//!
//! let config = ConfigStorage::load("/etc/onvif/config.toml")?;
//! let runtime = ConfigRuntime::new(config);
//!
//! let port = runtime.get_int("server.port")?;
//! runtime.set_int("server.port", 8080)?;
//! ```

mod runtime;
mod schema;
mod storage;

pub use runtime::*;
pub use schema::*;
pub use storage::*;
