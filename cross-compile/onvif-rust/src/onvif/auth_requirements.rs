//! ONVIF operation authentication requirements.
//!
//! This module defines the authentication level required for each ONVIF operation
//! according to the ONVIF Core Specification and common implementation practice.
//!
//! # Authorization Levels
//!
//! ONVIF defines three authorization levels:
//!
//! - **Administrator**: Full device control, user management, configuration
//! - **Operator**: PTZ control, stream access, presets management
//! - **User**: Read-only access to device information and streams
//! - **Anonymous**: No authentication required (device discovery)
//!
//! # Implementation Notes
//!
//! The authorization levels here follow the ONVIF Device Management Service
//! specification, with some adjustments for practical security:
//!
//! - User management operations require Administrator level
//! - PTZ control operations require Operator level
//! - Read-only information operations may allow User level
//! - Discovery operations allow Anonymous access
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::auth_requirements::{get_required_level, AuthLevel};
//!
//! let level = get_required_level("device", "CreateUsers");
//! assert_eq!(level, AuthLevel::Administrator);
//!
//! let level = get_required_level("ptz", "ContinuousMove");
//! assert_eq!(level, AuthLevel::Operator);
//! ```

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::users::UserLevel;

/// Authentication level required for an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthLevel {
    /// No authentication required (e.g., device discovery).
    Anonymous,
    /// Basic user level - read-only access.
    User,
    /// Operator level - can control device (PTZ, presets).
    Operator,
    /// Administrator level - full access including user management.
    Administrator,
}

impl AuthLevel {
    /// Check if a user level satisfies this auth requirement.
    pub fn is_satisfied_by(&self, user_level: Option<UserLevel>) -> bool {
        match self {
            AuthLevel::Anonymous => true,
            AuthLevel::User => user_level.is_some(),
            AuthLevel::Operator => matches!(
                user_level,
                Some(UserLevel::Operator | UserLevel::Administrator)
            ),
            AuthLevel::Administrator => matches!(user_level, Some(UserLevel::Administrator)),
        }
    }
}

impl From<UserLevel> for AuthLevel {
    fn from(level: UserLevel) -> Self {
        match level {
            UserLevel::Administrator => AuthLevel::Administrator,
            UserLevel::Operator => AuthLevel::Operator,
            UserLevel::User => AuthLevel::User,
        }
    }
}

/// Mapping of (service, operation) -> required auth level.
type AuthMap = HashMap<(&'static str, &'static str), AuthLevel>;

/// Global auth requirements map, lazily initialized.
static AUTH_REQUIREMENTS: LazyLock<AuthMap> = LazyLock::new(build_auth_requirements);

