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

pub use baseline::{
    apply_baseline_ops, baseline_direction_for, compare_against_baseline, update_baseline,
};
pub use config::{Args, EffectiveConfig, RtspValidationConfig, load_config};
pub use device::DeviceTelemetry;
pub use report::{
    StreamInfo, Summary, TestResult, TestRun, ValidationReport, compute_summary, result_ok,
};
