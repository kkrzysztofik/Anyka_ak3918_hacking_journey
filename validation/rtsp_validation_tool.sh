#!/bin/bash
# rtsp_validation_tool.sh - RTSP performance and conformity testing
# Uses ffmpeg, ffprobe, tshark for metrics. Optional: start onvif-rust in validation-mode.

set -euo pipefail

# Configuration (override via env or rtsp_validation_tool.conf)
RTSP_HOST="${RTSP_HOST:-127.0.0.1}"
RTSP_PORT="${RTSP_PORT:-554}"
RTSP_STREAM="${RTSP_STREAM:-/vs0}"
TEST_DURATION="${TEST_DURATION:-}"
OUTPUT_FILE="${OUTPUT_FILE:-rtsp_validation.json}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BASELINE_DIR="${BASELINE_DIR:-${SCRIPT_DIR}/rtsp_results/baselines}"
REPO_ROOT="${REPO_ROOT:-$(cd "${SCRIPT_DIR}/.." && pwd)}"

# Process management
ONVIF_PID=""

log_info() { echo "[INFO] $*"; }
log_error() { echo "[ERROR] $*" >&2; }

# Source config if present
if [ -f "${SCRIPT_DIR}/rtsp_validation_tool.conf" ]; then
  # shellcheck source=rtsp_validation_tool.conf
  source "${SCRIPT_DIR}/rtsp_validation_tool.conf"
fi

# Apply defaults after sourcing config.
TEST_DURATION="${TEST_DURATION:-${SHORT_TEST_DURATION:-30}}"
LONG_TEST_DURATION="${LONG_TEST_DURATION:-600}"
CONCURRENT_CLIENT_COUNT="${CONCURRENT_CLIENT_COUNT:-4}"
RTSP_TIMEOUT="${RTSP_TIMEOUT:-10}"
VIDEO_STARTUP_LATENCY_MS="${VIDEO_STARTUP_LATENCY_MS:-1500}"
BITRATE_TOLERANCE_PERCENT="${BITRATE_TOLERANCE_PERCENT:-15}"
FPS_TOLERANCE_PERCENT="${FPS_TOLERANCE_PERCENT:-10}"
PACKET_LOSS_TOLERANCE_PERCENT="${PACKET_LOSS_TOLERANCE_PERCENT:-1}"

is_local_host() {
    case "${1:-}" in
        127.0.0.1|localhost|::1) return 0 ;;
        *) return 1 ;;
    esac
}

