//! User management module.
//!
//! This module implements ONVIF user management functionality including:
//! - User storage with CRUD operations
//! - Secure password hashing using Argon2id
//! - TOML file persistence
//!
//! # Architecture
//!
//! The user management system follows these principles:
//! - **Thread-safe**: All operations use `RwLock` for concurrent access
//! - **Secure passwords**: Argon2id hashing with timing-safe comparison
//! - **Atomic persistence**: TOML file writes use temp file + rename pattern
//! - **ONVIF compliance**: User levels match ONVIF specification
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::users::{UserStorage, PasswordManager, UserLevel};
//!
//! let password_mgr = PasswordManager::new();
//! let storage = UserStorage::new();
//!
//! // Create admin user
//! let hash = password_mgr.hash_password("admin123")?;
//! storage.create_user("admin", &hash, UserLevel::Administrator)?;
//!
//! // Verify password
//! let user = storage.get_user("admin").unwrap();
//! assert!(password_mgr.verify_password("admin123", &user.password_hash)?);
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
        let _ = PasswordError::HashingFailed;
    }
}
