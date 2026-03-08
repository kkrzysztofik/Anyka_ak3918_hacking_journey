//! Analytics Service implementation.
//!
//! This module contains the AnalyticsService struct and the ServiceHandler trait implementation.
//! This is a scaffold implementation - most operations return ActionNotSupported.

use async_trait::async_trait;

use crate::onvif::common::dispatch_async;
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};

use super::types::*;

/// ONVIF Analytics Service (scaffold implementation).
///
/// Handles Analytics Service operations including:
/// - GetServiceCapabilities
/// - GetSupportedAnalyticsModules
/// - All other operations return ActionNotSupported
pub struct AnalyticsService;

impl AnalyticsService {
    /// Create a new Analytics Service.
    pub fn new() -> Self {
        Self
    }

    /// Handle GetServiceCapabilities request.
    ///
    /// Returns empty/default capabilities for this scaffold implementation.
    pub async fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        Ok(GetServiceCapabilitiesResponse {
            capabilities: Some(AnalyticsServiceCapabilities {
                // TODO: Accept config to construct dynamic XAddr from runtime address/port
                x_addr: Some("http://localhost/onvif/analytics_service".to_string()),
                rule_support: Some(false),
                analytics_module_support: Some(false),
                cell_based_analytics: Some(false),
            }),
        })
    }

    /// Handle GetSupportedAnalyticsModules request.
    ///
    /// Returns an empty list for this scaffold implementation.
    pub async fn handle_get_supported_analytics_modules(
        &self,
        _request: GetSupportedAnalyticsModules,
    ) -> OnvifResult<GetSupportedAnalyticsModulesResponse> {
        Ok(GetSupportedAnalyticsModulesResponse {
            supported_analytics_modules: Some(SupportedAnalyticsModules {
                analytics_module: Vec::new(),
            }),
        })
    }

    /// Handle CreateAnalyticsModules request (not supported).
    pub async fn handle_create_analytics_modules(
        &self,
        _request: CreateAnalyticsModules,
    ) -> OnvifResult<CreateAnalyticsModulesResponse> {
        Err(OnvifError::ActionNotSupported(
            "CreateAnalyticsModules not supported".to_string(),
        ))
    }

    /// Handle DeleteAnalyticsModules request (not supported).
    pub async fn handle_delete_analytics_modules(
        &self,
        _request: DeleteAnalyticsModules,
    ) -> OnvifResult<DeleteAnalyticsModulesResponse> {
        Err(OnvifError::ActionNotSupported(
            "DeleteAnalyticsModules not supported".to_string(),
        ))
    }

    /// Handle GetAnalyticsModules request (not supported).
    pub async fn handle_get_analytics_modules(
        &self,
        _request: GetAnalyticsModules,
    ) -> OnvifResult<GetAnalyticsModulesResponse> {
        Err(OnvifError::ActionNotSupported(
            "GetAnalyticsModules not supported".to_string(),
        ))
    }

    /// Handle ModifyAnalyticsModules request (not supported).
    pub async fn handle_modify_analytics_modules(
        &self,
        _request: ModifyAnalyticsModules,
    ) -> OnvifResult<ModifyAnalyticsModulesResponse> {
        Err(OnvifError::ActionNotSupported(
            "ModifyAnalyticsModules not supported".to_string(),
        ))
    }
}

impl Default for AnalyticsService {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================================================
// ServiceHandler Implementation for AnalyticsService
// ========================================================================

#[async_trait]
impl ServiceHandler for AnalyticsService {
    /// Handle a SOAP operation for the Analytics Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> OnvifResult<String> {
        tracing::debug!("AnalyticsService handling action: {}", action);

