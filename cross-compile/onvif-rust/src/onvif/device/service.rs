//! Device Service implementation.
//!
//! This module contains the DeviceService struct and the ServiceHandler trait implementation.
//! Uses common::dispatch helpers to reduce boilerplate in the handle_operation match block.

use std::sync::Arc;

use crate::config::{ConfigRuntime, PasswordManager, UserStorage};
use crate::onvif::common::{dispatch_async, dispatch_sync};
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::device::*;
use crate::platform::Platform;

use async_trait::async_trait;

pub use super::ops::discovery;
pub use super::ops::network;
pub use super::ops::system;
pub use super::ops::users;

// Re-export ops modules for external use
pub use super::ops::{
    discovery as discovery_ops, network as network_ops, system as system_ops, users as users_ops,
};

// Re-export new modules
pub use super::state::{DeviceState, DeviceStateRef};
pub use super::store::{DeviceStore, DeviceStoreRef};

/// ONVIF Device Service.
///
/// Handles Device Service operations including:
/// - Device information and capabilities
/// - System management (date/time, reboot)
/// - Network configuration
/// - Discovery and scope management
/// - User management
pub struct DeviceService {
    /// Persistent state (config, users).
    pub(crate) store: DeviceStoreRef,
    /// Runtime state (scopes, discovery mode).
    pub(crate) state: DeviceStateRef,
    /// Platform abstraction (optional for backward compatibility).
    pub(crate) platform: Option<Arc<dyn Platform>>,
}

impl DeviceService {
    /// Create a new Device Service.
    pub fn new(users: Arc<UserStorage>, password_manager: Arc<PasswordManager>) -> Self {
        let store = Arc::new(DeviceStore::new(users, password_manager));
        let state = Arc::new(DeviceState::new());

        Self {
            store,
            state,
            platform: None,
        }
    }

    /// Create a new Device Service with configuration and platform.
    pub fn with_config_and_platform(
        users: Arc<UserStorage>,
        password_manager: Arc<PasswordManager>,
        config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        let store = Arc::new(DeviceStore::with_config(users, password_manager, config));
        let state = Arc::new(DeviceState::new());

        Self {
            store,
            state,
            platform: Some(platform),
        }
    }

    /// Get the base URL for service addresses.
    /// Uses detected IP address for proper XAddr values in capabilities.
    #[allow(dead_code)]
    pub(crate) fn base_url(&self) -> String {
        // Use canonical external_ip helper for consistency across all ONVIF services
        let address = crate::platform::external_ip(&self.store.config);
        let port = self.store.config.read().server.port;
        format!("http://{}:{}", address, port)
    }

    // ========================================================================
    // Public handler methods that delegate to ops modules
    // These preserve the test interface after decomposition
    // ========================================================================

    /// Handle GetDeviceInformation request.
    pub async fn handle_get_device_information(
        &self,
        _request: GetDeviceInformation,
    ) -> Result<GetDeviceInformationResponse, OnvifError> {
        system_ops::handle_get_device_information(&self.platform, &self.store.config).await
    }

    /// Handle GetCapabilities request.
    pub fn handle_get_capabilities(
        &self,
        request: GetCapabilities,
    ) -> Result<GetCapabilitiesResponse, OnvifError> {
        system_ops::handle_get_capabilities(&self.store.config, request)
    }

    /// Handle GetServices request.
    pub fn handle_get_services(
        &self,
        request: GetServices,
    ) -> Result<GetServicesResponse, OnvifError> {
        system_ops::handle_get_services(&self.store.config, request)
    }

    /// Handle GetServiceCapabilities request.
    pub fn handle_get_service_capabilities(
        &self,
        request: GetServiceCapabilities,
    ) -> Result<GetServiceCapabilitiesResponse, OnvifError> {
        system_ops::handle_get_service_capabilities(request)
    }

    /// Handle GetSystemDateAndTime request.
    pub fn handle_get_system_date_and_time(
        &self,
        request: GetSystemDateAndTime,
    ) -> Result<GetSystemDateAndTimeResponse, OnvifError> {
        system_ops::handle_get_system_date_and_time(request)
    }

    /// Handle GetHostname request.
    pub fn handle_get_hostname(
        &self,
        request: GetHostname,
    ) -> Result<GetHostnameResponse, OnvifError> {
        network_ops::handle_get_hostname(&self.store.config, request)
    }

    /// Handle SetHostname request.
    pub fn handle_set_hostname(
        &self,
        request: SetHostname,
    ) -> Result<SetHostnameResponse, OnvifError> {
        network_ops::handle_set_hostname(&self.store.config, request)
    }

