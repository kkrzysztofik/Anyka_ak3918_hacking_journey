//! Utility modules for the ONVIF implementation.
//!
//! This module provides common utilities used throughout the codebase:
//!
//! - [`validation`] - XML security validation (XXE, XML bombs, XSS prevention)
//!   and path traversal prevention
//! - [`memory`] - Memory management and allocation tracking with hard limit enforcement
//!
//! # Example: Security Validation
//!
//! ```ignore
//! use onvif_rust::utils::validation::{SecurityValidator, SecurityError};
//!
//! let validator = SecurityValidator::default();
//! validator.validate_xml_payload(xml_bytes)?;
//! ```
//!
//! # Example: Memory Monitoring
//!
//! ```ignore
//! use onvif_rust::utils::memory::MemoryMonitor;
//!
//! let memory = MemoryMonitor::new();
//! memory.check_available(request_size)?;  // Fails if hard limit would be exceeded
//! ```

pub mod memory;
pub mod validation;

// Re-exports for security validation
pub use validation::{PathValidator, SecurityError, SecurityValidator};

// Re-exports for memory management
pub use memory::{MemoryLimits, MemoryMonitor};

// Re-exports for memory profiling (feature-gated)
#[cfg(feature = "memory-profiling")]
pub use memory::{AllocationInfo, AllocationTracker};
