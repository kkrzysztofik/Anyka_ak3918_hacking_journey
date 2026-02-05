//! Report types (TestRun, TestResult, ValidationReport) and summary.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StreamInfo {
    pub media: String,
    pub encoding_name: String,
    pub control_present: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TestRun {
    pub timestamp: String,
    pub rtsp_host: String,
    pub rtsp_port: u16,
    pub rtsp_stream: String,
    pub test_duration_seconds: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum TestResult {
    Pass {
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        protocol: Option<String>,
    },
    Fail {
        name: String,
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        protocol: Option<String>,
    },
    Metric {
        name: String,
        value: serde_json::Value,
        pass: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        protocol: Option<String>,
    },
}

impl TestResult {
    /// Pass result with default protocol "rtsp" for backward compatibility.
    pub fn pass(name: impl Into<String>) -> Self {
        TestResult::Pass {
            name: name.into(),
            protocol: Some("rtsp".to_string()),
        }
    }

    /// Fail result with default protocol "rtsp".
    pub fn fail(name: impl Into<String>, reason: impl Into<String>) -> Self {
        TestResult::Fail {
            name: name.into(),
            reason: reason.into(),
            protocol: Some("rtsp".to_string()),
        }
    }

    /// Metric result with default protocol "rtsp".
    pub fn metric(name: impl Into<String>, value: serde_json::Value, pass: bool) -> Self {
        TestResult::Metric {
            name: name.into(),
            value,
            pass,
            protocol: Some("rtsp".to_string()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Summary {
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub overall_pass: bool,
}

/// Full validation report (RTSP + optional HTTP-FLV tests).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidationReport {
    pub test_run: TestRun,
    pub tests: Vec<TestResult>,
    pub summary: Summary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telemetry: Option<crate::device::DeviceTelemetry>,
}
