//! Tests for the `Platform` trait implementation on `AnykaPlatform`.
//!
//! These exercise the bring-up/teardown orchestration in `platform/anyka/mod.rs`
//! against mocked HALs; the per-subsystem behaviour lives in `video_input_tests`
//! and `video_encoder_tests`.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;

use super::super::AnykaPlatform;
use crate::hal::common::audio::MockAudioHalTrait;
use crate::hal::common::video::MockVideoHalTrait;
use crate::hal::common::{AK_FAILED_I32, AK_SUCCESS_I32};
use crate::platform::common::{Platform, PlatformError};

/// A platform whose ISP config path does not exist, so `match_sensor` fails and
/// `init_video_input` returns before touching any other subsystem.
fn platform_without_isp_config() -> AnykaPlatform {
    AnykaPlatform::with_mocked_hal(
        Arc::new(MockVideoHalTrait::new()),
        Arc::new(MockAudioHalTrait::new()),
        Some(PathBuf::from("/nonexistent/anyka-test-isp.conf")),
        false,
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

/// `video_control()` is derived from the `video_input` field rather than a
/// separate stored field (see `impl_platform_accessors!`'s doc comment), so
/// unlike the other optionals above it must always be `Some` on a real
/// `AnykaPlatform` — nothing gates it off.
#[test]
fn test_video_control_is_some_for_anyka_platform() {
    let platform = platform_without_isp_config();
    assert!(platform.video_control().is_some());
}

/// `with_isp_config`'s `initial_rotated` seed must land in `AnykaVideoInput`
/// at construction time, before the VI is ever opened. `with_mocked_hal`
/// mirrors that seeding so a copy-paste bug at the real `with_isp_config`
/// call site (passing the wrong bool, or `!initial_rotated`) would be
/// caught here rather than going unnoticed.
#[test]
fn test_with_mocked_hal_seeds_initial_rotated_flag() {
    let rotated = AnykaPlatform::with_mocked_hal(
        Arc::new(MockVideoHalTrait::new()),
        Arc::new(MockAudioHalTrait::new()),
        Some(PathBuf::from("/nonexistent/anyka-test-isp.conf")),
        true,
    );
    assert!(rotated.video_input.rotated());

    let not_rotated = platform_without_isp_config();
    assert!(!not_rotated.video_input.rotated());
}

#[tokio::test]
async fn test_initialize_propagates_sensor_match_failure_and_stays_uninitialized() {
    let platform = platform_without_isp_config();

    let result = platform.initialize().await;

    assert!(matches!(result, Err(PlatformError::HardwareUnavailable(_))));
    // A failed bring-up must not leave the platform flagged as ready.
    assert!(!platform.is_initialized());
}

/// A mock FFI that gets `init_video_input()` all the way through Step 5
/// (`capture_on`) successfully, but fails Step 5.5's flip/mirror reapply.
///
/// Only covers the VI subsystem — `init_video_input()` is the smallest unit
/// that actually contains Step 5.5, so this stays deliberately narrower than
/// a full `initialize()` fixture (which would also need VENC open/streaming
/// mocks that Step 5.5 itself never touches).
fn mock_ffi_vi_bring_up_succeeds_flip_mirror_fails() -> MockVideoHalTrait {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

    mock.expect_vi_match_sensor()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);
    mock.expect_vi_open()
        .times(1)
        .returning(move |_| test_ptr as *mut c_void);
    mock.expect_vpss_init().times(1).returning(|_, _| ());
    // Called once directly by init_video_input's Step 3, once more inside
    // set_channel_attr's own sensor-resolution query (Step 4).
    mock.expect_vi_get_sensor_resolution()
        .times(2)
        .returning(|_, res| {
            unsafe {
                (*res).width = 1280;
                (*res).height = 720;
                (*res).max_width = 1280;
                (*res).max_height = 720;
            }
            AK_SUCCESS_I32
        });
    mock.expect_vi_set_channel_attr()
        .times(1)
        .returning(|_, _| AK_SUCCESS_I32);
    mock.expect_vi_capture_on()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);
    mock.expect_vi_set_flip_mirror()
        .times(1)
        .returning(|_, _, _| AK_FAILED_I32);

    mock
}

// Multi-thread flavor because Step 5.6 (OSD bring-up) reaches `AnykaIpc::send_request`,
// which uses `block_in_place` and panics on a current-thread runtime. See the
// `# Panics` note on `send_request`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_init_video_input_step_5_5_flip_mirror_failure_is_soft_fail() {
    // Proves the `if let Err(e) = ... { warn!(...) }` shape at Step 5.5 —
    // if a future refactor tightens that to `?`, this test catches the
    // regression (VI bring-up must not abort over a cosmetic reapply
    // failure; an upside-down stream is still a working stream).
    let isp_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let platform = AnykaPlatform::with_mocked_hal(
        Arc::new(mock_ffi_vi_bring_up_succeeds_flip_mirror_fails()),
        Arc::new(MockAudioHalTrait::new()),
        Some(isp_path),
        false,
    );

    let result = platform.init_video_input().await;
    assert!(
        result.is_ok(),
        "Step 5.5's flip/mirror reapply failure must not abort VI bring-up: {:?}",
        result
    );
}

