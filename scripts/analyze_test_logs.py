#!/usr/bin/env python3
"""
Automated Log Analyzer for Video Latency Integration Tests

Analyzes vendor-daemon and onvif-rust logs to validate:
- Timestamp normalization (timestamps start at 0)
- RTP send performance (no slow sends, <15ms per frame)

Usage:
    python3 analyze_test_logs.py <daemon.log> <onvif.log>
    python3 analyze_test_logs.py vendor_daemon_test.log onvif_test.log

Output:
    - Pass/Fail for each check
    - Summary statistics
    - Detailed findings
"""

import sys
import re
import os
from typing import List, Tuple, Dict, Any


class Colors:
    """Terminal colors for output"""

    GREEN = "\033[0;32m"
    RED = "\033[0;31m"
    YELLOW = "\033[1;33m"
    BLUE = "\033[0;34m"
    NC = "\033[0m"  # No Color


def log_info(msg: str):
    print(f"{Colors.BLUE}[INFO]{Colors.NC} {msg}")


def log_pass(msg: str):
    print(f"{Colors.GREEN}[PASS]{Colors.NC} {msg}")


def log_fail(msg: str):
    print(f"{Colors.RED}[FAIL]{Colors.NC} {msg}")


def log_warn(msg: str):
    print(f"{Colors.YELLOW}[WARN]{Colors.NC} {msg}")


def read_log_file(filepath: str) -> List[str]:
    """Read log file and return lines"""
    if not os.path.exists(filepath):
        log_fail(f"Log file not found: {filepath}")
        return []

    with open(filepath, "r", encoding="utf-8", errors="ignore") as f:
        return f.readlines()


def analyze_timestamp_normalization(daemon_lines: List[str]) -> Dict[str, Any]:
    """Analyze timestamp normalization in vendor-daemon logs"""
    results: Dict[str, Any] = {
        "anchors_found": 0,
        "first_anchor_raw_ts": None,
        "first_timestamps": [],
        "timestamps_start_near_zero": False,
        "timestamps_increment_correctly": False,
        "status": "FAIL",
    }

    daemon_content = "".join(daemon_lines)

    # Count anchor events
    anchors = re.findall(
        r"event=timestamp_anchor[^:]*first_ts_ms=(\d+)", daemon_content
    )
    results["anchors_found"] = len(anchors)

    if anchors:
        results["first_anchor_raw_ts"] = int(anchors[0])
        log_info(f"Timestamp anchors found: {len(anchors)}")
        log_info(f"First anchor raw timestamp: ~{int(anchors[0]) / 1000000:.1f}M ms")

    # Extract slot timestamps
    slot_timestamps = re.findall(r"slot_ts_ms=(\d+)", daemon_content)
    results["first_timestamps"] = [int(t) for t in slot_timestamps[:20]]

    if results["first_timestamps"]:
        first_10 = results["first_timestamps"][:10]
        log_info(f"First 10 slot timestamps: {first_10}")

        # Check if timestamps start near 0
        if first_10[0] < 1000:
            results["timestamps_start_near_zero"] = True
            log_pass(f"Timestamps start near 0 (first: {first_10[0]}ms)")
        else:
            log_fail(f"Timestamps start high: {first_10[0]}ms (expected <1000)")

        # Check if timestamps increment correctly (should be ~60-67ms at 15fps)
        if len(first_10) >= 2:
            intervals = [
                first_10[i + 1] - first_10[i] for i in range(len(first_10) - 1)
            ]
            avg_interval = sum(intervals) / len(intervals)

            # Accept intervals between 50-100ms (allows for 10-20fps variation)
            valid_intervals = [50 <= i <= 100 for i in intervals]

            if sum(valid_intervals) >= len(intervals) * 0.7:  # 70% valid
                results["timestamps_increment_correctly"] = True
                log_pass(f"Timestamps increment correctly (avg: {avg_interval:.1f}ms)")
            else:
                log_warn(
                    f"Timestamp intervals vary: {intervals[:5]}... (avg: {avg_interval:.1f}ms)"
                )
    else:
        log_warn("No slot timestamps found in log")

    # Overall status
    if results["timestamps_start_near_zero"]:
        results["status"] = "PASS"

    return results


