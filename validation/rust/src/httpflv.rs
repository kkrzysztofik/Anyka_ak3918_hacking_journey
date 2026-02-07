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

#[cfg(test)]
mod tests {
    use super::{run_httpflv_harness, run_httpflv_validation};
    use crate::config::{EffectiveConfig, parse_args_from};

    #[tokio::test]
    async fn test_run_httpflv_validation_returns_empty() {
        let crate::config::ParsedArgs { args, sources } =
            parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &args, &sources);
        let result = run_httpflv_validation(&effective).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_run_httpflv_harness_no_mutation() {
        let crate::config::ParsedArgs { args, sources } =
            parse_args_from(["rtsp_validation_tool"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &args, &sources);
        let mut tests = vec![];
        run_httpflv_harness(&effective, &mut tests).await.unwrap();
        assert!(tests.is_empty());
    }
}
