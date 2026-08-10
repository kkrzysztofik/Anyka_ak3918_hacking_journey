//! Shared utilities (log tail, artifact paths).

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAX_TOOL_LOG_BYTES: usize = 2_000_000; // 2MB per log file (tail kept if exceeded)

pub fn rtsp_url(host: &str, port: u16, stream: &str) -> String {
    format!("rtsp://{}:{}{}", host, port, stream)
}

pub fn tail_lossy(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    let mut tail: Vec<char> = s.chars().rev().take(max_chars).collect();
    tail.reverse();
    format!("…{}", tail.into_iter().collect::<String>())
}

/// Parse an ffmpeg size token (`52kB`, `185KiB`, `1MB`, `1MiB`, `500B`) into kilobytes.
///
/// ffmpeg 7 and earlier typically print decimal units (`kB`/`MB`); ffmpeg 8+ prints
/// binary IEC units (`KiB`/`MiB`) on the null-muxer summary line. Longer suffixes are
/// matched first so `KiB` is not misread as `B`.
pub fn parse_ffmpeg_kbytes_token(raw: &str) -> Option<f64> {
    let trimmed = raw.trim().trim_end_matches(',');
    for (suffix, scale) in [
        ("KiB", 1.0),
        ("MiB", 1024.0),
        ("kB", 1.0),
        ("MB", 1000.0),
        ("B", 0.001),
    ] {
        if let Some(value) = trimmed.strip_suffix(suffix) {
            return value.trim().parse::<f64>().ok().map(|v| v * scale);
        }
    }
    None
}

/// Estimate kbps from an ffmpeg summary line (`video:52kB` / `video:185KiB` …) over `duration_sec`.
pub fn parse_ffmpeg_summary_bitrate_kbps(log_line: &str, duration_sec: u64) -> Option<f64> {
    if duration_sec == 0 {
        return None;
    }
    let total_kbytes: f64 = log_line
        .split_whitespace()
        .filter_map(|field| {
            field
                .strip_prefix("video:")
                .or_else(|| field.strip_prefix("audio:"))
        })
        .filter_map(parse_ffmpeg_kbytes_token)
        .sum();
    if total_kbytes <= 0.0 {
        return None;
    }
    Some((total_kbytes * 8.0) / duration_sec as f64)
}

/// Build a pcap path under the process temp dir.
///
/// Ubuntu AppArmor confines `/usr/bin/tshark` so it can read capture files via
/// `abstractions/user-tmp`, but not arbitrary paths under `$HOME`. Capture and
/// analyze under this path, then [`finalize_tshark_pcap`] into the run artifacts.
pub fn tshark_readable_pcap_path(file_name: &str) -> PathBuf {
    let safe_name = file_name.replace(['/', '\\'], "_");
    std::env::temp_dir().join(format!(
        "rtsp_validation_pid{}_{}",
        std::process::id(),
        safe_name
    ))
}

/// After tshark analysis, optionally keep a copy under `artifacts_dir` and always
/// remove the temp capture file.
pub fn finalize_tshark_pcap(
    tmp_pcap: &Path,
    artifacts_dir: &Path,
    file_name: &str,
    keep_pcaps: bool,
) -> Result<()> {
    if keep_pcaps {
        let dest = artifacts_dir.join(file_name);
        std::fs::copy(tmp_pcap, &dest)
            .with_context(|| format!("copy pcap to {}", dest.display()))?;
    }
    let _ = std::fs::remove_file(tmp_pcap);
    Ok(())
}

