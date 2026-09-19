//! Client for the `anyka-init` control socket.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ServiceStatus {
    pub name: String,
    pub state: String,
    /// `None` when the service is not running.
    pub pid: Option<i32>,
    pub uptime_s: u64,
    pub restarts: u64,
    pub retry_in_s: u64,
}

/// Must match `anyka_init::control::SOCKET_PATH` (anyka-init/src/control.rs):
/// both sides of this Unix-socket contract are checked on-device, since the
/// two crates do not share code.
pub const SOCKET_PATH: &str = "/tmp/anyka-supervisor.sock";

const TIMEOUT: Duration = Duration::from_secs(2);

/// Decode the TSV status frame. Malformed rows are dropped rather than
/// failing the whole snapshot: one bad row must not blank the page.
pub fn decode_status(text: &str) -> Vec<ServiceStatus> {
    text.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            if f.len() < 6 {
                return None;
            }
            let pid: i32 = f[2].parse().ok()?;
            Some(ServiceStatus {
                name: f[0].to_owned(),
                state: f[1].to_owned(),
                pid: if pid > 0 { Some(pid) } else { None },
                uptime_s: f[3].parse().ok()?,
                restarts: f[4].parse().ok()?,
                retry_in_s: f[5].parse().ok()?,
            })
        })
        .collect()
}

fn round_trip(path: &Path, request: &str) -> Option<String> {
    let mut stream = UnixStream::connect(path).ok()?;
    stream.set_read_timeout(Some(TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(TIMEOUT)).ok()?;
    stream.write_all(request.as_bytes()).ok()?;
    let mut out = String::new();
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        // EOF or the blank terminator line both end the frame.
        if reader.read_line(&mut line).ok()? == 0 || line == "\n" {
            break;
        }
        out.push_str(&line);
    }
    Some(out)
}

/// Blocking. `None` means the supervisor is unreachable — an older
/// `anyka-init` in the other A/B slot, or a control thread that failed to
/// bind. Callers render the raw process table anyway.
pub fn query_status(path: &Path) -> Option<Vec<ServiceStatus>> {
    round_trip(path, "status\n").map(|t| decode_status(&t))
}

/// Blocking. `true` means the restart was accepted, not that it completed.
pub fn request_restart(path: &Path, name: &str) -> bool {
    round_trip(path, &format!("restart {name}\n")).is_some_and(|r| r.trim() == "ok")
}

/// The supervisor's answer to an `enable`/`disable` request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleReply {
    /// Applied, or an idempotent no-op.
    Accepted,
    /// Not a configured service, or not toggleable.
    Unknown,
    /// The supervisor refused: the config write failed, or its loop did not
    /// answer within its own 1 s budget.
    Error,
    /// Socket unreachable — an older anyka-init in the other A/B slot.
    Unreachable,
}

