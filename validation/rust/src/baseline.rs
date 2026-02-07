//! Baseline update/compare and apply_baseline_ops.

use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, info, warn};

use crate::config::EffectiveConfig;
use crate::device::DeviceTelemetry;
use crate::report::{TestResult, ValidationReport};

#[derive(Debug, Serialize, Deserialize)]
struct BaselineFile {
    test: String,
    created: String,
    baseline_value: f64,
    tolerance_percent: u32,
    direction: String,
}

pub fn baseline_direction_for(test_name: &str) -> &'static str {
    match test_name {
        "startup_latency_ms"
        | "harness_startup_latency_ms"
        | "packet_loss_percent"
        | "harness_packet_loss_percent" => "lower",
        "bitrate_kbps" | "harness_bitrate_kbps" | "fps" | "harness_fps" => "higher",
        "telemetry_mem_free_kib" | "telemetry_mem_available_kib" | "telemetry_mem_total_kib" => {
            "higher"
        }
        "telemetry_load_avg_1m"
        | "telemetry_load_avg_5m"
        | "telemetry_load_avg_15m"
        | "telemetry_onvif_rss_kib"
        | "telemetry_onvif_vmsize_kib" => "lower",
        _ => "lower",
    }
}

pub fn update_baseline(
    baseline_dir: &Path,
    test_name: &str,
    value: f64,
    direction: &str,
) -> Result<()> {
    std::fs::create_dir_all(baseline_dir).context("create baseline dir")?;
    let path = baseline_dir.join(format!("{}_baseline.json", test_name));
    let created = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let file = BaselineFile {
        test: test_name.to_string(),
        created,
        baseline_value: value,
        tolerance_percent: 20,
        direction: direction.to_string(),
    };
    let json = serde_json::to_string_pretty(&file).context("serialize baseline")?;
    std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
    info!(test = %test_name, value = %value, "baseline updated");
    Ok(())
}

