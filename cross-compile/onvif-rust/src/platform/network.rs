//! Network utilities for IP address resolution.
//!
//! This module provides network-related utilities for the ONVIF implementation,
//! particularly for determining the external IP address to use in URLs.

use std::net::UdpSocket;

use crate::config::ConfigRuntime;

/// Determine the external IP address to use in URLs.
///
/// Precedence:
/// 1. `network.ip_address` if set and non-empty (static IP configured by user)
/// 2. `network.detected_ip` if set and non-empty (DHCP detected IP)
/// 3. `server.address` if set, non-empty, and not "0.0.0.0"
/// 4. Autodetect using UDP socket trick
/// 5. Fallback to "127.0.0.1"
pub fn external_ip(config: &ConfigRuntime) -> String {
    // 1. Static IP configured by user (highest priority for static-IP configs)
    if let Ok(ip) = config.get_string("network.ip_address")
        && !ip.is_empty()
    {
        return ip;
    }

    // 2. DHCP detected IP (runtime detected)
    if let Ok(ip) = config.get_string("network.detected_ip")
        && !ip.is_empty()
    {
        return ip;
    }

    // 3. Server binding address
    if let Ok(ip) = config.get_string("server.address")
        && !ip.is_empty()
        && ip != "0.0.0.0"
    {
        return ip;
    }

    // 4. Autodetect
    if let Some(ip) = detect_local_ip() {
        return ip;
    }

    // 5. Fallback
    "127.0.0.1".to_string()
}

fn detect_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    if socket.connect("8.8.8.8:80").is_ok()
        && let Ok(addr) = socket.local_addr()
    {
        return Some(addr.ip().to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // network.ip_address tests (static IP - highest priority)
    // ========================================================================

    #[test]
    fn test_external_ip_static_ip_highest_priority() {
        let config = ConfigRuntime::new(Default::default());
        // Static IP takes precedence over detected_ip and server.address
        config
            .set_string("network.ip_address", "192.168.1.50")
            .unwrap();
        config
            .set_string("network.detected_ip", "192.168.1.100")
            .unwrap();
        config.set_string("server.address", "10.0.0.1").unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.50");
    }

    #[test]
    fn test_external_ip_static_ip_takes_precedence_over_detected() {
        let config = ConfigRuntime::new(Default::default());
        config
            .set_string("network.ip_address", "192.168.1.50")
            .unwrap();
        config
            .set_string("network.detected_ip", "192.168.1.100")
            .unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.50");
    }

    #[test]
    fn test_external_ip_static_ip_takes_precedence_over_server_address() {
        let config = ConfigRuntime::new(Default::default());
        config
            .set_string("network.ip_address", "192.168.1.50")
            .unwrap();
        config.set_string("server.address", "10.0.0.1").unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.50");
    }

    #[test]
    fn test_external_ip_empty_static_ip_falls_through() {
        let config = ConfigRuntime::new(Default::default());
        // Empty static IP should fall through to detected_ip
        config.set_string("network.ip_address", "").unwrap();
        config
            .set_string("network.detected_ip", "192.168.1.100")
            .unwrap();
        config.set_string("server.address", "10.0.0.1").unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.100");
    }

    // ========================================================================
    // network.detected_ip tests (DHCP - second priority)
    // ========================================================================

    #[test]
    fn test_external_ip_detected_ip_precedence() {
        let config = ConfigRuntime::new(Default::default());
        config
            .set_string("network.detected_ip", "192.168.1.100")
            .unwrap();
        config.set_string("server.address", "10.0.0.1").unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.100");
    }

    #[test]
    fn test_external_ip_server_address_fallback() {
        let config = ConfigRuntime::new(Default::default());
        // No detected_ip
        config.set_string("server.address", "10.0.0.1").unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_external_ip_server_address_zero_ignored() {
        let config = ConfigRuntime::new(Default::default());
        config.set_string("server.address", "0.0.0.0").unwrap();

        // Should fallback to detect_local_ip or 127.0.0.1
        let ip = external_ip(&config);
        // Either detected IP or fallback
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_external_ip_empty_detected_ip_fallback() {
        let config = ConfigRuntime::new(Default::default());
        config.set_string("network.detected_ip", "").unwrap();
        config.set_string("server.address", "10.0.0.1").unwrap();

        let ip = external_ip(&config);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_external_ip_no_config() {
        let config = ConfigRuntime::new(Default::default());
        // No config set

        let ip = external_ip(&config);
        // Should be detected IP or 127.0.0.1
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_detect_local_ip() {
        // This may or may not succeed depending on network availability
        let result = detect_local_ip();
        // If it succeeds, should be a valid IP
        if let Some(ip) = result {
            assert!(!ip.is_empty());
            assert_ne!(ip, "0.0.0.0");
        }
    }
}
