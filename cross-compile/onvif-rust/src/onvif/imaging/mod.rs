//! ONVIF Imaging Service implementation.
//!
//! This module implements the ONVIF Imaging Service (timg namespace) providing:
//! - Image settings management (GetImagingSettings, SetImagingSettings)
//! - Options retrieval (GetOptions)
//! - Status monitoring (GetStatus)
//! - Focus control (Move, Stop, GetMoveOptions)
//! - Service capabilities (GetServiceCapabilities)
//!
//! # Module Structure
//!
//! The service is organized into domain-specific operation modules:
//! - [`ops::settings`] - GetImagingSettings, SetImagingSettings, GetOptions
//! - [`ops::status`] - GetStatus
//! - [`ops::focus`] - Move, Stop, GetMoveOptions
//! - [`ops::capabilities`] - GetServiceCapabilities
//! - [`ops::presets`] - GetPresets, GetCurrentPreset, SetCurrentPreset
//!
//! # Focus Operations
//!
//! The imaging service supports focus control when the hardware supports it:
//! - `Move` - Perform absolute, relative, or continuous focus movement
//! - `Stop` - Stop ongoing focus movement
//! - `GetMoveOptions` - Get supported focus move options
//!
//! # Error Handling
//!
//! Imaging Service operations return ONVIF-compliant SOAP faults:
//! - `ter:InvalidToken` - Unknown video source token
//! - `ter:InvalidArgVal` - Invalid parameter value
//! - `ter:ActionNotSupported` - Focus operations not supported
//! - `ter:HardwareFailure` - Hardware error during operation

pub mod faults;
pub mod ops;
pub mod service;
pub mod state;
pub mod store;
pub mod types;
pub mod validation;

pub use service::ImagingService;
pub use store::ImagingSettingsStore;
pub use types::*;

// Re-export WSDL types for imaging
pub use crate::onvif::types::imaging::{
    GetImagingSettings, GetImagingSettingsResponse, GetMoveOptions, GetMoveOptionsResponse,
    GetOptions, GetOptionsResponse, GetServiceCapabilities, GetServiceCapabilitiesResponse,
    GetStatus, GetStatusResponse, ImagingOptions20, ImagingServiceCapabilities, Move,
    MoveOptions20, MoveResponse, SetImagingSettings, SetImagingSettingsResponse, Stop,
    StopResponse,
};
