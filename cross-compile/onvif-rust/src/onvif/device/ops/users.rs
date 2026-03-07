//! User management Device Service handlers.
//!
//! This module contains handlers for:
//! - GetUsers - List all users
//! - CreateUsers - Create new users
//! - DeleteUsers - Delete users
//! - SetUser - Update user password/level

use std::sync::Arc;

use crate::config::{UserLevel, UserStorage};
use crate::onvif::device::user_types::{validate_password, validate_username};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::User as OnvifUser;
use crate::onvif::types::device::{
    CreateUsers, CreateUsersResponse, DeleteUsers, DeleteUsersResponse, GetUsers, GetUsersResponse,
    SetUser, SetUserResponse,
};

/// Handle GetUsers request.
///
/// Returns a list of all users with their usernames and levels.
/// Passwords are never returned.
///
/// # Authorization
///
/// Any authenticated user can call this operation.
pub fn handle_get_users(
    users: &Arc<UserStorage>,
    _request: GetUsers,
) -> OnvifResult<GetUsersResponse> {
    tracing::debug!("GetUsers request");

    let users_list: Vec<OnvifUser> = users
        .list_users()
        .into_iter()
        .map(OnvifUser::from)
        .collect();

    tracing::info!("GetUsers: returning {} users", users_list.len());

    Ok(GetUsersResponse { users: users_list })
}

/// Handle CreateUsers request.
///
/// Creates new user accounts with the specified usernames, passwords, and levels.
///
/// # Authorization
///
/// Only Administrator users can call this operation.
///
/// # Errors
///
/// - `ter:NotAuthorized` - Caller is not an Administrator
/// - `ter:MaxUsers` - Maximum 8 users reached
/// - `ter:UsernameExists` - Username already exists
/// - `ter:InvalidArgVal` - Invalid username or password format
pub fn handle_create_users(
    users: &Arc<UserStorage>,
    request: CreateUsers,
    caller_level: UserLevel,
) -> OnvifResult<CreateUsersResponse> {
    tracing::debug!("CreateUsers request: {} users", request.users.len());

    // Authorization check
    if !caller_level.is_admin() {
        tracing::warn!(
            "CreateUsers: unauthorized (caller level: {:?})",
            caller_level
        );
        return Err(OnvifError::NotAuthorized(
            "Administrator privileges required".to_string(),
        ));
    }

    for user in &request.users {
        // Validate username
        validate_username(&user.username).map_err(|e| OnvifError::InvalidArgVal {
            subcode: "ter:InvalidUsername".to_string(),
            reason: e.to_string(),
        })?;

        // Validate password (required for create)
        // Note: validate_password returns error for None when required=true
        let password = user
            .password
            .as_deref()
            .ok_or_else(|| OnvifError::InvalidArgVal {
                subcode: "ter:InvalidPassword".to_string(),
                reason: "Password is required when creating a user".to_string(),
            })?;

        // Additional validation (defense in depth - in case validate_password changes)
        validate_password(Some(password), true).map_err(|e| OnvifError::InvalidArgVal {
            subcode: "ter:InvalidPassword".to_string(),
            reason: e.to_string(),
        })?;

        // Get the password - must be present for create operation
        let password = user
            .password
            .as_deref()
            .ok_or_else(|| OnvifError::InvalidArgVal {
                subcode: "ter:MissingPassword".to_string(),
                reason: "Password is required when creating a user".to_string(),
            })?;

        // Convert ONVIF user level to internal level
        let level: UserLevel = user.user_level.clone().into();

        // Create the user
        users
            .create_user(&user.username, password, level)
            .map_err(|e| match e {
                crate::config::UserError::MaxUsersReached => OnvifError::MaxUsers,
                crate::config::UserError::UserExists(name) => OnvifError::InvalidArgVal {
                    subcode: "ter:UsernameExists".to_string(),
                    reason: format!("User '{}' already exists", name),
                },
                _ => OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidArgVal".to_string(),
                    reason: e.to_string(),
                },
            })?;

        tracing::info!(
            "CreateUsers: created user '{}' with level {:?}",
            user.username,
            level
        );
    }

    // Persist users to config file
    if let Err(e) = users.save_to_toml("/mnt/anyka_hack/onvif/users.toml") {
        tracing::warn!("Failed to save users to file: {}", e);
    }

    Ok(CreateUsersResponse {})
}

