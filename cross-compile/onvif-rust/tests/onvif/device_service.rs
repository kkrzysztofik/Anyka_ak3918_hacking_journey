//! Integration tests for ONVIF Device Service.
//!
//! Tests the complete Device Service operations including:
//! - Device information retrieval
//! - Capabilities and services discovery
//! - System date/time management
//! - Hostname and scopes management
//! - Discovery mode configuration

use onvif_rust::onvif::device::DeviceService;
use onvif_rust::onvif::types::common::DiscoveryMode;
use onvif_rust::onvif::types::device::{
    AddScopes, GetCapabilities, GetDeviceInformation, GetDiscoveryMode, GetHostname, GetScopes,
    GetServiceCapabilities, GetServices, GetSystemDateAndTime, SetDiscoveryMode, SetHostname,
    SetScopes,
};
use onvif_rust::users::{PasswordManager, UserLevel, UserStorage};
use std::sync::Arc;

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_test_service() -> DeviceService {
    let users = Arc::new(UserStorage::new());
    let password_manager = Arc::new(PasswordManager::new());

    // Create initial admin user with plaintext password
    users
        .create_user("admin", "admin123", UserLevel::Administrator)
        .unwrap();

    DeviceService::new(users, password_manager)
}

// ============================================================================
// GetDeviceInformation Integration Tests
// ============================================================================

#[tokio::test]
async fn test_get_device_information_returns_correct_values() {
    let service = create_test_service();

    let response = service
        .handle_get_device_information(GetDeviceInformation {})
        .await
        .unwrap();

    // Default values from config
    assert_eq!(response.manufacturer, "Anyka");
    assert_eq!(response.model, "AK3918 Camera");
    assert!(!response.firmware_version.is_empty());
    // serial_number and hardware_id may be empty in default config
    // They are populated from platform or config in production
    // Just verify the fields exist (they have default values)
    let _ = response.serial_number;
    let _ = response.hardware_id;
}

// ============================================================================
// GetCapabilities Integration Tests
// ============================================================================

#[test]
fn test_get_capabilities_returns_device_caps() {
    let service = create_test_service();

    let response = service
        .handle_get_capabilities(GetCapabilities { category: vec![] })
        .unwrap();

    // Should have device capabilities at minimum
    assert!(response.capabilities.device.is_some());
}

#[test]
fn test_get_capabilities_device_has_xaddr() {
    let service = create_test_service();

    let response = service
        .handle_get_capabilities(GetCapabilities { category: vec![] })
        .unwrap();

    let device_caps = response
        .capabilities
        .device
        .as_ref()
        .expect("Device capabilities should exist");
    assert!(!device_caps.x_addr.is_empty());
}

// ============================================================================
// GetServices Integration Tests
// ============================================================================

#[test]
fn test_get_services_returns_all_services() {
    let service = create_test_service();

    let response = service
        .handle_get_services(GetServices {
            include_capability: true,
        })
        .unwrap();

    // Should have at least device service
    assert!(!response.services.is_empty());

    // Find device service
    let device_service = response
        .services
        .iter()
        .find(|s| s.namespace.contains("device"))
        .expect("Device service should exist");

    assert!(!device_service.x_addr.is_empty());
}

#[test]
fn test_get_services_version_2_5() {
    let service = create_test_service();

    let response = service
        .handle_get_services(GetServices {
            include_capability: false,
        })
        .unwrap();

    // All services should be version 2.5
    for svc in &response.services {
        assert_eq!(svc.version.major, 2);
        assert_eq!(svc.version.minor, 5);
    }
}

// ============================================================================
// GetSystemDateAndTime Integration Tests
// ============================================================================

#[test]
fn test_get_system_date_and_time() {
    let service = create_test_service();

    let response = service
        .handle_get_system_date_and_time(GetSystemDateAndTime {})
        .unwrap();

    let sdt = &response.system_date_and_time;

    // Should have UTC time
    assert!(sdt.utc_date_time.is_some());
    let utc = sdt.utc_date_time.as_ref().unwrap();

    // Basic sanity checks
    assert!(utc.date.year >= 2024);
    assert!(utc.date.month >= 1 && utc.date.month <= 12);
    assert!(utc.date.day >= 1 && utc.date.day <= 31);
}

// ============================================================================
// Hostname Management Integration Tests
// ============================================================================

