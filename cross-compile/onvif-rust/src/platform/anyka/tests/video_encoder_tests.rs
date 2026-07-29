//! Tests for AnykaVideoEncoder and streaming functionality.

use super::super::video_encoder::{
    EncoderState, NO_DATA_IDR_RECOVERY_EVERY_ERRORS, SDK_ERROR_NO_DATA, SDK_MIN_QP_RANGE,
    StreamHealthCounters, StreamState, handle_pushed_frame, invoke_owned_callbacks_from_map,
    is_push_mode_transient_error, run_push_loop, sdk_frame_type_to_frame_type,
    unified_frame_read_loop,
};
use super::super::*;
use crate::hal::common::video::{MockVideoHalTrait, VideoStreamHandle};
use crate::hal::common::{
    AK_FAILED_I32, AK_SUCCESS_I32, VideoFrameType, bitrate_ctrl_mode, encode_group_type,
    encode_output_type, encode_use_chn,
};
use crate::platform::common::{CallbackId, FrameType, OwnedFrame, OwnedFrameCallback, StreamId};
use parking_lot::RwLock;
use portable_atomic::AtomicU64;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

/// Create a mock FFI that expects a successful venc_open call.
fn mock_ffi_with_successful_encoder_open() -> MockVideoHalTrait {
    let mut mock = MockVideoHalTrait::new();
    mock.expect_venc_set_cfg_path().returning(|_| 0);
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_open()
        .returning(move |_| test_ptr as *mut c_void);
    mock
}

#[tokio::test]
async fn test_video_encoder_init_main_stream() {
    let mock = mock_ffi_with_successful_encoder_open();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        quality: 80,
        ..Default::default()
    };

    let result = encoder.init(&config).await;
    assert!(result.is_ok());
    assert!(encoder.main_handle.read().is_some());
    assert_eq!(*encoder.main_state.read(), EncoderState::Initialized);
}

#[tokio::test]
async fn test_video_encoder_init_sub_stream() {
    let mock = mock_ffi_with_successful_encoder_open();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let config = VideoEncoderConfig {
        token: "VideoEncoder_2".to_string(),
        name: "Sub Stream".to_string(),
        resolution: Resolution::new(640, 360),
        framerate: 15,
        bitrate: 300,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        quality: 70,
        ..Default::default()
    };

    let result = encoder.init(&config).await;
    assert!(result.is_ok());
    assert!(encoder.sub_handle.read().is_some());
    assert_eq!(*encoder.sub_state.read(), EncoderState::Initialized);
}

#[tokio::test]
async fn test_video_encoder_init_dual_streams() {
    let mock = mock_ffi_with_successful_encoder_open();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let main_config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };
    let sub_config = VideoEncoderConfig {
        token: "VideoEncoder_2".to_string(),
        name: "Sub Stream".to_string(),
        resolution: Resolution::new(640, 360),
        framerate: 15,
        bitrate: 300,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };

    encoder.init(&main_config).await.unwrap();
    encoder.init(&sub_config).await.unwrap();

    assert!(encoder.main_handle.read().is_some());
    assert!(encoder.sub_handle.read().is_some());

    let configs = encoder.get_configurations().await.unwrap();
    assert_eq!(configs.len(), 2);
}

#[tokio::test]
async fn test_video_encoder_init_invalid_token() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let config = VideoEncoderConfig {
        token: "VideoEncoder_99".to_string(),
        ..Default::default()
    };

    let result = encoder.init(&config).await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::InvalidParameter(msg)) => {
            assert!(msg.contains("VideoEncoder_99"));
        }
        other => panic!("Expected InvalidParameter, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_encoder_init_ffi_failure() {
    let mut mock = MockVideoHalTrait::new();
    mock.expect_venc_set_cfg_path().returning(|_| 0);
    mock.expect_venc_open().returning(|_| std::ptr::null_mut());

    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        ..Default::default()
    };

    let result = encoder.init(&config).await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(_)) => {}
        other => panic!("Expected HardwareUnavailable, got {:?}", other),
    }
    // Handle should remain None on failure
    assert!(encoder.main_handle.read().is_none());
}

#[tokio::test]
async fn test_video_encoder_set_configuration_bitrate_change() {
    let mut mock = mock_ffi_with_successful_encoder_open();
    mock.expect_venc_set_rc()
        .withf(|_, bps| *bps == 6000) // kbps passed directly to SDK
        .times(1)
        .returning(|_, _| AK_SUCCESS_I32);

    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    // Initialize first
    let init_config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };
    encoder.init(&init_config).await.unwrap();

    // Change bitrate
    let new_config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 6000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };
    let result = encoder.set_configuration(&new_config).await;
    assert!(result.is_ok());

    // Verify configuration updated
    let config = encoder.get_configuration().await.unwrap();
    assert_eq!(config.bitrate, 6000);
}

#[tokio::test]
async fn test_video_encoder_set_configuration_invalid_token() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let config = VideoEncoderConfig {
        token: "VideoEncoder_99".to_string(),
        ..Default::default()
    };

    let result = encoder.set_configuration(&config).await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::InvalidParameter(msg)) => {
            assert!(msg.contains("VideoEncoder_99"));
        }
        other => panic!("Expected InvalidParameter, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_encoder_set_configuration_ffi_error() {
    let mut mock = mock_ffi_with_successful_encoder_open();
    mock.expect_venc_set_rc().returning(|_, _| AK_FAILED_I32);

    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    // Initialize first
    let init_config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };
    encoder.init(&init_config).await.unwrap();

    // Attempt bitrate change that fails at FFI level
    let new_config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 6000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };
    let result = encoder.set_configuration(&new_config).await;
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareFailure(_)) => {}
        other => panic!("Expected HardwareFailure, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_encoder_get_configuration() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let config = encoder.get_configuration().await.unwrap();
    assert_eq!(config.token, "VideoEncoder_1");
    assert_eq!(config.resolution.width, 1280);
    assert_eq!(config.resolution.height, 720);
    assert_eq!(config.framerate, 15);
    assert_eq!(config.bitrate, 2000);
}

#[tokio::test]
async fn test_video_encoder_get_configurations() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let configs = encoder.get_configurations().await.unwrap();
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].token, "VideoEncoder_1");
    assert_eq!(configs[1].token, "VideoEncoder_2");
}

