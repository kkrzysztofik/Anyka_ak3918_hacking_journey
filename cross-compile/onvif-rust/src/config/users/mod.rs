//! User management module.
//!
//! This module implements ONVIF user management functionality including:
//! - User storage with CRUD operations
//! - Password validation and verification
//! - TOML file persistence
//!
//! # Architecture
//!
//! The user management system follows these principles:
//! - **Thread-safe**: All operations use `RwLock` for concurrent access
//! - **WS-Security compatible**: Passwords stored in plaintext for digest auth
//! - **Atomic persistence**: TOML file writes use temp file + rename pattern
//! - **ONVIF compliance**: User levels match ONVIF specification
//!
//! # Security Note
//!
//! Passwords are stored in plaintext because WS-Security UsernameToken
//! digest authentication requires computing SHA1(Nonce + Created + Password).
//! File permissions should be restricted (`chmod 600`).
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::config::users::{UserStorage, PasswordManager, UserLevel};
//!
//! let password_mgr = PasswordManager::new();
//! let storage = UserStorage::new();
//!
//! // Validate and create admin user
//! password_mgr.validate_password("admin123")?;
//! storage.create_user("admin", "admin123", UserLevel::Administrator)?;
//!
//! // Verify password
//! let user = storage.get_user("admin").unwrap();
//! assert!(password_mgr.verify_password("admin123", &user.password));
//! ```
//!
//! # ONVIF Operations
//!
//! This module supports the following Device Service operations:
//! - `GetUsers` - List all users (username and level only)
//! - `CreateUsers` - Create new user accounts (admin only)
//! - `DeleteUsers` - Remove user accounts (admin only)
//! - `SetUser` - Update user password or level (admin only)

pub mod password;

pub use password::{PasswordError, PasswordManager};

use parking_lot::RwLock;
use password::SecurePassword;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::sync::OnceLock;
use thiserror::Error;

use super::persistence::PersistenceHandle;

/// Maximum number of users allowed.
///
/// ONVIF devices typically have limited storage, so we cap users at 8.
/// This is a common limit for embedded IP cameras.
pub const MAX_USERS: usize = 8;

/// Default admin username.
pub const DEFAULT_ADMIN_USERNAME: &str = "admin";

/// Character set for random password generation.
/// Includes uppercase, lowercase, digits, and safe special characters.
const PASSWORD_CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                                  abcdefghijklmnopqrstuvwxyz\
                                  0123456789\
                                  !@#$%^&*()_+-=[]{}";

/// Length of randomly generated passwords.
const GENERATED_PASSWORD_LENGTH: usize = 16;

/// Generate a secure random password.
///
/// Creates a 16-character password using uppercase, lowercase, digits,
/// and safe special characters. Uses cryptographically secure random
/// number generation via `rand::thread_rng()`.
///
/// # Returns
///
/// A randomly generated password string.
///
/// # Example
///
/// ```ignore
/// let password = generate_secure_password();
/// assert_eq!(password.len(), 16);
/// ```
fn generate_secure_password() -> String {
    use rand::RngExt;
    let mut rng = rand::rng();
    (0..GENERATED_PASSWORD_LENGTH)
        .map(|_| {
            let idx = rng.random_range(0..PASSWORD_CHARSET.len());
            PASSWORD_CHARSET[idx] as char
        })
        .collect()
}

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
/// Stores the username, password (secure), and privilege level.
/// Passwords are stored using `SecurePassword` which zeros memory on drop.
/// Plaintext storage is still required for WS-Security UsernameToken digest
/// authentication which computes SHA1(Nonce + Created + Password).
///
/// # Security
///
/// - Passwords are automatically zeroed from memory on drop
/// - File permissions should be restricted (`chmod 600`)
/// - Consider encryption-at-rest for production deployments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserAccount {
    /// The unique username.
    pub username: String,

    /// The secure password (automatically zeroed on drop).
    ///
    /// Required for WS-Security digest authentication.
    pub password: SecurePassword,

    /// The user's privilege level.
    pub level: UserLevel,
}