#[test]
fn test_get_hostname() {
    let service = create_test_service();

    let response = service.handle_get_hostname(GetHostname {}).unwrap();

    // Should return hostname info
    assert!(response.hostname_information.name.is_some());
    assert!(
        !response
            .hostname_information
            .name
            .as_ref()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn test_set_hostname_valid() {
    let service = create_test_service();

    let result = service.handle_set_hostname(SetHostname {
        name: "new-camera-hostname".to_string(),
    });

    assert!(result.is_ok());
}

#[test]
fn test_set_hostname_invalid_empty() {
    let service = create_test_service();

    let result = service.handle_set_hostname(SetHostname {
        name: "".to_string(),
    });

    assert!(result.is_err());
}

// ============================================================================
// Scopes Management Integration Tests
// ============================================================================

#[test]
fn test_get_scopes_includes_fixed_and_configurable() {
    let service = create_test_service();

    let response = service.handle_get_scopes(GetScopes {}).unwrap();

    // Should have both fixed and configurable scopes
    let has_fixed = response.scopes.iter().any(|s| {
        matches!(
            s.scope_def,
            onvif_rust::onvif::types::common::ScopeDefinition::Fixed
        )
    });
    let has_configurable = response.scopes.iter().any(|s| {
        matches!(
            s.scope_def,
            onvif_rust::onvif::types::common::ScopeDefinition::Configurable
        )
    });

    assert!(has_fixed, "Should have fixed scopes");
    assert!(has_configurable, "Should have configurable scopes");
}

#[test]
fn test_set_scopes_replaces_configurable() {
    let service = create_test_service();

    // Get initial scopes
    let initial = service.handle_get_scopes(GetScopes {}).unwrap();
    let initial_fixed_count = initial
        .scopes
        .iter()
        .filter(|s| {
            matches!(
                s.scope_def,
                onvif_rust::onvif::types::common::ScopeDefinition::Fixed
            )
        })
        .count();

    // Set new scopes
    service
        .handle_set_scopes(SetScopes {
            scopes: vec![
                "onvif://www.onvif.org/name/NewCamera".to_string(),
                "onvif://www.onvif.org/location/NewLocation".to_string(),
            ],
        })
        .unwrap();

    // Verify fixed scopes preserved, configurable replaced
    let after = service.handle_get_scopes(GetScopes {}).unwrap();
    let after_fixed_count = after
        .scopes
        .iter()
        .filter(|s| {
            matches!(
                s.scope_def,
                onvif_rust::onvif::types::common::ScopeDefinition::Fixed
            )
        })
        .count();

    assert_eq!(initial_fixed_count, after_fixed_count);

    // Check new configurable scopes exist
    let has_new_name = after
        .scopes
        .iter()
        .any(|s| s.scope_item.contains("NewCamera"));
    let has_new_location = after
        .scopes
        .iter()
        .any(|s| s.scope_item.contains("NewLocation"));

    assert!(has_new_name, "Should have new name scope");
    assert!(has_new_location, "Should have new location scope");
}

#[test]
fn test_add_scopes_appends_to_existing() {
    let service = create_test_service();

    // Get initial count
    let initial_count = service
        .handle_get_scopes(GetScopes {})
        .unwrap()
        .scopes
        .len();

    // Add new scope
    service
        .handle_add_scopes(AddScopes {
            scope_item: vec!["onvif://www.onvif.org/hardware/CustomHardware".to_string()],
        })
        .unwrap();

    // Verify scope was added
    let after_count = service
        .handle_get_scopes(GetScopes {})
        .unwrap()
        .scopes
        .len();
    assert_eq!(after_count, initial_count + 1);
}

// ============================================================================
// Discovery Mode Integration Tests
// ============================================================================

#[test]
fn test_discovery_mode_default_discoverable() {
    let service = create_test_service();

    let response = service
        .handle_get_discovery_mode(GetDiscoveryMode {})
        .unwrap();

    assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
}

#[test]
fn test_set_discovery_mode_toggle() {
    let service = create_test_service();

    // Set to NonDiscoverable
    service
        .handle_set_discovery_mode(SetDiscoveryMode {
            discovery_mode: DiscoveryMode::NonDiscoverable,
        })
        .unwrap();

    let response = service
        .handle_get_discovery_mode(GetDiscoveryMode {})
        .unwrap();
    assert_eq!(response.discovery_mode, DiscoveryMode::NonDiscoverable);

    // Set back to Discoverable
    service
        .handle_set_discovery_mode(SetDiscoveryMode {
            discovery_mode: DiscoveryMode::Discoverable,
        })
        .unwrap();

    let response = service
        .handle_get_discovery_mode(GetDiscoveryMode {})
        .unwrap();
    assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
}

// ============================================================================
// GetServiceCapabilities Integration Tests
// ============================================================================

#[test]
fn test_get_service_capabilities_returns_valid_caps() {
    let service = create_test_service();

    let response = service
        .handle_get_service_capabilities(GetServiceCapabilities {})
        .unwrap();

    // Verify security capabilities
    assert!(response.capabilities.security.max_users.is_some());
    assert!(response.capabilities.security.username_token.is_some());
    assert!(response.capabilities.security.http_digest.is_some());

    // Verify system capabilities
    assert!(response.capabilities.system.discovery_bye.is_some());
}