#[tokio::test]
async fn test_video_encoder_get_options() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let options = encoder.get_options().await.unwrap();
    assert_eq!(options.resolutions.len(), 3);
    assert_eq!(options.framerate_range, (1, 30));
    assert_eq!(options.bitrate_range, (128, 8000));
}

#[test]
fn test_config_to_encode_param_main() {
    let config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };

    let param = AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
    assert_eq!(param.width, 1280);
    assert_eq!(param.height, 720);
    assert_eq!(param.fps, 15);
    assert_eq!(param.bps, 2000); // kbps passed directly
    assert_eq!(param.goplen, 30);
    assert_eq!(param.minqp, 20);
    assert_eq!(param.use_chn, encode_use_chn::ENCODE_MAIN_CHN);
    assert_eq!(param.enc_grp, encode_group_type::ENCODE_MAINCHN_NET);
    assert_eq!(param.enc_out_type, encode_output_type::H264_ENC_TYPE);
}

#[test]
fn test_config_to_encode_param_sub() {
    let config = VideoEncoderConfig {
        token: "VideoEncoder_2".to_string(),
        resolution: Resolution::new(640, 360),
        framerate: 15,
        bitrate: 300,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };

    let param = AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_SUB_CHN);
    assert_eq!(param.width, 640);
    assert_eq!(param.height, 360);
    assert_eq!(param.fps, 15);
    assert_eq!(param.bps, 300); // kbps passed directly
    assert_eq!(param.goplen, 30);
    assert_eq!(param.use_chn, encode_use_chn::ENCODE_SUB_CHN);
    assert_eq!(param.enc_grp, encode_group_type::ENCODE_SUBCHN_NET);
}

#[test]
fn test_config_to_encode_param_h265() {
    let config = VideoEncoderConfig {
        encoding: VideoEncoding::H265,
        ..Default::default()
    };

    let param = AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
    assert_eq!(param.enc_out_type, encode_output_type::HEVC_ENC_TYPE);
}

#[test]
fn test_config_to_encode_param_vbr_mode() {
    let config = VideoEncoderConfig {
        bitrate_mode: crate::platform::BitrateMode::Vbr,
        ..Default::default()
    };

    let param = AnykaVideoEncoder::config_to_encode_param(&config, encode_use_chn::ENCODE_MAIN_CHN);
    assert_eq!(param.br_mode, bitrate_ctrl_mode::BR_MODE_VBR);
}

// =========================================================================
// Frame Callback Tests
// =========================================================================

/// Test owned callback that counts invocations.
struct CountingOwnedCallback {
    count: AtomicU64,
    last_size: AtomicU64,
}

impl CountingOwnedCallback {
    fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
            last_size: AtomicU64::new(0),
        }
    }

    fn call_count(&self) -> u64 {
        self.count.load(Ordering::SeqCst)
    }

    fn last_size(&self) -> u64 {
        self.last_size.load(Ordering::SeqCst)
    }
}

impl OwnedFrameCallback for CountingOwnedCallback {
    fn on_owned_frame(&self, frame: OwnedFrame) {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.last_size
            .store(frame.data.len() as u64, Ordering::SeqCst);
    }
}

/// Test owned callback that deliberately panics.
struct PanickingOwnedCallback;

impl OwnedFrameCallback for PanickingOwnedCallback {
    fn on_owned_frame(&self, _frame: OwnedFrame) {
        panic!("intentional panic in owned callback test");
    }
}

// ===== Owned Frame Callback Tests =====

/// Helper to register an owned callback directly in tests (bypasses AnykaVideoEncoder).
fn register_owned_callback_for_test(
    callbacks: &Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
    callback: Arc<dyn OwnedFrameCallback>,
) -> CallbackId {
    static NEXT_ID: portable_atomic::AtomicU64 = portable_atomic::AtomicU64::new(1);
    let id = NEXT_ID.fetch_add(1, portable_atomic::Ordering::SeqCst);
    callbacks.write().insert(id, callback);
    id
}

#[test]
fn test_register_owned_frame_callback() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let cb = Arc::new(CountingOwnedCallback::new());
    let id = encoder.register_owned_frame_callback(cb);
    assert!(id > 0);
    assert_eq!(encoder.owned_callbacks.read().len(), 1);
}

#[test]
fn test_unregister_owned_frame_callback() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let cb = Arc::new(CountingOwnedCallback::new());
    let id = encoder.register_owned_frame_callback(cb);
    assert_eq!(encoder.owned_callbacks.read().len(), 1);

    let removed = encoder.unregister_owned_frame_callback(id);
    assert!(removed);
    assert_eq!(encoder.owned_callbacks.read().len(), 0);
}

#[test]
fn test_invoke_owned_callbacks_from_map_empty() {
    use parking_lot::RwLock;
    use std::collections::HashMap;

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let owned_frame = OwnedFrame {
        data: bytes::BytesMut::from(&b"test data"[..]),
        timestamp: 1000,
        frame_type: FrameType::VideoIFrame,
        stream_id: StreamId::VideoMain,
    };

    // With no callbacks registered the frame is dropped without panicking.
    invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
    assert_eq!(owned_callbacks.read().len(), 0);
}

#[test]
fn test_invoke_owned_callbacks_from_map_single() {
    use parking_lot::RwLock;
    use std::collections::HashMap;

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let cb = Arc::new(CountingOwnedCallback::new());
    let cb_ref = Arc::clone(&cb);
    let _id = register_owned_callback_for_test(&owned_callbacks, cb);

    let owned_frame = OwnedFrame {
        data: bytes::BytesMut::from(&b"test data for single callback"[..]),
        timestamp: 1000,
        frame_type: FrameType::VideoIFrame,
        stream_id: StreamId::VideoMain,
    };

    // Frame is consumed by the registered callback(s).
    invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
    assert_eq!(cb_ref.call_count(), 1);
}

