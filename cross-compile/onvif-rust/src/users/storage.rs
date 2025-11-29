//! User storage and account management.
//!
//! This module provides thread-safe user storage with CRUD operations
//! and TOML file persistence for the ONVIF user management system.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use thiserror::Error;

/// Maximum number of users allowed.
///
/// ONVIF devices typically have limited storage, so we cap users at 8.
/// This is a common limit for embedded IP cameras.
pub const MAX_USERS: usize = 8;

/// Default admin username.
pub const DEFAULT_ADMIN_USERNAME: &str = "admin";

/// Default admin password (should be changed on first use).
pub const DEFAULT_ADMIN_PASSWORD: &str = "admin";

// ============================================================================
// UserLevel
// ============================================================================

/// User privilege level.
///
/// Matches the ONVIF `tt:UserLevel` enumeration from the schema.
/// The levels determine what operations a user can perform.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UserLevel {
    /// Full administrative access. Can manage users, configure device.
    Administrator,
    /// Operational access. Can control PTZ, view streams.
    Operator,
    /// Basic user. Can view streams only.
    #[default]
    User,
}

impl UserLevel {
    /// Check if this level has admin privileges.
    pub fn is_admin(&self) -> bool {
        matches!(self, Self::Administrator)
    }

    /// Check if this level can control the device (admin or operator).
    pub fn can_control(&self) -> bool {
        matches!(self, Self::Administrator | Self::Operator)
    }
}

impl std::fmt::Display for UserLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Administrator => write!(f, "Administrator"),
            Self::Operator => write!(f, "Operator"),
            Self::User => write!(f, "User"),
        }
    }
}

// ============================================================================
// UserAccount
// ============================================================================

/// A user account in the system.
///
/// Stores the username, password (plaintext), and privilege level.
/// Plaintext storage is required for WS-Security UsernameToken digest
/// authentication which computes SHA1(Nonce + Created + Password).
///
/// # Security
///
/// - File permissions should be restricted (`chmod 600`)
/// - Consider encryption-at-rest for production deployments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserAccount {
    /// The unique username.
    pub username: String,

    /// The plaintext password.
    ///
    /// Required for WS-Security digest authentication.
    pub password: String,

    /// The user's privilege level.
    pub level: UserLevel,
}

impl UserAccount {
    /// Create a new user account.
    pub fn new(username: impl Into<String>, password: impl Into<String>, level: UserLevel) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            level,
        }
    }
}

// ============================================================================
// UserError
// ============================================================================

/// Errors that can occur during user operations.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum UserError {
    /// Maximum number of users reached (8 users).
    #[error("Maximum number of users ({}) reached", MAX_USERS)]
    MaxUsersReached,

    /// User already exists.
    #[error("User '{0}' already exists")]
    UserExists(String),

    /// User not found.
    #[error("User '{0}' not found")]
    UserNotFound(String),

    /// Invalid credentials.
    #[error("Invalid credentials")]
    InvalidCredentials,

    /// Cannot delete the last admin user.
    #[error("Cannot delete the last administrator user")]
    CannotDeleteLastAdmin,

    /// Storage I/O error.
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Invalid username (empty or too long).
    #[error("Invalid username: {0}")]
    InvalidUsername(String),
}

impl From<io::Error> for UserError {
    fn from(err: io::Error) -> Self {
        Self::StorageError(err.to_string())
    }
}

impl From<toml::de::Error> for UserError {
    fn from(err: toml::de::Error) -> Self {
        Self::StorageError(format!("TOML parse error: {}", err))
    }
}

impl From<toml::ser::Error> for UserError {
    fn from(err: toml::ser::Error) -> Self {
        Self::StorageError(format!("TOML serialize error: {}", err))
    }
}

// ============================================================================
// UsersFile (for TOML serialization)
// ============================================================================

/// TOML file structure for user storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct UsersFile {
    /// Array of user accounts.
    #[serde(default)]
    users: Vec<UserAccount>,
}

// ============================================================================
// UserStorage
// ============================================================================

/// Thread-safe user storage with TOML persistence.
///
/// Provides CRUD operations for user accounts with the following guarantees:
/// - Thread-safe access via `RwLock`
/// - Maximum 8 users enforced
/// - At least one admin user always exists
/// - Atomic file writes using temp file + rename
///
/// # Example
///
/// ```ignore
/// let storage = UserStorage::new();
///
/// // Create a user
/// storage.create_user("operator", "hash123", UserLevel::Operator)?;
///
/// // Get user
/// if let Some(user) = storage.get_user("operator") {
///     println!("User level: {}", user.level);
/// }
///
/// // List all users
/// for user in storage.list_users() {
///     println!("{}: {}", user.username, user.level);
/// }
/// ```
pub struct UserStorage {
    /// In-memory user storage.
    users: RwLock<HashMap<String, UserAccount>>,

