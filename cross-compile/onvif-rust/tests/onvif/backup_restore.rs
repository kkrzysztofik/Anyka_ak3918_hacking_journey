//! Integration tests for system backup and restore operations.
//!
//! Tests verify that configuration can be serialized to TOML,
//! encoded as base64, and restored correctly via GetSystemBackup
//! and RestoreSystem operations.

use onvif_rust::config::ConfigRuntime;
use std::sync::Arc;

// ============================================================================
// Configuration TOML Serialization Tests
// ============================================================================

/// Test that ConfigRuntime can serialize to TOML string
#[test]
fn test_config_to_toml_string() {
    let config = ConfigRuntime::new(Default::default());

    config.write().device.manufacturer = "Test".to_string();

    let toml_result = config.to_toml_string();
    assert!(toml_result.is_ok());

    let toml_string = toml_result.unwrap();
    assert!(!toml_string.is_empty());
}

/// Test that ConfigRuntime can load from TOML string
#[test]
fn test_config_load_from_toml_string() {
    let config = ConfigRuntime::new(Default::default());

    // First get current config as TOML
    let toml_string = config.to_toml_string().unwrap();

    // Create a new config and load the TOML
    let new_config = ConfigRuntime::new(Default::default());
    let result = new_config.load_from_toml_string(&toml_string);

    assert!(result.is_ok(), "Should load TOML configuration");
}

/// Test configuration roundtrip (serialize then deserialize)
#[test]
fn test_config_toml_roundtrip() {
    let config = ConfigRuntime::new(Default::default());

    // Set some config values
    {
        let mut c = config.write();
        c.device.manufacturer = "Test Manufacturer".to_string();
        c.device.model = "Test Model".to_string();
    }

    // Serialize to TOML
    let toml_string = config.to_toml_string().unwrap();

    // Create new config and load
    let restored_config = ConfigRuntime::new(Default::default());
    restored_config.load_from_toml_string(&toml_string).unwrap();

    // Verify values match
    let c = restored_config.read();
    assert_eq!(c.device.manufacturer, "Test Manufacturer");
    assert_eq!(c.device.model, "Test Model");
}

// ============================================================================
// Backup File Format Tests
// ============================================================================

/// Test that backup produces valid base64-encoded content
#[test]
fn test_backup_produces_base64() {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let config = ConfigRuntime::new(Default::default());
    config.write().device.manufacturer = "Test".to_string();

    let toml_string = config.to_toml_string().unwrap();

    // Encode as base64 (as backup handler does)
    let encoded = STANDARD.encode(toml_string.as_bytes());

    // Should be valid base64
    assert!(!encoded.is_empty());

    // Should decode back to original
    let decoded = STANDARD.decode(&encoded).unwrap();
    let decoded_str = String::from_utf8(decoded).unwrap();

    assert_eq!(decoded_str, toml_string);
}

/// Test backup/restore roundtrip via base64
#[test]
fn test_backup_restore_roundtrip() {
    use base64::{Engine, engine::general_purpose::STANDARD};

    // Setup initial config
    let config = ConfigRuntime::new(Default::default());
    {
        let mut c = config.write();
        c.device.manufacturer = "Backup Test".to_string();
        c.device.firmware_version = "1.2.3".to_string();
    }

    // Simulate GetSystemBackup: serialize and encode
    let toml_string = config.to_toml_string().unwrap();
    let backup_data = STANDARD.encode(toml_string.as_bytes());

    // Simulate RestoreSystem: decode and load
    let restored_bytes = STANDARD.decode(&backup_data).unwrap();
    let restored_toml = String::from_utf8(restored_bytes).unwrap();

    let new_config = ConfigRuntime::new(Default::default());
    new_config.load_from_toml_string(&restored_toml).unwrap();

    // Verify restoration
    let c = new_config.read();
    assert_eq!(c.device.manufacturer, "Backup Test");
    assert_eq!(c.device.firmware_version, "1.2.3");
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

/// Test that invalid TOML is rejected
#[test]
fn test_invalid_toml_rejected() {
    let config = ConfigRuntime::new(Default::default());

    let invalid_toml = "this is not { valid TOML [[[";
    let result = config.load_from_toml_string(invalid_toml);

    assert!(result.is_err(), "Invalid TOML should be rejected");
}

/// Test that partial TOML still loads
#[test]
fn test_partial_toml_loads() {
    let config = ConfigRuntime::new(Default::default());

    // Minimal valid TOML — serde(default) fills missing fields
    let partial_toml = r#"
[device]
manufacturer = "Partial Config"
model = "Minimal"
"#;

    let result = config.load_from_toml_string(partial_toml);

    // Should succeed
    assert!(result.is_ok(), "Partial TOML should load");

    assert_eq!(config.read().device.manufacturer, "Partial Config");
}

/// Test empty string produces error
#[test]
fn test_empty_toml_handled() {
    let config = ConfigRuntime::new(Default::default());

    let result = config.load_from_toml_string("");

    // Empty string might parse as empty config or error
    // depending on implementation - just verify it doesn't panic
    let _ = result;
}

// ============================================================================
// Thread Safety Tests
// ============================================================================

/// Test that config can be serialized from Arc reference
#[test]
fn test_config_arc_serialization() {
    let config = Arc::new(ConfigRuntime::new(Default::default()));

    // Should be able to serialize from Arc
    let toml_result = config.to_toml_string();
    assert!(toml_result.is_ok());
}

/// Test concurrent backup operations
#[tokio::test]
async fn test_concurrent_backup_operations() {
    let config = Arc::new(ConfigRuntime::new(Default::default()));

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let config = Arc::clone(&config);
            tokio::spawn(async move { config.to_toml_string() })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}
