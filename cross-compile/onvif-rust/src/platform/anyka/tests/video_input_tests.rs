//! Tests for AnykaVideoInput functionality.

use super::super::*;
use crate::hal::common::video::MockVideoHalTrait;
use crate::hal::common::{AK_FAILED_I32, AK_SUCCESS_I32};
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;

fn video_dev0() -> crate::hal::common::video_dev_type {
    crate::hal::common::video_dev_type::Dev0
}

/// Create a mock FFI that expects a successful vi_open call.
fn mock_ffi_with_successful_open() -> MockVideoHalTrait {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_vi_open()
        .with(mockall::predicate::eq(video_dev0()))
        .times(1)
        .returning(move |_| test_ptr as *mut c_void);
    mock
}

#[tokio::test]
async fn test_video_input_open_success() {
    let mock = mock_ffi_with_successful_open();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    let result = vi.open().await;
    assert!(result.is_ok());
    assert!(vi.opened.load(Ordering::SeqCst));
    assert!(vi.handle.read().is_some());
}

#[tokio::test]
async fn test_video_input_open_already_opened() {
    let mock = mock_ffi_with_successful_open();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    // First open succeeds
    vi.open().await.unwrap();

    // Second open returns ResourceBusy
    let result = vi.open().await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::ResourceBusy(msg)) => {
            assert!(msg.contains("already opened"));
        }
        other => panic!("Expected ResourceBusy, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_input_open_hardware_failure() {
    let mut mock = MockVideoHalTrait::new();
    mock.expect_vi_open()
        .with(mockall::predicate::eq(video_dev0()))
        .times(1)
        .returning(|_| std::ptr::null_mut());

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    let result = vi.open().await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(_)) => {}
        other => panic!("Expected HardwareUnavailable, got {:?}", other),
    }
    // State should remain false on failure
    assert!(!vi.opened.load(Ordering::SeqCst));
}

#[tokio::test]
async fn test_video_input_close_success() {
    let mock = mock_ffi_with_successful_open();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    vi.open().await.unwrap();
    assert!(vi.opened.load(Ordering::SeqCst));

    let result = vi.close().await;
    assert!(result.is_ok());
    assert!(!vi.opened.load(Ordering::SeqCst));
    assert!(vi.handle.read().is_none());
}

#[tokio::test]
async fn test_video_input_close_idempotent() {
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    // Close without ever opening — should succeed (idempotent)
    let result = vi.close().await;
    assert!(result.is_ok());

    // Close again — still idempotent
    let result = vi.close().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_video_input_get_resolution_success() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, res| {
            unsafe {
                (*res).width = 1920;
                (*res).height = 1080;
                (*res).max_width = 1920;
                (*res).max_height = 1080;
            }
            AK_SUCCESS_I32
        });

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.get_resolution().await;
    assert!(result.is_ok());
    let res = result.unwrap();
    assert_eq!(res.width, 1920);
    assert_eq!(res.height, 1080);
}

#[tokio::test]
async fn test_video_input_get_resolution_not_opened() {
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    let result = vi.get_resolution().await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(msg)) => {
            assert!(msg.contains("not opened"));
        }
        other => panic!("Expected HardwareUnavailable, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_input_get_resolution_ffi_error() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, _| AK_FAILED_I32);

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.get_resolution().await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareFailure(msg)) => {
            assert!(msg.contains("ak_vi_get_sensor_resolution"));
        }
        other => panic!("Expected HardwareFailure, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_input_get_sources_returns_config() {
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    // get_sources works even when not opened (uses default resolution)
    let result = vi.get_sources().await;
    assert!(result.is_ok());
    let sources = result.unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].token, "VideoSource_1");
    assert_eq!(sources[0].name, "Main Camera");
    assert_eq!(sources[0].resolution.width, 1920);
    assert_eq!(sources[0].resolution.height, 1080);
    assert!((sources[0].max_framerate - 30.0).abs() < f32::EPSILON);
}

#[tokio::test]
async fn test_video_input_get_sources_with_hardware_resolution() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, res| {
            unsafe {
                (*res).width = 2560;
                (*res).height = 1440;
                (*res).max_width = 2560;
                (*res).max_height = 1440;
            }
            AK_SUCCESS_I32
        });

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let sources = vi.get_sources().await.unwrap();
    assert_eq!(sources[0].resolution.width, 2560);
    assert_eq!(sources[0].resolution.height, 1440);
}