#[test]
fn test_invoke_owned_callbacks_from_map_multiple() {
    use parking_lot::RwLock;
    use std::collections::HashMap;

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let cb1 = Arc::new(CountingOwnedCallback::new());
    let cb2 = Arc::new(CountingOwnedCallback::new());
    let cb1_ref = Arc::clone(&cb1);
    let cb2_ref = Arc::clone(&cb2);
    register_owned_callback_for_test(&owned_callbacks, cb1);
    register_owned_callback_for_test(&owned_callbacks, cb2);

    let owned_frame = OwnedFrame {
        data: bytes::BytesMut::from(&b"test data for multiple callbacks"[..]),
        timestamp: 1000,
        frame_type: FrameType::VideoIFrame,
        stream_id: StreamId::VideoMain,
    };

    // Frame is consumed by the registered callback(s).
    invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);
    assert_eq!(cb1_ref.call_count(), 1);
    assert_eq!(cb2_ref.call_count(), 1);
}

#[test]
fn test_invoke_owned_callbacks_from_map_panic_recovery() {
    use parking_lot::RwLock;
    use std::collections::HashMap;

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Register a panicking callback
    let panicking = Arc::new(PanickingOwnedCallback);
    register_owned_callback_for_test(&owned_callbacks, panicking);

    // Register a normal callback
    let normal = Arc::new(CountingOwnedCallback::new());
    let normal_ref = Arc::clone(&normal);
    register_owned_callback_for_test(&owned_callbacks, normal);

    let owned_frame = OwnedFrame {
        data: bytes::BytesMut::from(&b"test panic recovery"[..]),
        timestamp: 1000,
        frame_type: FrameType::VideoIFrame,
        stream_id: StreamId::VideoMain,
    };

    // Should not panic - panic should be caught
    invoke_owned_callbacks_from_map(&owned_callbacks, owned_frame);

    // Panicking callback should be removed, normal should remain
    assert_eq!(owned_callbacks.read().len(), 1);

    // Normal callback may or may not have been invoked depending on iteration order
    let _ = normal_ref.call_count();
}

// ===== handle_pushed_frame / run_push_loop Tests =====

/// Shared setup for the push-delivery tests: an `AnykaIpc` over socketpairs, one
/// registered counting callback, and fresh per-stream state.
struct PushFixture {
    ipc: crate::hal::anyka::ipc::AnykaIpc,
    owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>>,
    cb: Arc<CountingOwnedCallback>,
    stream_health: StreamHealthCounters,
    main_state: StreamState,
    sub_state: StreamState,
    // Peer ends are kept alive so the sockets stay open (a dropped peer would
    // turn a quiet poll into POLLHUP).
    _ctrl_peer: std::os::unix::net::UnixStream,
    _frame_peer: Option<std::os::unix::net::UnixStream>,
}

impl PushFixture {
    /// `with_frame_socket` attaches a silent frame-notification socket, so
    /// `recv_pushed_frame` polls and times out instead of failing immediately.
    fn new(with_frame_socket: bool) -> Self {
        use std::os::unix::net::UnixStream;

        let (ctrl_a, ctrl_peer) = UnixStream::pair().unwrap();
        let (frame_reader, frame_peer) = if with_frame_socket {
            let (reader, peer) = UnixStream::pair().unwrap();
            (Some(reader), Some(peer))
        } else {
            (None, None)
        };
        let ipc =
            crate::hal::anyka::ipc::AnykaIpc::from_parts_for_test(ctrl_a, frame_reader, None, None);

        let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let cb = Arc::new(CountingOwnedCallback::new());
        register_owned_callback_for_test(
            &owned_callbacks,
            Arc::clone(&cb) as Arc<dyn OwnedFrameCallback>,
        );

        Self {
            ipc,
            owned_callbacks,
            cb,
            stream_health: StreamHealthCounters::default(),
            main_state: StreamState::new(StreamId::VideoMain, None),
            sub_state: StreamState::new(StreamId::VideoSub, None),
            _ctrl_peer: ctrl_peer,
            _frame_peer: frame_peer,
        }
    }

    fn push(&mut self, frame: OwnedFrame) {
        handle_pushed_frame(
            &self.ipc,
            &self.owned_callbacks,
            &self.stream_health,
            &mut self.main_state,
            &mut self.sub_state,
            frame,
        );
    }
}

fn test_frame(payload: &[u8], frame_type: FrameType, stream_id: StreamId) -> OwnedFrame {
    OwnedFrame {
        data: bytes::BytesMut::from(payload),
        timestamp: 1000,
        frame_type,
        stream_id,
    }
}

#[test]
fn test_handle_pushed_frame_updates_state_and_invokes_callback() {
    let mut fx = PushFixture::new(false);
    let payload = b"iframe payload";

    fx.push(test_frame(
        payload,
        FrameType::VideoIFrame,
        StreamId::VideoMain,
    ));

    assert_eq!(fx.cb.call_count(), 1);
    assert_eq!(fx.cb.last_size(), payload.len() as u64);
    assert_eq!(fx.main_state.frame_count, 1);
    assert_eq!(fx.main_state.iframe_count, 1);
    assert_eq!(fx.main_state.total_bytes, payload.len() as u64);
    assert_eq!(fx.sub_state.frame_count, 0);
    assert_eq!(fx.stream_health.main_frames.load(Ordering::SeqCst), 1);
    assert_eq!(fx.stream_health.sub_frames.load(Ordering::SeqCst), 0);
}

#[test]
fn test_handle_pushed_frame_routes_sub_stream_and_skips_iframe_count() {
    let mut fx = PushFixture::new(false);
    let payload = b"pframe payload";

    fx.push(test_frame(
        payload,
        FrameType::VideoPFrame,
        StreamId::VideoSub,
    ));

    assert_eq!(fx.cb.call_count(), 1);
    assert_eq!(fx.sub_state.frame_count, 1);
    assert_eq!(fx.sub_state.iframe_count, 0);
    assert_eq!(fx.main_state.frame_count, 0);
    assert_eq!(fx.stream_health.sub_frames.load(Ordering::SeqCst), 1);
    assert_eq!(fx.stream_health.main_frames.load(Ordering::SeqCst), 0);
}

#[test]
fn test_handle_pushed_frame_ignores_audio_frame() {
    let mut fx = PushFixture::new(false);

    fx.push(test_frame(
        b"audio payload",
        FrameType::AudioPacket,
        StreamId::Audio,
    ));

    assert_eq!(fx.cb.call_count(), 0);
    assert_eq!(fx.main_state.frame_count, 0);
    assert_eq!(fx.sub_state.frame_count, 0);
}

