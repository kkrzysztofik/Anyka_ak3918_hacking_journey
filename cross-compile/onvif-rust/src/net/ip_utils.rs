use std::net::UdpSocket;

use crate::config::ConfigRuntime;

/// Determine the external IP address to use in URLs.
///
/// Precedence:
/// 1. `network.detected_ip` if set and non-empty
/// 2. `server.address` if set, non-empty, and not "0.0.0.0"
/// 3. Autodetect using UDP socket trick
/// 4. Fallback to "127.0.0.1"
pub fn external_ip(config: &ConfigRuntime) -> String {
    if let Ok(ip) = config.get_string("network.detected_ip") {
        if !ip.is_empty() {
            return ip;
        }
    }

    if let Ok(ip) = config.get_string("server.address") {
        if !ip.is_empty() && ip != "0.0.0.0" {
            return ip;
        }
    }

    if let Some(ip) = detect_local_ip() {
        return ip;
    }

    "127.0.0.1".to_string()
}

fn detect_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    if socket.connect("8.8.8.8:80").is_ok() {
        if let Ok(addr) = socket.local_addr() {
            return Some(addr.ip().to_string());
        }
    }
    None
}