is_ip_literal() {
    local host="${1:-}"
    # Very small validator: IPv4 dotted-quad only (sufficient for tshark capture filter).
    [[ "$host" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]
}

CAPTURE_IFACE="${CAPTURE_IFACE:-}"
if [ -z "$CAPTURE_IFACE" ]; then
    if is_local_host "$RTSP_HOST"; then
        CAPTURE_IFACE="lo"
    else
        CAPTURE_IFACE="any"
    fi
fi

# --- Process management ---
start_onvif_server() {
    local h264_file="$1"
    local rtsp_port="${2:-554}"
    local httpflv_port="${3:-8080}"
    local stream_path="${4:-/stream1}"

    log_info "Starting onvif-rust in validation mode..."

    local onvif_bin="${REPO_ROOT}/cross-compile/onvif-rust/target/debug/onvif-rust"
    if [ ! -f "$onvif_bin" ]; then
        onvif_bin="${REPO_ROOT}/cross-compile/onvif-rust/target/x86_64-unknown-linux-gnu/debug/onvif-rust"
    fi
    if [ ! -f "$onvif_bin" ]; then
        log_error "onvif-rust binary not found. Build with: cd cross-compile/onvif-rust && cargo build --features validation-mode"
        return 1
    fi

    "$onvif_bin" \
        --validation-mode \
        --h264-file "$h264_file" \
        --rtsp-port "$rtsp_port" \
        --httpflv-port "$httpflv_port" \
        > /tmp/onvif_server.log 2>&1 &
    ONVIF_PID=$!
    log_info "Started onvif-rust with PID $ONVIF_PID"
    wait_for_server "127.0.0.1" "$rtsp_port" "$stream_path"
}

wait_for_server() {
    local host="$1"
    local port="$2"
    local stream_path="$3"
    local max_attempts=30
    local attempt=0

    log_info "Waiting for RTSP server on ${host}:${port}..."

    while [ $attempt -lt $max_attempts ]; do
        if timeout 2 ffmpeg -rtsp_transport tcp \
            -i "rtsp://${host}:${port}${stream_path}" \
            -t 0.1 -f null - >/dev/null 2>&1; then
            log_info "Server is ready"
            return 0
        fi
        sleep 1
        ((attempt++)) || true
    done

    log_error "Server failed to start within timeout"
    return 1
}

stop_onvif_server() {
    if [ -n "${ONVIF_PID:-}" ]; then
        log_info "Stopping onvif-rust (PID $ONVIF_PID)..."
        kill -TERM "$ONVIF_PID" 2>/dev/null || true
        wait "$ONVIF_PID" 2>/dev/null || true
        ONVIF_PID=""
        log_info "Server stopped"
    fi
}

cleanup() {
    stop_onvif_server
    rm -f /tmp/rtsp_capture_$$.pcap /tmp/rtp_capture_$$.pcap
}
trap cleanup EXIT

# --- Metric validation (expected + tolerance percent) ---
within_tolerance() {
    local current="$1"
    local expected="$2"
    local tolerance_pct="${3:-10}"
    local delta_pct
    if [ -z "$expected" ] || [ "$expected" = "0" ]; then
        return 1
    fi
    delta_pct=$(awk "BEGIN {v=($current - $expected) / $expected * 100; if (v<0) v=-v; print v}" 2>/dev/null || echo "999")
    awk "BEGIN {exit ($delta_pct <= $tolerance_pct) ? 0 : 1}" 2>/dev/null
}

# --- JSON helpers (always use jq for escaping) ---
json_check() {
    local name="$1"
    local result="$2" # PASS|FAIL
    local reason="${3:-}"
    jq -n \
        --arg name "$name" \
        --arg result "$result" \
        --arg reason "$reason" \
        '{
            name: $name,
            result: $result
        } + (if ($reason | length) > 0 then {reason: $reason} else {} end)'
}

json_metric() {
    local name="$1"
    local result="$2" # PASS|FAIL
    local value_json="$3" # must be valid JSON scalar/object/array
    local unit="${4:-}"
    jq -n \
        --arg name "$name" \
        --arg result "$result" \
        --arg unit "$unit" \
        --argjson value "$value_json" \
        '{
            name: $name,
            result: $result,
            value: $value
        } + (if ($unit | length) > 0 then {unit: $unit} else {} end)'
}

# --- Baseline management ---
compare_against_baseline() {
    local test_name="$1"
    local current_value="$2"
    local direction="${3:-}"
    local baseline_file="${BASELINE_DIR}/${test_name}_baseline.json"

    if [ ! -f "$baseline_file" ]; then
        log_info "No baseline found for $test_name, skipping comparison"
        return 0
    fi

    local baseline_value
    baseline_value=$(jq -r '.baseline_value' "$baseline_file")
    local tolerance
    tolerance=$(jq -r '.tolerance_percent' "$baseline_file")
    local stored_direction
    stored_direction=$(jq -r '.direction // empty' "$baseline_file" 2>/dev/null || true)
    direction="${direction:-${stored_direction:-}}"

    if [ -z "$direction" ]; then
        # Sensible defaults.
        case "$test_name" in
            startup_latency_ms|packet_loss_percent) direction="lower" ;;
            bitrate|bitrate_kbps|fps) direction="higher" ;;
            *) direction="lower" ;;
        esac
    fi

    if [ -z "$baseline_value" ] || [ "$baseline_value" = "null" ] || [ "$baseline_value" = "0" ]; then
        log_info "Baseline value missing/zero for $test_name, skipping comparison"
        return 0
    fi

    local regression_pct="0"
    case "$direction" in
        lower)
            # Regression = percent increase over baseline.
            regression_pct=$(awk "BEGIN {print int(100 * ($current_value - $baseline_value) / $baseline_value)}" 2>/dev/null || echo "0")
            ;;
        higher)
            # Regression = percent decrease below baseline (only when current < baseline).
            regression_pct=$(awk "BEGIN {d=100 * ($baseline_value - $current_value) / $baseline_value; if (d<0) d=0; print int(d)}" 2>/dev/null || echo "0")
            ;;
        *)
            regression_pct=$(awk "BEGIN {print int(100 * ($current_value - $baseline_value) / $baseline_value)}" 2>/dev/null || echo "0")
            ;;
    esac

    if [ "$regression_pct" -gt "$tolerance" ] 2>/dev/null; then
        log_error "REGRESSION: $test_name degraded by ${regression_pct}% (direction=$direction, tolerance=${tolerance}%)"
        return 1
    else
        log_info "OK: $test_name within tolerance (${regression_pct}% vs ${tolerance}%)"
        return 0
    fi
}

