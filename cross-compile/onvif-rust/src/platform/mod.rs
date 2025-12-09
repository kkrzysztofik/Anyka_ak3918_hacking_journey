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

mod stubs;
mod traits;

// Conditional compilation for Anyka implementation
#[cfg(not(use_stubs))]
mod anyka;

pub use stubs::*;
pub use traits::*;

#[cfg(not(use_stubs))]
pub use anyka::*;
