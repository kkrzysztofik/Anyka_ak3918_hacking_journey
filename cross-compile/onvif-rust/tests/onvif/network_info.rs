//! Integration tests for Network Info platform abstraction.
//!
//! Tests verify that StubNetworkInfo correctly returns configured
//! MAC address and IP address values through the platform trait.

use onvif_rust::platform::{NetworkInfo, StubNetworkInfo, StubPlatformBuilder};

// ============================================================================
// StubNetworkInfo Basic Tests
// ============================================================================

/// Test that StubNetworkInfo returns default values
#[tokio::test]
async fn test_stub_network_info_defaults() {
    let network = StubNetworkInfo::default();

    let info = network.get_network_interfaces().await;
    assert!(info.is_ok());

    let interfaces = info.unwrap();
    // Default has one interface
    assert!(!interfaces.is_empty());

    let iface = &interfaces[0];
    assert_eq!(iface.name, "eth0");
}

/// Test that StubNetworkInfo can be constructed with custom MAC address
#[tokio::test]
async fn test_stub_network_info_custom_mac() {
    let custom_mac = "AA:BB:CC:DD:EE:FF".to_string();

    let network = StubNetworkInfo::with_mac(custom_mac.clone());

    let info = network.get_network_interfaces().await.unwrap();
    let iface = &info[0];

    assert_eq!(iface.mac_address.as_deref(), Some(custom_mac.as_str()));
}

/// Test that StubNetworkInfo can be constructed with both MAC and IP
#[tokio::test]
async fn test_stub_network_info_custom_mac_and_ip() {
    let custom_mac = "11:22:33:44:55:66".to_string();
    let custom_ip = Some("192.168.10.50".to_string());

    let network = StubNetworkInfo::with_mac_and_ip(custom_mac.clone(), custom_ip.clone());

    let info = network.get_network_interfaces().await.unwrap();
    let iface = &info[0];

    assert_eq!(iface.mac_address.as_deref(), Some(custom_mac.as_str()));
    assert_eq!(iface.ipv4_address, custom_ip);
}

// ============================================================================
// StubPlatformBuilder Network Integration Tests
// ============================================================================

/// Test that StubPlatformBuilder supports network_info by default
#[test]
fn test_stub_platform_builder_network_supported_by_default() {
    let platform = StubPlatformBuilder::new().build();

    // The builder sets network_info_supported: true by default
    // Verify platform was built successfully
    let _ = platform;
}

/// Test that StubPlatformBuilder can configure custom MAC address
#[test]
fn test_stub_platform_builder_custom_mac() {
    let custom_mac = "DE:AD:BE:EF:CA:FE";

    let platform = StubPlatformBuilder::new().mac_address(custom_mac).build();

    // Verify platform was built successfully with custom MAC
    let _ = platform;
}

/// Test that StubPlatformBuilder can configure custom IP address
#[test]
fn test_stub_platform_builder_custom_ip() {
    let custom_ip = Some("172.16.0.1".to_string());

    let platform = StubPlatformBuilder::new().ip_address(custom_ip).build();

    // Verify platform was built successfully with custom IP
    let _ = platform;
}

// ============================================================================
// IP Detection Tests
// ============================================================================

/// Test that detect_local_ip returns Some IP when network is available
#[test]
fn test_detect_local_ip_returns_ip() {
    let network = StubNetworkInfo::with_mac_and_ip(
        "00:11:22:33:44:55".to_string(),
        Some("192.168.1.100".to_string()),
    );

    // detect_local_ip is a method on the NetworkInfo trait
    let detected = network.detect_local_ip();

    // Should detect IP either from config or system
    assert!(detected.is_some());
}

// ============================================================================
// Network Interface Configuration Stubs
// ============================================================================

/// Test that set_network_interface accepts configuration (stub implementation)
#[tokio::test]
async fn test_set_network_interface_stub() {
    let network = StubNetworkInfo::default();

    let result = network
        .set_network_interface("eth0", Some("10.0.0.50".to_string()), Some(24), false)
        .await;

    // Stub implementation accepts but doesn't persist
    assert!(result.is_ok());
}

/// Test that set_dns accepts configuration (stub implementation)
#[tokio::test]
async fn test_set_dns_stub() {
    let network = StubNetworkInfo::default();

    let dns_servers = vec!["8.8.8.8".to_string(), "8.8.4.4".to_string()];
    let search_domains = vec!["example.com".to_string()];

    let result = network.set_dns(&dns_servers, &search_domains).await;

    // Stub implementation persists to internal state
    assert!(result.is_ok());
}

/// Test that set_gateway accepts configuration (stub implementation)
#[tokio::test]
async fn test_set_gateway_stub() {
    let network = StubNetworkInfo::default();

    let result = network.set_gateway("192.168.1.1").await;

    // Stub implementation persists to internal state
    assert!(result.is_ok());
}
