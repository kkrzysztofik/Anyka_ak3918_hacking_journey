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
pub mod osd;
pub mod profiles;
pub mod streaming;
pub mod video_encoders;
pub mod video_sources;

use crate::onvif::media::ProfileManager;

/// Type alias for the concrete `ProfileManager` struct — provides an
/// indirection point so every operation signature can be updated in one
/// place should the backing type ever change (e.g. to `Arc<ProfileManager>`).
pub type ProfileManagerRef = ProfileManager;
