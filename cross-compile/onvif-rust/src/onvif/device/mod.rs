//! ONVIF Device Service implementation.
//!
//! This module implements the ONVIF Device Service (tds namespace) providing:
//! - Device information and capabilities (GetDeviceInformation, GetCapabilities, GetServices)
//! - System date/time management (GetSystemDateAndTime, SetSystemDateAndTime)
//! - System operations (SystemReboot, SetSystemFactoryDefault)
//! - Hostname management (GetHostname, SetHostname)
//! - Network configuration (GetNetworkInterfaces, SetNetworkInterfaces)
//! - Discovery configuration (GetScopes, SetScopes, AddScopes, GetDiscoveryMode, SetDiscoveryMode)
//! - User management (GetUsers, CreateUsers, DeleteUsers, SetUser)
//!
//! # Module Structure
//!
//! The service is organized into domain-specific operation modules:
//! - [`ops::system`] - Device info, capabilities, date/time, system operations
//! - [`ops::network`] - Network interfaces, hostname, DNS, NTP, protocols
//! - [`ops::discovery`] - Scopes and discovery mode management
//! - [`ops::users`] - User management operations
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
//! Device Service operations return ONVIF-compliant SOAP faults:
//! - `ter:NotAuthorized` - Insufficient privileges
//! - `ter:MaxUsers` - Maximum 8 users reached
//! - `ter:UsernameExists` - Duplicate username
//! - `ter:UserNotFound` - Unknown username
//! - `ter:InvalidHostname` - Invalid hostname format
//! - `ter:InvalidScope` - Invalid scope URI format
//! - `ter:FixedScope` - Cannot modify fixed scope
//! - `ter:InvalidNetworkInterface` - Network interface not found

pub mod faults;
pub mod ops;
pub mod service;
pub mod state;
pub mod store;
pub mod types;
pub mod user_types;
pub mod validation;

pub use service::DeviceService;
pub use types::*;
pub use user_types::*;

// Re-export WSDL types for user management
pub use crate::onvif::types::device::{
    CreateUsers as CreateUsersRequest, CreateUsersResponse as CreateUsersResponseType,
    DeleteUsers as DeleteUsersRequest, DeleteUsersResponse as DeleteUsersResponseType,
    GetUsers as GetUsersRequest, GetUsersResponse as GetUsersResponseType,
    SetUser as SetUserRequest, SetUserResponse as SetUserResponseType,
};
