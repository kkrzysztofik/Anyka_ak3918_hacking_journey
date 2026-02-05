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
            if d < 0.0 {
                0.0
            } else {
                d
            }
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

fn telemetry_baseline_metrics(t: &DeviceTelemetry) -> Vec<(String, f64)> {
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
