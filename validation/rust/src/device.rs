//! Device telemetry and telnet helpers.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::net::ToSocketAddrs;
use std::path::Path;
use std::time::Duration;
use telnet::{Event, Telnet};
use tracing::{debug, info, trace, warn};

use crate::util::{write_bytes_tail, MAX_TOOL_LOG_BYTES};

const DEVICE_ONVIF_DIR: &str = "/mnt/anyka_hack/onvif";
const DEVICE_ONVIF_LOG_GLOB: &str = "onvif.log*";
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
    let start = s.find(DEVICE_MARKER_BEGIN)?;
    let rest = &s[start + DEVICE_MARKER_BEGIN.len()..];
    let end = rest.rfind(DEVICE_MARKER_END)?;
    let between = &rest[..end];
    let between = between.trim_matches(['\r', '\n', ' ']);
    Some(between.to_string())
}

fn device_cleanup_onvif_logs_blocking(host: &str, port: u16) -> Result<()> {
    let cmd = format!(
        "cd {} && rm -f {} 2>/dev/null",
        DEVICE_ONVIF_DIR, DEVICE_ONVIF_LOG_GLOB
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
        "cd {dir} && echo {b} && set -- {glob}; if [ \"$1\" = \"{glob}\" ]; then :; else for f in \"$@\"; do [ -f \"$f\" ] && printf '%s\\n' \"$f\"; done; fi; echo {e}",
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
        .filter(|l| l.starts_with("onvif.log"))
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
    let cmd = if let Some(h264) = h264_file {
        let mut c = format!(
            "cd {} && nohup ./onvif-rust --validation-mode --h264-file '{}' --rtsp-port {}",
            DEVICE_ONVIF_DIR, h264, rtsp_port
        );
        if let Some(aac) = aac_file {
            c.push_str(&format!(" --aac-file '{}'", aac));
        }
        if loop_playback {
            c.push_str(" --loop-playback");
        }
        c.push_str(&format!(" {}/config.toml &", DEVICE_ONVIF_DIR));
        c
    } else {
        format!(
            "cd {} && nohup ./onvif-rust {}/config.toml &",
            DEVICE_ONVIF_DIR, DEVICE_ONVIF_DIR
        )
    };
    debug!(command = %cmd, "device start command");
    run_telnet_command_blocking(host, port, &cmd, DEVICE_TELNET_READ_TIMEOUT_SEC)?;
    Ok(())
}

/// Stop onvif-rust on the device (blocking).
pub fn device_stop_onvif_blocking(host: &str, port: u16) -> Result<()> {
    debug!(%host, port, "stopping onvif-rust on device");
    run_telnet_command_blocking(
        host,
        port,
        "pkill -f onvif-rust",
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    )?;
    Ok(())
}

/// Collect device telemetry (blocking).
pub fn device_collect_telemetry_blocking(host: &str, port: u16) -> DeviceTelemetry {
    let mut t = DeviceTelemetry::default();
    debug!(%host, port, "collecting device telemetry");

    let meminfo_raw = match run_telnet_command_blocking(
        host,
        port,
        &format!(
            "echo {} && cat /proc/meminfo && echo {}",
            DEVICE_MARKER_BEGIN, DEVICE_MARKER_END
        ),
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = Some(format!("meminfo: {}", e));
            return t;
        }
    };
    let meminfo = extract_between_markers(&meminfo_raw).unwrap_or(meminfo_raw);
    for line in meminfo.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let key = parts[0].trim_end_matches(':');
            let value: u64 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match key {
                "MemTotal" => t.mem_total_kib = Some(value),
                "MemFree" => t.mem_free_kib = Some(value),
                "MemAvailable" => t.mem_available_kib = Some(value),
                _ => {}
            }
        }
    }
    if t.mem_available_kib.is_none() {
        t.mem_available_kib = t.mem_free_kib;
    }

    let loadavg_raw = match run_telnet_command_blocking(
        host,
        port,
        &format!(
            "echo {} && cat /proc/loadavg && echo {}",
            DEVICE_MARKER_BEGIN, DEVICE_MARKER_END
        ),
        DEVICE_TELNET_READ_TIMEOUT_SEC,
    ) {
        Ok(s) => s,
        Err(e) => {
            t.error = Some(format!("loadavg: {}", e));
            return t;
        }
    };
    let loadavg = extract_between_markers(&loadavg_raw).unwrap_or(loadavg_raw);
    for line in loadavg.lines() {
        let line = line.trim_matches(|c| c == '\r' || c == '\n' || c == ' ');
        let parts: Vec<&str> = line.split_whitespace().take(3).collect();
        if parts.len() >= 3 {
            let a: Option<f64> = parts[0].parse().ok();
            let b: Option<f64> = parts[1].parse().ok();
            let c: Option<f64> = parts[2].parse().ok();
            if let (Some(x), Some(y), Some(z)) = (a, b, c) {
                t.load_avg_1m = Some(x);
                t.load_avg_5m = Some(y);
                t.load_avg_15m = Some(z);
                break;
            }
        }
    }

    let pgrep_raw = match run_telnet_command_blocking(
        host,
        port,
        &format!(
            "echo {} && ( pgrep -f onvif-rust; true ) && echo {}",
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
    let pid_str = pgrep_out.lines().find_map(|l| {
        let l = l.trim();
        if l.is_empty() || !l.chars().all(|c| c.is_ascii_digit()) {
            None
        } else {
            l.parse::<u32>().ok()
        }
    });
    let Some(pid) = pid_str else {
        debug!("pgrep did not find onvif-rust; skipping process status");
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
    for line in status.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let key = parts[0].trim_end_matches(':');
            let value: u64 = match parts[1].parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            match key {
                "VmRSS" => t.onvif_rss_kib = Some(value),
                "VmSize" => t.onvif_vmsize_kib = Some(value),
                _ => {}
            }
        }
    }
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
    use super::{extract_between_markers, sh_single_quote};

    #[test]
    fn test_sh_single_quote() {
        assert_eq!(sh_single_quote("abc"), "'abc'");
        assert_eq!(sh_single_quote("a'b"), "'a'\\''b'");
    }

    #[test]
    fn test_extract_between_markers() {
        let s = "noise\n__ANYKA_BEGIN__\nhello\nworld\n__ANYKA_END__\nmore";
        let extracted = extract_between_markers(s).unwrap();
        assert_eq!(extracted, "hello\nworld");
    }
}
