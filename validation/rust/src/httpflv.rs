//! HTTP-FLV validation (stub for future implementation).

use anyhow::Result;

use crate::config::EffectiveConfig;
use crate::report::TestResult;

/// Run HTTP-FLV protocol validation (stub). Returns empty test list until implemented.
pub async fn run_httpflv_validation(_effective: &EffectiveConfig) -> Result<Vec<TestResult>> {
    Ok(vec![])
}

/// Run HTTP-FLV harness scenarios (stub). No-op until implemented.
pub async fn run_httpflv_harness(
    _effective: &EffectiveConfig,
    _tests: &mut Vec<TestResult>,
) -> Result<()> {
    Ok(())
}