/// Build the authentication requirements map.
fn build_auth_requirements() -> AuthMap {
    let mut map = HashMap::new();

    // ========================================================================
    // Device Service Operations
    // ========================================================================

    // Anonymous/User level - read-only information
    map.insert(("device", "GetDeviceInformation"), AuthLevel::User);
    map.insert(("device", "GetCapabilities"), AuthLevel::User);
    map.insert(("device", "GetServices"), AuthLevel::User);
    map.insert(("device", "GetServiceCapabilities"), AuthLevel::User);
    map.insert(("device", "GetSystemDateAndTime"), AuthLevel::Anonymous);
    map.insert(("device", "GetHostname"), AuthLevel::User);
    map.insert(("device", "GetNetworkInterfaces"), AuthLevel::User);
    map.insert(("device", "GetScopes"), AuthLevel::User);
    map.insert(("device", "GetDiscoveryMode"), AuthLevel::User);
    map.insert(("device", "GetUsers"), AuthLevel::Administrator);

    // Operator level - device configuration
    map.insert(("device", "SetSystemDateAndTime"), AuthLevel::Operator);
    map.insert(("device", "SetHostname"), AuthLevel::Operator);
    map.insert(("device", "SetScopes"), AuthLevel::Operator);
    map.insert(("device", "AddScopes"), AuthLevel::Operator);
    map.insert(("device", "RemoveScopes"), AuthLevel::Operator);
    map.insert(("device", "SetDiscoveryMode"), AuthLevel::Operator);

    // Administrator level - system management
    map.insert(("device", "SystemReboot"), AuthLevel::Administrator);
    map.insert(("device", "CreateUsers"), AuthLevel::Administrator);
    map.insert(("device", "DeleteUsers"), AuthLevel::Administrator);
    map.insert(("device", "SetUser"), AuthLevel::Administrator);

    // ========================================================================
    // Media Service Operations
    // ========================================================================

    // User level - read-only media information
    map.insert(("media", "GetProfiles"), AuthLevel::User);
    map.insert(("media", "GetProfile"), AuthLevel::User);
    map.insert(("media", "GetVideoSources"), AuthLevel::User);
    map.insert(("media", "GetVideoSourceConfigurations"), AuthLevel::User);
    map.insert(("media", "GetVideoSourceConfiguration"), AuthLevel::User);
    map.insert(
        ("media", "GetVideoSourceConfigurationOptions"),
        AuthLevel::User,
    );
    map.insert(("media", "GetVideoEncoderConfigurations"), AuthLevel::User);
    map.insert(("media", "GetVideoEncoderConfiguration"), AuthLevel::User);
    map.insert(
        ("media", "GetVideoEncoderConfigurationOptions"),
        AuthLevel::User,
    );
    map.insert(("media", "GetAudioSources"), AuthLevel::User);
    map.insert(("media", "GetAudioSourceConfigurations"), AuthLevel::User);
    map.insert(("media", "GetAudioSourceConfiguration"), AuthLevel::User);
    map.insert(("media", "GetAudioEncoderConfigurations"), AuthLevel::User);
    map.insert(("media", "GetAudioEncoderConfiguration"), AuthLevel::User);
    map.insert(
        ("media", "GetAudioEncoderConfigurationOptions"),
        AuthLevel::User,
    );
    map.insert(("media", "GetStreamUri"), AuthLevel::User);
    map.insert(("media", "GetSnapshotUri"), AuthLevel::User);

    // Operator level - media configuration
    map.insert(("media", "CreateProfile"), AuthLevel::Operator);
    map.insert(("media", "DeleteProfile"), AuthLevel::Operator);
    map.insert(
        ("media", "SetVideoSourceConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "AddVideoSourceConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "RemoveVideoSourceConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "SetVideoEncoderConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "AddVideoEncoderConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "RemoveVideoEncoderConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "SetAudioSourceConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "AddAudioSourceConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "RemoveAudioSourceConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "SetAudioEncoderConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "AddAudioEncoderConfiguration"),
        AuthLevel::Operator,
    );
    map.insert(
        ("media", "RemoveAudioEncoderConfiguration"),
        AuthLevel::Operator,
    );

    // ========================================================================
    // PTZ Service Operations
    // ========================================================================

    // User level - read-only PTZ information
    map.insert(("ptz", "GetNodes"), AuthLevel::User);
    map.insert(("ptz", "GetNode"), AuthLevel::User);
    map.insert(("ptz", "GetConfigurations"), AuthLevel::User);
    map.insert(("ptz", "GetConfiguration"), AuthLevel::User);
    map.insert(("ptz", "GetConfigurationOptions"), AuthLevel::User);
    map.insert(("ptz", "GetStatus"), AuthLevel::User);
    map.insert(("ptz", "GetPresets"), AuthLevel::User);
    map.insert(("ptz", "GetServiceCapabilities"), AuthLevel::User);
    map.insert(("ptz", "GetCompatibleConfigurations"), AuthLevel::User);

    // Operator level - PTZ control
    map.insert(("ptz", "SetConfiguration"), AuthLevel::Operator);
    map.insert(("ptz", "AbsoluteMove"), AuthLevel::Operator);
    map.insert(("ptz", "RelativeMove"), AuthLevel::Operator);
    map.insert(("ptz", "ContinuousMove"), AuthLevel::Operator);
    map.insert(("ptz", "Stop"), AuthLevel::Operator);
    map.insert(("ptz", "GotoHomePosition"), AuthLevel::Operator);
    map.insert(("ptz", "SetHomePosition"), AuthLevel::Operator);
    map.insert(("ptz", "SetPreset"), AuthLevel::Operator);
    map.insert(("ptz", "GotoPreset"), AuthLevel::Operator);
    map.insert(("ptz", "RemovePreset"), AuthLevel::Operator);
    map.insert(("ptz", "SendAuxiliaryCommand"), AuthLevel::Operator);

    // ========================================================================
    // Imaging Service Operations
    // ========================================================================

    // User level - read-only imaging information
    map.insert(("imaging", "GetImagingSettings"), AuthLevel::User);
    map.insert(("imaging", "GetOptions"), AuthLevel::User);
    map.insert(("imaging", "GetStatus"), AuthLevel::User);
    map.insert(("imaging", "GetMoveOptions"), AuthLevel::User);
    map.insert(("imaging", "GetServiceCapabilities"), AuthLevel::User);
    map.insert(("imaging", "GetPresets"), AuthLevel::User);
    map.insert(("imaging", "GetCurrentPreset"), AuthLevel::User);

    // Operator level - imaging control
    map.insert(("imaging", "SetImagingSettings"), AuthLevel::Operator);
    map.insert(("imaging", "Move"), AuthLevel::Operator);
    map.insert(("imaging", "Stop"), AuthLevel::Operator);
    map.insert(("imaging", "SetCurrentPreset"), AuthLevel::Operator);

    map
}

