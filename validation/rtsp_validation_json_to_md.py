#!/usr/bin/env python3
"""Convert rtsp_validation.json to Markdown."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast


def cell(s: str) -> str:
    """Escape pipe so table cells don't break."""
    return str(s).replace("|", ",").replace("\n", " ")


def format_value(val: Any) -> str:
    """Format a JSON value for a Markdown table cell."""
    if val is None:
        return "—"
    if isinstance(val, bool):
        return "yes" if val else "no"
    if isinstance(val, (list, dict)):
        raw = json.dumps(val, separators=(",", ":"))
        snippet = raw[:80] + "…" if len(raw) > 80 else raw
        return cell("`" + snippet + "`")
    return cell(str(val))


def _render_test_run(run: dict[str, Any], artifacts_dir: str | None, lines: list[str]) -> None:
    lines.append("# RTSP Validation Report\n")
    lines.append("## Test run\n")
    lines.append(f"- **Timestamp:** {run.get('timestamp', '—')}")
    host = run.get("rtsp_host", "")
    port = run.get("rtsp_port", "")
    stream = run.get("rtsp_stream", "")
    lines.append(f"- **RTSP:** `rtsp://{host}:{port}{stream}`")
    lines.append(f"- **Duration:** {run.get('test_duration_seconds', '—')} s")
    if artifacts_dir:
        lines.append(f"- **Artifacts:** `{cell(artifacts_dir)}`")
    lines.append("")


def _render_summary(summary: dict[str, Any], lines: list[str]) -> None:
    lines.append("## Summary\n")
    lines.append("| | Count |")
    lines.append("|---|------|")
    lines.append(f"| Total | {summary.get('total_tests', 0)} |")
    lines.append(f"| Passed | {summary.get('passed', 0)} |")
    lines.append(f"| Failed | {summary.get('failed', 0)} |")
    result = "PASS" if summary.get("overall_pass", False) else "FAIL"
    lines.append(f"| **Result** | **{result}** |\n")


def _render_tests(tests: list[dict[str, Any]], lines: list[str]) -> None:
    lines.append("## Tests\n")
    lines.append("| Name | Type | Status / Value |")
    lines.append("|------|------|----------------|")
    for t in tests:
        typ = t.get("type", "")
        val = t.get("value", {})
        name = cell(val.get("name", ""))
        if typ == "Pass":
            lines.append(f"| {name} | Pass | ✓ |")
        elif typ == "Fail":
            reason_raw = val.get("reason", "")
            reason = reason_raw.strip()[:200] + ("…" if len(reason_raw) > 200 else "")
            lines.append(f"| {name} | Fail | {cell(reason)} |")
        elif typ == "Metric":
            v = val.get("value")
            status = "✓" if val.get("pass", None) else "✗"
            lines.append(f"| {name} | Metric | {format_value(v)} {status} |")
        else:
            lines.append(f"| {name} | {cell(typ)} | — |")


def _render_telemetry(telemetry: dict[str, Any], lines: list[str]) -> None:
    lines.append("\n## Telemetry\n")
    lines.append("| Metric | Value |")
    lines.append("|--------|-------|")
    for k, v in telemetry.items():
        lines.append(f"| {cell(k)} | {format_value(v)} |")


def main() -> int:
    parser = argparse.ArgumentParser(description="Convert rtsp_validation.json to Markdown")
    parser.add_argument(
        "input",
        nargs="?",
        default="rtsp_validation.json",
        help="Input JSON path (default: rtsp_validation.json)",
    )
    parser.add_argument(
        "-o", "--output",
        help="Output Markdown path (default: stdout)",
    )
    args = parser.parse_args()

    path = Path(args.input)
    if not path.exists():
        print(f"Error: {path} not found", file=sys.stderr)
        return 1

    with open(path, encoding="utf-8") as f:
        data: dict[str, Any] = json.load(f)

    lines: list[str] = []
    _render_test_run(
        cast(dict[str, Any], data.get("test_run") or {}),
        cast(str | None, data.get("artifacts_dir")),
        lines,
    )
    _render_summary(cast(dict[str, Any], data.get("summary") or {}), lines)
    _render_tests(cast(list[dict[str, Any]], data.get("tests") or []), lines)
    telemetry = data.get("telemetry")
    if telemetry and isinstance(telemetry, dict):
        _render_telemetry(cast(dict[str, Any], telemetry), lines)

    out = "\n".join(lines)
    if args.output:
        Path(args.output).write_text(out, encoding="utf-8")
    else:
        print(out)
    return 0


if __name__ == "__main__":
    sys.exit(main())
