//! Device Service operation handlers organized by domain.
//!
//! This module contains the operation handlers split by logical domain:
//! - `system` - Device info, capabilities, date/time, system operations
//! - `network` - Network interfaces, hostname, DNS, NTP, protocols
//! - `discovery` - Scopes and discovery mode management
//! - `users` - User management operations

pub mod discovery;
pub mod network;
pub mod system;
pub mod users;
