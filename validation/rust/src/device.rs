//! Device telemetry and telnet helpers.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::time::Duration;
use telnet::{Event, Telnet};
use tracing::{debug, info, trace, warn};

use crate::util::{MAX_TOOL_LOG_BYTES, tail_lossy, write_bytes_tail};

const DEVICE_ONVIF_DIR: &str = "/mnt/anyka_hack/onvif";
const DEVICE_ONVIF_LOG_GLOB: &str = "onvif.log*";
const DEVICE_ONVIF_PIDFILE: &str = "onvif.pid";
const DEVICE_TELNET_CONNECT_TIMEOUT_SEC: u64 = 15;
const DEVICE_TELNET_READ_TIMEOUT_SEC: u64 = 8;
const DEVICE_TELNET_LOG_COPY_READ_TIMEOUT_SEC: u64 = 45;
const DEVICE_MARKER_BEGIN: &str = "__ANYKA_BEGIN__";
const DEVICE_MARKER_END: &str = "__ANYKA_END__";

/// Device telemetry snapshot (RAM, CPU, onvif-rust memory) when using --launch-on-device.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeviceTelemetry {
    pub mem_total_kib: Option<u64>,
    pub mem_free_kib: Option<u64>,
    pub mem_available_kib: Option<u64>,
    pub load_avg_1m: Option<f64>,
    pub load_avg_5m: Option<f64>,
    pub load_avg_15m: Option<f64>,
    pub onvif_rss_kib: Option<u64>,
    pub onvif_vmsize_kib: Option<u64>,
    pub onvif_pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run a single command on the device via telnet (blocking). Returns accumulated output.
pub fn run_telnet_command_blocking(
    host: &str,
    port: u16,
    command: &str,
    read_timeout_sec: u64,
) -> Result<String> {
    debug!(%host, port, command = %command, "telnet command");
    let addr = (host, port)
        .to_socket_addrs()
        .context("resolve device address")?
        .next()
        .ok_or_else(|| anyhow!("no address for {}:{}", host, port))?;

    let mut telnet = Telnet::connect_timeout(
        &addr,
        4096,
        Duration::from_secs(DEVICE_TELNET_CONNECT_TIMEOUT_SEC),
    )
    .with_context(|| format!("telnet connect to {}:{}", host, port))?;

    let cmd_line = format!("{}\n", command);
    telnet
        .write(cmd_line.as_bytes())
        .context("telnet write command")?;

    let timeout_dur = Duration::from_secs(read_timeout_sec);
    let mut out = Vec::new();
    loop {
        let event = telnet.read_timeout(timeout_dur).context("telnet read")?;
        match event {
            Event::Data(buf) => out.extend_from_slice(&buf),
            Event::TimedOut => break,
            _ => {}
        }
    }
    let s = String::from_utf8_lossy(&out).into_owned();
    trace!(output_len = s.len(), "telnet output");
    Ok(s)
}