impl UserAccount {
    /// Create a new user account.
    pub fn new(
        username: impl Into<String>,
        password: impl Into<SecurePassword>,
        level: UserLevel,
    ) -> Self {
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
    /// Optional debounced, off-executor persistence handle.
    ///
    /// Set once during application startup via [`Self::set_persistence`]. When
    /// present, mutating handlers call [`Self::request_save`] (non-blocking)
    /// instead of writing to disk synchronously on the async executor.
    persistence: OnceLock<PersistenceHandle>,
}

impl UserStorage {
    /// Create a new empty user storage.
    pub fn new() -> Self {
        Self {
            users: RwLock::new(HashMap::with_capacity(MAX_USERS)),
            persistence: OnceLock::new(),
        }
    }

    /// Create user storage with file persistence.
    pub fn with_file(_file_path: impl Into<String>) -> Self {
        Self {
            users: RwLock::new(HashMap::with_capacity(MAX_USERS)),
            persistence: OnceLock::new(),
        }
    }

    /// Attach a debounced persistence handle.
    ///
    /// Idempotent: only the first call installs a handle. Subsequent calls are
    /// ignored (a warning is logged) since the handle is expected to be wired
    /// exactly once at startup.
    pub fn set_persistence(&self, handle: PersistenceHandle) {
        if self.persistence.set(handle).is_err() {
            tracing::warn!("UserStorage persistence handle already set; ignoring");
        }
    }

    /// Request a non-blocking, debounced save of the current user set.
    ///
    /// No-op (with a debug log) when no persistence handle is configured, which
    /// is the case in unit tests and when persistence is disabled.
    pub fn request_save(&self) {
        match self.persistence.get() {
            Some(handle) => handle.request_save(),
            None => tracing::debug!("No user persistence handle configured; skipping save request"),
        }
    }

