//! Media Service Operations.
//!
//! This module organizes ONVIF Media Service operations by domain:
//! - `profiles` - Profile management (GetProfiles, CreateProfile, DeleteProfile)
//! - `video_sources` - Video source and configuration management
//! - `video_encoders` - Video encoder configuration management
//! - `audio` - Audio source and encoder configuration management
//! - `streaming` - Stream and snapshot URI generation
//! - `capabilities` - Service capabilities

pub mod audio;
pub mod capabilities;
pub mod profiles;
pub mod streaming;
pub mod video_encoders;
pub mod video_sources;

// Re-export validation from parent module for use by operations
pub use crate::onvif::media::validation;

use crate::onvif::media::ProfileManager;

/// Type alias for ProfileManager reference.
/// This allows operations to work with both Arc<ProfileManager> and &ProfileManager.
pub type ProfileManagerRef = ProfileManager;