#[test]
fn test_run_push_loop_exits_on_stop_signal_without_invoking_callbacks() {
    // Frame socket stays open but silent, so each poll cycle times out with no
    // data — exactly what run_push_loop sees while waiting for a stop signal.
    let mut fx = PushFixture::new(true);

    let stop_signal = Arc::new(AtomicBool::new(false));
    let stop_signal_setter = Arc::clone(&stop_signal);
    let setter_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(50));
        stop_signal_setter.store(true, Ordering::SeqCst);
    });

    run_push_loop(
        &fx.ipc,
        &fx.owned_callbacks,
        &stop_signal,
        &fx.stream_health,
        None,
        &mut fx.main_state,
        &mut fx.sub_state,
    );

    setter_thread.join().unwrap();

    assert_eq!(fx.cb.call_count(), 0);
    assert_eq!(fx.stream_health.main_frames.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn test_video_encoder_request_idr_main() {
    let mut mock = mock_ffi_with_successful_encoder_open();
    mock.expect_venc_set_iframe()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    // Initialize main encoder
    let config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        ..Default::default()
    };
    encoder.init(&config).await.unwrap();

    let result = encoder.request_idr_frame(true);
    assert!(result.is_ok());
}

#[test]
fn test_video_encoder_request_idr_not_initialized() {
    let mock = MockVideoHalTrait::new();
    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));

    let result = encoder.request_idr_frame(true);
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareUnavailable(msg)) => {
            assert!(msg.contains("main encoder not initialized"));
        }
        other => panic!("Expected HardwareUnavailable, got {:?}", other),
    }
}

#[tokio::test]
async fn test_video_encoder_concurrent_config_access() {
    let mock = MockVideoHalTrait::new();
    let encoder = Arc::new(AnykaVideoEncoder::with_ffi(Arc::new(mock)));

    let e1 = encoder.clone();
    let e2 = encoder.clone();

    let (r1, r2) = tokio::join!(
        tokio::spawn(async move { e1.get_configurations().await }),
        tokio::spawn(async move { e2.get_configurations().await }),
    );

    assert!(r1.unwrap().is_ok());
    assert!(r2.unwrap().is_ok());
}

// =========================================================================
// VideoStreamHandle Tests
// =========================================================================

#[test]
fn test_video_stream_handle_creation_success() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

    mock.expect_venc_request_stream()
        .times(1)
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

    let result = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock));
    assert!(result.is_ok());
    // Drop triggers venc_cancel_stream
}

#[test]
fn test_video_stream_handle_creation_null_returns_error() {
    let mut mock = MockVideoHalTrait::new();

    mock.expect_venc_request_stream()
        .times(1)
        .returning(|_, _| std::ptr::null_mut());

    let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

    let result = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock));
    assert!(result.is_err());
    match result {
        Err(PlatformError::HardwareFailure(msg)) => {
            assert!(msg.contains("ak_venc_request_stream"));
        }
        _ => panic!("Expected HardwareFailure error"),
    }
}

#[test]
fn test_video_stream_handle_drop_calls_cancel() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .withf(move |handle| *handle == test_ptr as *mut c_void)
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

    let sh = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock)).unwrap();
    drop(sh); // Should call venc_cancel_stream exactly once
}

#[test]
fn test_video_stream_handle_explicit_cancel_is_idempotent() {
    let mut mock = MockVideoHalTrait::new();
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;

    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .withf(move |handle| *handle == test_ptr as *mut c_void)
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let vi_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_handle = std::ptr::NonNull::<c_void>::dangling().as_ptr();

    let sh = VideoStreamHandle::new(vi_handle, venc_handle, Arc::new(mock)).unwrap();
    assert!(sh.cancel());
    assert!(sh.cancel()); // Second cancel is a no-op success
    drop(sh); // Drop must not invoke cancel again
}

// =========================================================================
// Frame Type Conversion Tests
// =========================================================================

#[cfg(use_stubs)]
#[test]
fn test_frame_type_conversion_i_frame() {
    use crate::hal::common::VideoFrameType;
    assert_eq!(
        sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeI),
        FrameType::VideoIFrame
    );
}

#[cfg(use_stubs)]
#[test]
fn test_frame_type_conversion_pi_frame() {
    use crate::hal::common::VideoFrameType;
    assert_eq!(
        sdk_frame_type_to_frame_type(VideoFrameType::FrameTypePi),
        FrameType::VideoPiFrame
    );
}

#[cfg(use_stubs)]
#[test]
fn test_frame_type_conversion_p_frame() {
    use crate::hal::common::VideoFrameType;
    assert_eq!(
        sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeP),
        FrameType::VideoPFrame
    );
}

#[cfg(use_stubs)]
#[test]
fn test_frame_type_conversion_b_frame() {
    use crate::hal::common::VideoFrameType;
    assert_eq!(
        sdk_frame_type_to_frame_type(VideoFrameType::FrameTypeB),
        FrameType::VideoBFrame
    );
}

#[test]
fn test_push_mode_transient_error_classification() {
    assert!(is_push_mode_transient_error(&PlatformError::Timeout));
    assert!(is_push_mode_transient_error(&PlatformError::ResourceBusy(
        "frame dropped".to_string()
    )));
    assert!(!is_push_mode_transient_error(
        &PlatformError::HardwareFailure("socket disconnected".to_string())
    ));
}

// =========================================================================
// Timestamp Conversion Tests
// =========================================================================

#[test]
fn test_timestamp_conversion_ms_to_us() {
    // SDK timestamps are in ms, Frame uses µs
    let sdk_ts_ms: u64 = 12345;
    let frame_ts_us = sdk_ts_ms.wrapping_mul(1000);
    assert_eq!(frame_ts_us, 12_345_000);
}

#[test]
fn test_timestamp_conversion_zero() {
    let sdk_ts_ms: u64 = 0;
    let frame_ts_us = sdk_ts_ms.wrapping_mul(1000);
    assert_eq!(frame_ts_us, 0);
}

#[test]
fn test_timestamp_conversion_wrapping() {
    // Verify wrapping_mul won't panic on large values
    let sdk_ts_ms: u64 = u64::MAX;
    let frame_ts_us = sdk_ts_ms.wrapping_mul(1000);
    // Just verify it doesn't panic; exact value isn't important
    let _ = frame_ts_us;
}

// =========================================================================
// Frame Read Loop Tests
// =========================================================================