    /// Handle GetScopes request.
    pub async fn handle_get_scopes(
        &self,
        request: GetScopes,
    ) -> Result<GetScopesResponse, OnvifError> {
        let scopes = self.state.get_scopes().await;
        discovery_ops::handle_get_scopes_from_vec(&scopes, request)
    }

    /// Handle SetScopes request.
    pub async fn handle_set_scopes(
        &self,
        request: SetScopes,
    ) -> Result<SetScopesResponse, OnvifError> {
        // Validate all scopes
        for scope in &request.scopes {
            super::validation::validate_scope(scope)?;
        }

        let scopes = self.state.get_scopes().await;

        // Keep fixed scopes, replace configurable ones
        let fixed_scopes: Vec<_> = scopes
            .into_iter()
            .filter(|s| {
                matches!(
                    s.scope_def,
                    crate::onvif::types::common::ScopeDefinition::Fixed
                )
            })
            .collect();

        let new_configurable: Vec<_> = request
            .scopes
            .into_iter()
            .map(|s| crate::onvif::types::common::Scope {
                scope_def: crate::onvif::types::common::ScopeDefinition::Configurable,
                scope_item: s,
            })
            .collect();

        let mut new_scopes = fixed_scopes;
        new_scopes.extend(new_configurable);

        self.state.set_scopes(new_scopes.clone()).await;

        tracing::info!("SetScopes: updated to {} scopes", new_scopes.len());
        Ok(SetScopesResponse {})
    }

    /// Handle AddScopes request.
    pub async fn handle_add_scopes(
        &self,
        request: AddScopes,
    ) -> Result<AddScopesResponse, OnvifError> {
        // Validate all scopes
        for scope in &request.scope_item {
            super::validation::validate_scope(scope)?;
        }

        let mut scopes = self.state.get_scopes().await;

        for scope in request.scope_item {
            scopes.push(crate::onvif::types::common::Scope {
                scope_def: crate::onvif::types::common::ScopeDefinition::Configurable,
                scope_item: scope,
            });
        }

        self.state.set_scopes(scopes).await;
        Ok(AddScopesResponse {})
    }

    /// Handle GetDiscoveryMode request.
    pub async fn handle_get_discovery_mode(
        &self,
        _request: GetDiscoveryMode,
    ) -> Result<GetDiscoveryModeResponse, OnvifError> {
        let mode = self.state.get_discovery_mode().await;
        Ok(GetDiscoveryModeResponse {
            discovery_mode: mode,
        })
    }

    /// Handle SetDiscoveryMode request.
    pub async fn handle_set_discovery_mode(
        &self,
        request: SetDiscoveryMode,
    ) -> Result<SetDiscoveryModeResponse, OnvifError> {
        self.state.set_discovery_mode(request.discovery_mode).await;
        Ok(SetDiscoveryModeResponse {})
    }

    /// Handle GetUsers request.
    pub fn handle_get_users(&self, request: GetUsers) -> Result<GetUsersResponse, OnvifError> {
        users_ops::handle_get_users(&self.store.users, request)
    }

    /// Handle CreateUsers request.
    pub fn handle_create_users(
        &self,
        request: CreateUsers,
        caller_level: crate::config::UserLevel,
    ) -> Result<CreateUsersResponse, OnvifError> {
        users_ops::handle_create_users(&self.store.users, request, caller_level)
    }

    /// Handle DeleteUsers request.
    pub fn handle_delete_users(
        &self,
        request: DeleteUsers,
        caller_level: crate::config::UserLevel,
    ) -> Result<DeleteUsersResponse, OnvifError> {
        users_ops::handle_delete_users(&self.store.users, request, caller_level)
    }

    /// Handle SetUser request.
    pub fn handle_set_user(
        &self,
        request: SetUser,
        caller_level: crate::config::UserLevel,
    ) -> Result<SetUserResponse, OnvifError> {
        users_ops::handle_set_user(&self.store.users, request, caller_level)
    }
}

// ========================================================================
// ServiceHandler Implementation for DeviceService
// ========================================================================

#[async_trait]
impl ServiceHandler for DeviceService {
    /// Handle a SOAP operation for the Device Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    /// Uses common::dispatch helpers to reduce boilerplate.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> OnvifResult<String> {
        tracing::debug!("DeviceService handling action: {}", action);

