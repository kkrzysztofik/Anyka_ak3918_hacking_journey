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

/// What the rest of the application observes about the hardware attachment.
///
/// Lives here rather than beside the Anyka supervisor that produces it because
/// the ONVIF services that consume it are common code and must also compile in
/// stub builds, where there is no supervisor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// Attached and the hardware pipeline is initialised.
    Available,
    /// Not attached; the supervisor is retrying.
    Unavailable,
    /// The breaker is open. No further attach attempts without intervention.
    GivenUp,
}

impl Availability {
    /// Whether media operations can be expected to produce frames.
    pub fn is_available(self) -> bool {
        matches!(self, Availability::Available)
    }

    /// Short reason for a SOAP fault, or `None` while available.
    ///
    /// `GivenUp` is deliberately distinguishable from `Unavailable`: the first
    /// needs a human, the second resolves on its own.
    pub fn unavailable_reason(self) -> Option<&'static str> {
        match self {
            Availability::Available => None,
            Availability::Unavailable => {
                Some("video pipeline is not attached to the vendor daemon; retrying")
            }
            Availability::GivenUp => Some(
                "video pipeline gave up attaching to the vendor daemon; manual intervention \
                 required",
            ),
        }
    }
}