#[test]
#[cfg(use_stubs)]
fn test_frame_read_loop_invokes_callbacks() {
    use crate::hal::common::VideoFrameType;
    use std::sync::atomic::AtomicUsize;

    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = Arc::clone(&call_count);

    struct CountingCallback(Arc<AtomicUsize>);
    impl OwnedFrameCallback for CountingCallback {
        fn on_owned_frame(&self, frame: OwnedFrame) {
            assert_eq!(frame.stream_id, StreamId::VideoMain);
            assert_eq!(frame.frame_type, FrameType::VideoIFrame);
            assert_eq!(frame.data.len(), 100);
            // SDK ts=5000ms → OwnedFrame ts=5000ms (both in milliseconds)
            assert_eq!(frame.timestamp, 5000);
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);

    // Frame data buffer
    let frame_data: Vec<u8> = vec![0xAB; 100];
    let frame_data_ptr = frame_data.as_ptr() as usize;
    let get_stream_ptr = Arc::new(AtomicUsize::new(0));
    let get_stream_ptr_clone = Arc::clone(&get_stream_ptr);

    // Drain-loop pattern: first call returns a frame (sets stop signal),
    // second call returns failure to break inner drain loop.
    let call_idx = Arc::new(AtomicUsize::new(0));
    let call_idx_clone = Arc::clone(&call_idx);
    mock.expect_venc_get_stream()
        .returning(move |_, stream_ptr| {
            get_stream_ptr_clone.store(stream_ptr as usize, Ordering::SeqCst);
            let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
            if idx == 0 {
                // First call: return a frame
                unsafe {
                    let stream = &mut *stream_ptr;
                    stream.data = frame_data_ptr as *mut u8;
                    stream.len = 100;
                    stream.ts = 5000; // ms
                    stream.seq_no = 1;
                    stream.frame_type = VideoFrameType::FrameTypeI;
                }
                stop_clone.store(true, Ordering::SeqCst);
                AK_SUCCESS_I32
            } else {
                // Subsequent calls: no more frames (breaks drain loop)
                crate::hal::common::AK_FAILED_I32
            }
        });
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);

    mock.expect_venc_release_stream()
        .times(1)
        .returning(move |_, stream_ptr| {
            assert_eq!(
                stream_ptr as usize,
                get_stream_ptr.load(Ordering::SeqCst),
                "release must use same video_stream pointer as get"
            );
            AK_SUCCESS_I32
        });

    // Stream handle creation + cancel
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));
    owned_callbacks
        .write()
        .insert(1, Arc::new(CountingCallback(call_count_clone)));

    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );

    assert_eq!(call_count.load(Ordering::SeqCst), 1);
    // Keep frame_data alive until after the loop
    drop(frame_data);
}

#[test]
fn test_frame_read_loop_handles_no_data_and_retries() {
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let error_count = Arc::new(AtomicU32::new(0));
    let error_count_clone = Arc::clone(&error_count);

    // Return no-data errors, then signal stop after 2 errors.
    // In the drain-loop pattern, get_stream returning non-success breaks
    // the inner loop, then the outer loop sleeps and retries.
    mock.expect_venc_get_stream().returning(move |_, _| {
        let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
        if count >= 1 {
            stop_clone.store(true, Ordering::SeqCst);
        }
        crate::hal::common::AK_FAILED_I32
    });
    // No-data errors don't call get_error_str (only non-no-data errors do)
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);

    // No release_stream calls expected (errors don't produce frames)
    // Stream handle
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );

    // Should have retried at least twice
    assert!(error_count.load(Ordering::SeqCst) >= 2);
}

#[test]
fn test_stop_signal_terminates_loop() {
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(true)); // Pre-set stop

    // get_stream should never be called since stop is already set
    // (but allow 0 calls in case of timing)
    mock.expect_venc_get_stream()
        .times(0)
        .returning(|_, _| AK_FAILED_I32);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    // Should return immediately
    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );
}

#[tokio::test]
async fn test_start_stop_streaming_lifecycle() {
    let mut mock = MockVideoHalTrait::new();

    // Encoder open expectations
    mock.expect_venc_set_cfg_path().returning(|_| 0);
    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_open()
        .returning(move |_| test_ptr as *mut c_void);

    // Stream lifecycle expectations
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_set_iframe()
        .times(1..)
        .returning(|_| AK_SUCCESS_I32);
    mock.expect_venc_get_stream()
        .returning(|_, _| AK_FAILED_I32); // No frames in test
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);
    let encoder = Arc::new(AnykaVideoEncoder::with_ffi(Arc::new(mock)));
    // Keep the reader loop on active poll cadence in this lifecycle test
    // so stop/join timing is deterministic under host CI scheduling.
    let _callback_id =
        encoder.register_owned_frame_callback(Arc::new(CountingOwnedCallback::new()));

    // Initialize main encoder
    let config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        quality: 80,
        ..Default::default()
    };
    encoder.init(&config).await.unwrap();

    // Create a dummy VI handle for testing
    let vi_handle = Arc::new(crate::hal::common::video::VideoInputHandle::test_handle());
    let main_enc = encoder.main_handle.read().clone().unwrap();

    // Start streaming
    let result = encoder.start_streaming(&vi_handle, &main_enc, None);
    assert!(result.is_ok());

    // Verify threads are running
    assert!(encoder.main_stream_handle.read().is_some());
    assert!(encoder.read_thread.read().is_some());

    // Stop streaming
    let stop_result = encoder.stop_streaming();
    assert!(
        stop_result.is_ok(),
        "stop_streaming failed: {:?}",
        stop_result
    );

    // Verify cleanup
    assert!(encoder.main_stream_handle.read().is_none());
    assert!(encoder.read_thread.read().is_none());
}

#[test]
fn test_stop_streaming_join_timeout_after_cancel_marks_unsafe() {
    let mut mock = MockVideoHalTrait::new();
    let stream_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .times(1)
        .returning(move |_, _| stream_ptr as *mut c_void);
    // Cancel is called first in the new ordering.
    mock.expect_venc_cancel_stream()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());
    *encoder.main_stream_handle.write() = Some(sh);

    // Simulate a reader thread stuck in kernel I/O that doesn't unblock
    // even after cancel (e.g. blocked in a kernel ioctl).
    let blocked = std::thread::spawn(move || {
        // Force the thread to outlive STREAM_THREAD_JOIN_TIMEOUT in tests.
        std::thread::sleep(Duration::from_millis(200));
    });
    *encoder.read_thread.write() = Some(blocked);

    let result = encoder.stop_streaming();
    assert!(result.is_err());
    assert!(encoder.requires_hard_shutdown());
    match result {
        Err(PlatformError::HardwareFailure(msg)) => {
            assert!(msg.contains("unsafe teardown required"));
            assert!(msg.contains("join timeout"));
        }
        other => panic!("Expected unsafe teardown HardwareFailure, got {:?}", other),
    }
}

