//! RTSP validation tool binary: orchestration and main entrypoint.

use anyhow::{Context, Result, anyhow, bail};
use rtsp_validation_tool::baseline::apply_baseline_ops;
use rtsp_validation_tool::config::{
    Args, EffectiveConfig, RtspValidationConfig, load_config, parse_args,
};
use rtsp_validation_tool::device::{
    DevicePlaybackOptions, device_collect_telemetry_blocking, device_copy_onvif_logs_blocking,
    device_get_onvif_pid_blocking, device_start_onvif_blocking, device_stop_onvif_blocking,
};
use rtsp_validation_tool::report::{TestResult, ValidationReport, compute_summary};
use rtsp_validation_tool::rtsp::{critical_proto_failed, run_harness, run_validation};
use rtsp_validation_tool::util::{
    copy_log_to_artifacts, report_output_path_in_run_dir, run_artifacts_dir_name, tail_lossy,
};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn accept_already_initialized(e: impl std::error::Error + Send + Sync + 'static) -> Result<()> {
    let msg = e.to_string();
    if msg.contains("already") || msg.contains("set a logger") {
        Ok(())
    } else {
        Err(anyhow::Error::from(e))
    }
}

fn init_tracing(config: Option<&RtspValidationConfig>) -> Result<Option<PathBuf>> {
    let filter = env::var("RUST_LOG")
        .ok()
        .and_then(|s| EnvFilter::try_new(s).ok())
        .or_else(|| {
            let level = config
                .map(|c| c.logging.level.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or("info");
            let retina_level = config.and_then(|c| {
                let s = c.logging.retina_level.trim();
                if s.is_empty() { None } else { Some(s) }
            });
            let filter_str = if let Some(r) = retina_level {
                format!("{},retina={}", level, r)
            } else {
                level.to_string()
            };
            EnvFilter::try_new(&filter_str).ok()
        })
        .unwrap_or_else(|| EnvFilter::new("info"));

    let _ = tracing_log::LogTracer::init();

    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(true)
        .with_level(true);

    let log_file_path: Option<PathBuf> = if let Some(c) = config {
        if !c.logging.file.is_empty() {
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&c.logging.file)
            {
                Ok(file) => {
                    let path = PathBuf::from(&c.logging.file);
                    let file_layer = fmt::layer()
                        .with_writer(file)
                        .with_ansi(false)
                        .with_target(true)
                        .with_level(true);
                    tracing_subscriber::registry()
                        .with(filter)
                        .with(stdout_layer)
                        .with(file_layer)
                        .try_init()
                        .or_else(accept_already_initialized)
                        .context("init tracing")?;
                    Some(path.clone())
                }
                Err(e) => {
                    eprintln!("Failed to open log file {}: {}", c.logging.file, e);
                    tracing_subscriber::registry()
                        .with(filter)
                        .with(stdout_layer)
                        .try_init()
                        .or_else(accept_already_initialized)
                        .context("init tracing")?;
                    None
                }
            }
        } else {
            tracing_subscriber::registry()
                .with(filter)
                .with(stdout_layer)
                .try_init()
                .or_else(accept_already_initialized)
                .context("init tracing")?;
            None
        }
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(stdout_layer)
            .try_init()
            .or_else(accept_already_initialized)
            .context("init tracing")?;
        None
    };

    if let Some(ref path) = log_file_path {
        info!(path = %path.display(), "logging to file");
    }
    Ok(log_file_path)
}

async fn wait_for_signal() {
    #[cfg(unix)]
    {
        let ctrl_c = tokio::signal::ctrl_c();
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("register SIGTERM");
        tokio::select! {
            _ = ctrl_c => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.ok();
    }
}

fn read_text_log_tail(path: &Path, max_chars: usize) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    Some(tail_lossy(&text, max_chars))
}