#[tokio::test]
async fn test_rollback_video_pipeline_is_best_effort_when_hal_is_unopened() {
    // Every teardown call below targets an unopened device, so each one fails.
    // rollback_video_pipeline must swallow all of them and still return.
    let platform = platform_without_isp_config();

    platform.rollback_video_pipeline().await;

    assert!(!platform.is_initialized());
}

/// `ptz.enabled = false` must skip PTZ bring-up entirely.
///
/// The config key existed but reached nothing except a `tracing::info!` line, so PTZ always
/// started: an actor thread, an open of /dev/ak-motor{0,1}, and a `ptz_check_self` calibration
/// sweep that costs ~2.1 s of every startup on the device (and then fails with -1 before the
/// vertical motor is even attempted). Disabled means none of that runs.
///
/// On host the stub HAL reports a successful open, so the enabled path yields Success — that is
/// what makes this test able to tell the gate apart from a plain hardware-open failure.
#[test]
fn test_init_ptz_control_disabled_reports_disabled_not_failed() {
    let cfg = crate::config::types::PtzConfig {
        enabled: false,
        ..Default::default()
    };
    let result = super::super::init_ptz_control(&cfg, None);
    assert!(matches!(
        result,
        crate::lifecycle::startup::OptionalInitResult::Disabled
    ));
    assert!(
        result.error_message().is_none(),
        "disabled is a choice, not a failure"
    );
}

#[test]
fn test_init_ptz_control_enabled_succeeds_with_the_stub_hal() {
    assert!(
        super::super::init_ptz_control(&crate::config::types::PtzConfig::default(), None)
            .is_success()
    );
}

#[tokio::test]
async fn test_stop_night_loop_aborts_a_task_that_ignores_shutdown() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    // A loop that keeps spinning regardless of the shutdown signal;
    // stop_night_loop must abort and join it instead of detaching it
    // (regression for the timeout path that used to consume the JoinHandle).
    let (tx, rx) = tokio::sync::broadcast::channel(1);
    let ticks = Arc::new(AtomicU32::new(0));
    let ticks_task = Arc::clone(&ticks);
    let task = tokio::spawn(async move {
        let mut rx = rx;
        loop {
            ticks_task.fetch_add(1, Ordering::SeqCst);
            let _ = rx.try_recv(); // ignore the shutdown signal
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });

    // Let the loop spin a few times so it is provably running before we stop it.
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(
        ticks.load(Ordering::SeqCst) > 0,
        "loop must tick before shutdown"
    );

    super::super::stop_night_loop(tx, task).await;

    // The abort terminates the loop: the tick counter must stop advancing
    // *after* stop_night_loop returns. (During the 2s shutdown wait the loop
    // legitimately keeps ticking; the detached-task bug would keep it ticking
    // after the return too.)
    let ticks_after_stop = ticks.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(
        ticks.load(Ordering::SeqCst),
        ticks_after_stop,
        "AUTO loop kept running after shutdown (was detached, not aborted)"
    );
}

#[tokio::test]
async fn test_stop_night_loop_joins_a_task_that_stops_promptly() {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    let (tx, mut rx) = tokio::sync::broadcast::channel(1);
    let ticks = Arc::new(AtomicU32::new(0));
    let ticks_task = Arc::clone(&ticks);
    let task = tokio::spawn(async move {
        let _ = rx.recv().await; // stops promptly on shutdown
        ticks_task.fetch_add(1, Ordering::SeqCst);
    });

    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(ticks.load(Ordering::SeqCst), 0);

    super::super::stop_night_loop(tx, task).await;

    // A well-behaved loop was joined (not aborted) and completed its work.
    assert_eq!(ticks.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_spawn_supervisor_loss_receiver_taken_returns_initialization_failed() {
    let platform = Arc::new(AnykaPlatform::with_mocked_hal(
        Arc::new(MockVideoHalTrait::new()),
        Arc::new(MockAudioHalTrait::new()),
        None,
        false,
    ));
    assert!(
        platform.ipc().take_loss_rx().is_some(),
        "first take must succeed"
    );
    let (_shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let err = platform
        .spawn_supervisor(shutdown_rx)
        .expect_err("second ownership of loss rx must fail without panicking");
    match err {
        PlatformError::InitializationFailed(msg) => {
            assert!(
                msg.contains("peer-loss") || msg.contains("already taken"),
                "got {msg}"
            );
        }
        other => panic!("expected InitializationFailed, got {other:?}"),
    }
}

#[tokio::test]
async fn test_spawn_supervisor_success_shuts_down_on_signal() {
    let platform = Arc::new(AnykaPlatform::with_mocked_hal(
        Arc::new(MockVideoHalTrait::new()),
        Arc::new(MockAudioHalTrait::new()),
        None,
        false,
    ));
    let (shutdown_tx, shutdown_rx) = tokio::sync::broadcast::channel(1);
    let (availability, handle) = platform
        .spawn_supervisor(shutdown_rx)
        .expect("first spawn must succeed");

    assert_eq!(
        *availability.borrow(),
        crate::platform::common::Availability::Unavailable
    );
    assert!(
        platform.ipc().take_loss_rx().is_none(),
        "supervisor owns the loss receiver"
    );

    let _ = shutdown_tx.send(());
    tokio::time::timeout(std::time::Duration::from_secs(2), handle)
        .await
        .expect("supervisor must exit after shutdown")
        .expect("supervisor task must not panic");
}
