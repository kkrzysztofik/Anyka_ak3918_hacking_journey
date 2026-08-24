//! On-screen display: camera name and timestamp burned into the video.
//!
//! Policy lives here rather than in the C daemon — see
//! docs/plans/2026-08-24-osd-overlay-design.md.

pub mod encode;
pub mod format;
pub mod layout;