update_baseline() {
    local test_name="$1"
    local current_value="$2"
    local direction="${3:-}"
    mkdir -p "$BASELINE_DIR"
    local baseline_file="${BASELINE_DIR}/${test_name}_baseline.json"
    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    if [ -z "$direction" ]; then
        case "$test_name" in
            startup_latency_ms|packet_loss_percent) direction="lower" ;;
            bitrate|bitrate_kbps|fps) direction="higher" ;;
            *) direction="lower" ;;
        esac
    fi

    jq -n \
        --arg test "$test_name" \
        --arg created "$timestamp" \
        --arg direction "$direction" \
        --argjson baseline_value "$current_value" \
        '{
            test: $test,
            created: $created,
            baseline_value: $baseline_value,
            tolerance_percent: 20,
            direction: $direction
        }' > "$baseline_file"
    log_info "Baseline updated for $test_name: $current_value (direction=$direction)"
}

# --- Test 1: Basic connectivity ---
test_basic_connectivity() {
    log_info "Testing basic connectivity..."

    local describe_output
    describe_output=$(timeout "$RTSP_TIMEOUT" ffmpeg -rtsp_transport tcp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -t 0.1 -f null - 2>&1 || true)

    if echo "$describe_output" | grep -q "Stream #0"; then
        echo "PASS"
        return 0
    else
        echo "FAIL"
        return 1
    fi
}

# --- Test 2: Stream startup latency ---
test_startup_latency() {
    log_info "Testing stream startup latency..."

    local start_time
    start_time=$(date +%s%N)
    local ffmpeg_output
    ffmpeg_output=$(timeout "$RTSP_TIMEOUT" ffmpeg -rtsp_transport tcp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -vframes 1 -f null - 2>&1 || true)
    local end_time
    end_time=$(date +%s%N)
    local latency_ms
    latency_ms=$(( (end_time - start_time) / 1000000 ))

    if echo "$ffmpeg_output" | grep -q "frame="; then
        echo "$latency_ms"
        return 0
    else
        echo "FAIL"
        return 1
    fi
}

# --- Test 3: Bitrate and FPS stability ---
test_bitrate_fps_stability() {
    log_info "Testing bitrate and FPS stability..."

    local ffmpeg_output
    ffmpeg_output=$(timeout $((TEST_DURATION + 5)) ffmpeg \
        -rtsp_transport tcp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -t "$TEST_DURATION" \
        -f null - 2>&1 || true)

    local bitrate
    bitrate=$(echo "$ffmpeg_output" | grep -Eo 'bitrate=[0-9.]+' | tail -1 | cut -d= -f2 || echo "0")
    local fps
    fps=$(echo "$ffmpeg_output" | grep -Eo 'fps=[0-9.]+' | tail -1 | cut -d= -f2 || echo "0")
    echo "bitrate=$bitrate fps=$fps"
}