pub fn sanitize_filename_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len().min(128));
    for ch in s.chars().take(128) {
        let ok = ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-');
        out.push(if ok { ch } else { '_' });
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

pub fn sh_single_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

pub fn extract_between_markers(s: &str) -> Option<String> {
    // Prefer markers that appear on their own line. This avoids accidentally
    // matching command-echo text such as: `echo __ANYKA_BEGIN__ && ...`.
    let marker_is_standalone = |idx: usize, marker: &str| -> bool {
        let line_start = s[..idx].rfind('\n').map_or(0, |p| p + 1);
        let prefix = &s[line_start..idx];
        if !prefix
            .trim_matches(|c: char| matches!(c, ' ' | '\t' | '\r'))
            .is_empty()
        {
            return false;
        }

        let after = idx + marker.len();
        let line_end = s[after..].find('\n').map_or(s.len(), |p| after + p);
        let suffix = &s[after..line_end];
        suffix
            .trim_matches(|c: char| matches!(c, ' ' | '\t' | '\r'))
            .is_empty()
    };

    for (start, _) in s.match_indices(DEVICE_MARKER_BEGIN) {
        if !marker_is_standalone(start, DEVICE_MARKER_BEGIN) {
            continue;
        }
        let rest_start = start + DEVICE_MARKER_BEGIN.len();
        for (rel_end, _) in s[rest_start..].match_indices(DEVICE_MARKER_END) {
            let end = rest_start + rel_end;
            if marker_is_standalone(end, DEVICE_MARKER_END) {
                let between = &s[rest_start..end];
                let between = between.trim_matches(['\r', '\n', ' ']);
                return Some(between.to_string());
            }
        }
    }

    // Fallback for older/simple outputs where markers are not line-delimited.
    let start = s.find(DEVICE_MARKER_BEGIN)?;
    let rest = &s[start + DEVICE_MARKER_BEGIN.len()..];
    let end = rest.rfind(DEVICE_MARKER_END)?;
    let between = &rest[..end];
    let between = between.trim_matches(['\r', '\n', ' ']);
    Some(between.to_string())
}

fn first_u64_in_text(s: &str) -> Option<u64> {
    let start = s
        .char_indices()
        .find_map(|(i, ch)| if ch.is_ascii_digit() { Some(i) } else { None })?;
    let end = s[start..]
        .char_indices()
        .find_map(|(i, ch)| {
            if ch.is_ascii_digit() {
                None
            } else {
                Some(start + i)
            }
        })
        .unwrap_or(s.len());
    s[start..end].parse::<u64>().ok()
}

/// Parse /proc/meminfo content. Returns (MemTotal, MemFree, MemAvailable) in KiB.
pub fn parse_meminfo(content: &str) -> (Option<u64>, Option<u64>, Option<u64>) {
    let mut mem_total = None;
    let mut mem_free = None;
    let mut mem_available = None;
    for line in content.lines() {
        if mem_total.is_none() && line.contains("MemTotal:") {
            mem_total = first_u64_in_text(line);
        }
        if mem_free.is_none() && line.contains("MemFree:") {
            mem_free = first_u64_in_text(line);
        }
        if mem_available.is_none() && line.contains("MemAvailable:") {
            mem_available = first_u64_in_text(line);
        }
        if mem_total.is_some() && mem_free.is_some() && mem_available.is_some() {
            break;
        }
    }
    if mem_available.is_none() {
        mem_available = mem_free;
    }
    (mem_total, mem_free, mem_available)
}

/// Parse /proc/loadavg content. Returns (1m, 5m, 15m) or None if unparseable.
pub fn parse_loadavg(content: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
    for line in content.lines() {
        let line = line.trim_matches(|c: char| c == '\r' || c == '\n' || c == ' ');
        let parts: Vec<&str> = line.split_whitespace().take(3).collect();
        if parts.len() >= 3 {
            let a: Option<f64> = parts[0].parse().ok();
            let b: Option<f64> = parts[1].parse().ok();
            let c: Option<f64> = parts[2].parse().ok();
            if let (Some(x), Some(y), Some(z)) = (a, b, c) {
                return (Some(x), Some(y), Some(z));
            }
        }
    }
    (None, None, None)
}

/// Parse pgrep output (one pid per line). Returns first valid pid found.
pub fn parse_pgrep_output(content: &str) -> Option<u32> {
    content.lines().find_map(|line| {
        let trimmed = line.trim_matches(|c: char| matches!(c, '\r' | '\n' | ' ' | '\t'));
        if trimmed.is_empty() {
            return None;
        }
        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        if tokens
            .iter()
            .all(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
        {
            return tokens
                .into_iter()
                .filter_map(|t| t.parse::<u32>().ok())
                .find(|pid| *pid > 1);
        }
        None
    })
}

/// Parse `/proc/<pid>/status` for `VmRSS` and `VmSize` (KiB).
pub fn parse_status_vmrss_vmsize(content: &str) -> (Option<u64>, Option<u64>) {
    let mut rss = None;
    let mut vmsize = None;
    for line in content.lines() {
        if rss.is_none() && line.contains("VmRSS:") {
            rss = first_u64_in_text(line);
        }
        if vmsize.is_none() && line.contains("VmSize:") {
            vmsize = first_u64_in_text(line);
        }
        if rss.is_some() && vmsize.is_some() {
            break;
        }
    }
    (rss, vmsize)
}

fn device_cleanup_onvif_logs_blocking(host: &str, port: u16) -> Result<()> {
    let cmd = format!(
        "cd {} && rm -f {} nohup.out {} 2>/dev/null",
        DEVICE_ONVIF_DIR, DEVICE_ONVIF_LOG_GLOB, DEVICE_ONVIF_PIDFILE
    );
    debug!(command = %cmd, "device cleanup onvif logs");
    let _ = run_telnet_command_blocking(host, port, &cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)?;
    Ok(())
}

/// Copy onvif-rust logs from device to artifacts dir (blocking).
pub fn device_copy_onvif_logs_blocking(host: &str, port: u16, artifacts_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(artifacts_dir)
        .with_context(|| format!("create artifacts dir {}", artifacts_dir.display()))?;

    let list_cmd = format!(
        "cd {dir} && echo {b} && (for f in {glob} nohup.out; do [ -f \"$f\" ] && printf '%s\\n' \"$f\"; done) && echo {e}",
        dir = DEVICE_ONVIF_DIR,
        glob = DEVICE_ONVIF_LOG_GLOB,
        b = DEVICE_MARKER_BEGIN,
        e = DEVICE_MARKER_END
    );
    let listing_raw = run_telnet_command_blocking(
        host,
        port,
        &list_cmd,
        DEVICE_TELNET_LOG_COPY_READ_TIMEOUT_SEC,
    )?;
    let listing = extract_between_markers(&listing_raw).unwrap_or(listing_raw);
    let files: Vec<String> = listing
        .lines()
        .map(|l| l.trim_matches(['\r', '\n', ' ']))
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    if files.is_empty() {
        debug!("no device onvif logs found");
        return Ok(());
    }

    for f in files {
        let safe = sanitize_filename_component(&f);
        let out_path = artifacts_dir.join(format!("device_{}", safe));

        let q = sh_single_quote(&f);
        let read_cmd = format!(
            "cd {dir} && echo {b} && (tail -c {bytes} {q} 2>/dev/null || tail -n 20000 {q} 2>/dev/null || cat {q} 2>/dev/null) && echo {e}",
            dir = DEVICE_ONVIF_DIR,
            b = DEVICE_MARKER_BEGIN,
            e = DEVICE_MARKER_END,
            bytes = MAX_TOOL_LOG_BYTES,
            q = q
        );
        let raw = run_telnet_command_blocking(
            host,
            port,
            &read_cmd,
            DEVICE_TELNET_LOG_COPY_READ_TIMEOUT_SEC,
        )?;
        let content = extract_between_markers(&raw).unwrap_or(raw);
        write_bytes_tail(&out_path, content.as_bytes())
            .with_context(|| format!("write {}", out_path.display()))?;
        info!(path = %out_path.display(), device_file = %f, "copied device onvif log");
    }

    Ok(())
}

/// Start onvif-rust on the device (blocking). If h264_file is Some, runs in validation mode.
pub fn device_start_onvif_blocking(
    host: &str,
    port: u16,
    rtsp_port: u16,
    h264_file: Option<&str>,
    aac_file: Option<&str>,
    loop_playback: bool,
) -> Result<()> {
    info!(
        %host,
        port,
        rtsp_port,
        h264 = ?h264_file,
        aac = ?aac_file,
        loop_playback,
        "starting onvif-rust on device"
    );
    if let Err(e) = device_cleanup_onvif_logs_blocking(host, port) {
        warn!(error = %e, "failed to cleanup device onvif logs before start");
    }
    let config_q = sh_single_quote(&format!("{}/config.toml", DEVICE_ONVIF_DIR));
    let onvif_cmd = if let Some(h264) = h264_file {
        let mut c = format!(
            "./onvif-rust --validation-mode --h264-file {} --rtsp-port {}",
            sh_single_quote(h264),
            rtsp_port
        );
        if let Some(aac) = aac_file {
            c.push_str(&format!(" --aac-file {}", sh_single_quote(aac)));
        }
        if loop_playback {
            c.push_str(" --loop-playback");
        }
        c.push_str(&format!(" {}", config_q));
        c
    } else {
        format!("./onvif-rust {}", config_q)
    };

    let cmd = format!(
        "cd {dir} && LOG=\"onvif.log.$(date +%Y%m%dT%H%M%S)\"; nohup {onvif_cmd} > \"$LOG\" 2>&1 & echo $! > {pidfile}",
        dir = DEVICE_ONVIF_DIR,
        onvif_cmd = onvif_cmd,
        pidfile = DEVICE_ONVIF_PIDFILE
    );
    debug!(command = %cmd, "device start command");
    run_telnet_command_blocking(host, port, &cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)?;
    Ok(())
}

/// Stop onvif-rust on the device (blocking).
pub fn device_stop_onvif_blocking(host: &str, port: u16) -> Result<()> {
    debug!(%host, port, "stopping onvif-rust on device");
    let cmd = format!(
        "cd {dir} && if [ -f {pidfile} ]; then pid=\"$(cat {pidfile} 2>/dev/null || true)\"; case \"$pid\" in ''|*[!0-9]*) ;; *) kill \"$pid\" 2>/dev/null ;; esac; fi; pkill -f onvif-rust",
        dir = DEVICE_ONVIF_DIR,
        pidfile = DEVICE_ONVIF_PIDFILE
    );
    run_telnet_command_blocking(host, port, &cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)?;
    Ok(())
}

/// Collect device telemetry (blocking).
pub fn device_collect_telemetry_blocking(host: &str, port: u16) -> DeviceTelemetry {
    let mut t = DeviceTelemetry::default();
    debug!(%host, port, "collecting device telemetry");

    let meminfo_cmd = format!(
        "echo {} && cat /proc/meminfo && echo {}",
        DEVICE_MARKER_BEGIN, DEVICE_MARKER_END
    );
    let mut meminfo_raw =
        match run_telnet_command_blocking(host, port, &meminfo_cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)
        {
            Ok(s) => s,
            Err(e) => {
                t.error = Some(format!("meminfo: {}", e));
                return t;
            }
        };
    let mut meminfo = extract_between_markers(&meminfo_raw).unwrap_or(meminfo_raw.clone());
    let mut parsed = parse_meminfo(&meminfo);
    if parsed.0.is_none() && parsed.1.is_none() && parsed.2.is_none() {
        match run_telnet_command_blocking(
            host,
            port,
            &meminfo_cmd,
            DEVICE_TELNET_READ_TIMEOUT_SEC.saturating_add(10),
        ) {
            Ok(s) => {
                meminfo_raw = s;
                meminfo = extract_between_markers(&meminfo_raw).unwrap_or(meminfo_raw.clone());
                parsed = parse_meminfo(&meminfo);
            }
            Err(e) => {
                t.error = Some(format!("meminfo retry: {}", e));
                return t;
            }
        }
    }
    let (mem_total, mem_free, mem_available) = parsed;
    t.mem_total_kib = mem_total;
    t.mem_free_kib = mem_free;
    t.mem_available_kib = mem_available;
    if t.mem_total_kib.is_none() && t.mem_free_kib.is_none() && t.mem_available_kib.is_none() {
        t.error = t.error.or_else(|| {
            Some(format!(
                "meminfo parse failed: {}",
                tail_lossy(meminfo.trim(), 300)
            ))
        });
    }

    let loadavg_cmd = format!(
        "echo {} && cat /proc/loadavg && echo {}",
        DEVICE_MARKER_BEGIN, DEVICE_MARKER_END
    );
    let mut loadavg_raw =
        match run_telnet_command_blocking(host, port, &loadavg_cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)
        {
            Ok(s) => s,
            Err(e) => {
                t.error = Some(format!("loadavg: {}", e));
                return t;
            }
        };
    let mut loadavg = extract_between_markers(&loadavg_raw).unwrap_or(loadavg_raw.clone());
    let mut loads = parse_loadavg(&loadavg);
    if loads.0.is_none() && loads.1.is_none() && loads.2.is_none() {
        match run_telnet_command_blocking(
            host,
            port,
            &loadavg_cmd,
            DEVICE_TELNET_READ_TIMEOUT_SEC.saturating_add(10),
        ) {
            Ok(s) => {
                loadavg_raw = s;
                loadavg = extract_between_markers(&loadavg_raw).unwrap_or(loadavg_raw.clone());
                loads = parse_loadavg(&loadavg);
            }
            Err(e) => {
                t.error = t.error.or_else(|| Some(format!("loadavg retry: {}", e)));
                return t;
            }
        }
    }
    let (load_1m, load_5m, load_15m) = loads;
    t.load_avg_1m = load_1m;
    t.load_avg_5m = load_5m;
    t.load_avg_15m = load_15m;
    if t.load_avg_1m.is_none() && t.load_avg_5m.is_none() && t.load_avg_15m.is_none() {
        t.error = t.error.or_else(|| {
            Some(format!(
                "loadavg parse failed: {}",
                tail_lossy(loadavg.trim(), 200)
            ))
        });
    }

    let pgrep_raw = match run_telnet_command_blocking(
        host,
        port,
        &format!(
            "echo {} && ( pgrep -f onvif-rust 2>/dev/null || pidof onvif-rust 2>/dev/null || ps | grep '[o]nvif-rust' | awk '{{print $1}}'; true ) && echo {}",
            DEVICE_MARKER_BEGIN, DEVICE_MARKER_END
        ),
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = t.error.or_else(|| Some(format!("pgrep: {}", e)));
            return t;
        }
    };
    let pgrep_out = extract_between_markers(&pgrep_raw).unwrap_or(pgrep_raw);
    let mut pid = parse_pgrep_output(&pgrep_out);
    if pid.is_none() {
        let pidfile_cmd = format!(
            "echo {} && cat {}/{} 2>/dev/null && echo {}",
            DEVICE_MARKER_BEGIN, DEVICE_ONVIF_DIR, DEVICE_ONVIF_PIDFILE, DEVICE_MARKER_END
        );
        if let Ok(raw) =
            run_telnet_command_blocking(host, port, &pidfile_cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)
        {
            let pidfile_out = extract_between_markers(&raw).unwrap_or(raw);
            pid = parse_pgrep_output(&pidfile_out);
        }
    }
    let Some(pid) = pid else {
        debug!("could not determine onvif-rust pid; skipping process status");
        return t;
    };
    trace!(pid = pid, "onvif-rust process found");
    t.onvif_pid = Some(pid);

    let status_cmd = format!(
        "echo {} && cat /proc/{}/status 2>/dev/null && echo {}",
        DEVICE_MARKER_BEGIN, pid, DEVICE_MARKER_END
    );
    let status_raw = match run_telnet_command_blocking(
        host,
        port,
        &status_cmd,
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = t.error.or_else(|| Some(format!("status: {}", e)));
            return t;
        }
    };
    let status = extract_between_markers(&status_raw).unwrap_or(status_raw);
    let (rss, vmsize) = parse_status_vmrss_vmsize(&status);
    t.onvif_rss_kib = rss;
    t.onvif_vmsize_kib = vmsize;
    debug!(
        mem_available_kib = ?t.mem_available_kib,
        load_avg_1m = ?t.load_avg_1m,
        onvif_rss_kib = ?t.onvif_rss_kib,
        onvif_pid = ?t.onvif_pid,
        "telemetry collected"
    );
    t
}

