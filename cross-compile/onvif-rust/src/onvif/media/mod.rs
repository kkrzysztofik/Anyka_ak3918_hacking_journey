//! ONVIF Media Service implementation.
//!
//! This module implements the ONVIF Media Service (trt namespace) providing:
//! - Profile management (GetProfiles, GetProfile, CreateProfile, DeleteProfile)
//! - Video source management (GetVideoSources, GetVideoSourceConfigurations)
//! - Video encoder configuration (GetVideoEncoderConfigurations, SetVideoEncoderConfiguration)
//! - Audio source/encoder configuration
//! - Stream URI generation (GetStreamUri)
//! - Snapshot URI generation (GetSnapshotUri)
//!
//! # Architecture
//!
//! The Media service is organized into the following modules:
//! - `service.rs` - Main MediaService struct and SOAP handler implementation
//! - `ops/` - Domain-specific operation handlers
//!   - `profiles.rs` - Profile management operations
//!   - `video_sources.rs` - Video source operations
//!   - `video_encoders.rs` - Video encoder operations
//!   - `audio.rs` - Audio operations
//!   - `streaming.rs` - Stream URI operations
//!   - `capabilities.rs` - Service capabilities
//! - `state.rs` - In-memory state management with RwLock
//! - `store.rs` - Persistence and data conversion
//! - `validation.rs` - Input validation functions
//! - `faults.rs` - ONVIF fault codes and error helpers
//! - `types.rs` - Type constants and definitions
//! - `profile_manager.rs` - ProfileManager facade
//!
//! # User Stories
//!
//! **User Story 2 (P1)**: Media Service operations for profile and stream configuration.
//!
//! # Profile Management
//!
//! Media profiles define the configuration of media streaming. Each profile contains:
//! - Video source configuration (camera source selection)
//! - Video encoder configuration (codec, resolution, bitrate)
//! - Audio source/encoder configuration (optional)
//! - PTZ configuration (optional)
//!
//! # Error Handling
//!
//! Media Service operations return ONVIF-compliant SOAP faults:
//! - `ter:NoProfile` - Profile token not found
//! - `ter:NoSource` - Source token not found
//! - `ter:NoConfig` - Configuration token not found
//! - `ter:ConfigModify` - Cannot modify fixed configuration
//! - `ter:InvalidArgVal` - Invalid argument value
//! - `ter:ConfigurationConflict` - Configuration conflict detected

mod defaults;
pub mod faults;
pub(crate) mod ops;
pub mod profile_manager;
pub mod service;
pub(crate) mod state;
pub(crate) mod store;
pub mod types;
pub(crate) mod validation;

// Re-export for public API
pub use profile_manager::ProfileManager;
pub use service::MediaService;
pub use types::*;

// Re-export WSDL types for media operations
pub use crate::onvif::types::media::{
    // Profile configuration operations
    AddAudioEncoderConfiguration,
    AddAudioEncoderConfigurationResponse,
    AddAudioSourceConfiguration,
    AddAudioSourceConfigurationResponse,
    AddVideoEncoderConfiguration,
    AddVideoEncoderConfigurationResponse,
    AddVideoSourceConfiguration,
    AddVideoSourceConfigurationResponse,
    // Profile operations
    CreateProfile,
    CreateProfileResponse,
    DeleteProfile,
    DeleteProfileResponse,
    // Audio encoder operations
    GetAudioEncoderConfiguration,
    GetAudioEncoderConfigurationOptions,
    GetAudioEncoderConfigurationOptionsResponse,
    GetAudioEncoderConfigurationResponse,
    GetAudioEncoderConfigurations,
    GetAudioEncoderConfigurationsResponse,
    // Audio source operations
    GetAudioSourceConfiguration,
    GetAudioSourceConfigurationResponse,
    GetAudioSourceConfigurations,
    GetAudioSourceConfigurationsResponse,
    GetAudioSources,
    GetAudioSourcesResponse,
    GetProfile,
    GetProfileResponse,
    GetProfiles,
    GetProfilesResponse,
    // Service capabilities
    GetServiceCapabilities,
    GetServiceCapabilitiesResponse,
    // Stream URI operations
    GetSnapshotUri,
    GetSnapshotUriResponse,
    GetStreamUri,
    GetStreamUriResponse,
    // Video encoder operations
    GetVideoEncoderConfiguration,
    GetVideoEncoderConfigurationOptions,
    GetVideoEncoderConfigurationOptionsResponse,
    GetVideoEncoderConfigurationResponse,
    GetVideoEncoderConfigurations,
    GetVideoEncoderConfigurationsResponse,
    // Video source operations
    GetVideoSourceConfiguration,
    GetVideoSourceConfigurationOptions,
    GetVideoSourceConfigurationOptionsResponse,
    GetVideoSourceConfigurationResponse,
    GetVideoSourceConfigurations,
    GetVideoSourceConfigurationsResponse,
    GetVideoSources,
    GetVideoSourcesResponse,
    MediaServiceCapabilities,
    ProfileCapabilities,
    RemoveAudioEncoderConfiguration,
    RemoveAudioEncoderConfigurationResponse,
    RemoveAudioSourceConfiguration,
    RemoveAudioSourceConfigurationResponse,
    RemoveVideoEncoderConfiguration,
    RemoveVideoEncoderConfigurationResponse,
    RemoveVideoSourceConfiguration,
    RemoveVideoSourceConfigurationResponse,
    SetAudioEncoderConfiguration,
    SetAudioEncoderConfigurationResponse,
    SetVideoEncoderConfiguration,
    SetVideoEncoderConfigurationResponse,
    SetVideoSourceConfiguration,
    SetVideoSourceConfigurationResponse,
    StreamingCapabilities,
};
pub use crate::onvif::types::media_osd::*;