# --- Test 4: SDP validation (ffprobe) ---
test_sdp_validation() {
    log_info "Testing SDP validation..."

    local probe_output
    probe_output=$(timeout "$RTSP_TIMEOUT" ffprobe -v error \
        -rtsp_transport tcp \
        -show_streams \
        -of json \
        "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" 2>&1 || true)

    local video_count audio_count
    video_count=$(echo "$probe_output" | jq '[.streams[]? | select(.codec_type=="video")] | length' 2>/dev/null || echo "0")
    audio_count=$(echo "$probe_output" | jq '[.streams[]? | select(.codec_type=="audio")] | length' 2>/dev/null || echo "0")

    if [ "${video_count:-0}" -le 0 ] 2>/dev/null; then
        echo "FAIL no_video_streams"
        return 1
    fi

    local video_codecs
    video_codecs=$(echo "$probe_output" | jq -r '[.streams[]? | select(.codec_type=="video") | .codec_name] | unique | join(",")' 2>/dev/null || echo "")
    local video_has_h264
    video_has_h264=$(echo "$probe_output" | jq -r '[.streams[]? | select(.codec_type=="video") | .codec_name] | any(.=="h264")' 2>/dev/null || echo "false")

    if [ "$video_has_h264" != "true" ]; then
        echo "FAIL video_codec_not_h264 codecs=$video_codecs audio_streams=$audio_count"
        return 1
    fi

    echo "PASS video_streams=$video_count audio_streams=$audio_count video_codecs=$video_codecs"
    return 0
}

# --- Test 5: RTSP protocol sequence (tshark) ---
test_rtsp_protocol_sequence() {
    log_info "Testing RTSP protocol sequence..."

    local pcap_file="/tmp/rtsp_capture_$$.pcap"

    timeout $((TEST_DURATION + 5)) tshark -i "$CAPTURE_IFACE" -f "tcp port $RTSP_PORT" \
        -w "$pcap_file" >/dev/null 2>&1 &
    local tshark_pid=$!

    timeout $((TEST_DURATION + 5)) ffmpeg -rtsp_transport tcp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -t "$TEST_DURATION" \
        -f null - >/dev/null 2>&1 || true

    wait $tshark_pid 2>/dev/null || true

    if [ -f "$pcap_file" ]; then
        local describe_count
        describe_count=$(tshark -r "$pcap_file" -Y "rtsp.method == \"DESCRIBE\"" 2>/dev/null | wc -l)
        local setup_count
        setup_count=$(tshark -r "$pcap_file" -Y "rtsp.method == \"SETUP\"" 2>/dev/null | wc -l)
        local play_count
        play_count=$(tshark -r "$pcap_file" -Y "rtsp.method == \"PLAY\"" 2>/dev/null | wc -l)
        local teardown_count
        teardown_count=$(tshark -r "$pcap_file" -Y "rtsp.method == \"TEARDOWN\"" 2>/dev/null | wc -l)
        local status_200
        status_200=$(tshark -r "$pcap_file" -Y "rtsp.status_code == 200" 2>/dev/null | wc -l)
        local status_err
        status_err=$(tshark -r "$pcap_file" -Y "rtsp.status_code >= 400" 2>/dev/null | wc -l)
        rm -f "$pcap_file"

        if [ "$describe_count" -gt 0 ] && [ "$setup_count" -gt 0 ] && [ "$play_count" -gt 0 ] && [ "$status_err" -eq 0 ] && [ "$status_200" -gt 0 ]; then
            echo "PASS describe=$describe_count setup=$setup_count play=$play_count teardown=$teardown_count status_200=$status_200"
            return 0
        fi
    fi

    echo "FAIL"
    return 1
}

# --- Test 6: Packet loss (tshark RTP over UDP) ---
# Uses -rtsp_transport udp so RTP is sent over UDP and tshark can capture it.
# Fails when no RTP packets are observed instead of reporting zero loss.
test_packet_loss() {
    log_info "Testing packet loss (UDP transport for RTP capture)..."

    local pcap_file="/tmp/rtp_capture_$$.pcap"
    local capture_filter="udp"
    if is_ip_literal "$RTSP_HOST"; then
        capture_filter="udp and host ${RTSP_HOST}"
    fi

    timeout $((TEST_DURATION + 5)) tshark -i "$CAPTURE_IFACE" -f "$capture_filter" \
        -w "$pcap_file" >/dev/null 2>&1 &
    local tshark_pid=$!

    timeout $((TEST_DURATION + 5)) ffmpeg -rtsp_transport udp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -t "$TEST_DURATION" \
        -f null - >/dev/null 2>&1 || true

    wait $tshark_pid 2>/dev/null || true

    if [ -f "$pcap_file" ]; then
        local rtp_packets
        rtp_packets=$(tshark -r "$pcap_file" -Y "rtp" -T fields -e rtp.seq 2>/dev/null | wc -l)

        if [ "$rtp_packets" -eq 0 ]; then
            rm -f "$pcap_file"
            log_info "No RTP packets observed (UDP capture); skipping/failing packet-loss test"
            echo "FAIL rtp_packets=0 (no RTP captured)"
            return 1
        fi

        local packet_loss=0
        packet_loss=$(tshark -r "$pcap_file" -Y "rtp" -T fields -e rtp.seq 2>/dev/null | \
            awk 'NR>1 {if ($1 - prev != 1 && $1 - prev != -65535) loss++} {prev=$1+0} END {print loss+0}' 2>/dev/null || echo "0")
        rm -f "$pcap_file"
        echo "rtp_packets=$rtp_packets packet_loss=$packet_loss"
        return 0
    fi

    echo "FAIL"
    return 1
}

