//! RTSP validation tool binary: orchestration and main entrypoint.

use anyhow::{Context, Result, anyhow, bail};
use clap::Parser;
use rtsp_validation_tool::baseline::apply_baseline_ops;
use rtsp_validation_tool::config::{Args, EffectiveConfig, RtspValidationConfig, load_config};
use rtsp_validation_tool::device::{
    device_collect_telemetry_blocking, device_copy_onvif_logs_blocking,
    device_start_onvif_blocking, device_stop_onvif_blocking,
};
use rtsp_validation_tool::report::{TestResult, ValidationReport, compute_summary};
use rtsp_validation_tool::rtsp::{critical_proto_failed, run_harness, run_validation};
use rtsp_validation_tool::util::{report_output_path_in_run_dir, run_artifacts_dir_name};
use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tracing::{debug, info, trace, warn};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn accept_already_initialized(e: impl std::error::Error + Send + Sync + 'static) -> Result<()> {
    let msg = e.to_string();
    if msg.contains("already") || msg.contains("set a logger") {
        Ok(())
    } else {
        Err(anyhow::Error::from(e))
    }
}

fn init_tracing(config: Option<&RtspValidationConfig>) -> Result<()> {
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

    let log_file_path: Option<String> = if let Some(c) = config {
        if !c.logging.file.is_empty() {
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&c.logging.file)
            {
                Ok(file) => {
                    let path = c.logging.file.clone();
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
                    Some(path)
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
        info!(path = %path, "logging to file");
    }
    Ok(())
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

async fn cleanup_on_signal(
    launch_on_device: bool,
    device_host: &str,
    device_telnet_port: u16,
    artifacts_dir: &Path,
    child: &mut Option<Child>,
) {
    info!("interrupted by signal, cleaning up");
    if launch_on_device {
        let host = device_host.to_string();
        let port = device_telnet_port;
        let artifacts_dir = artifacts_dir.to_path_buf();
        let host_for_stop = host.clone();
        match tokio::task::spawn_blocking(move || device_stop_onvif_blocking(&host_for_stop, port))
            .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!(error = %e, "device stop onvif-rust during cleanup"),
            Err(e) => warn!(error = %e, "spawn_blocking device stop during cleanup"),
        }
        match tokio::task::spawn_blocking(move || {
            device_copy_onvif_logs_blocking(&host, port, &artifacts_dir)
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!(error = %e, "device log copy during cleanup"),
            Err(e) => warn!(error = %e, "spawn_blocking device log copy during cleanup"),
        }
    }
    if let Some(c) = child {
        let _ = c.kill();
        let _ = c.wait();
        debug!("local server process terminated");
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

    if !args.rtsp_stream.starts_with('/') {
        bail!(
            "--rtsp-stream must start with '/' (got {})",
            args.rtsp_stream
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
        if cwd.to_string_lossy().ends_with("onvif-rust") {
            format!("{}/target/debug/onvif-rust", cwd.display())
        } else {
            format!(
                "{}/cross-compile/onvif-rust/target/debug/onvif-rust",
                cwd.display()
            )
        }
    });

    info!(%bin, "starting onvif-rust in validation mode");

    let mut cmd = Command::new(&bin);
    cmd.arg("--validation-mode")
        .arg("--h264-file")
        .arg(h264_file)
        .arg("--rtsp-port")
        .arg(args.rtsp_port.to_string())
        .arg("--httpflv-port")
        .arg(args.httpflv_port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if args.loop_playback {
        cmd.arg("--loop-playback");
    }
    if effective.capture_tool_output {
        let stdout_path = effective.artifacts_dir.join("onvif_rust.stdout.log");
        let stderr_path = effective.artifacts_dir.join("onvif_rust.stderr.log");
        let stdout = File::create(&stdout_path)
            .with_context(|| format!("create {}", stdout_path.display()))?;
        let stderr = File::create(&stderr_path)
            .with_context(|| format!("create {}", stderr_path.display()))?;
        cmd.stdout(Stdio::from(stdout));
        cmd.stderr(Stdio::from(stderr));
        info!(
            stdout_log = %stdout_path.display(),
            stderr_log = %stderr_path.display(),
            "capturing onvif-rust output"
        );
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn {}", bin))?;
    Ok(Some(child))
}

async fn wait_for_server(host: &str, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port);
    info!(%addr, "waiting for RTSP server");
    for _attempt in 1..=30u32 {
        match TcpStream::connect(&addr).await {
            Ok(_) => {
                sleep(Duration::from_millis(200)).await;
                info!(%addr, "RTSP server ready");
                return Ok(());
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
    let args = Args::parse();
    let config_path = args
        .config
        .clone()
        .or_else(|| env::var("RTSP_VALIDATION_CONFIG").ok());
    let config = load_config(config_path.as_deref())?;
    init_tracing(config.as_ref())?;

    let mut effective = EffectiveConfig::from_config_and_args(config.as_ref(), &args);
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
        update_baseline = effective.update_baseline,
        compare_baseline = effective.compare_baseline,
        "effective config"
    );

    let mut child =
        maybe_launch_server(&args, &effective).context("failed to launch onvif-rust")?;
    if child.is_some() {
        wait_for_server(&effective.rtsp_host, effective.rtsp_port)
            .await
            .context("server did not become ready")?;
    }

    if effective.launch_on_device {
        let host = effective.device_host.clone();
        let port = effective.device_telnet_port;
        let rtsp_port = effective.rtsp_port;
        let h264 = effective.device_h264_file.clone();
        let aac = effective.device_aac_file.clone();
        let loop_playback = effective.device_loop_playback;
        tokio::task::spawn_blocking(move || {
            device_start_onvif_blocking(
                &host,
                port,
                rtsp_port,
                h264.as_deref(),
                aac.as_deref(),
                loop_playback,
            )
        })
        .await
        .context("spawn_blocking device start")?
        .context("device start onvif-rust")?;
        sleep(Duration::from_secs(2)).await;
        wait_for_server(&effective.rtsp_host, effective.rtsp_port)
            .await
            .context("device RTSP server did not become ready")?;
    }

    let run_validation_and_harness = async {
        let mut report = run_validation(&args, &effective).await?;
        if effective.launch_on_device && effective.collect_telemetry {
            let host = effective.device_host.clone();
            let port = effective.device_telnet_port;
            let telemetry =
                tokio::task::spawn_blocking(move || device_collect_telemetry_blocking(&host, port))
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

    let mut report: ValidationReport = tokio::select! {
        biased;
        res = run_validation_and_harness => res?,
        _ = wait_for_signal() => {
            cleanup_on_signal(
                effective.launch_on_device,
                &effective.device_host,
                effective.device_telnet_port,
                &effective.artifacts_dir,
                &mut child,
            )
            .await;
            return Err(anyhow!("interrupted by signal (Ctrl-C or SIGTERM)"));
        }
    };
    report.artifacts_dir = Some(effective.artifacts_dir.to_string_lossy().to_string());

    if effective.launch_on_device {
        let host = effective.device_host.clone();
        let port = effective.device_telnet_port;
        let stop_result =
            tokio::task::spawn_blocking(move || device_stop_onvif_blocking(&host, port))
                .await
                .context("spawn_blocking device stop");
        if let Err(e) = stop_result.and_then(|r| r) {
            warn!(error = %e, "device stop onvif-rust failed");
        }

        let host = effective.device_host.clone();
        let port = effective.device_telnet_port;
        let artifacts_dir = effective.artifacts_dir.clone();
        match tokio::task::spawn_blocking(move || {
            device_copy_onvif_logs_blocking(&host, port, &artifacts_dir)
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(e)) => warn!(error = %e, "device onvif log copy failed"),
            Err(e) => warn!(error = %e, "spawn_blocking device onvif log copy failed"),
        }
    }

    if effective.update_baseline || effective.compare_baseline {
        apply_baseline_ops(&args, &effective, &mut report)?;
    }

    if let Some(ref mut c) = child {
        let _ = c.kill();
        let _ = c.wait();
    }

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
    if !report.summary.overall_pass {
        return Err(anyhow!("{} test(s) failed", report.summary.failed));
    }
    Ok(())
}
