use snmp_agent::server::{self, DEFAULT_PIDFILE};
use std::path::PathBuf;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config_path = server::parse_args(std::env::args());
    tracing::info!(?config_path, "snmp-agent starting");
    if let Err(e) = server::run(config_path, PathBuf::from(DEFAULT_PIDFILE)).await {
        tracing::error!(error = %e, "snmp-agent exited");
        std::process::exit(1);
    }
}
