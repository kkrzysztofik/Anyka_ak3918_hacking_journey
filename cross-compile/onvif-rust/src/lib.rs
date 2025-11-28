#![deny(unsafe_op_in_unsafe_fn)]

//! Library root for the ONVIF Rust rewrite.
//!
//! This crate provides a complete ONVIF 2.5 implementation for Anyka AK3918-based
//! IP cameras. The implementation follows a "kinda-hybrid" lifecycle approach:
//!
//! - **Explicit `start()`/`shutdown()`**: Ordered async initialization and graceful shutdown
//! - **No global state**: All state owned by the `Application` struct
//! - **Dependency injection**: Components receive dependencies via constructors
//! - **Graceful degradation**: Optional components can fail without stopping the app
//!
//! # Quick Start
//!
//! ```ignore
//! use onvif_rust::app::Application;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let app = Application::start("/etc/onvif/config.toml").await?;
//!     app.run().await?;
//!     let report = app.shutdown().await;
//!     Ok(())
//! }
//! ```
//!
//! # Module Organization
//!
//! - [`app`] - Main Application struct with lifecycle management
//! - [`lifecycle`] - Startup, shutdown, and health check utilities
//! - [`config`] - Configuration system (TOML-based)
//! - [`platform`] - Platform abstraction for hardware access
//! - [`onvif`] - ONVIF service implementations
//! - [`auth`] - Authentication (WS-Security, HTTP Digest)
//! - [`security`] - Security hardening (rate limiting, brute force protection)

pub mod app;
pub mod lifecycle;

pub mod config {
    //! Configuration system placeholder.
    //!
    //! Will be implemented in tasks T051-T065.
}

pub mod logging {
    //! Logging subsystem placeholder.
    //!
    //! Will be implemented in tasks T066-T071.
}

pub mod platform {
    //! Platform abstraction placeholder.
    //!
    //! Will be implemented in tasks T034-T050.
}

pub mod onvif {
    //! HTTP/SOAP server placeholder.
    //!
    //! Will be implemented in tasks T072-T088 and Phases 6-9.
}

pub mod utils {
    //! Utilities placeholder.
    //!
    //! Will be implemented in tasks T100-T111.
}

pub mod auth {
    //! Authentication placeholder.
    //!
    //! Will be implemented in tasks T112-T130.
}

pub mod users {
    //! User management placeholder.
    //!
    //! Will be implemented in tasks T155-T185.
}

pub mod discovery {
    //! WS-Discovery placeholder.
    //!
    //! Will be implemented in tasks T190-T203.
}

pub mod security {
    //! Security hardening placeholder.
    //!
    //! Will be implemented in tasks T131-T154.
}

pub mod ffi {
    //! Anyka SDK FFI placeholder.
    //!
    //! Will be implemented in tasks T023-T033.
}

// Re-export main types for convenience
pub use app::Application;
pub use lifecycle::{RuntimeError, ShutdownReport, ShutdownStatus, StartupError};