    /// Optional file path for persistence.
    #[allow(dead_code)]
    file_path: Option<String>,
}

impl UserStorage {
    /// Create a new empty user storage.
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::with_capacity(MAX_USERS)),
            file_path: None,
        }
    }

    /// Create user storage with file persistence.
    pub fn with_file(file_path: impl Into<String>) -> Self {
        Self {
            users: RwLock::new(HashMap::with_capacity(MAX_USERS)),
            file_path: Some(file_path.into()),
        }
    }

    /// Get the number of users.
    pub fn len(&self) -> usize {
        self.users.read().len()
    }

    /// Check if storage is empty.
    pub fn is_empty(&self) -> bool {
        self.users.read().is_empty()
    }

    /// Get a user by username.
    ///
    /// Returns `None` if the user doesn't exist.
    pub fn get_user(&self, username: &str) -> Option<UserAccount> {
        self.users.read().get(username).cloned()
    }

    /// Create a new user.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Maximum users (8) reached
    /// - Username already exists
    /// - Username is empty or invalid
    pub fn create_user(
        &self,
        username: &str,
        password: &str,
        level: UserLevel,
    ) -> Result<(), UserError> {
        // Validate username
        if username.is_empty() {
            return Err(UserError::InvalidUsername(
                "Username cannot be empty".into(),
            ));
        }
        if username.len() > 64 {
            return Err(UserError::InvalidUsername(
                "Username too long (max 64 chars)".into(),
            ));
        }

        let mut users = self.users.write();

        // Check max users
        if users.len() >= MAX_USERS {
            return Err(UserError::MaxUsersReached);
        }

        // Check for duplicate
        if users.contains_key(username) {
            return Err(UserError::UserExists(username.to_string()));
        }

        // Create account
        let account = UserAccount::new(username, password, level);
        users.insert(username.to_string(), account);

        Ok(())
    }

    /// Delete a user by username.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User doesn't exist
    /// - Trying to delete the last admin user
    pub fn delete_user(&self, username: &str) -> Result<(), UserError> {
        let mut users = self.users.write();

        // Check user exists
        let user = users
            .get(username)
            .ok_or_else(|| UserError::UserNotFound(username.to_string()))?;

        // Prevent deleting the last admin
        if user.level == UserLevel::Administrator {
            let admin_count = users
                .values()
                .filter(|u| u.level == UserLevel::Administrator)
                .count();
            if admin_count <= 1 {
                return Err(UserError::CannotDeleteLastAdmin);
            }
        }

        users.remove(username);
        Ok(())
    }

    /// Update an existing user's password and/or level.
    ///
    /// # Arguments
    ///
    /// * `username` - The user to update
    /// * `password` - New password (if Some)
    /// * `level` - New user level (if Some)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - User doesn't exist
    /// - Trying to demote the last admin
    pub fn update_user(
        &self,
        username: &str,
        password: Option<&str>,
        level: Option<UserLevel>,
    ) -> Result<(), UserError> {
        let mut users = self.users.write();

        // First check if user exists and get current level
        let current_level = users
            .get(username)
            .ok_or_else(|| UserError::UserNotFound(username.to_string()))?
            .level;

        // Check if demoting the last admin
        if let Some(new_level) = level
            && current_level == UserLevel::Administrator
            && new_level != UserLevel::Administrator
        {
            let admin_count = users
                .values()
                .filter(|u| u.level == UserLevel::Administrator)
                .count();
            if admin_count <= 1 {
                return Err(UserError::CannotDeleteLastAdmin);
            }
        }

        // Now get mutable reference and apply updates
        let user = users.get_mut(username).unwrap();

        if let Some(new_level) = level {
            user.level = new_level;
        }

        if let Some(pwd) = password {
            user.password = pwd.to_string();
        }

        Ok(())
    }

    /// List all users.
    ///
    /// Returns a vector of all user accounts (cloned).
    pub fn list_users(&self) -> Vec<UserAccount> {
        self.users.read().values().cloned().collect()
    }

    /// Count admin users.
    pub fn admin_count(&self) -> usize {
        self.users
            .read()
            .values()
            .filter(|u| u.level == UserLevel::Administrator)
            .count()
    }

    /// Load users from a TOML file.
    ///
    /// If the file doesn't exist, returns Ok without loading anything.
    /// Call `ensure_default_admin()` after this to create a default admin if needed.
    pub fn load_from_toml(&self, path: impl AsRef<Path>) -> Result<(), UserError> {
        let path = path.as_ref();

        if !path.exists() {
            tracing::info!("Users file does not exist: {:?}, starting fresh", path);
            return Ok(());
        }

        let content = fs::read_to_string(path)?;
        let users_file: UsersFile = toml::from_str(&content)?;

        let mut storage = self.users.write();
        storage.clear();

        for user in users_file.users {
            if storage.len() >= MAX_USERS {
                tracing::warn!("Skipping user '{}': max users reached", user.username);
                continue;
            }
            storage.insert(user.username.clone(), user);
        }

        tracing::info!("Loaded {} users from {:?}", storage.len(), path);
        Ok(())
    }

    /// Save users to a TOML file using atomic write.
    ///
    /// Uses a temp file + rename pattern to ensure atomic writes.
    pub fn save_to_toml(&self, path: impl AsRef<Path>) -> Result<(), UserError> {
        let path = path.as_ref();

        let users_file = UsersFile {
            users: self.list_users(),
        };

        let content = toml::to_string_pretty(&users_file)?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Atomic write: write to temp file, then rename
        let temp_path = path.with_extension("toml.tmp");

        {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(content.as_bytes())?;
            file.sync_all()?;
        }

        fs::rename(&temp_path, path)?;

        tracing::debug!("Saved {} users to {:?}", users_file.users.len(), path);
        Ok(())
    }

    /// Ensure at least one admin user exists.
    ///
    /// If no admin user exists, creates a default admin account.
    /// Returns the username if a new admin was created.
    pub fn ensure_default_admin(
        &self,
        default_password: &str,
    ) -> Result<Option<String>, UserError> {
        let has_admin = self
            .users
            .read()
            .values()
            .any(|u| u.level == UserLevel::Administrator);

        if has_admin {
            return Ok(None);
        }

        tracing::info!("Creating default admin user '{}'", DEFAULT_ADMIN_USERNAME);
        self.create_user(
            DEFAULT_ADMIN_USERNAME,
            default_password,
            UserLevel::Administrator,
        )?;

        Ok(Some(DEFAULT_ADMIN_USERNAME.to_string()))
    }

    /// Validate credentials against stored users.
    ///
    /// Returns the user account if credentials are valid.
    /// This method is for use with external password verification.
    pub fn validate_user(&self, username: &str) -> Option<UserAccount> {
        self.get_user(username)
    }
}

