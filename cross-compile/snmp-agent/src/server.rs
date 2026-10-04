//! UDP SNMPv2c agent loop.

use crate::config::SnmpConfig;
use crate::mib::{self, Snapshot, interfaces};
use crate::pdu::{Pdu, PduType, SnmpMessage};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::net::UdpSocket;

/// Default pidfile path for SIGHUP from onvif-rust.
pub const DEFAULT_PIDFILE: &str = "/tmp/snmp-agent.pid";

/// How long to wait before retrying a bind that failed.
// ponytail: fixed interval, not exponential. Move to backoff only if a real
// deployment shows the retries themselves costing anything.
#[cfg(not(test))]
const BIND_RETRY: std::time::Duration = std::time::Duration::from_secs(30);
#[cfg(test)]
const BIND_RETRY: std::time::Duration = std::time::Duration::from_millis(50);

/// Owns the config and the filesystem roots the MIB is built from.
pub struct Agent {
    pub config: SnmpConfig,
    proc_root: PathBuf,
    sys_class_net: PathBuf,
}

impl Agent {
    pub fn new(config: SnmpConfig) -> Self {
        Self::with_roots(
            config,
            PathBuf::from("/proc"),
            PathBuf::from("/sys/class/net"),
        )
    }

    pub fn with_roots(config: SnmpConfig, proc_root: PathBuf, sys_class_net: PathBuf) -> Self {
        Self {
            config,
            proc_root,
            sys_class_net,
        }
    }

    /// System uptime in hundredths of a second.
    ///
    /// `/proc/uptime`, not process uptime: anyka-init restarts this binary on
    /// crash, and an NMS reads a sysUpTime reset as a device reboot.
    fn uptime_ticks(&self) -> u32 {
        proc_uptime_ticks(&self.proc_root.join("uptime")).unwrap_or(0)
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            config: self.config.clone(),
            uptime_ticks: self.uptime_ticks(),
            ifaces: interfaces::load_interfaces(
                &self.proc_root.join("net/dev"),
                &self.sys_class_net,
            ),
        }
    }
}

fn proc_uptime_ticks(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    let secs: f64 = text.split_whitespace().next()?.parse().ok()?;
    if !secs.is_finite() || secs < 0.0 {
        return None;
    }
    Some((secs * 100.0).min(f64::from(u32::MAX)) as u32)
}

/// Process one inbound datagram. Returns response bytes, or `None` to silent-drop.
pub fn handle_datagram(bytes: &[u8], agent: &Agent) -> Option<Vec<u8>> {
    let msg = SnmpMessage::parse(bytes).ok()?;

    // Only ever answer requests. Answering a GetResponse lets one packet with a
    // spoofed source address make the agent respond to itself forever.
    if !matches!(
        msg.pdu.pdu_type,
        PduType::GetRequest
            | PduType::GetNextRequest
            | PduType::GetBulkRequest
            | PduType::SetRequest
    ) {
        return None;
    }

    if msg.community != agent.config.community {
        // Wrong community: silent drop (no scanner oracle), and no /proc read.
        return None;
    }

    let (error_status, error_index, variable_bindings) = match msg.pdu.pdu_type {
        // Refused before any varbind is resolved, so /proc and sysfs are never read for it.
        PduType::SetRequest => (mib::ERR_NOT_WRITABLE, 1, msg.pdu.variable_bindings),
        PduType::GetBulkRequest => mib::handle_getbulk(
            msg.pdu.error_status,
            msg.pdu.error_index,
            &msg.pdu.variable_bindings,
            &agent.snapshot(),
        ),
        t => mib::handle_varbinds(t, &msg.pdu.variable_bindings, &agent.snapshot()),
    };

    SnmpMessage {
        community: msg.community,
        pdu: Pdu {
            pdu_type: PduType::GetResponse,
            request_id: msg.pdu.request_id,
            error_status,
            error_index,
            variable_bindings,
        },
    }
    .encode()
    .ok()
}

async fn bind_socket(port: u16) -> std::io::Result<UdpSocket> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    UdpSocket::bind(addr).await
}