# --- Test 7: Concurrent clients ---
test_concurrent_clients() {
    local client_count="${1:-2}"
    log_info "Testing concurrent clients ($client_count)..."

    local pids=()
    for _ in $(seq 1 "$client_count"); do
        timeout $((TEST_DURATION + 5)) ffmpeg -rtsp_transport tcp \
            -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
            -t "$TEST_DURATION" \
            -f null - >/dev/null 2>&1 &
        pids+=($!)
    done

    local failed=0
    for pid in "${pids[@]}"; do
        if ! wait "$pid" 2>/dev/null; then
            ((failed++)) || true
        fi
    done

    if [ "$failed" -eq 0 ]; then
        echo "PASS all_clients=$client_count"
        return 0
    else
        echo "FAIL failed_clients=$failed"
        return 1
    fi
}

# --- Test 8: Long duration stability ---
test_long_duration_stability() {
    log_info "Testing long duration stability (10 minutes)..."

    local long_duration="$LONG_TEST_DURATION"
    local ffmpeg_output
    ffmpeg_output=$(timeout $((long_duration + 10)) ffmpeg \
        -rtsp_transport tcp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -t "$long_duration" \
        -f null - 2>&1 || true)

    local initial_bitrate
    initial_bitrate=$(echo "$ffmpeg_output" | head -100 | grep -Eo 'bitrate=[0-9.]+' | head -1 | cut -d= -f2 || echo "0")
    local final_bitrate
    final_bitrate=$(echo "$ffmpeg_output" | tail -100 | grep -Eo 'bitrate=[0-9.]+' | tail -1 | cut -d= -f2 || echo "0")

    if [ -n "$initial_bitrate" ] && [ -n "$final_bitrate" ] && [ "$initial_bitrate" != "0" ]; then
        local degradation
        degradation=$(awk "BEGIN {print int(100 * (1 - $final_bitrate / $initial_bitrate))}" 2>/dev/null || echo "0")
        if [ "${degradation:-0}" -lt 20 ]; then
            echo "PASS degradation=${degradation}%"
            return 0
        fi
    fi
    echo "FAIL"
    return 1
}

# --- Test 9: Error handling ---
test_error_handling() {
    log_info "Testing error handling..."

    local test_results=()
    local any_fail=0

    local invalid_creds
    invalid_creds=$(timeout "$RTSP_TIMEOUT" ffmpeg -rtsp_transport tcp \
        -i "rtsp://invalid:invalid@${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}" \
        -t 0.1 -f null - 2>&1 || true)
    if echo "$invalid_creds" | grep -q "401\|Unauthorized"; then
        test_results+=("invalid_creds=PASS")
    else
        test_results+=("invalid_creds=FAIL")
        any_fail=1
    fi

    local bogus_url
    bogus_url=$(timeout "$RTSP_TIMEOUT" ffmpeg -rtsp_transport tcp \
        -i "rtsp://${RTSP_HOST}:${RTSP_PORT}/bogus_stream" \
        -t 0.1 -f null - 2>&1 || true)
    if echo "$bogus_url" | grep -q "404\|Not Found"; then
        test_results+=("bogus_url=PASS")
    else
        test_results+=("bogus_url=FAIL")
        any_fail=1
    fi

    if [ "$any_fail" -eq 1 ]; then
        echo "FAIL ${test_results[*]}"
        return 1
    fi
    echo "PASS ${test_results[*]}"
    return 0
}

