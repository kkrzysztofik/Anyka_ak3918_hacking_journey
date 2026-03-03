//! Platform abstraction layer for hardware access.
//!
//! This module provides a hardware abstraction layer that allows the ONVIF
//! implementation to work with different backends:
//!
//! - **Anyka**: Real hardware implementation using FFI to Anyka SDK
//! - **Stubs**: Mock implementations for testing without hardware
//!
//! # Architecture
//!
//! The platform abstraction uses trait objects to provide runtime polymorphism:
//!
//! ```text
//! Platform trait
//! ├── VideoInput trait
//! ├── VideoEncoder trait
//! ├── AudioInput trait
//! ├── AudioEncoder trait
//! ├── PTZControl trait
//! └── ImagingControl trait
//! ```
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::platform::{Platform, StubPlatform};
//!
//! let platform = StubPlatform::new();
//! let device_info = platform.get_device_info().await?;
//! ```

pub mod common;
pub(crate) mod hw_ptz;
mod stub;
pub mod validation;

// Anyka implementation is always compiled so unit tests can exercise it
// with MockVideoHalTrait, similar to hw_ptz.rs. Only the public re-export
// is gated to avoid name conflicts with stubs in native builds.
#[allow(dead_code)]
mod anyka;

// Re-export common types and traits
pub use common::*;
pub use stub::*;
pub use validation::ValidationPlatform;

#[cfg(not(use_stubs))]
pub use anyka::*;