/// Get the required authentication level for an operation.
///
/// # Arguments
///
/// * `service` - The service name (e.g., "device", "media", "ptz", "imaging")
/// * `operation` - The operation name (e.g., "GetDeviceInformation", "ContinuousMove")
///
/// # Returns
///
/// The required `AuthLevel`, or `AuthLevel::Administrator` if the operation is not found
/// (fail-secure default).
pub fn get_required_level(service: &str, operation: &str) -> AuthLevel {
    AUTH_REQUIREMENTS
        .get(&(service, operation))
        .copied()
        // Fail-secure: unknown operations require admin
        .unwrap_or(AuthLevel::Administrator)
}

/// Check if authentication is required for an operation.
pub fn requires_auth(service: &str, operation: &str) -> bool {
    get_required_level(service, operation) != AuthLevel::Anonymous
}

/// Get all operations for a given service.
pub fn get_service_operations(service: &str) -> Vec<(&'static str, AuthLevel)> {
    AUTH_REQUIREMENTS
        .iter()
        .filter(|((svc, _), _)| *svc == service)
        .map(|((_, op), level)| (*op, *level))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_required_level_device_operations() {
        // Anonymous
        assert_eq!(
            get_required_level("device", "GetSystemDateAndTime"),
            AuthLevel::Anonymous
        );

        // User level
        assert_eq!(
            get_required_level("device", "GetDeviceInformation"),
            AuthLevel::User
        );
        assert_eq!(
            get_required_level("device", "GetCapabilities"),
            AuthLevel::User
        );

        // Operator level
        assert_eq!(
            get_required_level("device", "SetHostname"),
            AuthLevel::Operator
        );

        // Administrator level
        assert_eq!(
            get_required_level("device", "CreateUsers"),
            AuthLevel::Administrator
        );
        assert_eq!(
            get_required_level("device", "SystemReboot"),
            AuthLevel::Administrator
        );
    }

    #[test]
    fn test_get_required_level_ptz_operations() {
        // User level
        assert_eq!(get_required_level("ptz", "GetStatus"), AuthLevel::User);
        assert_eq!(get_required_level("ptz", "GetPresets"), AuthLevel::User);

        // Operator level
        assert_eq!(
            get_required_level("ptz", "ContinuousMove"),
            AuthLevel::Operator
        );
        assert_eq!(get_required_level("ptz", "Stop"), AuthLevel::Operator);
        assert_eq!(get_required_level("ptz", "SetPreset"), AuthLevel::Operator);
    }

    #[test]
    fn test_get_required_level_media_operations() {
        // User level
        assert_eq!(get_required_level("media", "GetProfiles"), AuthLevel::User);
        assert_eq!(get_required_level("media", "GetStreamUri"), AuthLevel::User);

        // Operator level
        assert_eq!(
            get_required_level("media", "CreateProfile"),
            AuthLevel::Operator
        );
        assert_eq!(
            get_required_level("media", "SetVideoEncoderConfiguration"),
            AuthLevel::Operator
        );
    }

    #[test]
    fn test_get_required_level_imaging_operations() {
        // User level
        assert_eq!(
            get_required_level("imaging", "GetImagingSettings"),
            AuthLevel::User
        );

        // Operator level
        assert_eq!(
            get_required_level("imaging", "SetImagingSettings"),
            AuthLevel::Operator
        );
        assert_eq!(get_required_level("imaging", "Move"), AuthLevel::Operator);
    }

    #[test]
    fn test_unknown_operation_requires_admin() {
        // Unknown operations should fail-secure to Administrator
        assert_eq!(
            get_required_level("device", "UnknownOperation"),
            AuthLevel::Administrator
        );
        assert_eq!(
            get_required_level("unknown_service", "SomeOperation"),
            AuthLevel::Administrator
        );
    }

    #[test]
    fn test_auth_level_is_satisfied_by() {
        // Anonymous is satisfied by anyone
        assert!(AuthLevel::Anonymous.is_satisfied_by(None));
        assert!(AuthLevel::Anonymous.is_satisfied_by(Some(UserLevel::User)));

        // User requires any authenticated user
        assert!(!AuthLevel::User.is_satisfied_by(None));
        assert!(AuthLevel::User.is_satisfied_by(Some(UserLevel::User)));
        assert!(AuthLevel::User.is_satisfied_by(Some(UserLevel::Operator)));
        assert!(AuthLevel::User.is_satisfied_by(Some(UserLevel::Administrator)));

        // Operator requires operator or admin
        assert!(!AuthLevel::Operator.is_satisfied_by(None));
        assert!(!AuthLevel::Operator.is_satisfied_by(Some(UserLevel::User)));
        assert!(AuthLevel::Operator.is_satisfied_by(Some(UserLevel::Operator)));
        assert!(AuthLevel::Operator.is_satisfied_by(Some(UserLevel::Administrator)));

        // Administrator requires admin only
        assert!(!AuthLevel::Administrator.is_satisfied_by(None));
        assert!(!AuthLevel::Administrator.is_satisfied_by(Some(UserLevel::User)));
        assert!(!AuthLevel::Administrator.is_satisfied_by(Some(UserLevel::Operator)));
        assert!(AuthLevel::Administrator.is_satisfied_by(Some(UserLevel::Administrator)));
    }

    #[test]
    fn test_requires_auth() {
        assert!(!requires_auth("device", "GetSystemDateAndTime"));
        assert!(requires_auth("device", "GetDeviceInformation"));
        assert!(requires_auth("ptz", "ContinuousMove"));
    }

    #[test]
    fn test_get_service_operations() {
        let ptz_ops = get_service_operations("ptz");
        assert!(!ptz_ops.is_empty());

        // Check that ContinuousMove is in the list
        let continuous_move = ptz_ops.iter().find(|(op, _)| *op == "ContinuousMove");
        assert!(continuous_move.is_some());
        assert_eq!(continuous_move.unwrap().1, AuthLevel::Operator);
    }

    #[test]
    fn test_user_level_to_auth_level() {
        assert_eq!(AuthLevel::from(UserLevel::User), AuthLevel::User);
        assert_eq!(AuthLevel::from(UserLevel::Operator), AuthLevel::Operator);
        assert_eq!(
            AuthLevel::from(UserLevel::Administrator),
            AuthLevel::Administrator
        );
    }
}