#[cfg(test)]
mod tests {
    use super::{
        extract_between_markers, parse_loadavg, parse_meminfo, parse_pgrep_output,
        parse_status_vmrss_vmsize, sanitize_filename_component, sh_single_quote,
    };

    const FIXTURE_MEMINFO: &str = r#"MemTotal:          36540 kB
MemFree:           23256 kB
Buffers:             472 kB
Cached:             4676 kB
MemAvailable:       25000 kB"#;

    const FIXTURE_LOADAVG: &str = "2.00 2.01 2.05 1/57 23002";

    const FIXTURE_PROC_STATUS: &str = r#"Name:   onvif-rust
State:  S (sleeping)
VmRSS:  12456 kB
VmSize: 25680 kB
Threads:        1"#;

    #[test]
    fn test_sh_single_quote() {
        assert_eq!(sh_single_quote("abc"), "'abc'");
        assert_eq!(sh_single_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn test_sh_single_quote_empty_string() {
        assert_eq!(sh_single_quote(""), "''");
    }

    #[test]
    fn test_extract_between_markers() {
        let s = "noise\n__ANYKA_BEGIN__\nhello\nworld\n__ANYKA_END__\nmore";
        let extracted = extract_between_markers(s).unwrap();
        assert_eq!(extracted, "hello\nworld");
    }

    #[test]
    fn test_extract_between_markers_no_markers_returns_none() {
        assert!(extract_between_markers("no markers here").is_none());
    }

    #[test]
    fn test_extract_between_markers_only_begin_returns_none() {
        assert!(extract_between_markers("__ANYKA_BEGIN__\ncontent").is_none());
    }

    #[test]
    fn test_extract_between_markers_trimmed_content() {
        let s = "__ANYKA_BEGIN__\r\n  inner  \r\n__ANYKA_END__";
        let extracted = extract_between_markers(s).unwrap();
        assert_eq!(extracted, "inner");
    }

    #[test]
    fn test_extract_between_markers_ignores_inline_command_echo_markers() {
        let s = "echo __ANYKA_BEGIN__ && cat /proc/meminfo && echo __ANYKA_END__\n__ANYKA_BEGIN__\nMemTotal: 36540 kB\n__ANYKA_END__\n";
        let extracted = extract_between_markers(s).unwrap();
        assert_eq!(extracted, "MemTotal: 36540 kB");
    }

    #[test]
    fn test_sanitize_filename_component_alphanumeric_kept() {
        assert_eq!(sanitize_filename_component("abc123"), "abc123");
        assert_eq!(sanitize_filename_component("file.log"), "file.log");
        assert_eq!(sanitize_filename_component("a_b-c"), "a_b-c");
    }

    #[test]
    fn test_sanitize_filename_component_special_replaced() {
        assert_eq!(sanitize_filename_component("a/b"), "a_b");
        assert_eq!(sanitize_filename_component("a b"), "a_b");
        assert_eq!(sanitize_filename_component("a:b"), "a_b");
    }

    #[test]
    fn test_sanitize_filename_component_empty_returns_unknown() {
        assert_eq!(sanitize_filename_component(""), "unknown");
    }

    #[test]
    fn test_sanitize_filename_component_only_special_returns_underscores() {
        // All special chars replaced by _; result non-empty so not "unknown"
        assert_eq!(sanitize_filename_component("///"), "___");
    }

    #[test]
    fn test_sanitize_filename_component_length_capped_128() {
        let long = "a".repeat(200);
        let out = sanitize_filename_component(&long);
        assert_eq!(out.len(), 128);
        assert!(out.chars().all(|c| c == 'a'));
    }

    #[test]
    fn test_parse_meminfo() {
        let (total, free, avail) = parse_meminfo(FIXTURE_MEMINFO);
        assert_eq!(total, Some(36540));
        assert_eq!(free, Some(23256));
        assert_eq!(avail, Some(25000));
    }

    #[test]
    fn test_parse_meminfo_fallback_available_from_free() {
        let s = "MemTotal: 1000 kB\nMemFree: 200 kB";
        let (_t, free, avail) = parse_meminfo(s);
        assert_eq!(free, Some(200));
        assert_eq!(avail, Some(200));
    }

    #[test]
    fn test_parse_meminfo_empty() {
        let (total, free, avail) = parse_meminfo("");
        assert_eq!(total, None);
        assert_eq!(free, None);
        assert_eq!(avail, None);
    }

    #[test]
    fn test_parse_meminfo_with_prompt_noise() {
        let s = "~ # MemTotal: 36540 kB\n~ # MemFree: 10792 kB\n~ # MemAvailable: 12000 kB";
        let (total, free, avail) = parse_meminfo(s);
        assert_eq!(total, Some(36540));
        assert_eq!(free, Some(10792));
        assert_eq!(avail, Some(12000));
    }

    #[test]
    fn test_parse_loadavg() {
        let (a, b, c) = parse_loadavg(FIXTURE_LOADAVG);
        assert_eq!(a, Some(2.0));
        assert_eq!(b, Some(2.01));
        assert_eq!(c, Some(2.05));
    }

    #[test]
    fn test_parse_loadavg_with_newline() {
        let (a, b, c) = parse_loadavg("0.5 0.6 0.7 2/100 1234\n");
        assert_eq!(a, Some(0.5));
        assert_eq!(b, Some(0.6));
        assert_eq!(c, Some(0.7));
    }

    #[test]
    fn test_parse_loadavg_invalid_returns_none() {
        let (a, b, c) = parse_loadavg("not numbers");
        assert_eq!(a, None);
        assert_eq!(b, None);
        assert_eq!(c, None);
    }

    #[test]
    fn test_parse_pgrep_output() {
        assert_eq!(parse_pgrep_output("12345"), Some(12345));
        assert_eq!(parse_pgrep_output("12345\n"), Some(12345));
        assert_eq!(parse_pgrep_output(" 999 \n"), Some(999));
    }

    #[test]
    fn test_parse_pgrep_output_empty() {
        assert_eq!(parse_pgrep_output(""), None);
        assert_eq!(parse_pgrep_output("\n\n"), None);
    }

    #[test]
    fn test_parse_pgrep_output_skips_non_digit() {
        let s = "abc\n12345";
        assert_eq!(parse_pgrep_output(s), Some(12345));
    }

    #[test]
    fn test_parse_pgrep_output_handles_pidof_space_separated() {
        assert_eq!(parse_pgrep_output("2689 2710\n"), Some(2689));
    }

    #[test]
    fn test_parse_status_vmrss_vmsize() {
        let (rss, vmsize) = parse_status_vmrss_vmsize(FIXTURE_PROC_STATUS);
        assert_eq!(rss, Some(12456));
        assert_eq!(vmsize, Some(25680));
    }

    #[test]
    fn test_parse_status_vmrss_vmsize_empty() {
        let (rss, vmsize) = parse_status_vmrss_vmsize("");
        assert_eq!(rss, None);
        assert_eq!(vmsize, None);
    }

    #[test]
    fn test_parse_status_vmrss_vmsize_with_prompt_noise() {
        let s = "~ # VmRSS: 3660 kB\n~ # VmSize: 8820 kB\n";
        let (rss, vmsize) = parse_status_vmrss_vmsize(s);
        assert_eq!(rss, Some(3660));
        assert_eq!(vmsize, Some(8820));
    }
}
