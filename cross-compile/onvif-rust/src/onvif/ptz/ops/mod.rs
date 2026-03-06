//! PTZ Service operation modules.
//!
//! This module organizes PTZ operations into focused sub-modules:
//! - [`movement`] - Absolute, relative, continuous movement and stop
//! - [`presets`] - Preset management (get, set, goto, remove)
//! - [`config`] - Configuration and node operations
//! - [`status`] - Status and home position operations
//! - [`auxiliary`] - Auxiliary commands and capabilities

pub mod auxiliary;
pub mod config;
pub mod movement;
pub mod presets;
pub mod status;
