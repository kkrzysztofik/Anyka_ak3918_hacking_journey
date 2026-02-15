use std::path::PathBuf;

use rtsp_validation_tool::config::InitialTimestampPolicyArg;
use rtsp_validation_tool::httpflv::run_httpflv_validation;
use rtsp_validation_tool::{EffectiveConfig, TestResult};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

fn test_effective_config(httpflv_port: u16, httpflv_path: &str) -> EffectiveConfig {
    EffectiveConfig {
        rtsp_host: "127.0.0.1".to_string(),
        rtsp_port: 554,
        rtsp_stream: "/stream1".to_string(),
        stream_username: None,
        stream_password: None,
        rtsp_timeout_sec: 2,
        initial_timestamp_policy: InitialTimestampPolicyArg::Permissive,
        short_duration_sec: 10,
        long_duration_sec: 30,
        concurrent_clients: 1,
        video_startup_latency_ms: 1_500,
        harness_startup_latency_ms: 2_000,
        audio_startup_latency_ms: 2_000,
        bitrate_tolerance_percent: 20,
        fps_tolerance_percent: 20,
        packet_loss_tolerance_percent: 2.0,
        expected_bitrate_kbps: None,
        expected_fps: None,
        baseline_dir: PathBuf::from("rtsp_results/baselines"),
        capture_interface: String::new(),
        artifacts_root_dir: PathBuf::from("rtsp_results/runs"),
        artifacts_dir: PathBuf::from("rtsp_results/runs/test"),
        capture_tool_output: false,
        keep_pcaps: false,
        launch_on_device: false,
        device_host: "127.0.0.1".to_string(),
        device_ssh_port: 22,
        device_user: "root".to_string(),
        device_password: None,
        collect_telemetry: false,
        device_h264_file: None,
        device_aac_file: None,
        device_loop_playback: false,
        no_launch: true,
        h264_file: None,
        output: "report.json".to_string(),
        update_baseline: false,
        compare_baseline: false,
        ffmpeg_log_level: "warning".to_string(),
        httpflv_port,
        httpflv_path: httpflv_path.to_string(),
        httpflv_timeout_sec: 2,
    }
}

fn encode_flv_tag(tag_type: u8, timestamp: u32, body: &[u8], footer_prev_tag_size: u32) -> Vec<u8> {
    let data_size = body.len() as u32;
    let mut out = Vec::with_capacity(11 + body.len() + 4);
    out.push(tag_type);
    out.push(((data_size >> 16) & 0xFF) as u8);
    out.push(((data_size >> 8) & 0xFF) as u8);
    out.push((data_size & 0xFF) as u8);
    out.push(((timestamp >> 16) & 0xFF) as u8);
    out.push(((timestamp >> 8) & 0xFF) as u8);
    out.push((timestamp & 0xFF) as u8);
    out.push(((timestamp >> 24) & 0xFF) as u8);
    out.extend_from_slice(&[0, 0, 0]);
    out.extend_from_slice(body);
    out.extend_from_slice(&footer_prev_tag_size.to_be_bytes());
    out
}

fn minimal_valid_flv_stream() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"FLV");
    out.push(1);
    out.push(0x05);
    out.extend_from_slice(&[0, 0, 0, 9]);
    out.extend_from_slice(&0_u32.to_be_bytes());

    let video_body = [0x17, 0x00, 0x00, 0x00, 0x00];
    let video_footer = 11 + video_body.len() as u32;
    out.extend_from_slice(&encode_flv_tag(9, 0, &video_body, video_footer));

    let audio_body = [0xAF, 0x01];
    let audio_footer = 11 + audio_body.len() as u32;
    out.extend_from_slice(&encode_flv_tag(8, 23, &audio_body, audio_footer));

    out
}

async fn spawn_single_response_server(
    status_line: &str,
    body: Vec<u8>,
) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test HTTP server");
    let port = listener.local_addr().expect("read local addr").port();
    let status = status_line.to_string();

    let handle = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept test connection");
        let mut req_buf = [0_u8; 1024];
        let _ = socket.read(&mut req_buf).await;

        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write HTTP headers");
        socket.write_all(&body).await.expect("write HTTP body");
    });

    (port, handle)
}

fn has_pass(results: &[TestResult], name: &str) -> bool {
    results.iter().any(|result| {
        matches!(
            result,
            TestResult::Pass {
                name: result_name,
                ..
            } if result_name == name
        )
    })
}

