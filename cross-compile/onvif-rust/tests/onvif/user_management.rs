//! Integration tests for ONVIF User Management.
//!
//! Tests the complete user management workflow including:
//! - User storage and persistence
//! - Password validation and verification
//! - Device service handlers
//! - Authorization checks

use onvif_rust::onvif::device::DeviceService;
use onvif_rust::onvif::types::common::{User as OnvifUser, UserLevel as OnvifUserLevel};
use onvif_rust::onvif::types::device::{CreateUsers, DeleteUsers, GetUsers, SetUser};
use onvif_rust::users::{MAX_USERS, PasswordManager, UserLevel, UserStorage};
use std::sync::Arc;
use tempfile::tempdir;

// ============================================================================
// Test Fixtures
// ============================================================================

fn create_service_with_admin() -> DeviceService {
    let users = Arc::new(UserStorage::new());
    let password_manager = Arc::new(PasswordManager::new());

    // Create initial admin user with plaintext password
    users
        .create_user("admin", "admin123", UserLevel::Administrator)
        .unwrap();

    DeviceService::new(users, password_manager)
}

// ============================================================================
// User Storage Integration Tests
// ============================================================================

#[test]
fn test_user_storage_persistence_roundtrip() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("users.toml");
    let password_manager = PasswordManager::new();

    // Create and populate storage with plaintext passwords
    let storage1 = UserStorage::new();

    storage1
        .create_user("admin", "admin_secret", UserLevel::Administrator)
        .unwrap();
    storage1
        .create_user("operator", "user_secret", UserLevel::Operator)
        .unwrap();

    // Save to file
    storage1.save_to_toml(&file_path).unwrap();

    // Load into new storage
    let storage2 = UserStorage::new();
    storage2.load_from_toml(&file_path).unwrap();

    // Verify all users loaded correctly
    assert_eq!(storage2.len(), 2);

    let admin = storage2.get_user("admin").unwrap();
    assert_eq!(admin.level, UserLevel::Administrator);
    assert!(password_manager.verify_password("admin_secret", &admin.password));

    let operator = storage2.get_user("operator").unwrap();
    assert_eq!(operator.level, UserLevel::Operator);
}

#[test]
fn test_user_storage_default_admin_creation() {
    let storage = UserStorage::new();

    // Storage is empty
    assert!(storage.is_empty());

    // Ensure default admin creates a user with plaintext password
    let result = storage.ensure_default_admin("default").unwrap();

    assert!(result.is_some());
    assert_eq!(storage.len(), 1);
    assert_eq!(storage.admin_count(), 1);

    // Calling again should not create another admin
    let result2 = storage.ensure_default_admin("default").unwrap();
    assert!(result2.is_none());
    assert_eq!(storage.len(), 1);
}

#[test]
fn test_max_users_limit() {
    let storage = UserStorage::new();

    // Create MAX_USERS users with plaintext passwords
    for i in 0..MAX_USERS {
        let result = storage.create_user(&format!("user{}", i), "test_password", UserLevel::User);
        assert!(result.is_ok(), "Failed to create user {}", i);
    }

    // Attempt to create one more should fail
    let result = storage.create_user("overflow", "test_password", UserLevel::User);
    assert!(result.is_err());
}

// ============================================================================
// Password Manager Integration Tests
// ============================================================================

#[test]
fn test_password_verify_integration() {
    let manager = PasswordManager::new();

    // Test various password types with timing-safe verification
    let passwords = vec![
        "simple",
        "Complex123!@#",
        "пароль",   // Russian
        "密码",     // Chinese
        "🔐🔑",     // Emoji
        " spaces ", // With spaces
        "\ttabs\n", // Whitespace
    ];

    for password in passwords {
        // Verify correct password matches
        assert!(
            manager.verify_password(password, password),
            "Failed for password: {:?}",
            password
        );
        // Verify wrong password doesn't match
        assert!(
            !manager.verify_password("wrong", password),
            "False positive for password: {:?}",
            password
        );
    }
}

#[test]
fn test_password_validation() {
    let manager = PasswordManager::new();

    // Valid passwords
    assert!(manager.validate_password("simple").is_ok());
    assert!(manager.validate_password("a").is_ok());
    assert!(manager.validate_password(&"a".repeat(128)).is_ok());

    // Invalid: empty
    assert!(manager.validate_password("").is_err());

    // Invalid: too long
    assert!(manager.validate_password(&"a".repeat(129)).is_err());
}

// ============================================================================
// Device Service Integration Tests
// ============================================================================

#[test]
fn test_complete_user_lifecycle() {
    let service = create_service_with_admin();

    // 1. Verify initial state
    let users = service.handle_get_users(GetUsers {}).unwrap();
    assert_eq!(users.users.len(), 1);
    assert_eq!(users.users[0].username, "admin");

    // 2. Create a new user
    let create_request = CreateUsers {
        users: vec![OnvifUser {
            username: "newuser".to_string(),
            password: Some("userpass123".to_string()),
            user_level: OnvifUserLevel::User,
            extension: None,
        }],
    };
    service
        .handle_create_users(create_request, UserLevel::Administrator)
        .unwrap();

    // 3. Verify user was created
    let users = service.handle_get_users(GetUsers {}).unwrap();
    assert_eq!(users.users.len(), 2);

    // 4. Update user level
    let set_request = SetUser {
        users: vec![OnvifUser {
            username: "newuser".to_string(),
            password: None, // Keep existing password
            user_level: OnvifUserLevel::Operator,
            extension: None,
        }],
    };
    service
        .handle_set_user(set_request, UserLevel::Administrator)
        .unwrap();

    // 5. Verify update
    let users = service.handle_get_users(GetUsers {}).unwrap();
    let newuser = users
        .users
        .iter()
        .find(|u| u.username == "newuser")
        .unwrap();
    assert_eq!(newuser.user_level, OnvifUserLevel::Operator);

    // 6. Delete user
    let delete_request = DeleteUsers {
        usernames: vec!["newuser".to_string()],
    };
    service
        .handle_delete_users(delete_request, UserLevel::Administrator)
        .unwrap();

    // 7. Verify deletion
    let users = service.handle_get_users(GetUsers {}).unwrap();
    assert_eq!(users.users.len(), 1);
    assert_eq!(users.users[0].username, "admin");
}

