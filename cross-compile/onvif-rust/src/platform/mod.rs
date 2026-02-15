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

pub mod frame;
pub(crate) mod hw_ptz;
mod stubs;
mod traits;
pub mod validation;

// Anyka implementation is always compiled so unit tests can exercise it
// with MockVideoFfiTrait, similar to hw_ptz.rs. Only the public re-export
// is gated to avoid name conflicts with stubs in native builds.
#[allow(dead_code)]
mod anyka;

pub use stubs::*;
pub use traits::*;
pub use validation::ValidationPlatform;

#[cfg(not(use_stubs))]
pub use anyka::*;