#[tokio::test]
async fn test_video_input_set_channel_attr_success() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
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

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.set_channel_attr();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_video_input_set_channel_attr_matches_anyka_quirk_default() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, res| {
            unsafe {
                (*res).width = 1920;
                (*res).height = 1080;
                (*res).max_width = 1920;
                (*res).max_height = 1080;
            }
            AK_SUCCESS_I32
        });
    mock.expect_vi_set_channel_attr()
        .times(1)
        .returning(|_, attr| {
            unsafe {
                // Anyka default: main=sensor, sub=640x360.
                assert_eq!((*attr).res[0].width, 1920);
                assert_eq!((*attr).res[0].height, 1080);
                assert_eq!((*attr).res[1].width, 640);
                assert_eq!((*attr).res[1].height, 360);
                // libre_anyka_app quirk (via vendor IPC):
                // main.max_* drives sub-channel limits.
                assert_eq!((*attr).res[0].max_width, 640);
                assert_eq!((*attr).res[0].max_height, 360);
                assert_eq!((*attr).res[1].max_width, 1920);
                assert_eq!((*attr).res[1].max_height, 1080);
            }
            AK_SUCCESS_I32
        });

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.set_channel_attr();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_video_input_set_channel_attr_not_opened() {
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    let result = vi.set_channel_attr();
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(msg)) => {
            assert!(msg.contains("not opened"));
        }
        other => panic!("Expected HardwareUnavailable, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_input_set_channel_attr_ffi_error() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, res| {
            unsafe {
                (*res).width = 1280;
                (*res).height = 720;
                (*res).max_width = 1280;
                (*res).max_height = 720;
            }
            AK_SUCCESS_I32
        });
    // Both the inverted attempt and the main.max-bumped retry fail.
    mock.expect_vi_set_channel_attr()
        .times(2)
        .returning(|_, _| AK_FAILED_I32);

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.set_channel_attr();
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareFailure(msg)) => {
            assert!(msg.contains("ak_vi_set_channel_attr"));
        }
        other => panic!("Expected HardwareFailure, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_input_set_channel_attr_falls_back_to_sub_size_main() {
    // GC1084-class ISP: rejects sensor-native main (1280x720), accepts the retry
    // that clamps the main channel to the sub size (640x360, uniform).
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, res| {
            unsafe {
                (*res).width = 1280;
                (*res).height = 720;
                (*res).max_width = 1280;
                (*res).max_height = 720;
            }
            AK_SUCCESS_I32
        });

    let seq = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seq2 = seq.clone();
    mock.expect_vi_set_channel_attr()
        .times(2)
        .returning(move |_, attr| {
            let n = seq2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            unsafe {
                if n == 0 {
                    // First attempt: sensor-native main -> reject.
                    assert_eq!((*attr).res[0].width, 1280);
                    assert_eq!((*attr).res[0].height, 720);
                    AK_FAILED_I32
                } else {
                    // Retry: main clamped to sub size; sub untouched so its max
                    // stays at the sensor size (encoder headroom).
                    assert_eq!((*attr).res[0].width, 640);
                    assert_eq!((*attr).res[0].height, 360);
                    assert_eq!((*attr).res[0].max_width, 640);
                    assert_eq!((*attr).res[0].max_height, 360);
                    assert_eq!((*attr).res[1].width, 640);
                    assert_eq!((*attr).res[1].height, 360);
                    // Sub max preserved at sensor size (inverted mapping).
                    assert_eq!((*attr).res[1].max_width, 1280);
                    assert_eq!((*attr).res[1].max_height, 720);
                    AK_SUCCESS_I32
                }
            }
        });

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    assert!(vi.set_channel_attr().is_ok());
    assert_eq!(seq.load(std::sync::atomic::Ordering::SeqCst), 2);
    // channel_layout reflects the clamped main.
    let (main, _sub) = vi.channel_layout();
    assert_eq!((main.width, main.height), (640, 360));
}

#[tokio::test]
async fn test_video_input_concurrent_operations() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_vi_open()
        .with(mockall::predicate::eq(video_dev0()))
        .times(1)
        .returning(move |_| test_ptr as *mut c_void);
    mock.expect_vi_get_sensor_resolution()
        .times(2)
        .returning(|_, res| {
            unsafe {
                (*res).width = 1920;
                (*res).height = 1080;
                (*res).max_width = 1920;
                (*res).max_height = 1080;
            }
            AK_SUCCESS_I32
        });

    let vi = Arc::new(AnykaVideoInput::with_ffi(Arc::new(mock), None));
    vi.open().await.unwrap();

    // Spawn concurrent resolution queries
    let vi1 = vi.clone();
    let vi2 = vi.clone();

    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { vi1.get_resolution().await }),
        tokio::spawn(async move { vi2.get_resolution().await }),
    );

    assert!(r1.unwrap().is_ok());
    assert!(r2.unwrap().is_ok());
}