    /// Serialize the current users to TOML bytes for atomic persistence.
    ///
    /// Used by the persistence service's snapshot closure. Runs under the read
    /// lock via [`Self::list_users`].
    pub fn to_toml_bytes(&self) -> Result<Vec<u8>, UserError> {
        let users_file = UsersFile {
            users: self.list_users(),
        };
        let content = toml::to_string_pretty(&users_file)?;
        Ok(content.into_bytes())
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

        // SAFETY: We just verified username exists above, so get_mut must succeed
        let user = users
            .get_mut(username)
            .expect("User must exist: existence verified above");

        if let Some(new_level) = level {
            user.level = new_level;
        }

        if let Some(pwd) = password {
            user.password = SecurePassword::from(pwd);
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
    /// Sets file permissions to 0o600 (owner read/write only) for security.
    pub fn save_to_toml(&self, path: impl AsRef<Path>) -> Result<(), UserError> {
        let path = path.as_ref();

        let users_file = UsersFile {
            users: self.list_users(),
        };

        let content = toml::to_string_pretty(&users_file)?;
        super::file_ops::atomic_write(path, content.as_bytes(), Some(0o600))?;

        tracing::debug!("Saved {} users to {:?}", users_file.users.len(), path);
        Ok(())
    }

    /// Ensure at least one admin user exists.
    ///
    /// If no admin users exist, creates a default admin with a randomly
    /// generated password. The initial password is logged to
    /// `/var/log/onvif-initial-password.log` for one-time retrieval.
    ///
    /// # Security
    ///
    /// - Password is randomly generated (16 characters)
    /// - Initial password is logged once to a secure location
    /// - Log file should have restricted permissions (0o600)
    /// - Users should change the password on first login
    ///
    /// # Returns
    ///
    /// `Ok(())` if an admin exists or was created successfully.
    pub fn ensure_default_admin(&self) -> Result<(), UserError> {
        if self.admin_count() > 0 {
            return Ok(());
        }

        // Generate random password
        let password = generate_secure_password();

        // Log initial password to secure location (one-time)
        let log_path = "/var/log/onvif-initial-password.log";
        if let Err(e) = self.log_initial_password(DEFAULT_ADMIN_USERNAME, &password, log_path) {
            tracing::warn!(
                "Failed to log initial password to {}: {}. Password will be shown in console.",
                log_path,
                e
            );
            // Still show password in logs as fallback
            tracing::warn!(
                "⚠️  INITIAL ADMIN PASSWORD: username='{}' password='{}' - CHANGE IMMEDIATELY!",
                DEFAULT_ADMIN_USERNAME,
                password
            );
        }

        self.create_user(DEFAULT_ADMIN_USERNAME, &password, UserLevel::Administrator)?;

        tracing::info!(
            "Created default admin user '{}' with random password (see {})",
            DEFAULT_ADMIN_USERNAME,
            log_path
        );

        Ok(())
    }

    /// Log the initial password to a secure file.
    ///
    /// Creates a one-time log file with the initial admin credentials.
    /// Sets file permissions to 0o600 (owner read/write only).
    fn log_initial_password(&self, username: &str, password: &str, path: &str) -> io::Result<()> {
        let content = format!(
            "ONVIF Initial Admin Credentials\n\
             ================================\n\
             Generated: {}\n\
             Username: {}\n\
             Password: {}\n\
             \n\
             ⚠️  SECURITY WARNING:\n\
             - Change this password immediately after first login\n\
             - Delete this file after retrieving the password\n\
             - This password will not be logged again\n",
            chrono::Utc::now().to_rfc3339(),
            username,
            password
        );

        super::file_ops::atomic_write(Path::new(path), content.as_bytes(), Some(0o600))
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

    // ── Test fixture constants ──────────────────────────────────────────
    // These are intentionally hardcoded values used exclusively in unit
    // tests.  They are NOT production secrets and carry no security risk.
    // CodeQL rule: rust/hard-coded-cryptographic-value
    const TEST_PASSWORD: &str = "password";
    const TEST_PASSWORD_1: &str = "pwd1";
    const TEST_PASSWORD_2: &str = "pwd2";
    const TEST_PASSWORD_3: &str = "pwd3";
    const TEST_PASSWORD_SECRET: &str = "secret123";
    const TEST_PASSWORD_OLD: &str = "old_password";
    const TEST_PASSWORD_NEW: &str = "new_password";
    const TEST_PASSWORD_OTHER: &str = "other_pwd";
    // ────────────────────────────────────────────────────────────────────

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = UserLevel::Administrator;
        let _ = UserError::MaxUsersReached;
        let _ = PasswordError::EmptyPassword;
    }

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

    // Password generation tests
    #[test]
    fn test_generate_secure_password_length() {
        let password = generate_secure_password();
        assert_eq!(password.len(), GENERATED_PASSWORD_LENGTH);
    }

    #[test]
    fn test_generate_secure_password_charset() {
        let password = generate_secure_password();
        let charset_str = std::str::from_utf8(PASSWORD_CHARSET).unwrap();

        // All characters should be from the allowed charset
        for ch in password.chars() {
            assert!(
                charset_str.contains(ch),
                "Password contains invalid character: {}",
                ch
            );
        }
    }

    #[test]
    fn test_generate_secure_password_uniqueness() {
        // Generate 100 passwords and verify they're all unique
        let mut passwords = std::collections::HashSet::new();
        for _ in 0..100 {
            let password = generate_secure_password();
            passwords.insert(password);
        }

        // With 16 characters from a 72-character set, collisions are astronomically unlikely
        assert_eq!(passwords.len(), 100, "Generated passwords should be unique");
    }

    #[test]
    fn test_generate_secure_password_contains_variety() {
        // Generate a password and check it has variety (not all same character)
        let password = generate_secure_password();
        let chars: Vec<char> = password.chars().collect();
        let unique_chars: std::collections::HashSet<_> = chars.iter().collect();

        // Should have more than 1 unique character
        assert!(
            unique_chars.len() > 1,
            "Password should have character variety"
        );
    }

    #[test]
    fn test_create_user() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", TEST_PASSWORD_SECRET, UserLevel::Administrator)
            .unwrap();

        assert_eq!(storage.len(), 1);
        let user = storage.get_user("admin").unwrap();
        assert_eq!(user.username, "admin");
        assert_eq!(user.password.as_str(), TEST_PASSWORD_SECRET);
        assert_eq!(user.level, UserLevel::Administrator);
    }

    #[test]
    fn test_create_duplicate_user() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", TEST_PASSWORD_SECRET, UserLevel::Administrator)
            .unwrap();

        let result = storage.create_user("admin", TEST_PASSWORD_OTHER, UserLevel::Operator);
        assert!(matches!(result, Err(UserError::UserExists(_))));
    }