# --- Report generation ---
generate_report() {
    local tests_json="$1"
    local output_file="$2"

    local timestamp
    timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    jq -n \
        --arg timestamp "$timestamp" \
        --arg rtsp_host "$RTSP_HOST" \
        --argjson rtsp_port "$RTSP_PORT" \
        --arg rtsp_stream "$RTSP_STREAM" \
        --argjson test_duration_seconds "$TEST_DURATION" \
        --argjson tests "$tests_json" \
        '{
            test_run: {
                timestamp: $timestamp,
                rtsp_host: $rtsp_host,
                rtsp_port: $rtsp_port,
                rtsp_stream: $rtsp_stream,
                test_duration_seconds: $test_duration_seconds
            },
            tests: $tests,
            summary: {
                total_tests: ($tests | length),
                passed: ($tests | map(select(.result == "PASS")) | length),
                failed: ($tests | map(select(.result == "FAIL")) | length),
                overall_pass: (($tests | map(select(.result == "FAIL")) | length) == 0)
            }
        }' > "$output_file"
    log_info "Report written to $output_file"
}

# --- Main ---
main() {
    local update_baseline_flag=""
    local compare_baseline_flag=""
    local launch_server=""
    local h264_file=""
    local concurrent_count=""
    local run_long_duration=""
    local skip_error_handling=""

    while [ $# -gt 0 ]; do
        case "$1" in
            --update-baseline) update_baseline_flag=1 ;;
            --compare-baseline) compare_baseline_flag=1 ;;
            --launch-server) launch_server=1 ;;
            --h264-file) h264_file="$2"; shift ;;
            --concurrent) concurrent_count="${2:-$CONCURRENT_CLIENT_COUNT}"; shift ;;
            --long-duration) run_long_duration=1 ;;
            --skip-error-handling) skip_error_handling=1 ;;
            *) ;;
        esac
        shift
    done

    if [ -n "$launch_server" ] && [ -n "$h264_file" ]; then
        RTSP_STREAM="/stream1"  # onvif-rust validation mode uses /stream1
        start_onvif_server "$h264_file" "$RTSP_PORT" "8080" "$RTSP_STREAM" || exit 1
    fi

    log_info "Starting RTSP validation tests..."
    log_info "Target: rtsp://${RTSP_HOST}:${RTSP_PORT}${RTSP_STREAM}"

    local test_results=()

    local result
    result=$(test_basic_connectivity)
    test_results+=("$(json_check "basic_connectivity" "${result%% *}")")

    local latency
    latency=$(test_startup_latency)
    if [ "$latency" = "FAIL" ]; then
        test_results+=("$(json_check "startup_latency_ms" "FAIL" "ffmpeg did not decode a frame within timeout")")
    else
        local latency_result="PASS"
        local latency_reason=""
        if [ "$latency" -gt "$VIDEO_STARTUP_LATENCY_MS" ] 2>/dev/null; then
            latency_result="FAIL"
            latency_reason="startup latency ${latency}ms > threshold ${VIDEO_STARTUP_LATENCY_MS}ms"
        fi
        test_results+=("$(json_metric "startup_latency_ms" "$latency_result" "$latency" "ms")")
        if [ -n "$latency_reason" ] && [ "$latency_result" = "FAIL" ]; then
            test_results+=("$(json_check "startup_latency_threshold" "FAIL" "$latency_reason")")
        else
            test_results+=("$(json_check "startup_latency_threshold" "PASS")")
        fi
        [ -n "$compare_baseline_flag" ] && compare_against_baseline "startup_latency_ms" "$latency" "lower" || true
        [ -n "$update_baseline_flag" ] && update_baseline "startup_latency_ms" "$latency" "lower" || true
    fi

    local bitrate_fps
    bitrate_fps=$(test_bitrate_fps_stability)
    test_results+=("$(json_check "bitrate_fps_capture" "PASS" "$bitrate_fps")")

    local bitrate fps
    bitrate="${bitrate_fps#*bitrate=}"
    bitrate="${bitrate%% *}"
    fps="${bitrate_fps#*fps=}"
    fps="${fps%% *}"
    bitrate="${bitrate:-0}"
    fps="${fps:-0}"

    local bitrate_result="PASS" fps_result="PASS"
    if [ -n "${EXPECTED_BITRATE:-}" ]; then
        if ! within_tolerance "$bitrate" "$EXPECTED_BITRATE" "$BITRATE_TOLERANCE_PERCENT"; then
            bitrate_result="FAIL"
        fi
    elif [ -n "$compare_baseline_flag" ]; then
        compare_against_baseline "bitrate_kbps" "$bitrate" "higher" || bitrate_result="FAIL"
    fi
    if [ -n "${EXPECTED_FPS:-}" ]; then
        if ! within_tolerance "$fps" "$EXPECTED_FPS" "$FPS_TOLERANCE_PERCENT"; then
            fps_result="FAIL"
        fi
    elif [ -n "$compare_baseline_flag" ]; then
        compare_against_baseline "fps" "$fps" "higher" || fps_result="FAIL"
    fi
    test_results+=("$(json_metric "bitrate_kbps" "$bitrate_result" "$bitrate" "kbps")")
    test_results+=("$(json_metric "fps" "$fps_result" "$fps" "fps")")
    [ -n "$update_baseline_flag" ] && { update_baseline "bitrate_kbps" "$bitrate" "higher"; update_baseline "fps" "$fps" "higher"; }

    local sdp
    sdp=$(test_sdp_validation)
    test_results+=("$(json_check "sdp_validation" "${sdp%% *}" "${sdp#* }")")

    local protocol
    protocol=$(test_rtsp_protocol_sequence)
    test_results+=("$(json_check "protocol_sequence" "${protocol%% *}" "${protocol#* }")")

    local loss
    loss=$(test_packet_loss) || true
    local loss_result="PASS"
    if [[ "$loss" == FAIL* ]]; then
        loss_result="FAIL"
        test_results+=("$(json_check "packet_loss" "FAIL" "$loss")")
    else
        local rtp_packets packet_loss
        rtp_packets=$(echo "$loss" | tr ' ' '\n' | awk -F= '/^rtp_packets=/{print $2}' | head -1)
        packet_loss=$(echo "$loss" | tr ' ' '\n' | awk -F= '/^packet_loss=/{print $2}' | head -1)
        rtp_packets="${rtp_packets:-0}"
        packet_loss="${packet_loss:-0}"

        local loss_percent="0"
        if [ "$rtp_packets" -gt 0 ] 2>/dev/null; then
            loss_percent=$(awk "BEGIN {print (100.0 * $packet_loss / $rtp_packets)}" 2>/dev/null || echo "0")
        fi

        local packet_loss_pass="PASS"
        awk "BEGIN {exit (!($loss_percent <= $PACKET_LOSS_TOLERANCE_PERCENT))}" 2>/dev/null || packet_loss_pass="FAIL"

        local loss_value_json
        loss_value_json=$(jq -n \
            --argjson rtp_packets "$rtp_packets" \
            --argjson packet_loss "$packet_loss" \
            --argjson loss_percent "$loss_percent" \
            '{rtp_packets:$rtp_packets,packet_loss:$packet_loss,loss_percent:$loss_percent}')
        test_results+=("$(json_metric "packet_loss_percent" "$packet_loss_pass" "$loss_value_json" "%")")
        [ -n "$compare_baseline_flag" ] && compare_against_baseline "packet_loss_percent" "$loss_percent" "lower" || true
        [ -n "$update_baseline_flag" ] && update_baseline "packet_loss_percent" "$loss_percent" "lower" || true
    fi

    if [ -n "$concurrent_count" ]; then
        local conc
        conc=$(test_concurrent_clients "$concurrent_count")
        test_results+=("$(json_check "concurrent_clients" "${conc%% *}" "${conc#* }")")
    fi

    if [ -n "$run_long_duration" ]; then
        local long_res
        long_res=$(test_long_duration_stability)
        test_results+=("$(json_check "long_duration_stability" "${long_res%% *}" "${long_res#* }")")
    fi

    if [ -z "$skip_error_handling" ]; then
        local err_res
        err_res=$(test_error_handling)
        test_results+=("$(json_check "error_handling" "${err_res%% *}" "${err_res#* }")")
    fi

    local results_json
    results_json="$(printf '%s\n' "${test_results[@]}" | jq -s '.')"

    generate_report "$results_json" "$OUTPUT_FILE"
    log_info "Results written to $OUTPUT_FILE"
}

main "$@"
