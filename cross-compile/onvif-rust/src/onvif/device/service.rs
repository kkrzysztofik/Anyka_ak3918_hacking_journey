//! Device Service implementation.
//!
//! This module contains the DeviceService struct and the ServiceHandler trait implementation.
//! Uses common::dispatch helpers to reduce boilerplate in the handle_operation match block.

use std::sync::{Arc, OnceLock};

use crate::config::{ConfigRuntime, UserStorage};
use crate::onvif::common::{dispatch_async, dispatch_sync};
use crate::onvif::discovery::WsDiscoveryHandle;
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::device::{
    AddScopes, AddScopesResponse, CreateCertificate, CreateUsers, CreateUsersResponse,
    DeleteCertificates, DeleteUsers, DeleteUsersResponse, GetCapabilities, GetCapabilitiesResponse,
    GetCertificates, GetCertificatesStatus, GetDNS, GetDeviceInformation,
    GetDeviceInformationResponse, GetDiscoveryMode, GetDiscoveryModeResponse, GetHostname,
    GetHostnameResponse, GetNTP, GetNetworkDefaultGateway, GetNetworkInterfaces,
    GetNetworkProtocols, GetRelayOutputs, GetScopes, GetScopesResponse, GetServiceCapabilities,
    GetServiceCapabilitiesResponse, GetServices, GetServicesResponse, GetSystemBackup,
    GetSystemDateAndTime, GetSystemDateAndTimeResponse, GetUsers, GetUsersResponse,
    LoadCertificates, RemoveScopes, RemoveScopesResponse, RestoreSystem, SetDNS, SetDiscoveryMode,
    SetDiscoveryModeResponse, SetHostname, SetHostnameResponse, SetNTP, SetNetworkProtocols,
    SetScopes, SetScopesResponse, SetSystemDateAndTime, SetSystemFactoryDefault, SetUser,
    SetUserResponse, SystemReboot,
};
use crate::platform::Platform;

use async_trait::async_trait;

// Re-export ops modules for crate-internal use
pub(crate) use super::ops::{
    discovery as discovery_ops, network as network_ops, system as system_ops, users as users_ops,
};

pub(crate) use super::store::{DeviceStore, DeviceStoreRef};

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
    /// Platform abstraction (optional for backward compatibility).
    pub(crate) platform: Option<Arc<dyn Platform>>,
    /// WS-Discovery handle, late-bound after the discovery phase starts.
    discovery_handle: Arc<OnceLock<WsDiscoveryHandle>>,
}

impl DeviceService {
    /// Create a new Device Service.
    pub fn new(users: Arc<UserStorage>) -> Self {
        let store = Arc::new(DeviceStore::new(users));

        Self {
            store,
            platform: None,
            discovery_handle: Arc::new(OnceLock::new()),
        }
    }

    /// Bind a late-populated WS-Discovery handle slot.
    pub fn with_discovery_handle(
        mut self,
        discovery_handle: Arc<OnceLock<WsDiscoveryHandle>>,
    ) -> Self {
        self.discovery_handle = discovery_handle;
        self
    }