#[test]
fn test_stop_streaming_cancel_failure_still_attempts_second_channel() {
    let mut mock = MockVideoHalTrait::new();
    let mut request_seq = mockall::Sequence::new();
    let stream_ptr_a = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    let stream_ptr_b = (stream_ptr_a.wrapping_add(16)) as *mut c_void as usize;

    mock.expect_venc_request_stream()
        .times(1)
        .in_sequence(&mut request_seq)
        .returning(move |_, _| stream_ptr_a as *mut c_void);
    mock.expect_venc_request_stream()
        .times(1)
        .in_sequence(&mut request_seq)
        .returning(move |_, _| stream_ptr_b as *mut c_void);

    // First channel cancel fails; sub-channel cancel should still be attempted
    // to avoid leaving SDK threads running.
    mock.expect_venc_cancel_stream()
        .times(1)
        .returning(|_| AK_FAILED_I32);
    mock.expect_venc_cancel_stream()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr_main = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr_sub = (venc_ptr_main as usize).wrapping_add(32) as *mut c_void;
    let main = Arc::new(
        VideoStreamHandle::new(vi_ptr, venc_ptr_main, Arc::clone(&ffi)).expect("main stream"),
    );
    let sub = Arc::new(
        VideoStreamHandle::new(vi_ptr, venc_ptr_sub, Arc::clone(&ffi)).expect("sub stream"),
    );
    *encoder.main_stream_handle.write() = Some(main);
    *encoder.sub_stream_handle.write() = Some(sub);

    let result = encoder.stop_streaming();
    assert!(result.is_err());
    assert!(encoder.requires_hard_shutdown());
}

#[test]
fn test_stop_streaming_cancel_timeout_marks_unsafe_and_attempts_both() {
    let mut mock = MockVideoHalTrait::new();
    let stream_ptr_a = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    let stream_ptr_b = (stream_ptr_a.wrapping_add(16)) as *mut c_void as usize;
    mock.expect_venc_request_stream()
        .times(1)
        .returning(move |_, _| stream_ptr_a as *mut c_void);
    mock.expect_venc_request_stream()
        .times(1)
        .returning(move |_, _| stream_ptr_b as *mut c_void);

    // Main cancel blocks past STREAM_CANCEL_TIMEOUT (20ms in tests).
    mock.expect_venc_cancel_stream().times(1).returning(|_| {
        std::thread::sleep(Duration::from_millis(200));
        AK_SUCCESS_I32
    });
    // Sub cancel still attempted.
    mock.expect_venc_cancel_stream()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr_main = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr_sub = (venc_ptr_main as usize).wrapping_add(32) as *mut c_void;
    let main = Arc::new(
        VideoStreamHandle::new(vi_ptr, venc_ptr_main, Arc::clone(&ffi)).expect("main stream"),
    );
    let sub = Arc::new(
        VideoStreamHandle::new(vi_ptr, venc_ptr_sub, Arc::clone(&ffi)).expect("sub stream"),
    );
    *encoder.main_stream_handle.write() = Some(main);
    *encoder.sub_stream_handle.write() = Some(sub);

    let result = encoder.stop_streaming();
    assert!(result.is_err());
    assert!(encoder.requires_hard_shutdown());
}

#[tokio::test]
async fn test_shutdown_order_is_cancel_then_join_then_close() {
    let mut mock = MockVideoHalTrait::new();
    let mut seq = mockall::Sequence::new();
    let enc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    let stream_ptr = (enc_ptr.wrapping_add(64)) as *mut c_void as usize;

    mock.expect_venc_set_cfg_path().times(1).returning(|_| 0);
    mock.expect_venc_open()
        .times(1)
        .returning(move |_| enc_ptr as *mut c_void);
    mock.expect_venc_request_stream()
        .times(1)
        .returning(move |_, _| stream_ptr as *mut c_void);

    // Cancel must happen before close (cancel-first pattern).
    mock.expect_venc_cancel_stream()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| AK_SUCCESS_I32);
    mock.expect_venc_close()
        .times(1)
        .in_sequence(&mut seq)
        .returning(|_| AK_SUCCESS_I32);

    let encoder = AnykaVideoEncoder::with_ffi(Arc::new(mock));
    let config = VideoEncoderConfig {
        token: "VideoEncoder_1".to_string(),
        name: "Main Stream".to_string(),
        resolution: Resolution::new(1280, 720),
        framerate: 15,
        bitrate: 2000,
        encoding: VideoEncoding::H264,
        gop_length: 30,
        quality: 80,
        ..Default::default()
    };
    encoder.init(&config).await.unwrap();

    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = encoder
        .main_handle
        .read()
        .as_ref()
        .expect("main encoder")
        .as_ptr();
    let sh = Arc::new(
        VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&encoder.ffi)).expect("stream handle"),
    );
    *encoder.main_stream_handle.write() = Some(sh);

    // Reader thread exits on stop_signal — simulates a non-stuck reader.
    let stop = Arc::clone(&encoder.stop_signal);
    let joinable_reader = std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    *encoder.read_thread.write() = Some(joinable_reader);

    encoder.stop_streaming().expect("stop_streaming");
    encoder.close_all_encoders().expect("close_all_encoders");
}

#[test]
fn test_stop_streaming_cancel_unblocks_reader_thread() {
    // Verify that the cancel-first pattern allows a reader thread that
    // exits on stop_signal to join successfully.
    let mut mock = MockVideoHalTrait::new();
    let stream_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .times(1)
        .returning(move |_, _| stream_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let encoder = AnykaVideoEncoder::with_ffi(Arc::clone(&ffi));
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());
    *encoder.main_stream_handle.write() = Some(sh);

    // Reader thread waits for stop_signal, then exits (simulates a thread
    // that would be stuck in get_stream until cancel fires).
    let stop = Arc::clone(&encoder.stop_signal);
    let reader = std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_millis(1));
        }
    });
    *encoder.read_thread.write() = Some(reader);

    let result = encoder.stop_streaming();
    assert!(result.is_ok(), "cancel-first should allow clean join");
    assert!(!encoder.requires_hard_shutdown());
    assert!(encoder.main_stream_handle.read().is_none());
}

