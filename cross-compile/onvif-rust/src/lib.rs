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
//! - [`security`] - Security hardening (rate limiting, brute force protection)
//! - [`validation`] - H264 playback validation and memory monitoring

use cap::Cap;
use std::alloc::System;

/// Global allocator with hard memory limit (24MB) for embedded target constraints.
///
/// The `cap` crate provides allocation tracking and hard-limit enforcement.
/// This is configured at crate root to ensure all allocations are tracked.
#[global_allocator]
static ALLOCATOR: Cap<System> = Cap::new(System, 24 * 1024 * 1024);

/// Get the current memory allocation in bytes.
///
/// This provides read-only access to the allocator's tracked usage.
/// Used by [`utils::MemoryMonitor`] for soft/hard limit checking.
pub fn allocated() -> usize {
    ALLOCATOR.allocated()
}

/// Delimited build stamp, and the only place the version string lives in the
/// binary.
///
/// `scripts/package_bundle.sh` greps the binary to prove it was compiled for
/// the version `manifest.meta` claims. A bare substring search cannot do that:
/// a clean stamp is a prefix of the dirty one, so `abc123` matches a binary
/// built from `abc123-dirty` and a mislabelled bundle passes. The delimiters
/// make the match exact.
///
/// Kept in the binary by [`BUILD_STAMP_ANCHOR`], not by `build_version()`.
const BUILD_STAMP: &str = concat!("<<ANYKA_BUILD_VERSION:", env!("ANYKA_BUILD_VERSION"), ">>");

const BUILD_STAMP_PREFIX: &str = "<<ANYKA_BUILD_VERSION:";

/// Forces the *delimited* stamp into the loadable image.
///
/// `build_version()` slices `BUILD_STAMP` at compile-time-constant bounds, so
/// under `lto = true` + `opt-level = 3` LLVM folds the slice to a reference to
/// the inner bytes alone and the delimiters never reach `.rodata`. They then
/// survived only in DWARF — which `profile.release`'s `strip = true` discards,
/// leaving `package_bundle.sh`'s grep to fail on every stripped release build
/// while passing on unstripped ones.
///
/// `#[used]` puts the reference in `llvm.used`, so the pointer (and with it the
/// full delimited string it points at) is emitted and kept.
#[used]
static BUILD_STAMP_ANCHOR: &[u8] = BUILD_STAMP.as_bytes();

/// `git describe` at build time — the version reported as `FirmwareVersion`
/// and in `/api/diagnostics`. Emitted by `build.rs`; never missing, so this
/// can be `env!` rather than `option_env!`.
pub fn build_version() -> &'static str {
    &BUILD_STAMP[BUILD_STAMP_PREFIX.len()..BUILD_STAMP.len() - ">>".len()]
}

pub mod app;
pub mod lifecycle;

pub mod config;

pub mod logging;

pub mod platform;

pub mod onvif;

pub mod utils;

// pub mod auth;

pub mod security;

pub mod hal;

pub mod streaming;

pub mod validation;

pub mod diagnostics;

pub mod osd;

pub mod time;

// Re-export main types for convenience
pub use app::{AppState, AppStateBuilder, AppStateError, Application};
pub use lifecycle::{RuntimeError, ShutdownReport, ShutdownStatus, StartupError};

#[cfg(test)]
mod build_stamp_tests {
    use super::*;

    #[test]
    fn test_build_version_strips_the_stamp_delimiters() {
        let version = build_version();
        assert!(!version.is_empty(), "build version must not be empty");
        assert!(!version.starts_with('<'), "prefix leaked: {version}");
        assert!(!version.ends_with('>'), "suffix leaked: {version}");
        assert_eq!(BUILD_STAMP, format!("<<ANYKA_BUILD_VERSION:{version}>>"));
    }

    #[test]
    fn test_build_stamp_is_greppable_exactly() {
        // package_bundle.sh greps for this literal. A clean version must not
        // match a dirty binary's stamp, which is what the delimiters buy.
        let clean = "abc123";
        let dirty_stamp = format!("<<ANYKA_BUILD_VERSION:{clean}-dirty>>");
        assert!(!dirty_stamp.contains(&format!("<<ANYKA_BUILD_VERSION:{clean}>>")));
    }
}