    /// Create a new Device Service with configuration and platform.
    pub fn with_config_and_platform(
        users: Arc<UserStorage>,
        config: Arc<ConfigRuntime>,
        platform: Arc<dyn Platform>,
    ) -> Self {
        let store = Arc::new(DeviceStore::with_config(users, config));

        Self {
            store,
            platform: Some(platform),
            discovery_handle: Arc::new(OnceLock::new()),
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
        let (ptz_enabled, configured) = {
            let c = self.store.config.read();
            (c.ptz.enabled, c.device.scopes.clone())
        };
        let scopes = discovery_ops::merge_scopes(ptz_enabled, &configured);
        discovery_ops::handle_get_scopes_from_vec(&scopes, request)
    }

    /// Handle SetScopes request.
    pub async fn handle_set_scopes(
        &self,
        request: SetScopes,
    ) -> Result<SetScopesResponse, OnvifError> {
        apply_set_scopes(self, request).await
    }

    /// Handle AddScopes request.
    pub async fn handle_add_scopes(
        &self,
        request: AddScopes,
    ) -> Result<AddScopesResponse, OnvifError> {
        apply_add_scopes(self, request).await
    }

    /// Handle GetDiscoveryMode request.
    pub async fn handle_get_discovery_mode(
        &self,
        _request: GetDiscoveryMode,
    ) -> Result<GetDiscoveryModeResponse, OnvifError> {
        let mode = self.store.config.read().discovery.mode.clone();
        Ok(GetDiscoveryModeResponse {
            discovery_mode: mode,
        })
    }

    /// Handle SetDiscoveryMode request.
    pub async fn handle_set_discovery_mode(
        &self,
        request: SetDiscoveryMode,
    ) -> Result<SetDiscoveryModeResponse, OnvifError> {
        self.store.config.write().discovery.mode = request.discovery_mode.clone();
        if let Some(handle) = self.discovery_handle.get() {
            handle
                .set_discovery_mode(request.discovery_mode.into())
                .await;
        } else {
            tracing::debug!("No WS-Discovery handle; discovery mode persisted for next boot");
        }
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
// Scope mutation helpers (shared by the public handlers and the dispatcher)
// ========================================================================

/// Push merged scopes to WS-Discovery if the handle has been bound.
async fn push_scopes_to_discovery(service: &DeviceService) {
    if let Some(handle) = service.discovery_handle.get() {
        let (ptz_enabled, configured) = {
            let c = service.store.config.read();
            (c.ptz.enabled, c.device.scopes.clone())
        };
        let announced: Vec<String> = discovery_ops::merge_scopes(ptz_enabled, &configured)
            .into_iter()
            .map(|s| s.scope_item)
            .collect();
        handle.set_scopes(announced).await;
    } else {
        tracing::debug!("No WS-Discovery handle; scope change persisted for next boot");
    }
}

/// Replace the configurable scopes. Fixed scopes are derived, never stored.
async fn apply_set_scopes(
    service: &DeviceService,
    request: SetScopes,
) -> Result<SetScopesResponse, OnvifError> {
    for scope in &request.scopes {
        super::validation::validate_scope(scope)?;
    }

    let count = request.scopes.len();
    service.store.config.write().device.scopes = request.scopes;
    tracing::info!("SetScopes: updated to {count} configurable scopes");
    push_scopes_to_discovery(service).await;
    Ok(SetScopesResponse {})
}

/// Append configurable scopes to the current scope list.
async fn apply_add_scopes(
    service: &DeviceService,
    request: AddScopes,
) -> Result<AddScopesResponse, OnvifError> {
    for scope in &request.scope_item {
        super::validation::validate_scope(scope)?;
    }

    let mut configurable = {
        let c = service.store.config.read();
        c.device.scopes.clone()
    };
    for item in request.scope_item {
        if !configurable.contains(&item) {
            configurable.push(item);
        }
    }
    service.store.config.write().device.scopes = configurable;
    push_scopes_to_discovery(service).await;
    Ok(AddScopesResponse {})
}

/// Remove the requested configurable scopes, reporting the ones actually removed.
async fn apply_remove_scopes(
    service: &DeviceService,
    request: RemoveScopes,
) -> Result<RemoveScopesResponse, OnvifError> {
    let (ptz_enabled, mut configurable) = {
        let c = service.store.config.read();
        (c.ptz.enabled, c.device.scopes.clone())
    };
    let fixed = discovery_ops::merge_scopes(ptz_enabled, &[]);
    for scope_item in &request.scope_item {
        if fixed.iter().any(|s| s.scope_item == *scope_item) {
            return Err(super::faults::fixed_scope(scope_item));
        }
    }

    let mut removed = Vec::new();
    for scope_item in &request.scope_item {
        if let Some(pos) = configurable.iter().position(|s| s == scope_item) {
            removed.push(configurable.remove(pos));
        }
    }

    service.store.config.write().device.scopes = configurable;
    tracing::info!("RemoveScopes: removed {} scopes", removed.len());
    push_scopes_to_discovery(service).await;

    Ok(RemoveScopesResponse {
        scope_item: removed,
    })
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
                dispatch_async(body_xml, |request: GetScopes| async {
                    self.handle_get_scopes(request).await
                })
                .await
            }

            "SetScopes" => {
                dispatch_async(body_xml, |request: SetScopes| async {
                    apply_set_scopes(self, request).await
                })
                .await
            }

            "AddScopes" => {
                dispatch_async(body_xml, |request: AddScopes| async {
                    apply_add_scopes(self, request).await
                })
                .await
            }

            "RemoveScopes" => {
                dispatch_async(body_xml, |request: RemoveScopes| async {
                    apply_remove_scopes(self, request).await
                })
                .await
            }

            // Discovery Operations
            "GetDiscoveryMode" => {
                dispatch_async(body_xml, |request: GetDiscoveryMode| async {
                    self.handle_get_discovery_mode(request).await
                })
                .await
            }

            "SetDiscoveryMode" => {
                dispatch_async(body_xml, |request: SetDiscoveryMode| async {
                    self.handle_set_discovery_mode(request).await
                })
                .await
            }

            // User Management Operations
            "GetUsers" => dispatch_sync(body_xml, |request: GetUsers| {
                users_ops::handle_get_users(&users, request)
            }),

            // SECURITY: The dispatcher's auth layer (auth_requirements.rs) enforces
            // Administrator level before this handler is reached. The hardcoded
            // UserLevel::Administrator here is a defense-in-depth assertion, not the
            // primary authorization check. To propagate the actual caller level,
            // the ServiceHandler trait would need to carry auth context.
            // TODO: Extend ServiceHandler::handle_operation to accept caller UserLevel
            "CreateUsers" => dispatch_sync(body_xml, |request: CreateUsers| {
                users_ops::handle_create_users(
                    &users,
                    request,
                    crate::config::UserLevel::Administrator,
                )
            }),

            // SECURITY: The dispatcher's auth layer (auth_requirements.rs) enforces
            // Administrator level before this handler is reached. The hardcoded
            // UserLevel::Administrator here is a defense-in-depth assertion, not the
            // primary authorization check. To propagate the actual caller level,
            // the ServiceHandler trait would need to carry auth context.
            // TODO: Extend ServiceHandler::handle_operation to accept caller UserLevel
            "DeleteUsers" => dispatch_sync(body_xml, |request: DeleteUsers| {
                users_ops::handle_delete_users(
                    &users,
                    request,
                    crate::config::UserLevel::Administrator,
                )
            }),

            // SECURITY: The dispatcher's auth layer (auth_requirements.rs) enforces
            // Administrator level before this handler is reached. The hardcoded
            // UserLevel::Administrator here is a defense-in-depth assertion, not the
            // primary authorization check. To propagate the actual caller level,
            // the ServiceHandler trait would need to carry auth context.
            // TODO: Extend ServiceHandler::handle_operation to accept caller UserLevel
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
        let _password_manager = Arc::new(PasswordManager::new());

        // Create initial admin user with plaintext password
        users
            .create_user("admin", TEST_PASSWORD, UserLevel::Administrator)
            .unwrap();

        DeviceService::new(users)
    }

    fn test_service() -> DeviceService {
        DeviceService::new(Arc::new(UserStorage::new()))
    }

    fn test_service_with_ptz(ptz_enabled: bool) -> DeviceService {
        let mut app = crate::config::AppConfig::default();
        app.ptz.enabled = ptz_enabled;
        DeviceService::with_config_and_platform(
            Arc::new(UserStorage::new()),
            Arc::new(ConfigRuntime::new(app)),
            Arc::new(crate::platform::StubPlatform::new()),
        )
    }

    fn test_service_with_discovery(
        slot: Arc<std::sync::OnceLock<crate::onvif::discovery::WsDiscoveryHandle>>,
    ) -> DeviceService {
        DeviceService::new(Arc::new(UserStorage::new())).with_discovery_handle(slot)
    }

    #[tokio::test]
    async fn test_set_scopes_reaches_discovery_and_bumps_metadata_version() {
        let discovery = crate::onvif::discovery::WsDiscovery::new(
            crate::onvif::discovery::DiscoveryConfig::default(),
        );
        let (handle, task) = discovery.run_service().await.unwrap();

        let slot = Arc::new(std::sync::OnceLock::new());
        assert!(slot.set(handle.clone()).is_ok());
        let service = test_service_with_discovery(slot);

        let before_version = handle.metadata_version();

        apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/Renamed".to_string()],
            },
        )
        .await
        .unwrap();

