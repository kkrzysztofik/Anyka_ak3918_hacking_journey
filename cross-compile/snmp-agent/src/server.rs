//! UDP SNMPv2c agent loop.

use crate::config::{DEFAULT_CONFIG_PATH, SnmpConfig};
use crate::mib::{self, MibSources};
use crate::pdu::{Pdu, PduType, SNMP_V2C_VERSION, SnmpMessage};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

/// Default pidfile path for SIGHUP from onvif-rust.
pub const DEFAULT_PIDFILE: &str = "/tmp/snmp-agent.pid";

/// How long to wait before retrying a bind that failed.
// ponytail: fixed interval, not exponential. Move to backoff only if a real
// deployment shows the retries themselves costing anything.
#[cfg(not(test))]
const BIND_RETRY: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(test)]
const BIND_RETRY: std::time::Duration = std::time::Duration::from_millis(50);

/// Live MIB sources backed by config + process start time.
pub struct LiveSources {
    pub config: SnmpConfig,
    started: Instant,
    ifaces: Vec<crate::mib::interfaces::IfRow>,
}

impl LiveSources {
    pub fn new(config: SnmpConfig) -> Self {
        Self {
            config,
            started: Instant::now(),
            ifaces: crate::mib::interfaces::load_interfaces(
                Path::new("/proc/net/dev"),
                Path::new("/sys/class/net"),
            ),
        }
    }
}

impl MibSources for LiveSources {
    fn uptime_ticks(&self) -> u32 {
        let elapsed = self.started.elapsed();
        let ticks = elapsed.as_secs() * 100 + u64::from(elapsed.subsec_millis()) / 10;
        ticks.min(u64::from(u32::MAX)) as u32
    }

    fn config(&self) -> &SnmpConfig {
        &self.config
    }

    fn interfaces(&self) -> &[crate::mib::interfaces::IfRow] {
        &self.ifaces
    }
}

/// Process one inbound datagram. Returns response bytes, or `None` to silent-drop.
pub fn handle_datagram(bytes: &[u8], sources: &dyn MibSources) -> Option<Vec<u8>> {
    let msg = match SnmpMessage::parse(bytes) {
        Ok(m) => m,
        Err(_) => return None,
    };

    if msg.community != sources.config().community {
        // Wrong community: silent drop (no scanner oracle).
        return None;
    }

    let (error_status, error_index, variable_bindings) =
        mib::handle_varbinds(msg.pdu.pdu_type, &msg.pdu.variable_bindings, sources);

    let response = SnmpMessage {
        version: SNMP_V2C_VERSION,
        community: msg.community,
        pdu: Pdu {
            pdu_type: PduType::GetResponse,
            request_id: msg.pdu.request_id,
            error_status,
            error_index,
            variable_bindings,
        },
    };
    response.encode().ok()
}

fn write_pidfile(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, format!("{}\n", std::process::id()))
}

fn remove_pidfile(path: &Path) {
    let _ = std::fs::remove_file(path);
}

async fn bind_socket(port: u16) -> std::io::Result<UdpSocket> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    UdpSocket::bind(addr).await
}