def analyze_rtp_performance(onvif_lines: List[str]) -> Dict[str, Any]:
    """Analyze RTP send performance in onvif logs"""
    results: Dict[str, Any] = {
        "slow_sends": 0,
        "frame_send_times": [],
        "avg_frame_send_ms": None,
        "max_frame_send_ms": None,
        "packet_counts": [],
        "status": "FAIL",
    }

    onvif_content = "".join(onvif_lines)

    # Count slow sends
    results["slow_sends"] = len(re.findall(r"rtp_send_slow", onvif_content))

    if results["slow_sends"] == 0:
        log_pass(f"No slow RTP sends (was: many)")
    else:
        log_fail(f"Slow RTP sends found: {results['slow_sends']}")

    # Extract frame send times
    frame_times = re.findall(r"frame_send_ms=(\d+)", onvif_content)
    times_list: List[int] = [int(t) for t in frame_times]
    results["frame_send_times"] = times_list

    if times_list:
        total = sum(times_list)
        count = len(times_list)
        avg: float = total / count
        results["avg_frame_send_ms"] = avg
        max_time: int = max(times_list)
        results["max_frame_send_ms"] = max_time

        log_info(f"Frame send times: avg={avg:.1f}ms, max={max_time}ms")

        # Check if max is acceptable (<15ms, ideally <10ms)
        if max_time < 15:
            log_pass(f"Send performance excellent (max: {max_time}ms)")
        elif max_time < 30:
            log_warn(
                f"Send performance acceptable but could be better (max: {max_time}ms)"
            )
        else:
            log_fail(f"Send performance poor (max: {max_time}ms)")

    # Extract packet counts
    packet_counts = re.findall(r"packet_count=(\d+)", onvif_content)
    results["packet_counts"] = [int(p) for p in packet_counts]

    if results["packet_counts"]:
        log_info(f"Packet counts: {results['packet_counts'][:5]}...")

    # Overall status
    if (
        results["slow_sends"] == 0
        and results["max_frame_send_ms"] is not None
        and results["max_frame_send_ms"] < 15
    ):
        results["status"] = "PASS"

    return results


def analyze_logs(daemon_log: str, onvif_log: str) -> Tuple[bool, bool]:
    """Main log analysis function"""
    print("=" * 50)
    print("  Video Latency Log Analyzer")
    print("=" * 50)
    print()

    # Read log files
    log_info(f"Reading daemon log: {daemon_log}")
    daemon_lines = read_log_file(daemon_log)

    log_info(f"Reading ONVIF log: {onvif_log}")
    onvif_lines = read_log_file(onvif_log)

    if not daemon_lines:
        log_fail("No daemon log content - cannot analyze")
        return False, False

    if not onvif_lines:
        log_fail("No ONVIF log content - cannot analyze")
        return False, False

    print()
    log_info("=== Timestamp Normalization Analysis ===")
    timestamp_results = analyze_timestamp_normalization(daemon_lines)

    print()
    log_info("=== RTP Send Performance Analysis ===")
    rtp_results = analyze_rtp_performance(onvif_lines)

    # Summary
    print()
    print("=" * 50)
    print("  Summary")
    print("=" * 50)

    ts_pass = timestamp_results["status"] == "PASS"
    rtp_pass = rtp_results["status"] == "PASS"

    print()
    print(f"Timestamp Normalization: {timestamp_results['status']}")
    print(f"RTP Send Performance:    {rtp_results['status']}")
    print()

    if ts_pass and rtp_pass:
        log_pass("All tests PASSED - video latency fix validated!")
        return True, True
    else:
        if not ts_pass:
            log_fail("Timestamp normalization FAILED")
        if not rtp_pass:
            log_fail("RTP send performance FAILED")
        return ts_pass, rtp_pass


def main():
    """Main entry point"""
    if len(sys.argv) != 3:
        print(f"Usage: {sys.argv[0]} <daemon.log> <onvif.log>")
        print()
        print("Example:")
        print(
            f"  {sys.argv[0]} /tmp/video_latency_test_daemon.log /tmp/video_latency_test_onvif.log"
        )
        sys.exit(1)

    daemon_log = sys.argv[1]
    onvif_log = sys.argv[2]

    # Check files exist
    if not os.path.exists(daemon_log):
        log_fail(f"Daemon log not found: {daemon_log}")
        sys.exit(1)

    if not os.path.exists(onvif_log):
        log_fail(f"ONVIF log not found: {onvif_log}")
        sys.exit(1)

    # Analyze
    ts_pass, rtp_pass = analyze_logs(daemon_log, onvif_log)

    # Exit with appropriate code
    if ts_pass and rtp_pass:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()
