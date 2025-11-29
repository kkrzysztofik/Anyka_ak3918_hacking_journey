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

pub mod config;

pub mod logging;

pub mod platform;

pub mod onvif;

pub mod utils;

pub mod auth;

pub mod users;

pub mod discovery;

pub mod security;

pub mod ffi;

// Re-export main types for convenience
pub use app::{AppState, AppStateBuilder, AppStateError, Application};
pub use lifecycle::{RuntimeError, ShutdownReport, ShutdownStatus, StartupError};