/// Run the agent until cancelled. Reloads config when `reload` yields.
pub async fn run(
    config_path: PathBuf,
    pidfile: PathBuf,
    mut reload: tokio::sync::mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let initial = SnmpConfig::load(&config_path)?;
    let state = Arc::new(RwLock::new(LiveSources::new(initial)));

    write_pidfile(&pidfile)?;
    struct PidGuard(PathBuf);
    impl Drop for PidGuard {
        fn drop(&mut self) {
            remove_pidfile(&self.0);
        }
    }
    let _pid_guard = PidGuard(pidfile);

    let mut socket: Option<UdpSocket> = None;

    {
        let cfg = state.read().await.config.clone();
        if cfg.enabled {
            match bind_socket(cfg.port).await {
                Ok(s) => {
                    tracing::info!(port = cfg.port, "snmp-agent listening");
                    socket = Some(s);
                }
                Err(e) => {
                    tracing::error!(error = %e, port = cfg.port, "bind failed; retrying on SIGHUP");
                }
            }
        } else {
            tracing::info!("snmp-agent disabled (unbound); waiting for SIGHUP");
        }
    }

    let mut buf = [0u8; 2048];
    loop {
        let (enabled, port) = {
            let g = state.read().await;
            (g.config.enabled, g.config.port)
        };
        tokio::select! {
            _ = reload.recv() => {
                match SnmpConfig::load(&config_path) {
                    Ok(new_cfg) => {
                        let mut guard = state.write().await;
                        let old_port = guard.config.port;
                        let old_enabled = guard.config.enabled;
                        guard.config = new_cfg.clone();
                        drop(guard);

                        if !new_cfg.enabled {
                            socket = None;
                            tracing::info!("snmp-agent disabled after reload");
                            continue;
                        }

                        let need_rebind = socket.is_none()
                            || !old_enabled
                            || old_port != new_cfg.port;
                        if need_rebind {
                            match bind_socket(new_cfg.port).await {
                                Ok(s) => {
                                    tracing::info!(port = new_cfg.port, "snmp-agent rebound");
                                    socket = Some(s);
                                }
                                Err(e) => {
                                    tracing::error!(
                                        error = %e,
                                        port = new_cfg.port,
                                        "rebind failed; keeping previous socket"
                                    );
                                }
                            }
                        } else {
                            tracing::info!("snmp-agent config reloaded (same bind)");
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "config reload failed; keeping last-good");
                    }
                }
            }
            _ = tokio::time::sleep(BIND_RETRY), if socket.is_none() && enabled => {
                match bind_socket(port).await {
                    Ok(s) => {
                        tracing::info!(port, "snmp-agent bound on retry");
                        socket = Some(s);
                    }
                    Err(e) => tracing::debug!(error = %e, port, "bind retry failed"),
                }
            }
            result = async {
                match socket.as_ref() {
                    Some(sock) => sock.recv_from(&mut buf).await,
                    None => {
                        std::future::pending::<std::io::Result<(usize, SocketAddr)>>().await
                    }
                }
            } => {
                match result {
                    Ok((n, peer)) => {
                        let sources = state.read().await;
                        if let Some(resp) = handle_datagram(&buf[..n], &*sources)
                            && let Some(sock) = socket.as_ref()
                            && let Err(e) = sock.send_to(&resp, peer).await
                        {
                            tracing::debug!(error = %e, "send_to failed");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "recv_from failed");
                    }
                }
            }
        }
    }
}