/// Blocking. Sends `enable <name>` or `disable <name>` to the supervisor.
pub fn request_toggle(path: &Path, name: &str, enabled: bool) -> ToggleReply {
    let verb = if enabled { "enable" } else { "disable" };
    match round_trip(path, &format!("{verb} {name}\n")) {
        None => ToggleReply::Unreachable,
        Some(r) => match r.trim() {
            "ok" => ToggleReply::Accepted,
            "unknown" => ToggleReply::Unknown,
            _ => ToggleReply::Error,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_status_parses_tab_separated_rows() {
        let got = decode_status("onvif\trunning\t42\t90\t3\t0\nsnmp\tbackoff\t0\t0\t7\t12\n\n");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "onvif");
        assert_eq!(got[0].pid, Some(42));
        assert_eq!(got[0].uptime_s, 90);
        assert_eq!(got[1].state, "backoff");
        assert_eq!(got[1].retry_in_s, 12);
    }

    /// A non-positive pid on the wire (the server sends -1, older drafts 0)
    /// means "not running" — JSON gets a real null so the UI never renders a
    /// process that does not exist.
    #[test]
    fn test_decode_status_maps_pid_zero_to_none() {
        let got = decode_status("snmp\tbackoff\t0\t0\t7\t12\n\n");
        assert_eq!(got[0].pid, None);
        let neg = decode_status("snmp\tbackoff\t-1\t0\t7\t12\n\n");
        assert_eq!(neg[0].pid, None);
    }

    #[test]
    fn test_decode_status_of_an_empty_frame_is_empty() {
        assert!(decode_status("\n").is_empty());
    }

    /// A malformed row must be dropped, not panic and not poison the rest.
    #[test]
    fn test_decode_status_skips_rows_with_too_few_fields() {
        let got = decode_status("broken\trunning\nonvif\trunning\t42\t90\t3\t0\n\n");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "onvif");
    }

    /// The socket is absent on an older anyka-init in the other A/B slot.
    /// That must read as "control unavailable", never as a 500.
    #[test]
    fn test_query_status_on_a_missing_socket_returns_none() {
        let missing = std::path::Path::new("/tmp/definitely-not-a-socket-xyz.sock");
        assert!(query_status(missing).is_none());
    }

    #[test]
    fn test_request_restart_on_a_missing_socket_is_false() {
        let missing = std::path::Path::new("/tmp/definitely-not-a-socket-xyz.sock");
        assert!(!request_restart(missing, "onvif"));
    }

    /// Full round trip against a mock that speaks the anyka-init protocol
    /// (TSV + blank-line terminator, `ok` for a restart). Catches drift in
    /// the client's framing/parse; the cross-crate path contract is checked
    /// on-device.
    #[test]
    fn test_round_trip_against_a_mock_control_server() {
        use std::os::unix::net::UnixListener;

        let path = format!("/tmp/onvif-svc-test-{}.sock", std::process::id());
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("bind test socket");
        let server_path = path.clone();
        let server = std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            // Exactly the two requests the test sends; then close up and exit
            // so `join` below cannot wait on a listener that accepts forever.
            for stream in listener.incoming().take(2) {
                let Ok(mut stream) = stream else { break };
                let mut line = String::new();
                if std::io::BufReader::new(&stream)
                    .read_line(&mut line)
                    .is_err()
                {
                    continue;
                }
                if line.trim() == "status" {
                    let _ = stream.write_all(
                        b"onvif\trunning\t42\t90\t3\t0\nvendor-daemon\tbackoff\t-1\t0\t7\t12\n\n",
                    );
                } else if line.starts_with("restart ") {
                    let _ = stream.write_all(b"ok\n");
                } else {
                    let _ = stream.write_all(b"unknown\n");
                }
            }
            drop(listener);
            let _ = std::fs::remove_file(&server_path);
        });

        let p = std::path::Path::new(&path);
        let rows = query_status(p).expect("status round trip");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "onvif");
        assert_eq!(rows[0].pid, Some(42));
        assert_eq!(rows[1].name, "vendor-daemon");
        assert_eq!(rows[1].pid, None);

        assert!(request_restart(p, "onvif"));

        server.join().expect("server thread");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_request_toggle_maps_all_reply_words() {
        use std::os::unix::net::UnixListener;

        let path = format!("/tmp/onvif-tog-test-{}.sock", std::process::id());
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("bind");
        let server = std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            // Four connections: ok, unknown, error, garbage.
            let replies: [&[u8]; 4] = [b"ok\n", b"unknown\n", b"error\n", b"nope\n"];
            for (stream, reply) in listener.incoming().take(4).zip(replies) {
                let Ok(mut stream) = stream else { continue };
                let mut line = String::new();
                if std::io::BufReader::new(&stream)
                    .read_line(&mut line)
                    .is_err()
                {
                    continue;
                }
                let _ = stream.write_all(reply);
            }
        });

        let p = std::path::Path::new(&path);
        assert_eq!(request_toggle(p, "snmp", true), ToggleReply::Accepted);
        assert_eq!(request_toggle(p, "snmp", false), ToggleReply::Unknown);
        assert_eq!(request_toggle(p, "snmp", true), ToggleReply::Error);
        // Anything that is not one of the three words is a failure, not a
        // success.
        assert_eq!(request_toggle(p, "snmp", false), ToggleReply::Error);

        server.join().expect("server thread");
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_request_toggle_sends_the_right_verb() {
        // The verb is the only thing distinguishing the two calls; pin it.
        use std::os::unix::net::UnixListener;
        let path = format!("/tmp/onvif-tog-verb-{}.sock", std::process::id());
        let _ = std::fs::remove_file(&path);
        let listener = UnixListener::bind(&path).expect("bind");
        let server = std::thread::spawn(move || {
            use std::io::{BufRead, Write};
            let mut seen = Vec::new();
            for stream in listener.incoming().take(2) {
                let Ok(mut stream) = stream else { continue };
                let mut line = String::new();
                let _ = std::io::BufReader::new(&stream).read_line(&mut line);
                seen.push(line);
                let _ = stream.write_all(b"ok\n");
            }
            seen
        });

        let p = std::path::Path::new(&path);
        request_toggle(p, "snmp", true);
        request_toggle(p, "onvif", false);
        let seen = server.join().expect("server thread");
        assert_eq!(seen, vec!["enable snmp\n", "disable onvif\n"]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_request_toggle_on_a_missing_socket_is_unreachable() {
        let missing = std::path::Path::new("/tmp/definitely-not-a-socket-xyz.sock");
        assert_eq!(
            request_toggle(missing, "snmp", true),
            ToggleReply::Unreachable
        );
    }
}