#[test]
fn test_authorization_levels() {
    let service = create_service_with_admin();

    // Create test users at different levels
    let create_request = CreateUsers {
        users: vec![
            OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            },
            OnvifUser {
                username: "user".to_string(),
                password: Some("user123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            },
        ],
    };
    service
        .handle_create_users(create_request, UserLevel::Administrator)
        .unwrap();

    // GetUsers should work for all levels (implicitly tested by calling handle_get_users)
    // This is a simplification - in real ONVIF, GetUsers may have different auth requirements

    // CreateUsers should fail for Operator
    let create_request = CreateUsers {
        users: vec![OnvifUser {
            username: "test".to_string(),
            password: Some("test123".to_string()),
            user_level: OnvifUserLevel::User,
            extension: None,
        }],
    };
    let result = service.handle_create_users(create_request.clone(), UserLevel::Operator);
    assert!(result.is_err());

    // CreateUsers should fail for User
    let result = service.handle_create_users(create_request, UserLevel::User);
    assert!(result.is_err());

    // DeleteUsers should fail for non-admins
    let delete_request = DeleteUsers {
        usernames: vec!["user".to_string()],
    };
    let result = service.handle_delete_users(delete_request.clone(), UserLevel::Operator);
    assert!(result.is_err());
    let result = service.handle_delete_users(delete_request, UserLevel::User);
    assert!(result.is_err());

    // SetUser should fail for non-admins
    let set_request = SetUser {
        users: vec![OnvifUser {
            username: "user".to_string(),
            password: Some("newpass".to_string()),
            user_level: OnvifUserLevel::User,
            extension: None,
        }],
    };
    let result = service.handle_set_user(set_request.clone(), UserLevel::Operator);
    assert!(result.is_err());
    let result = service.handle_set_user(set_request, UserLevel::User);
    assert!(result.is_err());
}

#[test]
fn test_admin_protection() {
    let service = create_service_with_admin();

    // Cannot delete the last admin
    let delete_request = DeleteUsers {
        usernames: vec!["admin".to_string()],
    };
    let result = service.handle_delete_users(delete_request, UserLevel::Administrator);
    assert!(result.is_err());

    // Cannot demote the last admin
    let set_request = SetUser {
        users: vec![OnvifUser {
            username: "admin".to_string(),
            password: None,
            user_level: OnvifUserLevel::User, // Try to demote
            extension: None,
        }],
    };
    let result = service.handle_set_user(set_request, UserLevel::Administrator);
    assert!(result.is_err());

    // Create second admin
    let create_request = CreateUsers {
        users: vec![OnvifUser {
            username: "admin2".to_string(),
            password: Some("admin2pass".to_string()),
            user_level: OnvifUserLevel::Administrator,
            extension: None,
        }],
    };
    service
        .handle_create_users(create_request, UserLevel::Administrator)
        .unwrap();

    // Now we can delete the first admin
    let delete_request = DeleteUsers {
        usernames: vec!["admin".to_string()],
    };
    let result = service.handle_delete_users(delete_request, UserLevel::Administrator);
    assert!(result.is_ok());

    // Verify admin2 is still there
    let users = service.handle_get_users(GetUsers {}).unwrap();
    assert_eq!(users.users.len(), 1);
    assert_eq!(users.users[0].username, "admin2");
}

#[test]
fn test_password_not_returned_in_get_users() {
    let service = create_service_with_admin();

    let users = service.handle_get_users(GetUsers {}).unwrap();

    for user in users.users {
        assert!(
            user.password.is_none(),
            "Password should never be returned for user {}",
            user.username
        );
    }
}

#[test]
fn test_bulk_user_operations() {
    let service = create_service_with_admin();

    // Create multiple users in one request
    let create_request = CreateUsers {
        users: vec![
            OnvifUser {
                username: "user1".to_string(),
                password: Some("pass1".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            },
            OnvifUser {
                username: "user2".to_string(),
                password: Some("pass2".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            },
            OnvifUser {
                username: "user3".to_string(),
                password: Some("pass3".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            },
        ],
    };
    service
        .handle_create_users(create_request, UserLevel::Administrator)
        .unwrap();

    let users = service.handle_get_users(GetUsers {}).unwrap();
    assert_eq!(users.users.len(), 4); // admin + 3 new users

    // Delete multiple users in one request
    let delete_request = DeleteUsers {
        usernames: vec!["user1".to_string(), "user2".to_string()],
    };
    service
        .handle_delete_users(delete_request, UserLevel::Administrator)
        .unwrap();

    let users = service.handle_get_users(GetUsers {}).unwrap();
    assert_eq!(users.users.len(), 2); // admin + user3
}