pub fn write_bytes_tail(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = File::create(path).with_context(|| format!("create {}", path.display()))?;
    if bytes.len() <= MAX_TOOL_LOG_BYTES {
        f.write_all(bytes)
            .with_context(|| format!("write {}", path.display()))?;
        return Ok(());
    }
    let tail = &bytes[bytes.len().saturating_sub(MAX_TOOL_LOG_BYTES)..];
    writeln!(
        f,
        "[truncated: wrote last {} bytes of {}]\n",
        tail.len(),
        bytes.len()
    )
    .with_context(|| format!("write {}", path.display()))?;
    f.write_all(tail)
        .with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn run_artifacts_dir_name(timestamp_utc: &str, pid: u32) -> String {
    format!("{}_pid{}", timestamp_utc, pid)
}

pub fn report_output_path_in_run_dir(artifacts_dir: &Path, output: &str) -> PathBuf {
    let output_path = Path::new(output);
    let filename = output_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("rtsp_validation.json"));
    artifacts_dir.join(filename)
}

pub fn copy_log_to_artifacts(log_path: &Path, artifacts_dir: &Path) -> Result<Option<PathBuf>> {
    let filename = match log_path.file_name() {
        Some(name) => name,
        None => return Ok(None),
    };
    let dest = artifacts_dir.join(filename);
    if dest == log_path {
        return Ok(Some(dest));
    }
    std::fs::copy(log_path, &dest)
        .with_context(|| format!("copy {} to {}", log_path.display(), dest.display()))?;
    Ok(Some(dest))
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_TOOL_LOG_BYTES, copy_log_to_artifacts, finalize_tshark_pcap, parse_ffmpeg_kbytes_token,
        parse_ffmpeg_summary_bitrate_kbps, report_output_path_in_run_dir, rtsp_url,
        run_artifacts_dir_name, tail_lossy, tshark_readable_pcap_path, write_bytes_tail,
    };
    use std::path::Path;

    #[test]
    fn test_rtsp_url() {
        assert_eq!(
            rtsp_url("192.168.1.1", 8554, "/live"),
            "rtsp://192.168.1.1:8554/live"
        );
    }

    #[test]
    fn test_parse_ffmpeg_kbytes_token() {
        assert_eq!(parse_ffmpeg_kbytes_token("52kB"), Some(52.0));
        assert_eq!(parse_ffmpeg_kbytes_token("1MB"), Some(1000.0));
        assert_eq!(parse_ffmpeg_kbytes_token("500B"), Some(0.5));
        assert!(parse_ffmpeg_kbytes_token("n/a").is_none());
    }

    #[test]
    fn test_parse_ffmpeg_kbytes_token_accepts_kib_mib() {
        // ffmpeg 8+ prints binary units (KiB/MiB) in the null-muxer summary.
        assert_eq!(parse_ffmpeg_kbytes_token("185KiB"), Some(185.0));
        assert_eq!(parse_ffmpeg_kbytes_token("1MiB"), Some(1024.0));
        assert_eq!(parse_ffmpeg_kbytes_token("0KiB"), Some(0.0));
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps() {
        let line = "[info] video:52kB audio:816kB subtitle:0kB";
        let bitrate = parse_ffmpeg_summary_bitrate_kbps(line, 4).unwrap();
        assert!((bitrate - 1736.0).abs() < 0.01);
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps_ffmpeg8_kib() {
        let line = "[out#0/null @ 0x1] [info] video:185KiB audio:0KiB subtitle:0KiB other streams:0KiB global headers:0KiB muxing overhead: unknown";
        let bitrate = parse_ffmpeg_summary_bitrate_kbps(line, 30).unwrap();
        // 185 KiB * 8 / 30s ≈ 49.333 kbps
        assert!((bitrate - (185.0 * 8.0 / 30.0)).abs() < 0.01);
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps_missing_fields() {
        let line = "[info] Output #0, null, to 'pipe:'";
        assert!(parse_ffmpeg_summary_bitrate_kbps(line, 10).is_none());
    }

    #[test]
    fn test_parse_ffmpeg_summary_bitrate_kbps_duration_zero() {
        let line = "[info] video:52kB audio:816kB subtitle:0kB";
        assert!(parse_ffmpeg_summary_bitrate_kbps(line, 0).is_none());
    }

    #[test]
    fn test_run_artifacts_dir_name_format() {
        assert_eq!(
            run_artifacts_dir_name("20260205T120102Z", 12345),
            "20260205T120102Z_pid12345"
        );
    }

    #[test]
    fn test_tshark_readable_pcap_path_is_under_temp_dir() {
        let path = tshark_readable_pcap_path("rtsp_protocol_sequence_tcp_port554.pcap");
        assert!(path.starts_with(std::env::temp_dir()));
        assert!(
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("rtsp_protocol_sequence_tcp_port554.pcap"))
        );
        assert!(
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.contains(&std::process::id().to_string()))
        );
    }

    #[test]
    fn test_finalize_tshark_pcap_copies_when_keep_and_removes_temp() {
        let artifacts = tempfile::tempdir().unwrap();
        let tmp = tshark_readable_pcap_path("keep_me.pcap");
        std::fs::write(&tmp, b"pcap-bytes").unwrap();

        finalize_tshark_pcap(&tmp, artifacts.path(), "keep_me.pcap", true).unwrap();

        assert!(!tmp.exists());
        let artifact = artifacts.path().join("keep_me.pcap");
        assert_eq!(std::fs::read(&artifact).unwrap(), b"pcap-bytes");
    }

    #[test]
    fn test_finalize_tshark_pcap_discards_artifact_when_not_keeping() {
        let artifacts = tempfile::tempdir().unwrap();
        let tmp = tshark_readable_pcap_path("drop_me.pcap");
        std::fs::write(&tmp, b"pcap-bytes").unwrap();

        finalize_tshark_pcap(&tmp, artifacts.path(), "drop_me.pcap", false).unwrap();

        assert!(!tmp.exists());
        assert!(!artifacts.path().join("drop_me.pcap").exists());
    }

    #[test]
    fn test_tail_lossy_no_truncation() {
        assert_eq!(tail_lossy("hello", 10), "hello");
    }

    #[test]
    fn test_tail_lossy_truncation_adds_ellipsis() {
        assert_eq!(tail_lossy("abcdef", 3), "…def");
    }

    #[test]
    fn test_tail_lossy_unicode_safe() {
        let s = "aé🙂b";
        assert_eq!(tail_lossy(s, 2), "…🙂b");
    }

    #[test]
    fn test_write_bytes_tail_truncates() {
        let path = std::env::temp_dir().join(format!(
            "rtsp_validation_tool_test_{}.log",
            std::process::id()
        ));
        let bytes = vec![b'a'; MAX_TOOL_LOG_BYTES + 16];
        write_bytes_tail(&path, &bytes).unwrap();
        let out = std::fs::read(&path).unwrap();
        assert!(out.starts_with(b"[truncated:"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_report_output_path_in_run_dir_uses_filename() {
        let artifacts = Path::new("rtsp_results/runs/20260205T120102Z_pid12345");
        let path = report_output_path_in_run_dir(artifacts, "nested/custom_report.json");
        assert_eq!(
            path,
            Path::new("rtsp_results/runs/20260205T120102Z_pid12345/custom_report.json")
        );
    }

    #[test]
    fn test_report_output_path_in_run_dir_handles_default_filename() {
        let artifacts = Path::new("rtsp_results/runs/20260205T120102Z_pid12345");
        let path = report_output_path_in_run_dir(artifacts, "rtsp_validation.json");
        assert_eq!(
            path,
            Path::new("rtsp_results/runs/20260205T120102Z_pid12345/rtsp_validation.json")
        );
    }

    #[test]
    fn test_copy_log_to_artifacts_copies_file() {
        let dir = tempfile::tempdir().unwrap();
        let artifacts = dir.path().join("artifacts");
        std::fs::create_dir_all(&artifacts).unwrap();
        let log_path = dir.path().join("rtsp_validation.log");
        std::fs::write(&log_path, b"hello").unwrap();

        let dest = copy_log_to_artifacts(&log_path, &artifacts)
            .unwrap()
            .unwrap();
        let content = std::fs::read(&dest).unwrap();
        assert_eq!(content, b"hello");
    }
}
