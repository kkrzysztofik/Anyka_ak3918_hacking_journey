use snmp_agent::server::{self, DEFAULT_PIDFILE};
use std::path::PathBuf;
use tokio::signal::unix::{SignalKind, signal};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    tracing_subscriber::fmt::init();
    let config_path = server::parse_args(std::env::args());
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
