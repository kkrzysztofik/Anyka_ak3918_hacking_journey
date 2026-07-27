//! Tests for the `Platform` trait implementation on `AnykaPlatform`.
//!
//! These exercise the bring-up/teardown orchestration in `platform/anyka/mod.rs`
//! against mocked HALs; the per-subsystem behaviour lives in `video_input_tests`
//! and `video_encoder_tests`.

use std::path::PathBuf;
use std::sync::Arc;

use super::super::AnykaPlatform;
use crate::hal::common::audio::MockAudioHalTrait;
use crate::hal::common::video::MockVideoHalTrait;
use crate::platform::common::{Platform, PlatformError};

/// A platform whose ISP config path does not exist, so `match_sensor` fails and
/// `init_video_input` returns before touching any other subsystem.
fn platform_without_isp_config() -> AnykaPlatform {
    AnykaPlatform::with_mocked_hal(
        Arc::new(MockVideoHalTrait::new()),
        Arc::new(MockAudioHalTrait::new()),
        Some(PathBuf::from("/nonexistent/anyka-test-isp.conf")),
    )
}

#[tokio::test]
async fn test_get_device_info_returns_static_ak3918_descriptor() {
    let platform = platform_without_isp_config();

    let info = platform.get_device_info().await.unwrap();

    assert_eq!(info.manufacturer, "Anyka");
    assert_eq!(info.model, "AK3918");
    assert_eq!(info.hardware_id, "ak3918-hw");
}

#[tokio::test]
async fn test_platform_is_not_initialized_before_initialize() {
    let platform = platform_without_isp_config();
    assert!(!platform.is_initialized());
}

#[test]
fn test_stream_frame_age_ms_is_none_before_any_frame() {
    let platform = platform_without_isp_config();
    // No frame has ever been produced, so the encoder reports no age.
    assert_eq!(platform.stream_frame_age_ms(), None);
}

#[test]
fn test_accessors_expose_subsystems_and_absent_optionals() {
    let platform = platform_without_isp_config();

    // Each accessor must hand back the stored subsystem rather than building a
    // fresh one, so repeated calls point at the same allocation.
    assert!(Arc::ptr_eq(
        &platform.video_input(),
        &platform.video_input()
    ));
    assert!(Arc::ptr_eq(
        &platform.video_encoder(),
        &platform.video_encoder()
    ));
    assert!(Arc::ptr_eq(
        &platform.audio_input(),
        &platform.audio_input()
    ));
    assert!(Arc::ptr_eq(
        &platform.audio_encoder(),
        &platform.audio_encoder()
    ));

    // ...while the optional ones are absent in this fixture.
    assert!(platform.ptz_control().is_none());
    assert!(platform.imaging_control().is_none());
    assert!(platform.network_info().is_none());
}

#[tokio::test]
async fn test_initialize_propagates_sensor_match_failure_and_stays_uninitialized() {
    let platform = platform_without_isp_config();

    let result = platform.initialize().await;

    assert!(matches!(result, Err(PlatformError::HardwareUnavailable(_))));
    // A failed bring-up must not leave the platform flagged as ready.
    assert!(!platform.is_initialized());
}

#[tokio::test]
async fn test_rollback_video_pipeline_is_best_effort_when_hal_is_unopened() {
    // Every teardown call below targets an unopened device, so each one fails.
    // rollback_video_pipeline must swallow all of them and still return.
    let platform = platform_without_isp_config();

    platform.rollback_video_pipeline().await;

    assert!(!platform.is_initialized());
}
