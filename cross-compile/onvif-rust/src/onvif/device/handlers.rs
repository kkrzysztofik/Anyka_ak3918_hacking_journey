//! Device Service request handlers.
//!
//! This module implements the ONVIF Device Service operation handlers.

use std::sync::Arc;

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::User as OnvifUser;
use crate::onvif::types::device::{
    CreateUsers, CreateUsersResponse, DeleteUsers, DeleteUsersResponse, GetUsers, GetUsersResponse,
    SetUser, SetUserResponse,
};
use crate::users::{PasswordManager, UserLevel, UserStorage};

use super::user_types::{validate_password, validate_username};

// ============================================================================
// DeviceService
// ============================================================================

/// ONVIF Device Service.
///
/// Handles Device Service operations including user management.
pub struct DeviceService {
    /// User storage.
    users: Arc<UserStorage>,
    /// Password manager for hashing.
    password_manager: Arc<PasswordManager>,
}

impl DeviceService {
    /// Create a new Device Service.
    pub fn new(users: Arc<UserStorage>, password_manager: Arc<PasswordManager>) -> Self {
        Self {
            users,
            password_manager,
        }
    }

    /// Handle GetUsers request.
    ///
    /// Returns a list of all users with their usernames and levels.
    /// Passwords are never returned.
    ///
    /// # Authorization
    ///
    /// Any authenticated user can call this operation.
    pub fn handle_get_users(&self, _request: GetUsers) -> OnvifResult<GetUsersResponse> {
        tracing::debug!("GetUsers request");

        let users: Vec<OnvifUser> = self
            .users
            .list_users()
            .into_iter()
            .map(OnvifUser::from)
            .collect();

        tracing::info!("GetUsers: returning {} users", users.len());

        Ok(GetUsersResponse { users })
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
        &self,
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
            return Err(OnvifError::NotAuthorized);
        }

        for user in &request.users {
            // Validate username
            validate_username(&user.username).map_err(|e| OnvifError::InvalidArgVal {
                subcode: "ter:InvalidUsername".to_string(),
                reason: e.to_string(),
            })?;

            // Validate password (required for create)
            validate_password(user.password.as_deref(), true).map_err(|e| {
                OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidPassword".to_string(),
                    reason: e.to_string(),
                }
            })?;

            // Hash the password
            let password_hash = self
                .password_manager
                .hash_password(user.password.as_deref().unwrap())
                .map_err(|e| {
                    OnvifError::HardwareFailure(format!("Password hashing failed: {}", e))
                })?;

            // Convert ONVIF user level to internal level
            let level: UserLevel = user.user_level.clone().into();