#[test]
fn test_frame_read_loop_no_initial_delay() {
    // Verify frame_read_loop exits quickly when stop signal is pre-set
    // (no 100ms initial delay — removed in drain-loop refactor).
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(true)); // Pre-set stop

    mock.expect_venc_get_stream()
        .times(0)
        .returning(|_, _| AK_FAILED_I32);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let start = std::time::Instant::now();
    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );
    let elapsed = start.elapsed();

    // Should exit very quickly — no initial delay
    assert!(
        elapsed < Duration::from_millis(50),
        "Expected fast exit with pre-set stop, got {:?}",
        elapsed
    );
}

#[test]
fn test_drain_loop_adaptive_sleep() {
    // Verify the drain-loop uses adaptive sleep (50ms default) between cycles.
    // With no-data errors, each cycle backs off: 50 + 100 + 200 + 200 = 550ms (capped at 4x).
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let error_count = Arc::new(AtomicU32::new(0));
    let error_count_clone = Arc::clone(&error_count);

    // Return 4 no-data errors, then stop
    mock.expect_venc_get_stream().returning(move |_, _| {
        let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
        if count >= 3 {
            stop_clone.store(true, Ordering::SeqCst);
        }
        crate::hal::common::AK_FAILED_I32
    });
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let start = std::time::Instant::now();
    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );
    let elapsed = start.elapsed();

    assert!(error_count.load(Ordering::SeqCst) >= 4);
    // 4 drain cycles × adaptive sleep: 50 + 100 + 200 + 200 = 550ms (capped at 4x base).
    // Allow tolerance: should be between 400ms and 800ms.
    assert!(
        elapsed >= Duration::from_millis(400) && elapsed <= Duration::from_millis(800),
        "Expected ~550ms with adaptive sleep over 4 cycles, got {:?}",
        elapsed
    );
}

#[test]
fn test_drain_loop_resets_sleep_on_frame() {
    // Verify adaptive sleep resets to base when frames are available.
    // Cycle 1: no-data (sleep 100ms), cycle 2: frame + no-data (sleep 50ms), cycle 3: no-data (sleep 100ms).
    // Total: ~250ms (100 + 50 + 100).
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let call_idx = Arc::new(AtomicU32::new(0));
    let call_idx_clone = Arc::clone(&call_idx);

    let frame_data: Vec<u8> = vec![0xCD; 64];
    let frame_data_ptr = frame_data.as_ptr() as usize;

    // Cycle 1: no-data (sleep backs off)
    // Cycle 2: frame found then no-data (sleep resets to base, then backs off)
    // Cycle 3: no-data (sleep backs off again)
    // Cycle 4: no-data and stop
    mock.expect_venc_get_stream()
        .returning(move |_, stream_ptr| {
            let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
            match idx {
                0 => crate::hal::common::AK_FAILED_I32, // no-data: sleep will back off to 100ms
                1 => {
                    // First call in cycle 2: return frame (sleep resets to 50ms)
                    unsafe {
                        let stream = &mut *stream_ptr;
                        stream.data = frame_data_ptr as *mut u8;
                        stream.len = 64;
                        stream.ts = 1000;
                        stream.seq_no = 1;
                        stream.frame_type = VideoFrameType::FrameTypeP;
                    }
                    crate::hal::common::AK_SUCCESS_I32
                }
                2 => crate::hal::common::AK_FAILED_I32, // Second call in cycle 2: no-data (sleep 100ms)
                3 => crate::hal::common::AK_FAILED_I32, // no-data (sleep 200ms capped)
                _ => {
                    stop_clone.store(true, Ordering::SeqCst);
                    crate::hal::common::AK_FAILED_I32
                }
            }
        });
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
    mock.expect_venc_release_stream()
        .returning(|_, _| AK_SUCCESS_I32);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    let start = std::time::Instant::now();
    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );
    let elapsed = start.elapsed();

    // Adaptive timing: 100 + 50 + 100 = ~250ms total
    // Allow tolerance: 180ms to 600ms (generous for CI environments)
    assert!(
        elapsed >= Duration::from_millis(180) && elapsed <= Duration::from_millis(600),
        "Expected ~250ms with reset on frame, got {:?}",
        elapsed
    );
}

#[test]
fn test_drain_loop_skips_get_error_no_on_fast_path() {
    // Verify get_error_no() is NOT called on every no-data cycle (optimistic fast path).
    // With the probe interval of 50, it should only probe occasionally.
    // First cycle always probes (to establish baseline), subsequent cycles skip until probe_interval.
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let call_idx = Arc::new(AtomicU32::new(0));
    let call_idx_clone = Arc::clone(&call_idx);

    // Run many no-data cycles and count how many times get_error_no is called
    mock.expect_venc_get_stream().returning(move |_, _| {
        let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
        if idx >= 10 {
            stop_clone.store(true, Ordering::SeqCst);
        }
        crate::hal::common::AK_FAILED_I32
    });
    mock.expect_get_error_no()
        .times(1) // Should be called only once (first cycle probes, rest skip)
        .returning(|| SDK_ERROR_NO_DATA);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        None,
        None,
        None,
        None,
    );

    // get_error_no should have been called only 1 time, not 10 times
    // (first cycle always probes, rest skip until probe_interval=50)
}

#[test]
fn test_frame_read_loop_skips_idr_before_first_frame() {
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let error_count = Arc::new(AtomicU32::new(0));
    let error_count_clone = Arc::clone(&error_count);

    mock.expect_venc_get_stream().returning(move |_, _| {
        let count = error_count_clone.fetch_add(1, Ordering::SeqCst);
        if count >= NO_DATA_IDR_RECOVERY_EVERY_ERRORS {
            stop_clone.store(true, Ordering::SeqCst);
        }
        AK_FAILED_I32
    });
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
    // No-data errors don't call get_error_str in drain-loop pattern
    mock.expect_venc_set_iframe().times(0);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        Some(venc_ptr as usize),
        None,
        None,
        None,
    );

    assert!(error_count.load(Ordering::SeqCst) >= NO_DATA_IDR_RECOVERY_EVERY_ERRORS);
}

