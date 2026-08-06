//! Host-side RTSP/HTTP-FLV validation tool library.
//!
//! Protocol conformance validation for Anyka camera streams (RTSP today, HTTP-FLV planned).

pub mod baseline;
pub mod config;
pub mod device;
pub mod harness;
pub mod httpflv;
pub mod probe;
pub mod report;
pub(crate) mod rtp;
pub mod util;

// Root re-exports cover the crate's main types. Everything else is reached by
// module path — `main.rs` and the integration tests use those directly.
pub use config::{Args, EffectiveConfig, RtspValidationConfig, load_config};
pub use report::{
    StreamInfo, Summary, TestResult, TestRun, ValidationReport, compute_summary, result_ok,
};
