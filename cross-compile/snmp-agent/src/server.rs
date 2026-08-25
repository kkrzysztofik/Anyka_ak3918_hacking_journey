//! UDP SNMPv2c agent loop.

use crate::config::{DEFAULT_CONFIG_PATH, SnmpConfig};
use crate::mib::{self, MibSources};
use crate::pdu::{Pdu, PduType, SNMP_V2C_VERSION, SnmpMessage};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::net::UdpSocket;
use tokio::signal::unix::{SignalKind, signal};
use tokio::sync::RwLock;

/// Default pidfile path for SIGHUP from onvif-rust.
pub const DEFAULT_PIDFILE: &str = "/tmp/snmp-agent.pid";

/// Live MIB sources backed by config + process start time.
pub struct LiveSources {
    pub config: SnmpConfig,
    started: Instant,
}

impl LiveSources {
    pub fn new(config: SnmpConfig) -> Self {
        Self {
            config,
            started: Instant::now(),
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

/// Run the agent until cancelled. Reloads config on SIGHUP.
pub async fn run(
    config_path: PathBuf,
    pidfile: PathBuf,
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

    let mut sighup = signal(SignalKind::hangup())?;
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
        tokio::select! {
            _ = sighup.recv() => {
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
    }

    impl MibSources for Fixed {
        fn uptime_ticks(&self) -> u32 {
            self.ticks
        }
        fn config(&self) -> &SnmpConfig {
            &self.cfg
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
        let sources = Fixed { cfg, ticks: 1 };
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
        let sources = Fixed { cfg, ticks: 1 };
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
}
