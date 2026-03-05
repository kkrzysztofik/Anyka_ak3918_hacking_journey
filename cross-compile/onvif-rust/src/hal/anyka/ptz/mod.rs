//! Anyka PTZ (Pan-Tilt-Zoom) hardware driver submodule.
//!
//! Contains the native Rust ioctl driver for `/dev/ak-motor*` devices
//! and the `PtzHalTrait` implementation that wraps it.

pub mod driver;
#[cfg(not(use_stubs))]
pub(crate) mod native;

// Re-export key types for external consumers
pub use driver::{NativePtzDriver, ptz_device, ptz_feedback_pin, ptz_turn_direction};
