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

    /// Return the test name for this result.
    pub fn name(&self) -> &str {
        match self {
            TestResult::Pass { name, .. } => name,
            TestResult::Fail { name, .. } => name,
            TestResult::Metric { name, .. } => name,
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

/// Compute summary from test results (passed = count of result_ok, failed = total - passed).
pub fn compute_summary(tests: &[TestResult]) -> Summary {
    let total_tests = tests.len();
    let passed = tests.iter().filter(|t| result_ok(t)).count();
    let failed = total_tests.saturating_sub(passed);
    Summary {
        total_tests,
        passed,
        failed,
        overall_pass: failed == 0,
    }
}

/// Returns true if the test result is a pass (Pass variant or Metric with pass true).
pub fn result_ok(r: &TestResult) -> bool {
    match r {
        TestResult::Pass { .. } => true,
        TestResult::Fail { .. } => false,
        TestResult::Metric { pass, .. } => *pass,
    }
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

#[cfg(test)]
mod tests {
    use super::{Summary, TestResult, TestRun, ValidationReport, compute_summary, result_ok};

    #[test]
    fn test_test_result_pass() {
        let r = TestResult::pass("describe_ok");
        match &r {
            TestResult::Pass { name, protocol } => {
                assert_eq!(name, "describe_ok");
                assert_eq!(protocol.as_deref(), Some("rtsp"));
            }
            _ => panic!("expected Pass"),
        }
    }

    #[test]
    fn test_test_result_fail() {
        let r = TestResult::fail("play_ok", "connection refused");
        match &r {
            TestResult::Fail {
                name,
                reason,
                protocol,
            } => {
                assert_eq!(name, "play_ok");
                assert_eq!(reason, "connection refused");
                assert_eq!(protocol.as_deref(), Some("rtsp"));
            }
            _ => panic!("expected Fail"),
        }
    }

    #[test]
    fn test_test_result_metric() {
        let r = TestResult::metric("bitrate_kbps", serde_json::json!(1500.0), true);
        match &r {
            TestResult::Metric {
                name,
                value,
                pass,
                protocol,
            } => {
                assert_eq!(name, "bitrate_kbps");
                assert_eq!(value.as_f64(), Some(1500.0));
                assert!(*pass);
                assert_eq!(protocol.as_deref(), Some("rtsp"));
            }
            _ => panic!("expected Metric"),
        }
    }

    #[test]
    fn test_summary_and_report_roundtrip() {
        let summary = Summary {
            total_tests: 3,
            passed: 2,
            failed: 1,
            overall_pass: false,
        };
        let report = ValidationReport {
            test_run: TestRun {
                timestamp: "2025-01-01T00:00:00Z".to_string(),
                rtsp_host: "127.0.0.1".to_string(),
                rtsp_port: 554,
                rtsp_stream: "/stream1".to_string(),
                test_duration_seconds: 30,
            },
            tests: vec![
                TestResult::pass("a"),
                TestResult::pass("b"),
                TestResult::fail("c", "reason"),
            ],
            summary: summary.clone(),
            artifacts_dir: None,
            telemetry: None,
        };
        assert_eq!(report.summary.total_tests, 3);
        assert_eq!(report.summary.passed, 2);
        assert_eq!(report.summary.failed, 1);
        assert!(!report.summary.overall_pass);
    }

    #[test]
    fn test_result_ok_helpers() {
        assert!(result_ok(&TestResult::pass("p")));
        assert!(!result_ok(&TestResult::fail("f", "r")));
        assert!(result_ok(&TestResult::metric(
            "m",
            serde_json::json!(1),
            true
        )));
        assert!(!result_ok(&TestResult::metric(
            "m",
            serde_json::json!(1),
            false
        )));
    }

    #[test]
    fn test_compute_summary() {
        let tests = vec![
            TestResult::pass("a"),
            TestResult::pass("b"),
            TestResult::fail("c", "reason"),
            TestResult::metric("d", serde_json::json!(1), true),
            TestResult::metric("e", serde_json::json!(2), false),
        ];
        let summary = compute_summary(&tests);
        assert_eq!(summary.total_tests, 5);
        assert_eq!(summary.passed, 3);
        assert_eq!(summary.failed, 2);
        assert!(!summary.overall_pass);
    }

    #[test]
    fn test_compute_summary_all_pass() {
        let tests = vec![TestResult::pass("a"), TestResult::pass("b")];
        let summary = compute_summary(&tests);
        assert_eq!(summary.total_tests, 2);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 0);
        assert!(summary.overall_pass);
    }
}