        let announced = handle.scopes().await;
        assert!(
            announced.iter().any(|s| s.contains("name/Renamed")),
            "SetScopes must change what WS-Discovery announces"
        );
        assert!(
            handle.metadata_version() > before_version,
            "ONVIF Sec. 4.1 requires metadata_version to increment on config change"
        );

        let _ = handle.stop().await;
        task.abort();
    }

    #[tokio::test]
    async fn test_set_scopes_succeeds_when_discovery_is_disabled() {
        let service = test_service_with_discovery(Arc::new(std::sync::OnceLock::new()));

        apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
            },
        )
        .await
        .expect("missing discovery handle is not a fault");

        assert_eq!(
            service.store.config.read().device.scopes,
            vec!["onvif://www.onvif.org/name/Cam"]
        );
    }

    #[tokio::test]
    async fn test_set_scopes_persists_to_config_and_bumps_generation() {
        let service = test_service();
        let before = service.store.config.generation();

        apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
            },
        )
        .await
        .unwrap();

        assert_eq!(
            service.store.config.read().device.scopes,
            vec!["onvif://www.onvif.org/name/Cam"]
        );
        assert!(
            service.store.config.generation() > before,
            "generation must bump so ConfigPersistenceService flushes"
        );
    }

    #[tokio::test]
    async fn test_set_scopes_does_not_store_fixed_scopes() {
        let service = test_service();
        apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
            },
        )
        .await
        .unwrap();

        // Config holds configurable scopes only; fixed ones are derived.
        assert!(
            !service
                .store
                .config
                .read()
                .device
                .scopes
                .iter()
                .any(|s| s.contains("/type/"))
        );

        // ...but GetScopes still reports them.
        let response = service.handle_get_scopes(GetScopes {}).await.unwrap();
        assert!(response.scopes.iter().any(|s| matches!(
            s.scope_def,
            crate::onvif::types::common::ScopeDefinition::Fixed
        )));
    }

    #[tokio::test]
    async fn test_set_discovery_mode_persists_to_config() {
        let service = test_service();
        service
            .handle_set_discovery_mode(SetDiscoveryMode {
                discovery_mode: crate::onvif::types::common::DiscoveryMode::NonDiscoverable,
            })
            .await
            .unwrap();

        assert_eq!(
            service.store.config.read().discovery.mode,
            crate::onvif::types::common::DiscoveryMode::NonDiscoverable
        );
    }

    #[tokio::test]
    async fn test_fixed_scopes_follow_ptz_config_without_stored_state() {
        let service = test_service_with_ptz(false);
        let response = service.handle_get_scopes(GetScopes {}).await.unwrap();
        assert!(
            !response
                .scopes
                .iter()
                .any(|s| s.scope_item.ends_with("/type/ptz"))
        );
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
        let _password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.network.mac_address = "AA:BB:CC:DD:EE:FF".to_string();
            c.network.dhcp_enabled = false;
            c.server.port = 8080;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(users, Arc::new(config), platform);

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
        let _password_manager = Arc::new(PasswordManager::new());
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
        let service = DeviceService::with_config_and_platform(users, Arc::new(config), platform);

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
        let _password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        {
            let mut c = config.write();
            // Use server.address as per canonical external_ip precedence
            c.server.address = "10.0.0.1".to_string();
            c.server.port = 9000;
        }

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(users, Arc::new(config), platform);

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
        let _password_manager = Arc::new(PasswordManager::new());
        let config = ConfigRuntime::new(Default::default());
        // No IP config, should fallback to 127.0.0.1

        let platform = Arc::new(crate::platform::StubPlatform::new());
        let service = DeviceService::with_config_and_platform(users, Arc::new(config), platform);

        let response = system_ops::handle_get_capabilities(
            &service.store.config,
            GetCapabilities { category: vec![] },
        )
        .unwrap();

        let device_caps = response.capabilities.device.unwrap();
        // Should contain 127.0.0.1 or detected IP
        assert!(device_caps.x_addr.contains("http://"));
    }

    // ========================================================================
    // Scope mutation tests (apply_set_scopes / apply_add_scopes / apply_remove_scopes)
    // ========================================================================

    #[tokio::test]
    async fn test_apply_set_scopes_keeps_fixed_replaces_configurable() {
        let service = test_service();
        let fixed_before: Vec<_> = service
            .handle_get_scopes(GetScopes {})
            .await
            .unwrap()
            .scopes
            .into_iter()
            .filter(|s| {
                matches!(
                    s.scope_def,
                    crate::onvif::types::common::ScopeDefinition::Fixed
                )
            })
            .collect();
        assert!(
            !fixed_before.is_empty(),
            "default scopes must include fixed entries"
        );

        let response = apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/NewCamera".to_string()],
            },
        )
        .await
        .unwrap();
        assert_eq!(response, SetScopesResponse {});

        let scopes = service
            .handle_get_scopes(GetScopes {})
            .await
            .unwrap()
            .scopes;
        let fixed_after: Vec<_> = scopes
            .iter()
            .filter(|s| {
                matches!(
                    s.scope_def,
                    crate::onvif::types::common::ScopeDefinition::Fixed
                )
            })
            .collect();
        assert_eq!(fixed_after.len(), fixed_before.len());

        let configurable: Vec<_> = scopes
            .iter()
            .filter(|s| {
                matches!(
                    s.scope_def,
                    crate::onvif::types::common::ScopeDefinition::Configurable
                )
            })
            .collect();
        assert_eq!(configurable.len(), 1);
        assert_eq!(
            configurable[0].scope_item,
            "onvif://www.onvif.org/name/NewCamera"
        );
    }

    #[tokio::test]
    async fn test_apply_set_scopes_rejects_invalid_scope() {
        let service = test_service();
        let result = apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["not-a-valid-scope".to_string()],
            },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_apply_add_scopes_appends_configurable() {
        let service = test_service();
        let before_len = service
            .handle_get_scopes(GetScopes {})
            .await
            .unwrap()
            .scopes
            .len();

        let response = apply_add_scopes(
            &service,
            AddScopes {
                scope_item: vec!["onvif://www.onvif.org/location/Lobby".to_string()],
            },
        )
        .await
        .unwrap();
        assert_eq!(response, AddScopesResponse {});

        let scopes = service
            .handle_get_scopes(GetScopes {})
            .await
            .unwrap()
            .scopes;
        assert_eq!(scopes.len(), before_len + 1);
        assert!(scopes.iter().any(|s| {
            s.scope_item == "onvif://www.onvif.org/location/Lobby"
                && matches!(
                    s.scope_def,
                    crate::onvif::types::common::ScopeDefinition::Configurable
                )
        }));
    }

    #[tokio::test]
    async fn test_apply_remove_scopes_removes_matching_configurable_only() {
        let service = test_service();
        apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
            },
        )
        .await
        .unwrap();

        let configurable_item = "onvif://www.onvif.org/name/Cam".to_string();
        let fixed_item = service
            .handle_get_scopes(GetScopes {})
            .await
            .unwrap()
            .scopes
            .into_iter()
            .find(|s| {
                matches!(
                    s.scope_def,
                    crate::onvif::types::common::ScopeDefinition::Fixed
                )
            })
            .map(|s| s.scope_item)
            .expect("default scopes must include a fixed entry");

        let response = apply_remove_scopes(
            &service,
            RemoveScopes {
                scope_item: vec![
                    configurable_item.clone(),
                    "onvif://www.onvif.org/does/not/exist".to_string(),
                ],
            },
        )
        .await
        .unwrap();

        assert_eq!(response.scope_item, vec![configurable_item.clone()]);

        let remaining = service
            .handle_get_scopes(GetScopes {})
            .await
            .unwrap()
            .scopes;
        assert!(!remaining.iter().any(|s| s.scope_item == configurable_item));
        assert!(remaining.iter().any(|s| s.scope_item == fixed_item));
    }

    #[tokio::test]
    async fn test_remove_scopes_rejects_fixed_scope_with_fault() {
        let service = test_service();
        let error = apply_remove_scopes(
            &service,
            RemoveScopes {
                scope_item: vec!["onvif://www.onvif.org/type/video_encoder".to_string()],
            },
        )
        .await
        .expect_err("removing a fixed scope must fault, not silently no-op");

        assert!(matches!(
            error,
            OnvifError::InvalidArgVal { ref subcode, .. } if subcode == "FixedScope"
        ));
    }

    #[tokio::test]
    async fn test_remove_scopes_reports_removed_configurable_items() {
        let service = test_service();
        apply_set_scopes(
            &service,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/Cam".to_string()],
            },
        )
        .await
        .unwrap();

        let response = apply_remove_scopes(
            &service,
            RemoveScopes {
                scope_item: vec!["onvif://www.onvif.org/name/Cam".to_string()],
            },
        )
        .await
        .unwrap();

        assert_eq!(response.scope_item, vec!["onvif://www.onvif.org/name/Cam"]);
        assert!(service.store.config.read().device.scopes.is_empty());
    }

    #[tokio::test]
    async fn test_add_scopes_is_idempotent() {
        let service = test_service();
        let request = || AddScopes {
            scope_item: vec!["onvif://www.onvif.org/name/Cam".to_string()],
        };

        apply_add_scopes(&service, request()).await.unwrap();
        apply_add_scopes(&service, request()).await.unwrap();

        assert_eq!(
            service.store.config.read().device.scopes.len(),
            1,
            "adding an existing scope must not duplicate it"
        );
    }
}
