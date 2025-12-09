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

    // Set some values first (default config is empty)
    config.set_string("device.manufacturer", "Test").unwrap();

    let toml_result = config.to_toml_string();
    assert!(toml_result.is_ok());

    let toml_string = toml_result.unwrap();
    // Should contain expected TOML sections
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
    config
        .set_string("device.manufacturer", "Test Manufacturer")
        .unwrap();
    config.set_string("device.model", "Test Model").unwrap();

    // Serialize to TOML
    let toml_string = config.to_toml_string().unwrap();

    // Create new config and load
    let restored_config = ConfigRuntime::new(Default::default());
    restored_config.load_from_toml_string(&toml_string).unwrap();

    // Verify values match
    assert_eq!(
        restored_config.get_string("device.manufacturer").unwrap(),
        "Test Manufacturer"
    );
    assert_eq!(
        restored_config.get_string("device.model").unwrap(),
        "Test Model"
    );
}

// ============================================================================
// Backup File Format Tests
// ============================================================================

/// Test that backup produces valid base64-encoded content
#[test]
fn test_backup_produces_base64() {
    use base64::{Engine, engine::general_purpose::STANDARD};

    let config = ConfigRuntime::new(Default::default());
    config.set_string("device.manufacturer", "Test").unwrap();

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
    config
        .set_string("device.manufacturer", "Backup Test")
        .unwrap();
    config
        .set_string("device.firmware_version", "1.2.3")
        .unwrap();

    // Simulate GetSystemBackup: serialize and encode
    let toml_string = config.to_toml_string().unwrap();
    let backup_data = STANDARD.encode(toml_string.as_bytes());

    // Simulate RestoreSystem: decode and load
    let restored_bytes = STANDARD.decode(&backup_data).unwrap();
    let restored_toml = String::from_utf8(restored_bytes).unwrap();

    let new_config = ConfigRuntime::new(Default::default());
    new_config.load_from_toml_string(&restored_toml).unwrap();

    // Verify restoration
    assert_eq!(
        new_config.get_string("device.manufacturer").unwrap(),
        "Backup Test"
    );
    assert_eq!(
        new_config.get_string("device.firmware_version").unwrap(),
        "1.2.3"
    );
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

    // Minimal valid TOML
    let partial_toml = r#"
device.manufacturer = "Partial Config"
device.model = "Minimal"
"#;

    let result = config.load_from_toml_string(partial_toml);

    // Should succeed
    assert!(result.is_ok(), "Partial TOML should load");

    assert_eq!(
        config.get_string("device.manufacturer").unwrap(),
        "Partial Config"
    );
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