    #[test]
    fn test_create_max_users() {
        let storage = UserStorage::new();

        for i in 0..MAX_USERS {
            storage
                .create_user(&format!("user{}", i), TEST_PASSWORD, UserLevel::User)
                .unwrap();
        }

        let result = storage.create_user("overflow", TEST_PASSWORD, UserLevel::User);
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
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("user1", TEST_PASSWORD, UserLevel::User)
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
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();

        let result = storage.delete_user("admin");
        assert!(matches!(result, Err(UserError::CannotDeleteLastAdmin)));
    }

    #[test]
    fn test_delete_admin_with_other_admin() {
        let storage = UserStorage::new();

        storage
            .create_user("admin1", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("admin2", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();

        storage.delete_user("admin1").unwrap();

        assert_eq!(storage.len(), 1);
        assert_eq!(storage.admin_count(), 1);
    }

    #[test]
    fn test_update_user_password() {
        let storage = UserStorage::new();

        storage
            .create_user("user1", TEST_PASSWORD_OLD, UserLevel::User)
            .unwrap();
        storage
            .update_user("user1", Some(TEST_PASSWORD_NEW), None)
            .unwrap();

        let user = storage.get_user("user1").unwrap();
        assert_eq!(user.password.as_str(), TEST_PASSWORD_NEW);
        assert_eq!(user.level, UserLevel::User);
    }

    #[test]
    fn test_update_user_level() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("user1", TEST_PASSWORD, UserLevel::User)
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

        let result = storage.update_user("nobody", Some(TEST_PASSWORD), None);
        assert!(matches!(result, Err(UserError::UserNotFound(_))));
    }

    #[test]
    fn test_demote_last_admin() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();

        let result = storage.update_user("admin", None, Some(UserLevel::Operator));
        assert!(matches!(result, Err(UserError::CannotDeleteLastAdmin)));
    }

    #[test]
    fn test_list_users() {
        let storage = UserStorage::new();

        storage
            .create_user("admin", TEST_PASSWORD_1, UserLevel::Administrator)
            .unwrap();
        storage
            .create_user("user1", TEST_PASSWORD_2, UserLevel::User)
            .unwrap();
        storage
            .create_user("operator", TEST_PASSWORD_3, UserLevel::Operator)
            .unwrap();

        let users = storage.list_users();
        assert_eq!(users.len(), 3);
    }

    #[test]
    fn test_ensure_default_admin_when_empty() {
        let storage = UserStorage::new();

        storage.ensure_default_admin().unwrap();

        assert_eq!(storage.len(), 1);
        let user = storage.get_user(DEFAULT_ADMIN_USERNAME).unwrap();
        assert_eq!(user.level, UserLevel::Administrator);
        // Password is randomly generated, so we can't check exact value
        assert!(!user.password.is_empty());
    }

    #[test]
    fn test_ensure_default_admin_when_exists() {
        let storage = UserStorage::new();
        storage
            .create_user("existing_admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();

        storage.ensure_default_admin().unwrap();

        // Should not create another admin
        assert_eq!(storage.len(), 1);
    }

    #[test]
    fn test_save_and_load_toml() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("users.toml");

        // Create and save
        let storage1 = UserStorage::new();
        storage1
            .create_user("admin", TEST_PASSWORD_1, UserLevel::Administrator)
            .unwrap();
        storage1
            .create_user("user1", TEST_PASSWORD_2, UserLevel::User)
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
            .create_user("admin1", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        assert_eq!(storage.admin_count(), 1);

        storage
            .create_user("user1", TEST_PASSWORD, UserLevel::User)
            .unwrap();
        assert_eq!(storage.admin_count(), 1);

        storage
            .create_user("admin2", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        assert_eq!(storage.admin_count(), 2);
    }
}