pub fn compare_against_baseline(
    baseline_dir: &Path,
    test_name: &str,
    current_value: f64,
    direction_override: Option<&str>,
) -> Result<bool> {
    let path = baseline_dir.join(format!("{}_baseline.json", test_name));
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            debug!(test = %test_name, "no baseline file, skipping comparison");
            return Ok(true);
        }
    };
    let file: BaselineFile = serde_json::from_str(&content).context("parse baseline json")?;
    let baseline_value = file.baseline_value;
    let tolerance = file.tolerance_percent as f64;
    let direction = direction_override.unwrap_or(&file.direction);

    if baseline_value == 0.0 {
        return Ok(true);
    }

    let regression_pct = match direction {
        "lower" => 100.0 * (current_value - baseline_value) / baseline_value,
        "higher" => {
            let d = 100.0 * (baseline_value - current_value) / baseline_value;
            if d < 0.0 { 0.0 } else { d }
        }
        _ => 100.0 * (current_value - baseline_value) / baseline_value,
    };

    if regression_pct > tolerance {
        warn!(
            test = %test_name,
            regression_pct = %regression_pct,
            tolerance = %tolerance,
            "baseline regression"
        );
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Exposed for unit tests (telemetry baseline metric names and values).
pub(crate) fn telemetry_baseline_metrics(t: &DeviceTelemetry) -> Vec<(String, f64)> {
    let mut out = Vec::new();
    if let Some(v) = t.mem_total_kib {
        out.push(("telemetry_mem_total_kib".to_string(), v as f64));
    }
    if let Some(v) = t.mem_free_kib {
        out.push(("telemetry_mem_free_kib".to_string(), v as f64));
    }
    if let Some(v) = t.mem_available_kib {
        out.push(("telemetry_mem_available_kib".to_string(), v as f64));
    }
    if let Some(v) = t.load_avg_1m {
        out.push(("telemetry_load_avg_1m".to_string(), v));
    }
    if let Some(v) = t.load_avg_5m {
        out.push(("telemetry_load_avg_5m".to_string(), v));
    }
    if let Some(v) = t.load_avg_15m {
        out.push(("telemetry_load_avg_15m".to_string(), v));
    }
    if let Some(v) = t.onvif_rss_kib {
        out.push(("telemetry_onvif_rss_kib".to_string(), v as f64));
    }
    if let Some(v) = t.onvif_vmsize_kib {
        out.push(("telemetry_onvif_vmsize_kib".to_string(), v as f64));
    }
    out
}

pub fn apply_baseline_ops(
    _args: &crate::config::Args,
    effective: &EffectiveConfig,
    report: &mut ValidationReport,
) -> Result<()> {
    if !effective.update_baseline && !effective.compare_baseline {
        return Ok(());
    }
    info!(
        update_baseline = effective.update_baseline,
        compare_baseline = effective.compare_baseline,
        "applying baseline ops"
    );
    let baseline_dir = &effective.baseline_dir;
    let tests = &mut report.tests;
    let mut metrics: Vec<(String, f64)> = Vec::new();
    for t in tests.iter() {
        if let TestResult::Metric { name, value, .. } = t {
            let v = match name.as_str() {
                "harness_startup_latency_ms" => value.as_f64(),
                "harness_bitrate_kbps" => value.as_f64(),
                "harness_fps" => value.as_f64(),
                "harness_packet_loss_percent" => value
                    .get("loss_percent")
                    .and_then(serde_json::Value::as_f64),
                _ => None,
            };
            if let Some(f) = v {
                metrics.push((name.clone(), f));
            }
        }
    }
    if let Some(ref telemetry) = report.telemetry {
        metrics.extend(telemetry_baseline_metrics(telemetry));
    }
    for (name, value) in metrics {
        if effective.update_baseline {
            let dir = baseline_direction_for(&name);
            update_baseline(baseline_dir, &name, value, dir)?;
            debug!(metric = %name, value, "baseline updated");
        }
        if effective.compare_baseline {
            let dir = baseline_direction_for(&name);
            let within = compare_against_baseline(baseline_dir, &name, value, Some(dir))?;
            if !within {
                debug!(metric = %name, value, "baseline regression");
                tests.push(TestResult::fail(
                    format!("baseline_regression_{}", name),
                    format!("{} value {} exceeds baseline tolerance", name, value),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        apply_baseline_ops, baseline_direction_for, compare_against_baseline,
        telemetry_baseline_metrics, update_baseline,
    };
    use crate::config::{EffectiveConfig, RtspValidationConfig};
    use crate::device::DeviceTelemetry;
    use crate::report::{TestResult, TestRun, ValidationReport};

    #[test]
    fn test_baseline_direction_for_lower_metrics() {
        assert_eq!(baseline_direction_for("startup_latency_ms"), "lower");
        assert_eq!(
            baseline_direction_for("harness_startup_latency_ms"),
            "lower"
        );
        assert_eq!(baseline_direction_for("packet_loss_percent"), "lower");
        assert_eq!(
            baseline_direction_for("harness_packet_loss_percent"),
            "lower"
        );
        assert_eq!(baseline_direction_for("telemetry_load_avg_1m"), "lower");
        assert_eq!(baseline_direction_for("telemetry_onvif_rss_kib"), "lower");
        assert_eq!(baseline_direction_for("unknown_metric"), "lower");
    }

    #[test]
    fn test_baseline_direction_for_higher_metrics() {
        assert_eq!(baseline_direction_for("bitrate_kbps"), "higher");
        assert_eq!(baseline_direction_for("harness_bitrate_kbps"), "higher");
        assert_eq!(baseline_direction_for("fps"), "higher");
        assert_eq!(baseline_direction_for("harness_fps"), "higher");
        assert_eq!(baseline_direction_for("telemetry_mem_free_kib"), "higher");
        assert_eq!(
            baseline_direction_for("telemetry_mem_available_kib"),
            "higher"
        );
        assert_eq!(baseline_direction_for("telemetry_mem_total_kib"), "higher");
    }

    #[test]
    fn test_update_baseline_creates_file_with_expected_fields() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_dir = dir.path();
        update_baseline(baseline_dir, "my_metric", 100.5, "lower").unwrap();
        let path = baseline_dir.join("my_metric_baseline.json");
        let content = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["test"], "my_metric");
        assert_eq!(json["baseline_value"], 100.5);
        assert_eq!(json["tolerance_percent"], 20);
        assert_eq!(json["direction"], "lower");
    }

    #[test]
    fn test_compare_against_baseline_no_file_returns_true() {
        let dir = tempfile::tempdir().unwrap();
        let result = compare_against_baseline(dir.path(), "nonexistent", 50.0, None).unwrap();
        assert!(result);
    }

    #[test]
    fn test_compare_against_baseline_lower_direction_within_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        update_baseline(dir.path(), "latency", 100.0, "lower").unwrap();
        // current 110 = 10% higher -> regression 10%; tolerance 20% -> ok
        assert!(compare_against_baseline(dir.path(), "latency", 110.0, None).unwrap());
        // current 130 = 30% higher -> regression 30% > 20% -> fail
        assert!(!compare_against_baseline(dir.path(), "latency", 130.0, None).unwrap());
    }

    #[test]
    fn test_compare_against_baseline_higher_direction_within_tolerance() {
        let dir = tempfile::tempdir().unwrap();
        update_baseline(dir.path(), "bitrate", 1000.0, "higher").unwrap();
        // current 900 = 10% lower -> regression 10%; tolerance 20% -> ok
        assert!(compare_against_baseline(dir.path(), "bitrate", 900.0, None).unwrap());
        // current 700 = 30% lower -> regression 30% > 20% -> fail
        assert!(!compare_against_baseline(dir.path(), "bitrate", 700.0, None).unwrap());
    }

    #[test]
    fn test_compare_against_baseline_zero_baseline_returns_true() {
        let dir = tempfile::tempdir().unwrap();
        update_baseline(dir.path(), "zero_metric", 0.0, "lower").unwrap();
        assert!(compare_against_baseline(dir.path(), "zero_metric", 100.0, None).unwrap());
    }

    #[test]
    fn test_compare_against_baseline_direction_override() {
        let dir = tempfile::tempdir().unwrap();
        update_baseline(dir.path(), "m", 100.0, "lower").unwrap();
        // override to "higher": 90 is "regression" when direction is higher (baseline - current) / baseline = 10%
        assert!(compare_against_baseline(dir.path(), "m", 90.0, Some("higher")).unwrap());
    }

    #[test]
    fn test_telemetry_baseline_metrics() {
        let mut t = DeviceTelemetry::default();
        assert!(telemetry_baseline_metrics(&t).is_empty());
        t.mem_total_kib = Some(64);
        t.load_avg_1m = Some(0.5);
        let metrics = telemetry_baseline_metrics(&t);
        assert_eq!(metrics.len(), 2);
        assert!(metrics.iter().any(|(k, _)| k == "telemetry_mem_total_kib"));
        assert!(metrics.iter().any(|(k, _)| k == "telemetry_load_avg_1m"));
    }

    #[test]
    fn test_apply_baseline_ops_no_flags_no_op() {
        let crate::config::ParsedArgs { args, sources } =
            crate::config::parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &args, &sources);
        let mut report = ValidationReport {
            test_run: TestRun {
                timestamp: String::new(),
                rtsp_host: String::new(),
                rtsp_port: 554,
                rtsp_stream: String::new(),
                test_duration_seconds: 30,
            },
            tests: vec![],
            summary: crate::report::Summary {
                total_tests: 0,
                passed: 0,
                failed: 0,
                overall_pass: true,
            },
            artifacts_dir: None,
            telemetry: None,
            telemetry_before_shutdown: None,
        };
        apply_baseline_ops(&args, &effective, &mut report).unwrap();
        assert!(report.tests.is_empty());
    }

    #[test]
    fn test_apply_baseline_ops_update_and_compare() {
        let dir = tempfile::tempdir().unwrap();
        let baseline_dir = dir.path().to_path_buf();
        let crate::config::ParsedArgs { mut args, sources } =
            crate::config::parse_args_from(["rtsp_validation_tool"]).unwrap();
        args.update_baseline = true;
        args.compare_baseline = true;
        let mut cfg = RtspValidationConfig::default();
        cfg.run.update_baseline = true;
        cfg.run.compare_baseline = true;
        cfg.baseline.dir = baseline_dir.to_string_lossy().to_string();
        let effective = EffectiveConfig::from_config_and_args(Some(&cfg), &args, &sources);
        let mut report = ValidationReport {
            test_run: TestRun {
                timestamp: String::new(),
                rtsp_host: String::new(),
                rtsp_port: 554,
                rtsp_stream: String::new(),
                test_duration_seconds: 30,
            },
            tests: vec![TestResult::metric(
                "harness_bitrate_kbps",
                serde_json::json!(1000.0),
                true,
            )],
            summary: crate::report::Summary {
                total_tests: 1,
                passed: 1,
                failed: 0,
                overall_pass: true,
            },
            artifacts_dir: None,
            telemetry: None,
            telemetry_before_shutdown: None,
        };
        apply_baseline_ops(&args, &effective, &mut report).unwrap();
        assert!(
            dir.path()
                .join("harness_bitrate_kbps_baseline.json")
                .exists()
        );
        // run again with compare: same value should pass
        apply_baseline_ops(&args, &effective, &mut report).unwrap();
        assert_eq!(report.tests.len(), 1);
    }
}