fn local_exit_diagnostics(artifacts_dir: &Path) -> String {
    let stdout_path = artifacts_dir.join("onvif_rust.stdout.log");
    let stderr_path = artifacts_dir.join("onvif_rust.stderr.log");
    let mut details: Vec<String> = Vec::new();
    if let Some(stdout_tail) = read_text_log_tail(&stdout_path, 1200) {
        details.push(format!(
            "stdout tail ({}): {}",
            stdout_path.display(),
            stdout_tail
        ));
    }
    if let Some(stderr_tail) = read_text_log_tail(&stderr_path, 1200) {
        details.push(format!(
            "stderr tail ({}): {}",
            stderr_path.display(),
            stderr_tail
        ));
    }
    if details.is_empty() {
        "no captured onvif-rust stdout/stderr logs found (enable capture_tool_output to persist local server logs)".to_string()
    } else {
        details.join(" | ")
    }
}

fn device_failure_diagnostics(artifacts_dir: &Path) -> String {
    let entries = match std::fs::read_dir(artifacts_dir) {
        Ok(entries) => entries,
        Err(e) => {
            return format!(
                "failed to read artifacts dir {}: {}",
                artifacts_dir.display(),
                e
            );
        }
    };

    let mut logs: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("device_onvif.log") {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        logs.push((modified, path));
    }

    logs.sort_by(|a, b| b.0.cmp(&a.0));
    if logs.is_empty() {
        return format!(
            "no copied device onvif logs found in artifacts dir {}",
            artifacts_dir.display()
        );
    }

    let mut details: Vec<String> = Vec::new();
    for (_, path) in logs.into_iter().take(2) {
        match read_text_log_tail(&path, 1200) {
            Some(tail) => details.push(format!("{} tail: {}", path.display(), tail.trim())),
            None => details.push(format!("{} tail unavailable", path.display())),
        }
    }
    details.join(" | ")
}

async fn cleanup_device_process(effective: &EffectiveConfig) {
    if !effective.launch_on_device {
        return;
    }
    let Some(device_password) = effective.device_password.clone() else {
        warn!("device password not set; skipping device cleanup over SSH");
        return;
    };
    let device_user = effective.device_user.clone();

    let host = effective.device_host.clone();
    let port = effective.device_ssh_port;
    let stop_user = device_user.clone();
    let stop_password = device_password.clone();
    let stop_result = tokio::task::spawn_blocking(move || {
        device_stop_onvif_blocking(&host, port, &stop_user, &stop_password)
    })
    .await
    .context("spawn_blocking device stop");
    if let Err(e) = stop_result.and_then(|r| r) {
        warn!(error = %e, "device stop onvif-rust failed");
    }

    let host = effective.device_host.clone();
    let port = effective.device_ssh_port;
    let user = device_user;
    let password = device_password;
    let artifacts_dir = effective.artifacts_dir.clone();
    match tokio::task::spawn_blocking(move || {
        device_copy_onvif_logs_blocking(&host, port, &user, &password, &artifacts_dir)
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => warn!(error = %e, "device onvif log copy failed"),
        Err(e) => warn!(error = %e, "spawn_blocking device onvif log copy failed"),
    }
}

fn cleanup_local_child(child: &mut Option<Child>) {
    if let Some(c) = child {
        let _ = c.kill();
        let _ = c.wait();
        debug!("local server process terminated");
    }
}

fn copy_validator_log(log_file_path: Option<&Path>, artifacts_dir: &Path) {
    if let Some(path) = log_file_path {
        match copy_log_to_artifacts(path, artifacts_dir) {
            Ok(Some(dest)) => info!(path = %dest.display(), "copied validator log to artifacts"),
            Ok(None) => {}
            Err(e) => warn!(error = %e, "failed to copy validator log to artifacts"),
        }
    }
}

fn create_run_artifacts_dir(root: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(root)
        .with_context(|| format!("create artifacts root dir {}", root.display()))?;
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let pid = std::process::id();
    let dir = root.join(run_artifacts_dir_name(&ts, pid));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("create artifacts dir {}", dir.display()))?;
    Ok(dir)
}