/// Apply a freshly loaded config. A new port is bound before `agent.config` is
/// replaced, so a failed rebind keeps serving on the old one.
async fn apply_reload(agent: &mut Agent, socket: &mut Option<UdpSocket>, new_cfg: SnmpConfig) {
    if socket.is_some() && agent.config.port == new_cfg.port {
        agent.config = new_cfg;
        tracing::info!("snmp-agent config reloaded (same bind)");
        return;
    }
    match bind_socket(new_cfg.port).await {
        Ok(s) => {
            tracing::info!(port = new_cfg.port, "snmp-agent rebound");
            *socket = Some(s);
            agent.config = new_cfg;
        }
        Err(e) => {
            tracing::error!(
                error = %e,
                port = new_cfg.port,
                "rebind failed; keeping previous socket when bound"
            );
            // No live socket: commit the requested config so BIND_RETRY
            // keeps trying the new port instead of stale last-good values.
            if socket.is_none() {
                agent.config = new_cfg;
            }
        }
    }
}

/// Run the agent until cancelled. Reloads config when `reload` yields.
pub async fn run(
    config_path: PathBuf,
    pidfile: PathBuf,
    mut reload: tokio::sync::mpsc::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut agent = Agent::new(SnmpConfig::load(&config_path)?);

    std::fs::write(&pidfile, format!("{}\n", std::process::id()))?;
    struct PidGuard(PathBuf);
    impl Drop for PidGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let _pid_guard = PidGuard(pidfile);

    let mut socket = match bind_socket(agent.config.port).await {
        Ok(s) => {
            tracing::info!(port = agent.config.port, "snmp-agent listening");
            Some(s)
        }
        Err(e) => {
            tracing::error!(error = %e, port = agent.config.port, "bind failed; will retry");
            None
        }
    };

    let mut buf = [0u8; 2048];
    loop {
        let port = agent.config.port;
        tokio::select! {
            _ = reload.recv() => {
                match SnmpConfig::load(&config_path) {
                    Ok(new_cfg) => {
                        apply_reload(&mut agent, &mut socket, new_cfg).await;
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "config reload failed; keeping last-good");
                    }
                }
            }
            _ = tokio::time::sleep(BIND_RETRY), if socket.is_none() => {
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
                        if let Some(resp) = handle_datagram(&buf[..n], &agent)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ber::Oid;
    use crate::pdu::{SnmpValue, VarBind};
    use std::time::Duration;

    fn test_agent() -> Agent {
        Agent::with_roots(
            SnmpConfig {
                community: "public".into(),
                sys_name: "cam-1".into(),
                ..Default::default()
            },
            PathBuf::from("/proc"),
            PathBuf::from("/sys/class/net"),
        )
    }

    fn get_sysname_bytes(community: &str) -> Vec<u8> {
        let msg = SnmpMessage {
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
        let agent = test_agent();
        let req = get_sysname_bytes("wrong");
        assert!(handle_datagram(&req, &agent).is_none());
    }

    #[test]
    fn test_handle_datagram_returns_sysname() {
        let agent = test_agent();
        let req = get_sysname_bytes("public");
        let resp = handle_datagram(&req, &agent).expect("response");
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
        let agent = test_agent();
        assert!(handle_datagram(&[0xff, 0x00], &agent).is_none());
    }

    #[test]
    fn test_get_response_is_never_answered() {
        let agent = test_agent();
        let mut req = get_sysname_bytes("public");
        // Flip the PDU tag from GetRequest [0] to GetResponse [2].
        let i = req.iter().position(|&b| b == 0xa0).expect("pdu tag");
        req[i] = 0xa2;
        assert!(
            handle_datagram(&req, &agent).is_none(),
            "answering a response lets a spoofed source loop us against ourselves"
        );
    }

    #[test]
    fn test_set_is_refused_without_reading_the_device() {
        // Unreadable roots: a SET must still answer notWritable, because it is
        // refused before any varbind is resolved and never consults a snapshot.
        let agent = Agent::with_roots(
            SnmpConfig {
                community: "public".into(),
                ..Default::default()
            },
            PathBuf::from("/nonexistent/proc"),
            PathBuf::from("/nonexistent/sys"),
        );
        let mut req = get_sysname_bytes("public");
        let i = req.iter().position(|&b| b == 0xa0).expect("pdu tag");
        req[i] = 0xa3; // SetRequest

        let resp = handle_datagram(&req, &agent).expect("SET must be answered");
        let msg = SnmpMessage::parse(&resp).expect("valid BER");
        assert_eq!(msg.pdu.error_status, crate::mib::ERR_NOT_WRITABLE);
        assert_eq!(msg.pdu.error_index, 1);
    }

    #[test]
    fn test_uptime_comes_from_proc_uptime() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("uptime"), "12345.67 98765.43\n").unwrap();
        let agent = Agent::with_roots(
            SnmpConfig::default(),
            dir.path().into(),
            dir.path().join("sys"),
        );
        assert_eq!(agent.snapshot().uptime_ticks, 1_234_567);
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
    async fn test_run_serves_udp_get_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        let pidfile = dir.path().join("snmp-agent.pid");

        let probe = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);

        std::fs::write(
            &cfg_path,
            format!(
                "port = {port}\ncommunity = \"public\"\nsys_name = \"run-cam\"\n"
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

        // Same-bind reload (port unchanged).
        std::fs::write(
            &cfg_path,
            format!(
                "port = {port}\ncommunity = \"public\"\nsys_name = \"run-cam2\"\n"
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

        handle.abort();
        let _ = handle.await;
    }

    #[tokio::test]
    async fn test_bind_failure_is_non_fatal() {
        let dir = tempfile::tempdir().unwrap();

        // Bind failure: hold the port, then start agent on it.
        let holder = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        let port = holder.local_addr().unwrap().port();
        let cfg_path = dir.path().join("bindfail.toml");
        let pidfile = dir.path().join("bindfail.pid");
        std::fs::write(
            &cfg_path,
            format!("port = {port}\ncommunity = \"public\"\n"),
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
            format!(
                "port = {port}\ncommunity = \"public\"\nsys_name = \"retry\"\n"
            ),
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

    #[tokio::test]
    async fn test_failed_rebind_keeps_old_port_until_retry_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("snmp.toml");
        let pidfile = dir.path().join("rebind.pid");

        let probe = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let old_port = probe.local_addr().unwrap().port();
        drop(probe);

        std::fs::write(
            &cfg_path,
            format!(
                "port = {old_port}\ncommunity = \"public\"\nsys_name = \"old\"\n"
            ),
        )
        .unwrap();

        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let handle = tokio::spawn({
            let cfg = cfg_path.clone();
            let pid = pidfile.clone();
            async move {
                let _ = run(cfg, pid, rx).await;
            }
        });
        wait_for_file(&pidfile, Duration::from_secs(2)).await;
        tokio::time::sleep(Duration::from_millis(50)).await;

        let holder = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let new_port = holder.local_addr().unwrap().port();
        std::fs::write(
            &cfg_path,
            format!(
                "port = {new_port}\ncommunity = \"public\"\nsys_name = \"new\"\n"
            ),
        )
        .unwrap();
        tx.send(()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;

        // Still answering on the old port with the old sysName.
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        client
            .send_to(&get_sysname_bytes("public"), ("127.0.0.1", old_port))
            .await
            .unwrap();
        let mut buf = [0u8; 2048];
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("old port must still answer")
            .unwrap();
        assert_eq!(
            SnmpMessage::parse(&buf[..n]).unwrap().pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"old".to_vec())
        );

        drop(holder);
        // Unchanged file: reload must retry the pending rebind now that the port is free.
        tx.send(()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;

        client
            .send_to(&get_sysname_bytes("public"), ("127.0.0.1", new_port))
            .await
            .unwrap();
        let (n, _) = tokio::time::timeout(Duration::from_secs(2), client.recv_from(&mut buf))
            .await
            .expect("new port must answer after successful rebind")
            .unwrap();
        assert_eq!(
            SnmpMessage::parse(&buf[..n]).unwrap().pdu.variable_bindings[0].value,
            SnmpValue::OctetString(b"new".to_vec())
        );

        handle.abort();
        let _ = handle.await;
    }
}