        match action {
            // Service Capabilities
            "GetServiceCapabilities" => {
                dispatch_async(body_xml, |request: GetServiceCapabilities| {
                    self.handle_get_service_capabilities(request)
                })
                .await
            }

            // Analytics Modules
            "GetSupportedAnalyticsModules" => {
                dispatch_async(body_xml, |request: GetSupportedAnalyticsModules| {
                    self.handle_get_supported_analytics_modules(request)
                })
                .await
            }

            "CreateAnalyticsModules" => {
                dispatch_async(body_xml, |request: CreateAnalyticsModules| {
                    self.handle_create_analytics_modules(request)
                })
                .await
            }

            "DeleteAnalyticsModules" => {
                dispatch_async(body_xml, |request: DeleteAnalyticsModules| {
                    self.handle_delete_analytics_modules(request)
                })
                .await
            }

            "GetAnalyticsModules" => {
                dispatch_async(body_xml, |request: GetAnalyticsModules| {
                    self.handle_get_analytics_modules(request)
                })
                .await
            }

            "ModifyAnalyticsModules" => {
                dispatch_async(body_xml, |request: ModifyAnalyticsModules| {
                    self.handle_modify_analytics_modules(request)
                })
                .await
            }

            // Unknown action
            _ => Err(OnvifError::ActionNotSupported(action.to_string())),
        }
    }

    /// Get the service name.
    fn service_name(&self) -> &str {
        "Analytics"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_analytics_service() {
        let service = AnalyticsService::new();
        assert_eq!(service.service_name(), "Analytics");
    }

    #[tokio::test]
    async fn test_service_handler_service_name() {
        let service = AnalyticsService::new();
        assert_eq!(service.service_name(), "Analytics");
    }

    #[tokio::test]
    async fn test_service_handler_unknown_action() {
        let service = AnalyticsService::new();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(
            matches!(&result, Err(OnvifError::ActionNotSupported(action)) if action == "UnknownAction")
        );
    }

    #[tokio::test]
    async fn test_get_service_capabilities() {
        let service = AnalyticsService::new();
        let request = GetServiceCapabilities {};
        let result = service.handle_get_service_capabilities(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.capabilities.is_some());
        let caps = response.capabilities.unwrap();
        assert!(caps.rule_support.is_some());
        assert_eq!(caps.rule_support, Some(false));
    }

    #[tokio::test]
    async fn test_get_supported_analytics_modules() {
        let service = AnalyticsService::new();
        let request = GetSupportedAnalyticsModules::default();
        let result = service
            .handle_get_supported_analytics_modules(request)
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.supported_analytics_modules.is_some());
        let modules = response.supported_analytics_modules.unwrap();
        assert!(modules.analytics_module.is_empty());
    }

    #[tokio::test]
    async fn test_create_analytics_modules_not_supported() {
        let service = AnalyticsService::new();
        let request = CreateAnalyticsModules::default();
        let result = service.handle_create_analytics_modules(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_delete_analytics_modules_not_supported() {
        let service = AnalyticsService::new();
        let request = DeleteAnalyticsModules::default();
        let result = service.handle_delete_analytics_modules(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_get_analytics_modules_not_supported() {
        let service = AnalyticsService::new();
        let request = GetAnalyticsModules::default();
        let result = service.handle_get_analytics_modules(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_modify_analytics_modules_not_supported() {
        let service = AnalyticsService::new();
        let request = ModifyAnalyticsModules::default();
        let result = service.handle_modify_analytics_modules(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_get_service_capabilities_xml() {
        let service = AnalyticsService::new();
        let xml = r#"<GetServiceCapabilities/>"#;
        let result = service
            .handle_operation("GetServiceCapabilities", xml)
            .await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetServiceCapabilitiesResponse"));
        // The response should contain some capabilities
        assert!(response_xml.contains("Capabilities"));
    }

    #[tokio::test]
    async fn test_get_supported_analytics_modules_xml() {
        let service = AnalyticsService::new();
        let xml = r#"<GetSupportedAnalyticsModules><ConfigurationToken>Profile_1</ConfigurationToken></GetSupportedAnalyticsModules>"#;
        let result = service
            .handle_operation("GetSupportedAnalyticsModules", xml)
            .await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetSupportedAnalyticsModulesResponse"));
        assert!(response_xml.contains("SupportedAnalyticsModules"));
    }

    #[tokio::test]
    async fn test_create_analytics_modules_xml_not_supported() {
        let service = AnalyticsService::new();
        let xml = r#"<CreateAnalyticsModules><ConfigurationToken>Profile_1</ConfigurationToken></CreateAnalyticsModules>"#;
        let result = service
            .handle_operation("CreateAnalyticsModules", xml)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_xml() {
        let service = AnalyticsService::new();
        let xml = "<GetServiceCapabilities><Broken";
        let result = service
            .handle_operation("GetServiceCapabilities", xml)
            .await;
        assert!(result.is_err());
    }
}