fn default_onvif_binary_path(cwd: &Path) -> PathBuf {
    let candidates = [
        // Workspace default (when building `cross-compile/onvif-rust` as part of the workspace)
        cwd.join("cross-compile/target/x86_64-unknown-linux-gnu/debug/onvif-rust"),
        cwd.join("cross-compile/target/x86_64-unknown-linux-gnu/release/onvif-rust"),
        // When running from inside `cross-compile/`
        cwd.join("target/x86_64-unknown-linux-gnu/debug/onvif-rust"),
        cwd.join("target/x86_64-unknown-linux-gnu/release/onvif-rust"),
        // Legacy single-crate target dir layouts (kept for backwards compatibility)
        cwd.join("cross-compile/onvif-rust/target/x86_64-unknown-linux-gnu/debug/onvif-rust"),
        cwd.join("cross-compile/onvif-rust/target/debug/onvif-rust"),
    ];

    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn validate_args(args: &Args, effective: &EffectiveConfig) -> Result<()> {
    if effective.launch_on_device && !effective.no_launch {
        bail!(
            "when using launch_on_device (config or --launch-on-device), no_launch must be true (server is started on device)"
        );
    }
    if !effective.no_launch && !effective.launch_on_device && effective.h264_file.is_none() {
        bail!(
            "when launching locally (no no_launch, no launch_on_device), h264_file is required (config [run] h264_file or --h264-file)"
        );
    }

    match (&args.username, &args.password) {
        (Some(_), None) | (None, Some(_)) => {
            bail!("--username and --password must be provided together");
        }
        _ => {}
    }
    if effective.launch_on_device && effective.device_password.is_none() {
        bail!(
            "launch_on_device requires a device SSH password; set --device-password, [device].password, or RTSP_VALIDATION_DEVICE_PASSWORD"
        );
    }

    if !effective.rtsp_stream.starts_with('/') {
        bail!(
            "--rtsp-stream must start with '/' (got {})",
            effective.rtsp_stream
        );
    }

    Ok(())
}

fn maybe_launch_server(args: &Args, effective: &EffectiveConfig) -> Result<Option<Child>> {
    if effective.no_launch || effective.launch_on_device {
        debug!(
            no_launch = effective.no_launch,
            launch_on_device = effective.launch_on_device,
            "skipping local server launch"
        );
        return Ok(None);
    }

    let h264_file = effective
        .h264_file
        .as_ref()
        .context("missing h264_file (config [run] h264_file or --h264-file)")?;

    let bin = args.onvif_binary.clone().unwrap_or_else(|| {
        let cwd = env::current_dir().unwrap_or_else(|_| ".".into());
        default_onvif_binary_path(&cwd)
            .to_string_lossy()
            .to_string()
    });

    info!(%bin, "starting onvif-rust in validation mode");

    let mut cmd = Command::new(&bin);
    cmd.arg("--validation-mode")
        .arg("--h264-file")
        .arg(h264_file)
        .arg("--rtsp-port")
        .arg(effective.rtsp_port.to_string())
        .arg("--httpflv-port")
        .arg(effective.httpflv_port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if args.loop_playback {
        cmd.arg("--loop-playback");
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin))?;
    Ok(Some(child))
}

#[cfg(test)]
mod tests {
    use super::{
        default_onvif_binary_path, device_failure_diagnostics, maybe_launch_server, validate_args,
    };
    use rtsp_validation_tool::config::{EffectiveConfig, parse_args_from};
    use std::{fs, thread, time::Duration};

    #[test]
    fn test_default_onvif_binary_path_prefers_workspace_debug() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let bin = root.join("cross-compile/target/x86_64-unknown-linux-gnu/debug/onvif-rust");
        fs::create_dir_all(bin.parent().unwrap()).unwrap();
        fs::write(&bin, b"stub").unwrap();

        let selected = default_onvif_binary_path(root);
        assert_eq!(selected, bin);
    }

    #[test]
    fn test_default_onvif_binary_path_falls_back_to_legacy_layout() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let legacy = root.join("cross-compile/onvif-rust/target/debug/onvif-rust");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, b"stub").unwrap();

        let selected = default_onvif_binary_path(root);
        assert_eq!(selected, legacy);
    }

    #[test]
    fn test_maybe_launch_server_does_not_create_onvif_stdout_stderr_artifacts() {
        let parsed = parse_args_from([
            "rtsp_validation_tool",
            "--h264-file",
            "dummy.h264",
            "--rtsp-port",
            "8554",
        ])
        .unwrap();
        let mut args = parsed.args;
        args.onvif_binary = Some("/bin/true".to_string());

        let mut effective = EffectiveConfig::from_config_and_args(None, &args, &parsed.sources);
        let dir = tempfile::tempdir().unwrap();
        let artifacts_dir = dir.path().join("artifacts");
        fs::create_dir_all(&artifacts_dir).unwrap();
        effective.artifacts_dir = artifacts_dir.clone();

        let mut child = maybe_launch_server(&args, &effective)
            .unwrap()
            .expect("expected local launch child");
        let _ = child.wait();

        assert!(!artifacts_dir.join("onvif_rust.stdout.log").exists());
        assert!(!artifacts_dir.join("onvif_rust.stderr.log").exists());
    }

    #[test]
    fn test_validate_args_launch_on_device_requires_device_password() {
        let parsed = parse_args_from(["rtsp_validation_tool", "--launch-on-device"]).unwrap();
        let effective = EffectiveConfig::from_config_and_args(None, &parsed.args, &parsed.sources);
        let err = validate_args(&parsed.args, &effective).unwrap_err();
        assert!(
            err.to_string().contains("device SSH password"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_device_failure_diagnostics_reports_missing_logs() {
        let dir = tempfile::tempdir().unwrap();
        let diagnostics = device_failure_diagnostics(dir.path());
        assert!(diagnostics.contains("no copied device onvif logs found"));
    }

    #[test]
    fn test_device_failure_diagnostics_reads_latest_log_tail() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("device_onvif.log.1");
        let second = dir.path().join("device_onvif.log.2");
        fs::write(&first, b"first log").unwrap();
        thread::sleep(Duration::from_millis(25));
        fs::write(&second, b"second log tail").unwrap();

        let diagnostics = device_failure_diagnostics(dir.path());
        assert!(diagnostics.contains("device_onvif.log.2"));
        assert!(diagnostics.contains("second log tail"));
    }
}

async fn wait_for_server(
    host: &str,
    port: u16,
    child: Option<&mut Child>,
    artifacts_dir: &Path,
) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    info!(%addr, "waiting for RTSP server");
    let mut child = child;
    for _attempt in 1..=30u32 {
        if let Some(local_child) = child.as_deref_mut() {
            match local_child.try_wait() {
                Ok(Some(status)) => {
                    let diagnostic = local_exit_diagnostics(artifacts_dir);
                    return Err(anyhow!(
                        "onvif-rust exited before RTSP became ready (status: {}). {}",
                        status,
                        diagnostic
                    ));
                }
                Ok(None) => {}
                Err(e) => {
                    return Err(anyhow!(e)).context("failed to query onvif-rust process status");
                }
            }
        }
        match TcpStream::connect(&addr).await {
            Ok(_) => {
                sleep(Duration::from_millis(200)).await;
                info!(%addr, "RTSP server ready");
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                // In some sandboxed environments, outbound connects are blocked and return EPERM.
                // Fail fast with a clear error so users don't wait ~30s without progress.
                return Err(anyhow!(
                    "connect to {} denied by environment (PermissionDenied). If you are running inside a sandbox, run this tool on the host or allow network access.",
                    addr
                ));
            }
            Err(e) => {
                trace!(error = %e, "RTSP port not ready yet");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
    Err(anyhow!("server did not become ready on {}", addr))
}

#[tokio::main]
async fn main() -> Result<()> {
    let parsed = parse_args()?;
    let args = parsed.args;
    let arg_sources = parsed.sources;
    let config_path = args
        .config
        .clone()
        .or_else(|| env::var("RTSP_VALIDATION_CONFIG").ok());
    let config = load_config(config_path.as_deref())?;
    let log_file_path = init_tracing(config.as_ref())?;

    let mut effective = EffectiveConfig::from_config_and_args(config.as_ref(), &args, &arg_sources);
    effective.resolve_capture_interface();
    validate_args(&args, &effective)?;

    effective.artifacts_dir =
        create_run_artifacts_dir(&effective.artifacts_root_dir).context("create artifacts dir")?;
    effective.output = report_output_path_in_run_dir(&effective.artifacts_dir, &effective.output)
        .to_string_lossy()
        .to_string();
    info!(path = %effective.artifacts_dir.display(), "artifacts directory");

    let run_mode = if effective.launch_on_device {
        "device"
    } else if effective.no_launch {
        "no-launch"
    } else {
        "local"
    };
    let rtsp_url = format!(
        "rtsp://{}:{}{}",
        effective.rtsp_host, effective.rtsp_port, effective.rtsp_stream
    );
    info!(
        run_mode,
        rtsp = %rtsp_url,
        output = %effective.output,
        "RTSP validation run"
    );
    debug!(
        duration_sec = effective.short_duration_sec,
        capture_interface = %effective.capture_interface,
        initial_timestamp_policy = ?effective.initial_timestamp_policy,
        update_baseline = effective.update_baseline,
        compare_baseline = effective.compare_baseline,
        "effective config"
    );

    let mut child =
        maybe_launch_server(&args, &effective).context("failed to launch onvif-rust")?;
    if let Some(local_child) = child.as_mut() {
        wait_for_server(
            &effective.rtsp_host,
            effective.rtsp_port,
            Some(local_child),
            &effective.artifacts_dir,
        )
        .await
        .context("server did not become ready")?;
    }

    if effective.launch_on_device {
        let host = effective.device_host.clone();
        let port = effective.device_ssh_port;
        let user = effective.device_user.clone();
        let password = effective.device_password.clone().unwrap_or_default();
        let rtsp_port = effective.rtsp_port;
        let h264 = effective.device_h264_file.clone();
        let aac = effective.device_aac_file.clone();
        let loop_playback = effective.device_loop_playback;
        let start_result = tokio::task::spawn_blocking(move || {
            device_start_onvif_blocking(
                &host,
                port,
                &user,
                &password,
                rtsp_port,
                DevicePlaybackOptions {
                    h264_file: h264.as_deref(),
                    aac_file: aac.as_deref(),
                    loop_playback,
                },
            )
        })
        .await
        .context("spawn_blocking device start")
        .and_then(|r| r.context("device start onvif-rust"));
        if let Err(e) = start_result {
            cleanup_device_process(&effective).await;
            let diagnostics = device_failure_diagnostics(&effective.artifacts_dir);
            error!(
                error = %e,
                artifacts_dir = %effective.artifacts_dir.display(),
                diagnostics = %diagnostics,
                "device start failed"
            );
            cleanup_local_child(&mut child);
            copy_validator_log(log_file_path.as_deref(), &effective.artifacts_dir);
            return Err(e).context(format!(
                "device start onvif-rust failed (artifacts: {})",
                effective.artifacts_dir.display()
            ));
        }
        sleep(Duration::from_secs(2)).await;

        match wait_for_server(
            &effective.rtsp_host,
            effective.rtsp_port,
            None,
            &effective.artifacts_dir,
        )
        .await
        {
            Ok(()) => {}
            Err(e) => {
                // If the RTSP port never opens, the most common causes are:
                // - onvif-rust exited immediately (missing H.264/AAC, loader errors, bad args)
                // - RTSP bind failed (permissions, port already in use)
                //
                // Pull the on-device logs into artifacts to make this actionable.
                let host = effective.device_host.clone();
                let port = effective.device_ssh_port;
                let user = effective.device_user.clone();
                let password = effective.device_password.clone().unwrap_or_default();
                let pid = tokio::task::spawn_blocking(move || {
                    device_get_onvif_pid_blocking(&host, port, &user, &password)
                })
                .await
                .context("spawn_blocking device pid check")?
                .context("device pid check")?;
                warn!(?pid, "device onvif-rust pid check after RTSP wait failure");

                let host = effective.device_host.clone();
                let port = effective.device_ssh_port;
                let user = effective.device_user.clone();
                let password = effective.device_password.clone().unwrap_or_default();
                let artifacts_dir = effective.artifacts_dir.clone();
                match tokio::task::spawn_blocking(move || {
                    device_copy_onvif_logs_blocking(&host, port, &user, &password, &artifacts_dir)
                })
                .await
                {
                    Ok(Ok(())) => {
                        info!(path = %effective.artifacts_dir.display(), "copied device onvif logs for diagnosis")
                    }
                    Ok(Err(copy_err)) => {
                        warn!(error = %copy_err, "failed to copy device onvif logs for diagnosis")
                    }
                    Err(join_err) => {
                        warn!(error = %join_err, "spawn_blocking device log copy failed")
                    }
                }

                cleanup_device_process(&effective).await;
                let diagnostics = device_failure_diagnostics(&effective.artifacts_dir);
                error!(
                    error = %e,
                    artifacts_dir = %effective.artifacts_dir.display(),
                    diagnostics = %diagnostics,
                    "device RTSP readiness failed"
                );

                return Err(e).context(
                    "device RTSP server did not become ready (see artifacts dir for device logs)",
                );
            }
        }
    }

    let run_validation_and_harness = async {
        let mut report = run_validation(&args, &effective).await?;
        if effective.launch_on_device && effective.collect_telemetry {
            let host = effective.device_host.clone();
            let port = effective.device_ssh_port;
            let user = effective.device_user.clone();
            let password = effective.device_password.clone().unwrap_or_default();
            let telemetry = tokio::task::spawn_blocking(move || {
                device_collect_telemetry_blocking(&host, port, &user, &password)
            })
            .await
            .context("spawn_blocking telemetry")?;
            report.telemetry = Some(telemetry);
        }
        if critical_proto_failed(&report.tests) {
            report.tests.push(TestResult::fail(
                "harness_skipped",
                "protocol validation failed (describe/setup/play); skipping harness",
            ));
        } else {
            run_harness(&args, &effective, &mut report.tests).await?;
        }
        Ok::<_, anyhow::Error>(report)
    };

    let run_result: Result<ValidationReport> = tokio::select! {
        biased;
        res = run_validation_and_harness => res,
        _ = wait_for_signal() => {
            Err(anyhow!("interrupted by signal (Ctrl-C or SIGTERM)"))
        }
    };
    let mut report = match run_result {
        Ok(report) => report,
        Err(e) => {
            cleanup_device_process(&effective).await;
            cleanup_local_child(&mut child);
            copy_validator_log(log_file_path.as_deref(), &effective.artifacts_dir);
            return Err(e);
        }
    };
    report.artifacts_dir = Some(effective.artifacts_dir.to_string_lossy().to_string());

    if effective.launch_on_device {
        if effective.collect_telemetry {
            let host = effective.device_host.clone();
            let port = effective.device_ssh_port;
            let user = effective.device_user.clone();
            let password = effective.device_password.clone().unwrap_or_default();
            match tokio::task::spawn_blocking(move || {
                device_collect_telemetry_blocking(&host, port, &user, &password)
            })
            .await
            {
                Ok(telemetry) => report.telemetry_before_shutdown = Some(telemetry),
                Err(e) => warn!(error = %e, "spawn_blocking telemetry before shutdown failed"),
            }
        }

        cleanup_device_process(&effective).await;
    }

    if effective.update_baseline || effective.compare_baseline {
        apply_baseline_ops(&args, &effective, &mut report)?;
    }

    cleanup_local_child(&mut child);

    report.summary = compute_summary(&report.tests);

    if report.summary.failed > 0 {
        for t in &report.tests {
            if let TestResult::Fail { name, reason, .. } = t {
                warn!(test = %name, reason = %reason, "test failed");
            }
        }
    }

    let json = serde_json::to_string_pretty(&report).context("failed to serialize JSON report")?;
    std::fs::write(&effective.output, json)
        .with_context(|| format!("failed to write {}", effective.output))?;
    info!(path = %effective.output, "RTSP validation report written");
    copy_validator_log(log_file_path.as_deref(), &effective.artifacts_dir);
    if !report.summary.overall_pass {
        return Err(anyhow!("{} test(s) failed", report.summary.failed));
    }
    Ok(())
}