            // Create the user
            self.users
                .create_user(&user.username, &password_hash, level)
                .map_err(|e| match e {
                    crate::users::UserError::MaxUsersReached => OnvifError::MaxUsers,
                    crate::users::UserError::UserExists(name) => OnvifError::InvalidArgVal {
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
        &self,
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
            return Err(OnvifError::NotAuthorized);
        }

        for username in &request.usernames {
            self.users.delete_user(username).map_err(|e| match e {
                crate::users::UserError::UserNotFound(name) => OnvifError::InvalidArgVal {
                    subcode: "ter:UserNotFound".to_string(),
                    reason: format!("User '{}' not found", name),
                },
                crate::users::UserError::CannotDeleteLastAdmin => OnvifError::InvalidArgVal {
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
        &self,
        request: SetUser,
        caller_level: UserLevel,
    ) -> OnvifResult<SetUserResponse> {
        tracing::debug!("SetUser request: {} users", request.users.len());

        // Authorization check
        if !caller_level.is_admin() {
            tracing::warn!("SetUser: unauthorized (caller level: {:?})", caller_level);
            return Err(OnvifError::NotAuthorized);
        }

        for user in &request.users {
            // Validate password if provided
            validate_password(user.password.as_deref(), false).map_err(|e| {
                OnvifError::InvalidArgVal {
                    subcode: "ter:InvalidPassword".to_string(),
                    reason: e.to_string(),
                }
            })?;

            // Hash password if provided
            let password_hash = user
                .password
                .as_deref()
                .map(|pwd| {
                    self.password_manager.hash_password(pwd).map_err(|e| {
                        OnvifError::HardwareFailure(format!("Password hashing failed: {}", e))
                    })
                })
                .transpose()?;

            // Convert level
            let level: UserLevel = user.user_level.clone().into();

            // Update the user
            self.users
                .update_user(&user.username, password_hash.as_deref(), Some(level))
                .map_err(|e| match e {
                    crate::users::UserError::UserNotFound(name) => OnvifError::InvalidArgVal {
                        subcode: "ter:UserNotFound".to_string(),
                        reason: format!("User '{}' not found", name),
                    },
                    crate::users::UserError::CannotDeleteLastAdmin => OnvifError::InvalidArgVal {
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

        Ok(SetUserResponse {})
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::UserLevel as OnvifUserLevel;

    fn create_test_service() -> DeviceService {
        let users = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());

        // Create initial admin user
        let admin_hash = password_manager.hash_password("admin123").unwrap();
        users
            .create_user("admin", &admin_hash, UserLevel::Administrator)
            .unwrap();

        DeviceService::new(users, password_manager)
    }

    #[test]
    fn test_get_users() {
        let service = create_test_service();

        let response = service.handle_get_users(GetUsers {}).unwrap();

        assert_eq!(response.users.len(), 1);
        assert_eq!(response.users[0].username, "admin");
        assert!(response.users[0].password.is_none()); // Password not returned
    }

    #[test]
    fn test_create_users_success() {
        let service = create_test_service();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Administrator);
        assert!(result.is_ok());

        // Verify user was created
        let users = service.handle_get_users(GetUsers {}).unwrap();
        assert_eq!(users.users.len(), 2);
    }

    #[test]
    fn test_create_users_unauthorized() {
        let service = create_test_service();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "operator".to_string(),
                password: Some("operator123".to_string()),
                user_level: OnvifUserLevel::Operator,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Operator);
        assert!(matches!(result, Err(OnvifError::NotAuthorized)));
    }

    #[test]
    fn test_create_users_max_users() {
        let service = create_test_service();

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
            service
                .handle_create_users(request, UserLevel::Administrator)
                .unwrap();
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

        let result = service.handle_create_users(request, UserLevel::Administrator);
        assert!(matches!(result, Err(OnvifError::MaxUsers)));
    }

    #[test]
    fn test_create_users_duplicate() {
        let service = create_test_service();

        let request = CreateUsers {
            users: vec![OnvifUser {
                username: "admin".to_string(), // Already exists
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = service.handle_create_users(request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:UsernameExists"
        ));
    }

    #[test]
    fn test_delete_users_success() {
        let service = create_test_service();

        // Create a user to delete
        let create_request = CreateUsers {
            users: vec![OnvifUser {
                username: "toDelete".to_string(),
                password: Some("password123".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };
        service
            .handle_create_users(create_request, UserLevel::Administrator)
            .unwrap();

        // Delete the user
        let delete_request = DeleteUsers {
            usernames: vec!["toDelete".to_string()],
        };
        let result = service.handle_delete_users(delete_request, UserLevel::Administrator);
        assert!(result.is_ok());

        // Verify user was deleted
        let users = service.handle_get_users(GetUsers {}).unwrap();
        assert_eq!(users.users.len(), 1);
    }

    #[test]
    fn test_delete_users_unauthorized() {
        let service = create_test_service();

        let request = DeleteUsers {
            usernames: vec!["admin".to_string()],
        };

        let result = service.handle_delete_users(request, UserLevel::User);
        assert!(matches!(result, Err(OnvifError::NotAuthorized)));
    }

    #[test]
    fn test_delete_last_admin() {
        let service = create_test_service();

        let request = DeleteUsers {
            usernames: vec!["admin".to_string()],
        };

        let result = service.handle_delete_users(request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:FixedUser"
        ));
    }

    #[test]
    fn test_set_user_password() {
        let service = create_test_service();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "admin".to_string(),
                password: Some("newpassword123".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            }],
        };

        let result = service.handle_set_user(request, UserLevel::Administrator);
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_user_unauthorized() {
        let service = create_test_service();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "admin".to_string(),
                password: Some("newpassword".to_string()),
                user_level: OnvifUserLevel::Administrator,
                extension: None,
            }],
        };

        let result = service.handle_set_user(request, UserLevel::Operator);
        assert!(matches!(result, Err(OnvifError::NotAuthorized)));
    }

    #[test]
    fn test_set_user_not_found() {
        let service = create_test_service();

        let request = SetUser {
            users: vec![OnvifUser {
                username: "nonexistent".to_string(),
                password: Some("password".to_string()),
                user_level: OnvifUserLevel::User,
                extension: None,
            }],
        };

        let result = service.handle_set_user(request, UserLevel::Administrator);
        assert!(matches!(
            result,
            Err(OnvifError::InvalidArgVal { subcode, .. }) if subcode == "ter:UserNotFound"
        ));
    }
}