#[tokio::test]
async fn test_video_input_open_close_reopen() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_vi_open()
        .with(mockall::predicate::eq(video_dev0()))
        .times(2) // open, close, open again
        .returning(move |_| test_ptr as *mut c_void);

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    // Open -> close -> reopen cycle
    vi.open().await.unwrap();
    assert!(vi.opened.load(Ordering::SeqCst));

    vi.close().await.unwrap();
    assert!(!vi.opened.load(Ordering::SeqCst));

    vi.open().await.unwrap();
    assert!(vi.opened.load(Ordering::SeqCst));
}

// =========================================================================
// match_sensor + capture_on Tests
// =========================================================================

#[test]
fn test_match_sensor_with_explicit_existing_path() {
    let mut mock = MockVideoHalTrait::new();
    mock.expect_vi_match_sensor()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    // Use a path that exists — Cargo.toml always exists in the project root
    let vi = AnykaVideoInput::with_ffi(
        Arc::new(mock),
        Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")),
    );
    let result = vi.match_sensor();
    assert!(result.is_ok());
}

#[test]
fn test_match_sensor_explicit_path_not_found_falls_back() {
    // With an explicit path that doesn't exist AND no search paths existing,
    // match_sensor should return an error.
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(
        Arc::new(mock),
        Some(PathBuf::from("/nonexistent/isp_config.conf")),
    );
    let result = vi.match_sensor();
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(msg)) => {
            assert!(msg.contains("No ISP sensor config file found"));
        }
        _ => panic!("Expected HardwareUnavailable error"),
    }
}

#[test]
fn test_match_sensor_no_config_found() {
    // No explicit path, no default search paths exist
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    let result = vi.match_sensor();
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(msg)) => {
            assert!(msg.contains("No ISP sensor config file found"));
        }
        _ => panic!("Expected HardwareUnavailable error"),
    }
}

#[tokio::test]
async fn test_capture_on_success() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_vi_open()
        .returning(move |_| test_ptr as *mut c_void);
    mock.expect_vi_capture_on()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.capture_on();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_capture_on_retry_runs_capture_off_cleanup() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

    mock.expect_vi_open()
        .times(1)
        .returning(move |_| test_ptr as *mut c_void);

    let mut attempts = 0;
    mock.expect_vi_capture_on().times(2).returning(move |_| {
        attempts += 1;
        if attempts == 1 {
            AK_FAILED_I32
        } else {
            AK_SUCCESS_I32
        }
    });

    mock.expect_vi_capture_off()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.capture_on();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_capture_on_retry_aborts_when_capture_off_cleanup_fails() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

    mock.expect_vi_open()
        .times(1)
        .returning(move |_| test_ptr as *mut c_void);

    mock.expect_vi_capture_on()
        .times(1)
        .returning(|_| AK_FAILED_I32);

    mock.expect_vi_capture_off()
        .times(1)
        .returning(|_| AK_FAILED_I32);

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.capture_on();
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareFailure(msg)) => {
            assert!(msg.contains("retry cleanup failed"));
        }
        other => panic!("Expected HardwareFailure, got {:?}", other),
    }
}

#[test]
fn test_capture_on_fails_when_not_opened() {
    let mock = MockVideoHalTrait::new();
    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);

    let result = vi.capture_on();
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(msg)) => {
            assert!(msg.contains("Video input not opened"));
        }
        _ => panic!("Expected HardwareUnavailable error"),
    }
}

#[tokio::test]
async fn test_video_input_set_channel_attr_small_sensor_fallback() {
    let mut mock = mock_ffi_with_successful_open();
    mock.expect_vi_get_sensor_resolution()
        .times(1)
        .returning(|_, res| {
            unsafe {
                (*res).width = 320;
                (*res).height = 240;
                (*res).max_width = 320;
                (*res).max_height = 240;
            }
            AK_SUCCESS_I32
        });
    mock.expect_vi_set_channel_attr()
        .times(1)
        .returning(|_, attr| {
            unsafe {
                assert_eq!((*attr).res[0].width, 320);
                assert_eq!((*attr).res[0].height, 240);
                assert_eq!((*attr).res[1].width, 320);
                assert_eq!((*attr).res[1].height, 240);
                assert_eq!((*attr).res[0].max_width, 320);
                assert_eq!((*attr).res[0].max_height, 240);
                assert_eq!((*attr).res[1].max_width, 320);
                assert_eq!((*attr).res[1].max_height, 240);
            }
            AK_SUCCESS_I32
        });

    let vi = AnykaVideoInput::with_ffi(Arc::new(mock), None);
    vi.open().await.unwrap();

    let result = vi.set_channel_attr();
    assert!(result.is_ok());
}