fn has_fail_with_reason(results: &[TestResult], name: &str, reason_substring: &str) -> bool {
    results.iter().any(|result| {
        matches!(
            result,
            TestResult::Fail {
                name: result_name,
                reason,
                ..
            } if result_name == name && reason.contains(reason_substring)
        )
    })
}

fn has_fail(results: &[TestResult], name: &str) -> bool {
    results.iter().any(|result| {
        matches!(
            result,
            TestResult::Fail {
                name: result_name,
                ..
            } if result_name == name
        )
    })
}

#[tokio::test]
async fn test_run_httpflv_validation_connection_failure() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("reserve free port");
    let port = listener.local_addr().expect("read reserved port").port();
    drop(listener);

    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for connection failure");

    assert!(has_fail(&results, "httpflv_connect_ok"));
}

#[tokio::test]
async fn test_run_httpflv_validation_non_200_status() {
    let (port, handle) = spawn_single_response_server("404 Not Found", Vec::new()).await;

    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for non-200 status");
    handle.await.expect("join test server");

    assert!(has_pass(&results, "httpflv_connect_ok"));
    assert!(has_fail_with_reason(
        &results,
        "httpflv_status_200_ok",
        "HTTP status"
    ));
}

#[tokio::test]
async fn test_run_httpflv_validation_short_flv_header() {
    let (port, handle) = spawn_single_response_server("200 OK", vec![0x46, 0x4C, 0x56, 0x01]).await;

    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for short FLV header");
    handle.await.expect("join test server");

    assert!(has_fail_with_reason(
        &results,
        "httpflv_header_valid",
        "response too short"
    ));
}

#[tokio::test]
async fn test_run_httpflv_validation_invalid_first_prev_tag_size() {
    let mut body = Vec::new();
    body.extend_from_slice(b"FLV");
    body.extend_from_slice(&[1, 0x05, 0, 0, 0, 9]);
    body.extend_from_slice(&1_u32.to_be_bytes());

    let (port, handle) = spawn_single_response_server("200 OK", body).await;
    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for invalid first PreviousTagSize");
    handle.await.expect("join test server");

    assert!(has_fail_with_reason(
        &results,
        "httpflv_first_prev_tag_size_zero",
        "expected 0"
    ));
}

#[tokio::test]
async fn test_run_httpflv_validation_invalid_tag_type() {
    let mut body = Vec::new();
    body.extend_from_slice(b"FLV");
    body.extend_from_slice(&[1, 0x05, 0, 0, 0, 9]);
    body.extend_from_slice(&0_u32.to_be_bytes());
    body.extend_from_slice(&encode_flv_tag(7, 0, &[0x00], 12));

    let (port, handle) = spawn_single_response_server("200 OK", body).await;
    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for invalid tag type");
    handle.await.expect("join test server");

    assert!(has_fail_with_reason(
        &results,
        "httpflv_tags_valid",
        "invalid tag type"
    ));
}

#[tokio::test]
async fn test_run_httpflv_validation_footer_mismatch() {
    let mut body = Vec::new();
    body.extend_from_slice(b"FLV");
    body.extend_from_slice(&[1, 0x05, 0, 0, 0, 9]);
    body.extend_from_slice(&0_u32.to_be_bytes());

    let video_body = [0x17, 0x00, 0x00, 0x00, 0x00];
    body.extend_from_slice(&encode_flv_tag(9, 0, &video_body, 0));

    let (port, handle) = spawn_single_response_server("200 OK", body).await;
    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for footer mismatch");
    handle.await.expect("join test server");

    assert!(has_fail_with_reason(
        &results,
        "httpflv_tags_valid",
        "incorrect PreviousTagSize"
    ));
}

#[tokio::test]
async fn test_run_httpflv_validation_valid_minimal_video_audio_stream() {
    let body = minimal_valid_flv_stream();
    let (port, handle) = spawn_single_response_server("200 OK", body).await;

    let effective = test_effective_config(port, "/live/stream.flv");
    let results = run_httpflv_validation(&effective)
        .await
        .expect("run validation for valid FLV stream");
    handle.await.expect("join test server");

    assert!(has_pass(&results, "httpflv_connect_ok"));
    assert!(has_pass(&results, "httpflv_status_200_ok"));
    assert!(has_pass(&results, "httpflv_header_valid"));
    assert!(has_pass(&results, "httpflv_first_prev_tag_size_zero"));
    assert!(has_pass(&results, "httpflv_tags_valid"));
    assert!(has_pass(&results, "httpflv_video_payload_header_valid"));
    assert!(has_pass(&results, "httpflv_audio_payload_header_valid"));
}
