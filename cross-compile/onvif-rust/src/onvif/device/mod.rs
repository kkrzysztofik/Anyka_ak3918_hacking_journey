//! ONVIF Device Service implementation.
//!
//! This module implements the ONVIF Device Service (tds namespace) providing:
//! - Device information and capabilities
//! - User management (GetUsers, CreateUsers, DeleteUsers, SetUser)
//! - Network configuration
//! - System management
//!
//! # User Management
//!
//! User management operations require Administrator privileges (except GetUsers):
//! - `GetUsers` - List all users (any authenticated user)
//! - `CreateUsers` - Create new users (admin only)
//! - `DeleteUsers` - Delete users (admin only)
//! - `SetUser` - Update user password/level (admin only)
//!
//! # Error Handling
//!
//! User management operations return ONVIF-compliant SOAP faults:
//! - `ter:NotAuthorized` - Insufficient privileges
//! - `ter:MaxUsers` - Maximum 8 users reached
//! - `ter:UsernameExists` - Duplicate username
//! - `ter:UserNotFound` - Unknown username

mod handlers;
pub mod user_types;

pub use handlers::DeviceService;
pub use user_types::*;

// Re-export WSDL types for user management
pub use crate::onvif::types::device::{
    CreateUsers as CreateUsersRequest, CreateUsersResponse as CreateUsersResponseType,
    DeleteUsers as DeleteUsersRequest, DeleteUsersResponse as DeleteUsersResponseType,
    GetUsers as GetUsersRequest, GetUsersResponse as GetUsersResponseType,
    SetUser as SetUserRequest, SetUserResponse as SetUserResponseType,
};
