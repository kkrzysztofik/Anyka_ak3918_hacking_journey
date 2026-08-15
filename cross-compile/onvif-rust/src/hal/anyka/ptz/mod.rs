//! Anyka PTZ (Pan-Tilt-Zoom) hardware driver submodule.
//!
//! Contains the native Rust ioctl driver for `/dev/ak-motor*` devices, which
//! implements `PtzHalTrait` directly.

pub mod driver;

// Re-export key types for external consumers
pub use driver::{NativePtzDriver, ptz_device, ptz_feedback_pin, ptz_turn_direction};
