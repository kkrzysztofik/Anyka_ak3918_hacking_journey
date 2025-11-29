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
//! use onvif_rust::users::{UserStorage, PasswordManager, UserLevel};
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
pub mod storage;

pub use password::{PasswordError, PasswordManager};
pub use storage::{MAX_USERS, UserAccount, UserError, UserLevel, UserStorage};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all public types are accessible
        let _ = UserLevel::Administrator;
        let _ = UserError::MaxUsersReached;
        let _ = PasswordError::EmptyPassword;
    }
}
