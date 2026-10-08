use snmp_agent::config::DEFAULT_CONFIG_PATH;
use snmp_agent::server::{self, DEFAULT_PIDFILE};
use std::path::PathBuf;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Parse TZ once. glibc caches the zone and will not re-read a changed TZ
    // on its own; the supervisor's zone never changes after boot.
    // Declared locally: the libc crate exposes tzset only on Windows.
    // SAFETY: tzset reads the process env and updates libc's own state.
    unsafe extern "C" {
        fn tzset();
    }
    unsafe { tzset() };

    tracing_subscriber::fmt()
        .with_timer(anyka_init::logging::LocalTimer)
        .with_ansi(false)
        .init();
    let config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
    tracing::info!(?config_path, "snmp-agent starting");

    let (tx, rx) = tokio::sync::mpsc::channel(1);
    match signal(SignalKind::hangup()) {
        Ok(mut sighup) => {
            tokio::spawn(async move {
                while sighup.recv().await.is_some() {
                    // Depth 1: a reload already queued subsumes this one.
                    let _ = tx.try_send(());
                }
            });
        }
        Err(e) => tracing::error!(error = %e, "SIGHUP unavailable; config reload disabled"),
    }

    if let Err(e) = server::run(config_path, PathBuf::from(DEFAULT_PIDFILE), rx).await {
        tracing::error!(error = %e, "snmp-agent exited");
        std::process::exit(1);
    }
}