#[test]
fn test_frame_read_loop_requests_idr_after_frames_then_sustained_no_data() {
    let mut mock = MockVideoHalTrait::new();
    let stop = Arc::new(AtomicBool::new(false));
    let stop_clone = Arc::clone(&stop);
    let no_data_count = Arc::new(AtomicU32::new(0));
    let no_data_count_clone = Arc::clone(&no_data_count);

    let frame_data: Vec<u8> = vec![0xCD; 64];
    let frame_data_ptr = frame_data.as_ptr() as usize;
    let call_idx = Arc::new(AtomicU32::new(0));
    let call_idx_clone = Arc::clone(&call_idx);

    mock.expect_venc_get_stream()
        .returning(move |_, stream_ptr| {
            let idx = call_idx_clone.fetch_add(1, Ordering::SeqCst);
            if idx == 0 {
                unsafe {
                    let stream = &mut *stream_ptr;
                    stream.data = frame_data_ptr as *mut u8;
                    stream.len = 64;
                    stream.ts = 9000;
                    stream.seq_no = 1;
                    stream.frame_type = VideoFrameType::FrameTypeI;
                }
                AK_SUCCESS_I32
            } else {
                let errs = no_data_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
                if errs > NO_DATA_IDR_RECOVERY_EVERY_ERRORS {
                    stop_clone.store(true, Ordering::SeqCst);
                }
                AK_FAILED_I32
            }
        });
    mock.expect_get_error_no().returning(|| SDK_ERROR_NO_DATA);
    mock.expect_venc_set_iframe_by_addr()
        .times(1)
        .returning(|_| AK_SUCCESS_I32);
    mock.expect_venc_release_stream()
        .times(1)
        .returning(|_, _| AK_SUCCESS_I32);

    let test_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr() as usize;
    mock.expect_venc_request_stream()
        .returning(move |_, _| test_ptr as *mut c_void);
    mock.expect_venc_cancel_stream()
        .returning(|_| AK_SUCCESS_I32);

    let ffi: Arc<dyn crate::hal::common::video::VideoHalTrait> = Arc::new(mock);
    let vi_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let venc_ptr = std::ptr::NonNull::<c_void>::dangling().as_ptr();
    let sh = Arc::new(VideoStreamHandle::new(vi_ptr, venc_ptr, Arc::clone(&ffi)).unwrap());

    let owned_callbacks: Arc<RwLock<HashMap<CallbackId, Arc<dyn OwnedFrameCallback>>>> =
        Arc::new(RwLock::new(HashMap::new()));

    unified_frame_read_loop(
        sh,
        None,
        ffi,
        owned_callbacks,
        stop,
        Arc::new(StreamHealthCounters::default()),
        Some(venc_ptr as usize),
        None,
        None,
        None,
    );

    assert!(no_data_count.load(Ordering::SeqCst) > NO_DATA_IDR_RECOVERY_EVERY_ERRORS);
    drop(frame_data);
}

/// `min_qp` must survive the trip from config into `encode_param`, and stay inside the range
/// `ak_venc.h` documents for it.
///
/// Both halves matter. The value was a hardcoded `20` in `config_to_encode_param`, so a
/// configured floor that never reached `ak_venc_open` would look exactly like today's behaviour
/// and go unnoticed — which is how `gop_length` stayed broken. And an out-of-range floor must
/// clamp rather than reach the SDK, since `encode_param` is only validated by the encoder opening
/// or not, and a camera that fails to open is worse than one at the wrong quantiser.
#[test]
fn test_min_qp_reaches_encode_param_and_clamps_to_sdk_range() {
    let param_for = |min_qp: u32| {
        AnykaVideoEncoder::config_to_encode_param(
            &VideoEncoderConfig {
                token: "VideoEncoder_1".to_string(),
                resolution: Resolution::new(1280, 720),
                framerate: 15,
                bitrate: 1500,
                encoding: VideoEncoding::H264,
                gop_length: 50,
                min_qp,
                ..Default::default()
            },
            encode_use_chn::ENCODE_MAIN_CHN,
        )
    };

    // In range: passed through untouched. 25 is the setting that halves I-frame size.
    assert_eq!(param_for(20).minqp, 20);
    assert_eq!(param_for(25).minqp, 25);
    assert_eq!(param_for(23).minqp, 23);

    // `Default` yields 0, which must read as the SDK floor, not as 0 — otherwise every
    // default-constructed config would silently ask the encoder for an illegal quantiser.
    assert_eq!(param_for(0).minqp, *SDK_MIN_QP_RANGE.start() as i32);
    assert_eq!(param_for(19).minqp, 20);
    assert_eq!(param_for(51).minqp, 25);

    // gop_length travels the same path and must not be re-hardcoded either.
    assert_eq!(param_for(20).goplen, 50);
}

/// The constructor is the only place `gop_length` and `min_qp` can be set, so it must actually
/// use what the caller passed instead of the built-in defaults.
#[tokio::test]
async fn test_open_params_seed_both_encoder_configurations() {
    let encoder = AnykaVideoEncoder::with_ffi_and_params(
        Arc::new(MockVideoHalTrait::new()),
        StreamOpenParams {
            gop_length: 50,
            min_qp: 25,
        },
        StreamOpenParams {
            gop_length: 45,
            min_qp: 22,
        },
    );

    let configs = encoder.get_configurations().await.unwrap();
    let main = configs
        .iter()
        .find(|c| c.token == "VideoEncoder_1")
        .expect("main encoder configuration");
    let sub = configs
        .iter()
        .find(|c| c.token == "VideoEncoder_2")
        .expect("sub encoder configuration");

    assert_eq!((main.gop_length, main.min_qp), (50, 25));
    assert_eq!((sub.gop_length, sub.min_qp), (45, 22));

    // An absent config must reproduce the values the encoder used before this was configurable.
    let defaults = AnykaVideoEncoder::with_ffi(Arc::new(MockVideoHalTrait::new()));
    let defaults = defaults.get_configurations().await.unwrap();
    assert_eq!((defaults[0].gop_length, defaults[0].min_qp), (30, 20));
}