impl Default for UserStorage {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_user_level_default() {
        assert_eq!(UserLevel::default(), UserLevel::User);
    }

    #[test]
    fn test_user_level_is_admin() {
        assert!(UserLevel::Administrator.is_admin());
        assert!(!UserLevel::Operator.is_admin());
        assert!(!UserLevel::User.is_admin());
    }

    #[test]
    fn test_user_level_can_control() {
        assert!(UserLevel::Administrator.can_control());
        assert!(UserLevel::Operator.can_control());
        assert!(!UserLevel::User.can_control());
    }

    #[test]
    fn test_user_storage_new() {
        let storage = UserStorage::new();
        assert!(storage.is_empty());
        assert_eq!(storage.len(), 0);
    }

    #[test]
    fn test_create_user() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "secret123", UserLevel::Administrator)
            .unwrap();

        assert_eq!(storage.len(), 1);
        let user = storage.get_user("admin").unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.password, "secret123");
        assert_eq!(user.level, UserLevel::Administrator);
    }

    #[test]
    fn test_create_duplicate_user() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "secret123", UserLevel::Administrator)
            .unwrap();

        let result = storage.create_user("admin", "other_pwd", UserLevel::Operator);
        assert!(matches!(result, Err(UserError::UserExists(_))));
    }

    #[test]
    fn test_create_max_users() {
        let storage = UserStorage::new();

        for i in 0..MAX_USERS {
            storage
                .create_user(&format!("user{}", i), "password", UserLevel::User)
                .unwrap();
        }

        let result = storage.create_user("overflow", "password", UserLevel::User);
        assert!(matches!(result, Err(UserError::MaxUsersReached)));
    }

    #[test]
    fn test_create_user_empty_username() {
        let storage = UserStorage::new();

        let result = storage.create_user("", "hash", UserLevel::User);
        assert!(matches!(result, Err(UserError::InvalidUsername(_))));
    }

    #[test]
    fn test_create_user_long_username() {
        let storage = UserStorage::new();

        let long_name = "a".repeat(65);
        let result = storage.create_user(&long_name, "hash", UserLevel::User);
        assert!(matches!(result, Err(UserError::InvalidUsername(_))));
    }

    #[test]
    fn test_delete_user() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "password", UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("user1", "password", UserLevel::User)
            .unwrap();

        storage.delete_user("user1").unwrap();

        assert_eq!(storage.len(), 1);
        assert!(storage.get_user("user1").is_none());
    }

    #[test]
    fn test_delete_nonexistent_user() {
        let storage = UserStorage::new();

        let result = storage.delete_user("nobody");
        assert!(matches!(result, Err(UserError::UserNotFound(_))));
    }

    #[test]
    fn test_delete_last_admin() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "password", UserLevel::Administrator)
            .unwrap();

        let result = storage.delete_user("admin");
        assert!(matches!(result, Err(UserError::CannotDeleteLastAdmin)));
    }

    #[test]
    fn test_delete_admin_with_other_admin() {
        let storage = UserStorage::new();

        storage
            .create_user("admin1", "password", UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("admin2", "password", UserLevel::Administrator)
            .unwrap();

        storage.delete_user("admin1").unwrap();

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.admin_count(), 1);
    }

    #[test]
    fn test_update_user_password() {
        let storage = UserStorage::new();

        storage
            .create_user("user1", "old_password", UserLevel::User)
            .unwrap();
        storage
            .update_user("user1", Some("new_password"), None)
            .unwrap();

        let user = storage.get_user("user1").unwrap();
        assert_eq!(user.password, "new_password");
        assert_eq!(user.level, UserLevel::User);
    }

    #[test]
    fn test_update_user_level() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "password", UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("user1", "password", UserLevel::User)
            .unwrap();
        storage
            .update_user("user1", None, Some(UserLevel::Operator))
            .unwrap();

        let user = storage.get_user("user1").unwrap();
        assert_eq!(user.level, UserLevel::Operator);
    }

    #[test]
    fn test_update_nonexistent_user() {
        let storage = UserStorage::new();

        let result = storage.update_user("nobody", Some("password"), None);
        assert!(matches!(result, Err(UserError::UserNotFound(_))));
    }

    #[test]
    fn test_demote_last_admin() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "password", UserLevel::Administrator)
            .unwrap();

        let result = storage.update_user("admin", None, Some(UserLevel::Operator));
        assert!(matches!(result, Err(UserError::CannotDeleteLastAdmin)));
    }

    #[test]
    fn test_list_users() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", "pwd1", UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("user1", "pwd2", UserLevel::User)
            .unwrap();
        storage
            .create_user("operator", "pwd3", UserLevel::Operator)
            .unwrap();

        let users = storage.list_users();
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_ensure_default_admin_when_empty() {
        let storage = UserStorage::new();

        let result = storage.ensure_default_admin("default_password").unwrap();

        assert!(result.is_some());
        assert_eq!(storage.len(), 1);
        let admin = storage.get_user(DEFAULT_ADMIN_USERNAME).unwrap();
        assert_eq!(admin.level, UserLevel::Administrator);
    }

    #[test]
    fn test_ensure_default_admin_when_exists() {
        let storage = UserStorage::new();

        storage
            .create_user("other_admin", "password", UserLevel::Administrator)
            .unwrap();

        let result = storage.ensure_default_admin("default_password").unwrap();

        assert!(result.is_none());
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_save_and_load_toml() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("users.toml");

        // Create and save
        let storage1 = UserStorage::new();
        storage1
            .create_user("admin", "pwd1", UserLevel::Administrator)
            .unwrap();
        storage1
            .create_user("user1", "pwd2", UserLevel::User)
            .unwrap();
        storage1.save_to_toml(&file_path).unwrap();

        // Load into new storage
        let storage2 = UserStorage::new();
        storage2.load_from_toml(&file_path).unwrap();

        assert_eq!(storage2.len(), 2);
        let admin = storage2.get_user("admin").unwrap();
        assert_eq!(admin.level, UserLevel::Administrator);
        let user = storage2.get_user("user1").unwrap();
        assert_eq!(user.level, UserLevel::User);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let storage = UserStorage::new();

        let result = storage.load_from_toml("/nonexistent/path/users.toml");
        assert!(result.is_ok()); // Should succeed but load nothing
        assert!(storage.is_empty());
    }

    #[test]
    fn test_admin_count() {
        let storage = UserStorage::new();

        assert_eq!(storage.admin_count(), 0);

        storage
            .create_user("admin1", "password", UserLevel::Administrator)
            .unwrap();
        assert_eq!(storage.admin_count(), 1);

        storage
            .create_user("user1", "password", UserLevel::User)
            .unwrap();
        assert_eq!(storage.admin_count(), 1);

        storage
            .create_user("admin2", "password", UserLevel::Administrator)
            .unwrap();
        assert_eq!(storage.admin_count(), 2);
    }
}