/// Handle DeleteUsers request.
///
/// Deletes the specified users by username.
///
/// # Authorization
///
/// Only Administrator users can call this operation.
///
/// # Errors
///
/// - `ter:NotAuthorized` - Caller is not an Administrator
/// - `ter:UserNotFound` - Unknown username
/// - `ter:FixedUser` - Cannot delete last administrator
pub fn handle_delete_users(
    users: &Arc<UserStorage>,
    request: DeleteUsers,
    caller_level: UserLevel,
) -> OnvifResult<DeleteUsersResponse> {
    tracing::debug!("DeleteUsers request: {} users", request.usernames.len());

    // Authorization check
    if !caller_level.is_admin() {
        tracing::warn!(
            "DeleteUsers: unauthorized (caller level: {:?})",
            caller_level
        );
        return Err(OnvifError::NotAuthorized(
            "Administrator privileges required".to_string(),
        ));
    }

    for username in &request.usernames {
        users.delete_user(username).map_err(|e| match e {
            crate::config::UserError::UserNotFound(name) => OnvifError::InvalidArgVal {
                subcode: "ter:UserNotFound".to_string(),
                reason: format!("User '{}' not found", name),
            },
            crate::config::UserError::CannotDeleteLastAdmin => OnvifError::InvalidArgVal {
                subcode: "ter:FixedUser".to_string(),
                reason: "Cannot delete the last administrator".to_string(),
            },
            _ => OnvifError::InvalidArgVal {
                subcode: "ter:InvalidArgVal".to_string(),
                reason: e.to_string(),
            },
        })?;

        tracing::info!("DeleteUsers: deleted user '{}'", username);
    }

    // Persist users to config file
    if let Err(e) = users.save_to_toml("/mnt/anyka_hack/onvif/users.toml") {
        tracing::warn!("Failed to save users to file: {}", e);
    }

    Ok(DeleteUsersResponse {})
}

