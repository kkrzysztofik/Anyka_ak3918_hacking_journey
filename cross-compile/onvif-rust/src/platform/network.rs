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
    let c = config.read();

    // 1. Static IP configured by user (highest priority for static-IP configs)
    if !c.network.ip_address.is_empty() {
        return c.network.ip_address.clone();
    }

    // 2. DHCP detected IP (runtime detected)
    if !c.network.detected_ip.is_empty() {
        return c.network.detected_ip.clone();
    }

    // 3. Server binding address
    if !c.server.address.is_empty() && c.server.address != "0.0.0.0" {
        return c.server.address.clone();
    }

    drop(c);

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
        {
            let mut c = config.write();
            c.network.ip_address = "192.168.1.50".to_string();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.server.address = "10.0.0.1".to_string();
        }

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.50");
    }

    #[test]
    fn test_external_ip_static_ip_takes_precedence_over_detected() {
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.ip_address = "192.168.1.50".to_string();
            c.network.detected_ip = "192.168.1.100".to_string();
        }

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.50");
    }

    #[test]
    fn test_external_ip_static_ip_takes_precedence_over_server_address() {
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.ip_address = "192.168.1.50".to_string();
            c.server.address = "10.0.0.1".to_string();
        }

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.50");
    }

    #[test]
    fn test_external_ip_empty_static_ip_falls_through() {
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.server.address = "10.0.0.1".to_string();
        }

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.100");
    }

    // ========================================================================
    // network.detected_ip tests (DHCP - second priority)
    // ========================================================================

    #[test]
    fn test_external_ip_detected_ip_precedence() {
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.server.address = "10.0.0.1".to_string();
        }

        let ip = external_ip(&config);
        assert_eq!(ip, "192.168.1.100");
    }

    #[test]
    fn test_external_ip_server_address_fallback() {
        let config = ConfigRuntime::new(Default::default());
        config.write().server.address = "10.0.0.1".to_string();

        let ip = external_ip(&config);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_external_ip_server_address_zero_ignored() {
        let config = ConfigRuntime::new(Default::default());
        config.write().server.address = "0.0.0.0".to_string();

        // Should fallback to detect_local_ip or 127.0.0.1
        let ip = external_ip(&config);
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_external_ip_empty_detected_ip_fallback() {
        let config = ConfigRuntime::new(Default::default());
        config.write().server.address = "10.0.0.1".to_string();

        let ip = external_ip(&config);
        assert_eq!(ip, "10.0.0.1");
    }

    #[test]
    fn test_external_ip_no_config() {
        let config = ConfigRuntime::new(Default::default());

        let ip = external_ip(&config);
        assert!(!ip.is_empty());
    }

    #[test]
    fn test_detect_local_ip() {
        let result = detect_local_ip();
        if let Some(ip) = result {
            assert!(!ip.is_empty());
            assert_ne!(ip, "0.0.0.0");
        }
    }
}