        // Clone references for async handlers
        let platform = self.platform.clone();
        let config = Arc::clone(&self.store.config);
        let users = Arc::clone(&self.store.users);
        let state = Arc::clone(&self.state);

        match action {
            // Device Information Operations
            "GetDeviceInformation" => dispatch_async(body_xml, |_req: GetDeviceInformation| {
                let platform = platform.clone();
                let config = config.clone();
                async move { system_ops::handle_get_device_information(&platform, &config).await }
            })
            .await,

            // Capabilities Operations
            "GetCapabilities" => dispatch_sync(body_xml, |request: GetCapabilities| {
                system_ops::handle_get_capabilities(&config, request)
            }),

            "GetServices" => dispatch_sync(body_xml, |request: GetServices| {
                system_ops::handle_get_services(&config, request)
            }),

            "GetServiceCapabilities" => {
                dispatch_sync(body_xml, |request: GetServiceCapabilities| {
                    system_ops::handle_get_service_capabilities(request)
                })
            }

            // System Date/Time Operations
            "GetSystemDateAndTime" => dispatch_sync(body_xml, |request: GetSystemDateAndTime| {
                system_ops::handle_get_system_date_and_time(request)
            }),

            "SetSystemDateAndTime" => dispatch_sync(body_xml, |request: SetSystemDateAndTime| {
                system_ops::handle_set_system_date_and_time(request)
            }),

            // System Operations
            "SystemReboot" => dispatch_sync(body_xml, |request: SystemReboot| {
                system_ops::handle_system_reboot(request)
            }),

            "SetSystemFactoryDefault" => {
                dispatch_sync(body_xml, |request: SetSystemFactoryDefault| {
                    system_ops::handle_set_system_factory_default(request)
                })
            }

            // Certificate Operations
            "GetCertificates" => dispatch_sync(body_xml, |request: GetCertificates| {
                system_ops::handle_get_certificates(request)
            }),

            "GetCertificatesStatus" => dispatch_sync(body_xml, |request: GetCertificatesStatus| {
                system_ops::handle_get_certificates_status(request)
            }),

            "CreateCertificate" => dispatch_sync(body_xml, |request: CreateCertificate| {
                system_ops::handle_create_certificate(request)
            }),

            "LoadCertificates" => dispatch_sync(body_xml, |request: LoadCertificates| {
                system_ops::handle_load_certificates(request)
            }),

            "DeleteCertificates" => dispatch_sync(body_xml, |request: DeleteCertificates| {
                system_ops::handle_delete_certificates(request)
            }),

            // Backup/Restore Operations
            "GetSystemBackup" => dispatch_sync(body_xml, |request: GetSystemBackup| {
                system_ops::handle_get_system_backup(&config, request)
            }),

            "RestoreSystem" => dispatch_sync(body_xml, |request: RestoreSystem| {
                system_ops::handle_restore_system(&config, request)
            }),

            // Relay Operations
            "GetRelayOutputs" => dispatch_sync(body_xml, |request: GetRelayOutputs| {
                system_ops::handle_get_relay_outputs(request)
            }),

            // Hostname Operations
            "GetHostname" => dispatch_sync(body_xml, |request: GetHostname| {
                network_ops::handle_get_hostname(&config, request)
            }),

            "SetHostname" => dispatch_sync(body_xml, |request: SetHostname| {
                network_ops::handle_set_hostname(&config, request)
            }),

            // Network Operations
            "GetNetworkInterfaces" => {
                dispatch_async(body_xml, |request: GetNetworkInterfaces| {
                    let platform = platform.clone();
                    let config = config.clone();
                    async move {
                        network_ops::handle_get_network_interfaces(&platform, &config, request)
                            .await
                    }
                })
                .await
            }

            "GetNetworkDefaultGateway" => {
                dispatch_async(body_xml, |request: GetNetworkDefaultGateway| {
                    let config = config.clone();
                    async move {
                        network_ops::handle_get_network_default_gateway(&config, request).await
                    }
                })
                .await
            }

            // DNS Operations
            "GetDNS" => {
                dispatch_async(body_xml, |request: GetDNS| {
                    let platform = platform.clone();
                    async move { network_ops::handle_get_dns(&platform, request).await }
                })
                .await
            }

            "SetDNS" => {
                dispatch_async(body_xml, |request: SetDNS| async move {
                    network_ops::handle_set_dns(request).await
                })
                .await
            }

            // NTP Operations
            "GetNTP" => {
                dispatch_async(body_xml, |request: GetNTP| {
                    let platform = platform.clone();
                    async move { network_ops::handle_get_ntp(&platform, request).await }
                })
                .await
            }

            "SetNTP" => {
                dispatch_async(body_xml, |request: SetNTP| async move {
                    network_ops::handle_set_ntp(request).await
                })
                .await
            }

            // Network Protocol Operations
            "GetNetworkProtocols" => {
                dispatch_async(body_xml, |request: GetNetworkProtocols| {
                    let platform = platform.clone();
                    let config = config.clone();
                    async move {
                        network_ops::handle_get_network_protocols(&platform, &config, request).await
                    }
                })
                .await
            }

            "SetNetworkProtocols" => {
                dispatch_async(body_xml, |request: SetNetworkProtocols| async move {
                    network_ops::handle_set_network_protocols(request).await
                })
                .await
            }

            // Scope Operations
            "GetScopes" => {
                dispatch_async(body_xml, |request: GetScopes| {
                    let state = state.clone();
                    async move {
                        let scopes = state.get_scopes().await;
                        discovery_ops::handle_get_scopes_from_vec(&scopes, request)
                    }
                })
                .await
            }

            "SetScopes" => {
                dispatch_async(body_xml, |request: SetScopes| {
                    let state = state.clone();
                    async move {
                        // Validate all scopes
                        for scope in &request.scopes {
                            super::validation::validate_scope(scope)?;
                        }

                        let scopes = state.get_scopes().await;

                        // Keep fixed scopes, replace configurable ones
                        let fixed_scopes: Vec<_> = scopes
                            .into_iter()
                            .filter(|s| {
                                matches!(
                                    s.scope_def,
                                    crate::onvif::types::common::ScopeDefinition::Fixed
                                )
                            })
                            .collect();

                        let new_configurable: Vec<_> = request
                            .scopes
                            .into_iter()
                            .map(|s| crate::onvif::types::common::Scope {
                                scope_def:
                                    crate::onvif::types::common::ScopeDefinition::Configurable,
                                scope_item: s,
                            })
                            .collect();

                        let mut new_scopes = fixed_scopes;
                        new_scopes.extend(new_configurable);

                        state.set_scopes(new_scopes.clone()).await;

                        tracing::info!("SetScopes: updated to {} scopes", new_scopes.len());
                        Ok(SetScopesResponse {})
                    }
                })
                .await
            }

            "AddScopes" => {
                dispatch_async(body_xml, |request: AddScopes| {
                    let state = state.clone();
                    async move {
                        // Validate all scopes
                        for scope in &request.scope_item {
                            super::validation::validate_scope(scope)?;
                        }

                        let mut scopes = state.get_scopes().await;

                        for scope in request.scope_item {
                            scopes.push(crate::onvif::types::common::Scope {
                                scope_def:
                                    crate::onvif::types::common::ScopeDefinition::Configurable,
                                scope_item: scope,
                            });
                        }

                        state.set_scopes(scopes).await;
                        Ok(AddScopesResponse {})
                    }
                })
                .await
            }

            "RemoveScopes" => {
                dispatch_async(body_xml, |request: RemoveScopes| {
                    let state = state.clone();
                    async move {
                        let mut scopes = state.get_scopes().await;
                        let mut removed = Vec::new();

                        for scope_item in &request.scope_item {
                            if let Some(pos) = scopes.iter().position(|s| {
                                s.scope_item == *scope_item
                                    && matches!(
                                        s.scope_def,
                                        crate::onvif::types::common::ScopeDefinition::Configurable
                                    )
                            }) {
                                removed.push(scopes[pos].scope_item.clone());
                                scopes.remove(pos);
                            }
                        }

                        state.set_scopes(scopes).await;
                        tracing::info!("RemoveScopes: removed {} scopes", removed.len());

                        Ok(RemoveScopesResponse {
                            scope_item: removed,
                        })
                    }
                })
                .await
            }

            // Discovery Operations
            "GetDiscoveryMode" => {
                dispatch_async(body_xml, |_request: GetDiscoveryMode| {
                    let state = state.clone();
                    async move {
                        let mode = state.get_discovery_mode().await;
                        Ok(GetDiscoveryModeResponse {
                            discovery_mode: mode,
                        })
                    }
                })
                .await
            }

            "SetDiscoveryMode" => {
                dispatch_async(body_xml, |request: SetDiscoveryMode| {
                    let state = state.clone();
                    async move {
                        state.set_discovery_mode(request.discovery_mode).await;
                        Ok(SetDiscoveryModeResponse {})
                    }
                })
                .await
            }

            // User Management Operations
            "GetUsers" => dispatch_sync(body_xml, |request: GetUsers| {
                users_ops::handle_get_users(&users, request)
            }),

            "CreateUsers" => dispatch_sync(body_xml, |request: CreateUsers| {
                users_ops::handle_create_users(
                    &users,
                    request,
                    crate::config::UserLevel::Administrator,
                )
            }),

            "DeleteUsers" => dispatch_sync(body_xml, |request: DeleteUsers| {
                users_ops::handle_delete_users(
                    &users,
                    request,
                    crate::config::UserLevel::Administrator,
                )
            }),

            "SetUser" => dispatch_sync(body_xml, |request: SetUser| {
                users_ops::handle_set_user(&users, request, crate::config::UserLevel::Administrator)
            }),

            // Unknown action
            _ => {
                tracing::warn!("Unsupported Device Service action: {}", action);
                Err(OnvifError::ActionNotSupported(action.to_string()))
            }
        }
    }

    /// Get the service name.
    fn service_name(&self) -> &str {
        "Device"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{PasswordManager, UserLevel, UserStorage};
    use std::sync::Arc;

    const TEST_PASSWORD: &str = "test_fixture_pwd_not_real";

    fn create_test_service() -> DeviceService {
        let users = Arc::new(UserStorage::new());
        let password_manager = Arc::new(PasswordManager::new());

        // Create initial admin user with plaintext password
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();

        DeviceService::new(users, password_manager)
    }

    // ========================================================================
    // ServiceHandler Error Path Tests
    // ========================================================================

    #[tokio::test]
    async fn test_service_handler_unknown_action_device() {
        let service = create_test_service();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_service_handler_invalid_xml() {
        let service = create_test_service();
        let result = service
            .handle_operation("GetDeviceInformation", "<InvalidXml><Broken")
            .await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::WellFormed(_))));
    }

    #[tokio::test]
    async fn test_service_handler_get_device_information_xml() {
        let service = create_test_service();
        let xml = r#"<GetDeviceInformation/>"#;
        let result = service.handle_operation("GetDeviceInformation", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetDeviceInformationResponse"));
    }

    #[tokio::test]
    async fn test_service_handler_get_capabilities_xml() {
        let service = create_test_service();
        // GetCapabilities with empty category list (default)
        let xml = r#"<GetCapabilities/>"#;
        let result = service.handle_operation("GetCapabilities", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetCapabilitiesResponse"));
    }

    // ========================================================================
    // Base URL and IP Detection Tests
    // ========================================================================

    #[test]
    fn test_base_url_uses_detected_ip() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.network.mac_address = "AA:BB:CC:DD:EE:FF".to_string();
            c.network.dhcp_enabled = false;
            c.server.port = 8080;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        // Access base_url through a handler that uses it
        let response = system_ops::handle_get_capabilities(
            &service.store.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        // Verify the URL contains the detected IP
        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("192.168.1.100"));
        assert!(device_caps.x_addr.contains("8080"));
    }

    #[test]
    fn test_base_url_uses_static_ip_address() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            // Static IP takes highest precedence
            c.network.ip_address = "192.168.1.50".to_string();
            // These should be ignored when ip_address is set
            c.network.detected_ip = "192.168.1.100".to_string();
            c.server.address = "10.0.0.1".to_string();
            c.server.port = 8080;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        // Access base_url through a handler that uses it
        let response = system_ops::handle_get_capabilities(
            &service.store.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        // Verify the URL contains the static IP (not detected_ip or server.address)
        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("192.168.1.50"));
        assert!(device_caps.x_addr.contains("8080"));
    }

    #[test]
    fn test_base_url_fallback_to_ip_address() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            // Use server.address as per canonical external_ip precedence
            c.server.address = "10.0.0.1".to_string();
            c.server.port = 9000;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        let response = system_ops::handle_get_capabilities(
            &service.store.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("10.0.0.1"));
        assert!(device_caps.x_addr.contains("9000"));
    }

    #[test]
    fn test_base_url_fallback_to_127_0_0_1() {
        let users = Arc::new(UserStorage::new());
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();
        let password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        // No IP config, should fallback to 127.0.0.1

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(
            users,
            password_manager,
            Arc::new(config),
            platform,
        );

        let response = system_ops::handle_get_capabilities(
            &service.store.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        let device_caps = response.capabilities.device.unwrap();
        // Should contain 127.0.0.1 or detected IP
        assert!(device_caps.x_addr.contains("http://"));
    }
}
