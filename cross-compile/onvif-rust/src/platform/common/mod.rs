//! Shared platform abstractions: traits, frame types, and utility functions.
//!
//! This module contains the common types and traits used across all platform
//! implementations (Anyka hardware, stubs, validation, etc.).

pub mod frame;
pub mod network;
pub mod traits;

// Re-export commonly used items
pub use frame::*;
pub use network::*;
pub use traits::*;