/// Parse CLI args: optional `--config PATH`.
pub fn parse_args(args: impl IntoIterator<Item = String>) -> PathBuf {
    let mut config = PathBuf::from(DEFAULT_CONFIG_PATH);
    let mut iter = args.into_iter();
    let _exe = iter.next();
    while let Some(arg) = iter.next() {
        if arg == "--config"
            && let Some(path) = iter.next()
        {
            config = PathBuf::from(path);
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ber::Oid;
    use crate::pdu::{SnmpValue, VarBind};
    use std::time::Duration;

    struct Fixed {
        cfg: SnmpConfig,
        ticks: u32,
        ifaces: Vec<crate::mib::interfaces::IfRow>,
    }

    impl MibSources for Fixed {
        fn uptime_ticks(&self) -> u32 {
            self.ticks
        }
        fn config(&self) -> &SnmpConfig {
            &self.cfg
        }
        fn interfaces(&self) -> &[crate::mib::interfaces::IfRow] {
            &self.ifaces
        }
    }

    fn get_sysname_bytes(community: &str) -> Vec<u8> {
        let msg = SnmpMessage {
            version: SNMP_V2C_VERSION,
            community: community.to_string(),
            pdu: Pdu {
                pdu_type: PduType::GetRequest,
                request_id: 7,
                error_status: 0,
                error_index: 0,
                variable_bindings: vec![VarBind {
                    name: Oid::from_slice(&[1, 3, 6, 1, 2, 1, 1, 5, 0]).unwrap(),
                    value: SnmpValue::Null,
                }],
            },
        };
        msg.encode().unwrap()
    }

    #[test]
    fn test_handle_datagram_wrong_community_silent_drop() {
        let cfg = SnmpConfig {
            community: "public".into(),
            sys_name: "cam-1".into(),
            ..Default::default()
        };
        let sources = Fixed {
            cfg,
            ticks: 1,
            ifaces: Vec::new(),
        };
        let req = get_sysname_bytes("wrong");
        assert!(handle_datagram(&req, &sources).is_none());
    }

    #[test]
    fn test_handle_datagram_returns_sysname() {
        let cfg = SnmpConfig {
            community: "public".into(),
            sys_name: "cam-1".into(),
            ..Default::default()
        };
        let sources = Fixed {
            cfg,
            ticks: 1,
            ifaces: Vec::new(),
        };
        let req = get_sysname_bytes("public");
        let resp = handle_datagram(&req, &sources).expect("response");
        let msg = SnmpMessage::parse(&resp).unwrap();
        assert_eq!(msg.pdu.pdu_type, PduType::GetResponse);
        assert_eq!(msg.pdu.request_id, 7);
        assert_eq!(msg.pdu.error_status, 0);
        assert_eq!(
            msg.pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"cam-1".to_vec())
        );
    }

    #[test]
    fn test_handle_datagram_bad_pdu_drop() {
        let sources = Fixed {
            cfg: SnmpConfig::default(),
            ticks: 1,
            ifaces: Vec::new(),
        };
        assert!(handle_datagram(&[0xff, 0x00], &sources).is_none());
    }

    #[test]
    fn test_parse_args_config_flag() {
        let path = parse_args(vec![
            "snmp-agent".into(),
            "--config".into(),
            "/tmp/x.toml".into(),
        ]);
        assert_eq!(path, PathBuf::from("/tmp/x.toml"));
    }

    #[tokio::test]
    async fn test_udp_get_sysname_ephemeral() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        std::fs::write(
            &cfg_path,
            r#"
enabled = true
port = 0
community = "public"
sys_name = "udp-cam"
"#,
        )
        .unwrap();
        // port 0 is rejected by SnmpConfig::load — use a free high port via bind(0) helper path.
        // Instead: bind ourselves, put that port in config.
        let probe = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        std::fs::write(
            &cfg_path,
            format!(
                "enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"udp-cam\"\n"
            ),
        )
        .unwrap();

        let cfg = SnmpConfig::load(&cfg_path).unwrap();
        let sources = LiveSources::new(cfg.clone());
        let server = UdpSocket::bind(("127.0.0.1", port)).await.unwrap();

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let req = get_sysname_bytes("public");
        client.send_to(&req, ("127.0.0.1", port)).await.unwrap();

        let mut buf = [0u8; 2048];
        let (n, peer) = tokio::time::timeout(Duration::from_secs(2), server.recv_from(&mut buf))
            .await
            .expect("recv timeout")
            .unwrap();
        let resp = handle_datagram(&buf[..n], &sources).expect("handled");
        server.send_to(&resp, peer).await.unwrap();

        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("client timeout")
            .unwrap();
        let msg = SnmpMessage::parse(&buf[..n]).unwrap();
        assert_eq!(
            msg.pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"udp-cam".to_vec())
        );
    }

    #[test]
    fn test_live_sources_uptime_and_config() {
        let cfg = SnmpConfig {
            sys_name: "live".into(),
            ..Default::default()
        };
        let sources = LiveSources::new(cfg);
        assert_eq!(sources.config().sys_name, "live");
        std::thread::sleep(Duration::from_millis(15));
        assert!(sources.uptime_ticks() >= 1);
    }

    #[test]
    fn test_write_and_remove_pidfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("agent.pid");
        write_pidfile(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.trim(), std::process::id().to_string());
        remove_pidfile(&path);
        assert!(!path.exists());
    }

    async fn wait_for_file(path: &Path, timeout: Duration) {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if path.exists() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        panic!("timed out waiting for {}", path.display());
    }

    

    #[tokio::test]
    async fn test_run_serves_udp_get_reload_and_disable() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        let pidfile = dir.path().join("snmp-agent.pid");

        let probe = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        std::fs::write(
            &cfg_path,
            format!(
                "enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"run-cam\"\n"
            ),
        )
        .unwrap();

        let run_cfg = cfg_path.clone();
        let run_pid = pidfile.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move {
            let _ = run(run_cfg, run_pid, rx).await;
        });

        wait_for_file(&pidfile, Duration::from_secs(2)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let req = get_sysname_bytes("public");
        client.send_to(&req, ("127.0.0.1", port)).await.unwrap();
        let mut buf = [0u8; 2048];
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("agent response timeout")
            .unwrap();
        let msg = SnmpMessage::parse(&buf[..n]).unwrap();
        assert_eq!(
            msg.pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"run-cam".to_vec())
        );

        // Same-bind reload (enabled + port unchanged).
        std::fs::write(
            &cfg_path,
            format!(
                "enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"run-cam2\"\n"
            ),
        )
        .unwrap();
        tx.send(()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        client.send_to(&req, ("127.0.0.1", port)).await.unwrap();
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("same-bind response timeout")
            .unwrap();
        assert_eq!(
            SnmpMessage::parse(&buf[..n]).unwrap().pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"run-cam2".to_vec())
        );

        // Bad config on reload keeps last-good.
        std::fs::write(&cfg_path, "port = \"nope\"\n").unwrap();
        tx.send(()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Disable on reload.
        std::fs::write(
            &cfg_path,
            format!(
                "enabled = false\nport = {port}\ncommunity = \"public\"\nsys_name = \"run-cam2\"\n"
            ),
        )
        .unwrap();
        tx.send(()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;

        // Re-enable (rebind after socket cleared).
        std::fs::write(
            &cfg_path,
            format!(
                "enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"run-cam3\"\n"
            ),
        )
        .unwrap();
        tx.send(()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        client.send_to(&req, ("127.0.0.1", port)).await.unwrap();
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("rebind response timeout")
            .unwrap();
        assert_eq!(
            SnmpMessage::parse(&buf[..n]).unwrap().pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"run-cam3".to_vec())
        );

        handle.abort();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_run_starts_disabled_and_bind_failure_is_non_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        let pidfile = dir.path().join("disabled.pid");

        std::fs::write(
            &cfg_path,
            "enabled = false\nport = 161\ncommunity = \"public\"\n",
        )
        .unwrap();
        let run_cfg = cfg_path.clone();
        let run_pid = pidfile.clone();
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move {
            let _ = run(run_cfg, run_pid, rx).await;
        });
        wait_for_file(&pidfile, Duration::from_secs(2)).await;
        handle.abort();
        let _ = handle.await;

        // Bind failure: hold the port, then start agent on it.
        let holder = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let port = holder.local_addr().unwrap().port();
        let cfg_path = dir.path().join("bindfail.toml");
        let pidfile = dir.path().join("bindfail.pid");
        std::fs::write(
            &cfg_path,
            format!("enabled = true\nport = {port}\ncommunity = \"public\"\n"),
        )
        .unwrap();
        let run_cfg = cfg_path.clone();
        let run_pid = pidfile.clone();
        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move {
            let _ = run(run_cfg, run_pid, rx).await;
        });
        wait_for_file(&pidfile, Duration::from_secs(2)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.abort();
        let _ = handle.await;
        drop(holder);
    }

    #[tokio::test]
    async fn test_bind_retry_recovers_after_the_port_frees() {
        // Uses the #[cfg(test)] BIND_RETRY (50ms): start_paused does not mix
        // cleanly with real UDP sockets.
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        let pidfile = dir.path().join("retry.pid");

        let holder = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = holder.local_addr().unwrap().port();
        std::fs::write(
            &cfg_path,
            format!("enabled = true\nport = {port}\ncommunity = \"public\"\nsys_name = \"retry\"\n"),
        )
        .unwrap();

        let (_tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn(async move {
            let _ = run(cfg_path, pidfile, rx).await;
        });

        tokio::time::sleep(Duration::from_millis(50)).await; // initial bind fails
        drop(holder);
        tokio::time::sleep(BIND_RETRY + Duration::from_millis(100)).await;

        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client
            .send_to(&get_sysname_bytes("public"), ("127.0.0.1", port))
            .await
            .unwrap();
        let mut buf = [0u8; 2048];
        let (n, _) = tokio::time::timeout(Duration::from_secs(5), client.recv_from(&mut buf))
            .await
            .expect("agent must answer after the retry")
            .unwrap();
        assert!(SnmpMessage::parse(&buf[..n]).is_ok());

        handle.abort();
        let _ = handle.await;
    }
}