/// Handle SetUser request.
///
/// Updates existing users' passwords and/or levels.
///
/// # Authorization
///
/// Only Administrator users can call this operation.
///
/// # Errors
///
/// - `ter:NotAuthorized` - Caller is not an Administrator
/// - `ter:UserNotFound` - Unknown username
/// - `ter:FixedUser` - Cannot demote last administrator
/// - `ter:InvalidArgVal` - Invalid password format
pub fn handle_set_user(
    users: &Arc<UserStorage>,
    request: SetUser,
    caller_level: UserLevel,
) -> OnvifResult<SetUserResponse> {
    tracing::debug!("SetUser request: {} users", request.users.len());

    // Authorization check
    if !caller_level.is_admin() {
        tracing::warn!("SetUser: unauthorized (caller level: {:?})", caller_level);
        return Err(OnvifError::NotAuthorized(
            "Administrator privileges required".to_string(),
        ));
    }

    for user in &request.users {
        // Validate password if provided
        validate_password(user.password.as_deref(), false).map_err(|e| {
            OnvifError::InvalidArgVal {
                subcode: "ter:InvalidPassword".to_string(),
                reason: e.to_string(),
            }
        })?;

        // Get password if provided
        let password = user.password.as_deref();

        // Convert level
        let level: UserLevel = user.user_level.clone().into();

        // Update the user
        users
            .update_user(&user.username, password, Some(level))
            .map_err(|e| match e {
                crate::config::UserError::UserNotFound(name) => OnvifError::InvalidArgVal {
                    subcode: "ter:UserNotFound".to_string(),
                    reason: format!("User '{}' not found", name),
                },
                crate::config::UserError::CannotDeleteLastAdmin => OnvifError::InvalidArgVal {
                    subcode: "ter:FixedUser".to_string(),
                    reason: "Cannot demote the last administrator".to_string(),
                },
                _ => OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidArgVal".to_string(),
                    reason: e.to_string(),
                },
            })?;

        tracing::info!("SetUser: updated user '{}'", user.username);
    }

    // Persist users to config file
    if let Err(e) = users.save_to_toml("/mnt/anyka_hack/onvif/users.toml") {
        tracing::warn!("Failed to save users to file: {}", e);
    }

    Ok(SetUserResponse {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PasswordManager, UserLevel, UserStorage};
    use crate::onvif::types::common::UserLevel as OnvifUserLevel;
    use std::sync::Arc;

    const TEST_PASSWORD: &str = "test_fixture_pwd_not_real";

    fn create_test_users() -> Arc<UserStorage> {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        users
    }

    // ========================================================================
    // GetUsers Tests
    // ========================================================================

    #[test]
    fn test_get_users() {
        let users = create_test_users();
        let response = handle_get_users(&users, GetUsers {}).unwrap();

        assert_eq!(response.users.len(), 1);
        assert_eq!(response.users[0].username, "admin");
        assert!(response.users[0].password.is_none()); // Password not returned
    }

    // ========================================================================
    // CreateUsers Tests
    // ========================================================================

    #[test]
    fn test_create_users_success() {
        let users = create_test_users();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            }],
        };

        let result = handle_create_users(&users, request, UserLevel::Administrator);
        assert!(result.is_ok());

        // Verify user was created - use the SAME users object, not a new one
        let response = handle_get_users(&users, GetUsers {}).unwrap();
        assert_eq!(response.users.len(), 2);
    }

    #[test]
    fn test_create_users_unauthorized() {
        let users = create_test_users();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            }],
        };

        let result = handle_create_users(&users, request, UserLevel::Operator);
        assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
    }

    #[test]
    fn test_create_users_max_users() {
        let users = create_test_users();

        // Create 7 more users to reach max (8)
        for i in 0..7 {
            let request = CreateUsers {
                users: vec![OnvifUser {
                    username: format!("user{}", i),
                    password: Some("password123".to_string()),
                    user_level: OnvifUserLevel::User,
                    extension: None,
                }],
            };
            handle_create_users(&users, request, UserLevel::Administrator).unwrap();
        }

        // 9th user should fail
        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "overflow".to_string(),
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = handle_create_users(&users, request, UserLevel::Administrator);
        assert!(matches!(result, Err(OnvifError::MaxUsers)));
    }

    #[test]
    fn test_create_users_duplicate() {
        let users = create_test_users();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "admin".to_string(), // Already exists
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = handle_create_users(&users, request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:UsernameExists"
        ));
    }

    #[test]
    fn test_create_users_empty_list() {
        let users = create_test_users();

        let result = handle_create_users(
            &users,
            CreateUsers { users: vec![] },
            UserLevel::Administrator,
        );

        // Empty list should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_users_missing_password() {
        let users = create_test_users();

        // Create user without password (should fail - password required for create)
        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "nopass".to_string(),
                password: None, // No password - should fail
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = handle_create_users(&users, request, UserLevel::Administrator);
        // PasswordRequired maps to ter:InvalidPassword
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:InvalidPassword"
        ));
    }

    // ========================================================================
    // DeleteUsers Tests
    // ========================================================================

    #[test]
    fn test_delete_users_success() {
        let users = create_test_users();

        // Create a user to delete
        let create_request = CreateUsers {
            users: vec![OnvifUser {
                username: "toDelete".to_string(),
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };
        handle_create_users(&users, create_request, UserLevel::Administrator).unwrap();

        // Delete the user
        let delete_request = DeleteUsers {
            usernames: vec!["toDelete".to_string()],
        };
        let result = handle_delete_users(&users, delete_request, UserLevel::Administrator);
        assert!(result.is_ok());

        // Verify user was deleted
        let response = handle_get_users(&users, GetUsers {}).unwrap();
        assert_eq!(response.users.len(), 1);
    }

    #[test]
    fn test_delete_users_unauthorized() {
        let users = create_test_users();

        let request = DeleteUsers {
            usernames: vec!["admin".to_string()],
        };

        let result = handle_delete_users(&users, request, UserLevel::User);
        assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
    }

    #[test]
    fn test_delete_last_admin() {
        let users = create_test_users();

        let request = DeleteUsers {
            usernames: vec!["admin".to_string()],
        };

        let result = handle_delete_users(&users, request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:FixedUser"
        ));
    }

    #[test]
    fn test_delete_users_empty_list() {
        let users = create_test_users();

        let result = handle_delete_users(
            &users,
            DeleteUsers { usernames: vec![] },
            UserLevel::Administrator,
        );

        // Empty list should succeed
        assert!(result.is_ok());
    }

    // ========================================================================
    // SetUser Tests
    // ========================================================================

    #[test]
    fn test_set_user_password() {
        let users = create_test_users();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "admin".to_string(),
                password: Some("newpassword123".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            }],
        };

        let result = handle_set_user(&users, request, UserLevel::Administrator);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_user_unauthorized() {
        let users = create_test_users();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "admin".to_string(),
                password: Some("newpassword".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            }],
        };

        let result = handle_set_user(&users, request, UserLevel::Operator);
        assert!(matches!(result, Err(OnvifError::NotAuthorized(_))));
    }

    #[test]
    fn test_set_user_not_found() {
        let users = create_test_users();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "nonexistent".to_string(),
                password: Some("password".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = handle_set_user(&users, request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:UserNotFound"
        ));
    }

    #[test]
    fn test_set_user_empty_list() {
        let users = create_test_users();

        let result = handle_set_user(&users, SetUser { users: vec![] }, UserLevel::Administrator);

        // Empty list should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_user_without_password() {
        let users = create_test_users();
        use crate::onvif::types::common::UserLevel as OnvifUserLevel;

        // Update user level without changing password
        let result = handle_set_user(
            &users,
            SetUser {
                users: vec![OnvifUser {
                    username: "admin".to_string(),
                    password: None, // No password change
                    user_level: OnvifUserLevel::Administrator,
                    extension: None,
                }],
            },
            UserLevel::Administrator,
        );

        assert!(result.is_ok());
    }
}
