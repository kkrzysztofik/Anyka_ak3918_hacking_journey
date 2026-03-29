# Video Latency Fix Validation Checklist

This checklist validates the complete video latency fix for the Anyka AK3918 device.

## Fix Summary

The fix addresses three issues:
1. **Timestamp normalization**: Timestamps now start at 0 instead of ~42M ms
2. **RTP send batching**: Packets batched per frame (190-285ms → **<15ms expected** for large I-frames, ideally **<10ms**; see RTP Send Performance below)
3. **Removed inter_frame_us**: Field removed from ring buffer

---

## Pre-Deployment Validation

### Build & Compilation

- [ ] **vendor-daemon compiles without errors**
  ```bash
  cd cross-compile/vendor-daemon && make clean && make
  ```

- [ ] **onvif-rust compiles without errors**
  ```bash
  cd cross-compile/onvif-rust
  ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo build --release
  ```

- [ ] **Clippy passes with zero warnings**
  ```bash
  ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
  ```

- [ ] **Code formatting passes**
  ```bash
  cargo fmt --check
  ```

- [ ] **Unit tests pass on host**
  ```bash
  ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu --lib
  ```

- [ ] **Struct size assertions pass**
  ```bash
  ../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu test_vd_slot_header_size
  ```

---

## Device Deployment

### Binary Transfer

- [ ] **vendor-daemon copied to SD card** (`/mnt/vendor-daemon`)
- [ ] **onvif-rust copied to SD card** (`/mnt/onvif-rust`)
- [ ] **Binaries have execute permissions**

### Service Startup

- [ ] **vendor-daemon starts without errors**
  ```bash
  /mnt/vendor-daemon &
  # Check: ps | grep vendor-daemon
  ```

- [ ] **onvif-rust starts without errors**
  ```bash
  /mnt/onvif-rust &
  # Check: ps | grep onvif-rust
  ```

- [ ] **RTSP port 8554 is listening**
  ```bash
  netstat -ln | grep 8554
  # Expected: tcp 0 0 0.0.0.0:8554 LISTEN
  ```

---

## Timestamp Normalization (vendor-daemon)

### Expected Log Patterns

```text
# vendor_daemon.log should contain:
event=timestamp_anchor stream=0 first_ts_ms=42543800...
event=timestamp_anchor stream=1 first_ts_ms=42543800...
```

- [ ] **Timestamp anchor events appear** (once per stream on startup)
  ```bash
  grep "timestamp_anchor" /mnt/logs/vendor_daemon.log
  ```

- [ ] **First anchor shows raw SDK value** (~42M ms)
  - Check first_ts_ms in the anchor event

- [ ] **slot_ts_ms values start at 0**
  ```bash
  grep -o "slot_ts_ms=[0-9]*" /mnt/logs/vendor_daemon.log | head -10
  # Expected: 0, 60, 127, 193, 260... (≈60-67ms intervals at 15fps)
  ```

- [ ] **Normalized timestamps increment correctly**
  - Each frame should increment by ~60-67ms (1000ms / 15fps ≈ 66.67ms)
  - Some variation is normal due to encoding bitrate fluctuations

---

## RTP Send Performance (onvif-rust)

### Expected Log Patterns

```text
# onvif.log should contain:
frame_send_ms=8
payload_len=104527 packet_count=72 send_ms=8
```

- [ ] **No rtp_send_slow warnings**
  ```bash
  grep "rtp_send_slow" /mnt/logs/onvif.log | wc -l
  # Expected: 0
  ```

- [ ] **Frame send times <15ms for large I-frames**
  ```bash
  grep -o "frame_send_ms=[0-9]*" /mnt/logs/onvif.log
  # Expected: All values <15ms (ideally <10ms)
  ```

- [ ] **Packet count logged per frame**
  - Should see `packet_count=N` for each frame

- [ ] **No lock contention errors**
  ```bash
  grep -i "lock\|contention\|timeout" /mnt/logs/onvif.log
  # Expected: No errors
  ```

---

## VLC Playback Test

### Connection

- [ ] **VLC connects within 2 seconds**
  - URL: `rtsp://<device-ip>:8554/stream`
  - No long delay before video appears

- [ ] **TCP interleaved mode works** (VLC default)
- [ ] **UDP mode works** (VLC: Tools → Preferences → Input/Codecs → RTP over RTSP: UDP)

### Playback Quality

- [ ] **No "5 seconds of late video" errors**
  - Check VLC: Tools → Messages (set verbosity to debug)

- [ ] **No "picture is too late to be displayed" warnings**
  - These indicate RTP timestamp issues

- [ ] **Smooth playback, no stuttering**
  - Consistent frame rate

- [ ] **Latency <100ms**
  - Check VLC: Tools → Media Information → Statistics
  - Look for "Video delay"

---

## Stress Test

### Continuous Operation

- [ ] **Run for 5+ minutes without errors**
  ```bash
  # Start stream and let it run
  # Monitor logs for any errors
  tail -f /mnt/logs/onvif.log
  ```

- [ ] **Ring buffer overflow count stays at 0**
  ```bash
  grep "ring_overflow\|buffer_overflow" /mnt/logs/vendor_daemon.log | wc -l
  ```

- [ ] **No memory leaks**
  ```bash
  # Before test
  free
  # After 5 minutes
  free
  # Memory usage should be stable
  ```

- [ ] **Both main and sub streams work**
  - Test switching between streams if applicable

---

## Log Analysis Summary

### Expected Patterns

**vendor_daemon.log:**
```text
event=timestamp_anchor stream=0 first_ts_ms=42543800 diag_monotonic_ms=...
event=timestamp_normalize stream=0 raw_ts=42543860 normalized_ts=60 seq_no=5
slot_ts_ms=0
slot_ts_ms=60
slot_ts_ms=127
slot_ts_ms=193
slot_ts_ms=260
```

**onvif.log:**
```text
frame_send_ms=8
payload_len=104527 packet_count=72 send_ms=8
NO "rtp_send_slow" warnings
```

---

## Failure Indicators

| Symptom | Likely Cause |
|---------|--------------|
| Timestamps still start at 42M+ | Timestamp normalization not applied |
| rtp_send_slow still appears | RTP batching not working |
| VLC errors persist | Check RTP timestamp or pacing |
| Services crash | Check logs for panic/segfault |
| High latency (>100ms) | Check RTP timestamps, network |

---

## Quick Test Commands

```bash
# On device:
/mnt/test_video_latency.sh

# Analyze logs (on host):
python3 scripts/analyze_test_logs.py /tmp/video_latency_test_*_daemon.log /tmp/video_latency_test_*_onvif.log
```

---

## Sign-Off

| Test Category | Pass | Fail | Notes |
|--------------|------|------|-------|
| Build & Compilation | [ ] | [ ] | |
| Device Deployment | [ ] | [ ] | |
| Timestamp Normalization | [ ] | [ ] | |
| RTP Send Performance | [ ] | [ ] | |
| VLC Playback | [ ] | [ ] | |
| Stress Test | [ ] | [ ] | |

**Overall Status**: ✅ PASS / ❌ FAIL

**Tested by**: _________________ **Date**: _________________

**Notes**:
________________________________________________________________
________________________________________________________________
________________________________________________________________
