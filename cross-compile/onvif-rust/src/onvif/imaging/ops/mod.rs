//! Imaging Service operation handlers organized by domain.
//!
//! This module contains the operation handlers split by logical domain:
//! - `settings` - GetImagingSettings, SetImagingSettings, GetOptions
//! - `status` - GetStatus
//! - `focus` - Move, Stop, GetMoveOptions
//! - `capabilities` - GetServiceCapabilities
//! - `presets` - GetPresets, GetCurrentPreset, SetCurrentPreset

pub mod capabilities;
pub mod error;
pub mod focus;
pub mod presets;
pub mod settings;
pub mod status;
