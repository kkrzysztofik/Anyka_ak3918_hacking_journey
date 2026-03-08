//! Events Service implementation.
//!
//! This module contains the EventsService struct and the ServiceHandler trait implementation.
//! This is a scaffold implementation - most operations return ActionNotSupported.

use async_trait::async_trait;

use crate::onvif::common::dispatch_async;
use crate::onvif::dispatcher::ServiceHandler;
use crate::onvif::error::{OnvifError, OnvifResult};

use super::types::*;

/// ONVIF Events Service (scaffold implementation).
///
/// Handles Events Service operations including:
/// - GetServiceCapabilities
/// - GetEventProperties
/// - All other operations return ActionNotSupported
pub struct EventsService;

impl EventsService {
    /// Create a new Events Service.
    pub fn new() -> Self {
        Self
    }

    /// Handle GetServiceCapabilities request.
    ///
    /// Returns default capabilities for this scaffold implementation.
    pub async fn handle_get_service_capabilities(
        &self,
        _request: GetServiceCapabilities,
    ) -> OnvifResult<GetServiceCapabilitiesResponse> {
        Ok(GetServiceCapabilitiesResponse {
            capabilities: Some(EventsServiceCapabilities {
                // TODO: Accept config to construct dynamic XAddr from runtime address/port
                x_addr: Some("http://localhost/onvif/events_service".to_string()),
                ws_subscription_policy_support: Some(false),
                ws_pull_point_support: Some(false),
                ws_pausable_subscription_manager_interface_support: Some(false),
                max_notification_producers: Some(1),
                max_pull_points: Some(0),
                persistent_notification_storage: Some(false),
            }),
        })
    }

    /// Handle GetEventProperties request.
    ///
    /// Returns basic event properties for this scaffold implementation.
    pub async fn handle_get_event_properties(
        &self,
        _request: GetEventProperties,
    ) -> OnvifResult<GetEventPropertiesResponse> {
        Ok(GetEventPropertiesResponse {
            topic_namespace_location: vec!["http://www.onvif.org/event/topics".to_string()],
            fixed_topic_set: Some(true),
            topic_expression_dialect: vec![
                "http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet".to_string(),
                "http://www.onvif.org/ver10/tev/topicExpression/JsonFilter".to_string(),
            ],
            message_content_filter_dialect: vec![
                "http://www.onvif.org/ver10/tev/messageContentFilter/ItemFilter".to_string(),
            ],
            production_limit: Some(100),
            production_node: Some("onvif://www.onvif.org/ver10/backplane".to_string()),
        })
    }

    /// Handle CreatePullPointSubscription request (not supported).
    pub async fn handle_create_pull_point_subscription(
        &self,
        _request: CreatePullPointSubscription,
    ) -> OnvifResult<CreatePullPointSubscriptionResponse> {
        Err(OnvifError::ActionNotSupported(
            "CreatePullPointSubscription not supported".to_string(),
        ))
    }

    /// Handle PullMessages request (not supported).
    pub async fn handle_pull_messages(
        &self,
        _request: PullMessages,
    ) -> OnvifResult<PullMessagesResponse> {
        Err(OnvifError::ActionNotSupported(
            "PullMessages not supported".to_string(),
        ))
    }

    /// Handle Seek request (not supported).
    pub async fn handle_seek(&self, _request: Seek) -> OnvifResult<SeekResponse> {
        Err(OnvifError::ActionNotSupported(
            "Seek not supported".to_string(),
        ))
    }

    /// Handle SetSynchronizationPoint request (not supported).
    pub async fn handle_set_synchronization_point(
        &self,
        _request: SetSynchronizationPoint,
    ) -> OnvifResult<SetSynchronizationPointResponse> {
        Err(OnvifError::ActionNotSupported(
            "SetSynchronizationPoint not supported".to_string(),
        ))
    }
}

impl Default for EventsService {
    fn default() -> Self {
        Self::new()
    }
}

// ========================================================================
// ServiceHandler Implementation for EventsService
// ========================================================================

#[async_trait]
impl ServiceHandler for EventsService {
    /// Handle a SOAP operation for the Events Service.
    ///
    /// Routes the SOAP action to the appropriate handler method and returns
    /// the serialized XML response.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> OnvifResult<String> {
        tracing::debug!("EventsService handling action: {}", action);

        match action {
            // Service Capabilities
            "GetServiceCapabilities" => {
                dispatch_async(body_xml, |request: GetServiceCapabilities| {
                    self.handle_get_service_capabilities(request)
                })
                .await
            }

            // Event Properties
            "GetEventProperties" => {
                dispatch_async(body_xml, |request: GetEventProperties| {
                    self.handle_get_event_properties(request)
                })
                .await
            }

            // Subscriptions
            "CreatePullPointSubscription" => {
                dispatch_async(body_xml, |request: CreatePullPointSubscription| {
                    self.handle_create_pull_point_subscription(request)
                })
                .await
            }

            // Pull Messages
            "PullMessages" => {
                dispatch_async(body_xml, |request: PullMessages| {
                    self.handle_pull_messages(request)
                })
                .await
            }

            // Seek
            "Seek" => dispatch_async(body_xml, |request: Seek| self.handle_seek(request)).await,

            // Synchronization
            "SetSynchronizationPoint" => {
                dispatch_async(body_xml, |request: SetSynchronizationPoint| {
                    self.handle_set_synchronization_point(request)
                })
                .await
            }

            // Unknown action
            _ => Err(OnvifError::ActionNotSupported(action.to_string())),
        }
    }

    /// Get the service name.
    fn service_name(&self) -> &str {
        "Events"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_events_service() {
        let service = EventsService::new();
        assert_eq!(service.service_name(), "Events");
    }

    #[tokio::test]
    async fn test_service_handler_service_name() {
        let service = EventsService::new();
        assert_eq!(service.service_name(), "Events");
    }

    #[tokio::test]
    async fn test_service_handler_unknown_action() {
        let service = EventsService::new();
        let result = service.handle_operation("UnknownAction", "<test/>").await;
        assert!(result.is_err());
        assert!(
            matches!(&result, Err(OnvifError::ActionNotSupported(action)) if action == "UnknownAction")
        );
    }

    #[tokio::test]
    async fn test_get_service_capabilities() {
        let service = EventsService::new();
        let request = GetServiceCapabilities {};
        let result = service.handle_get_service_capabilities(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.capabilities.is_some());
        let caps = response.capabilities.unwrap();
        assert!(caps.ws_subscription_policy_support.is_some());
        assert_eq!(caps.ws_subscription_policy_support, Some(false));
    }

    #[tokio::test]
    async fn test_get_event_properties() {
        let service = EventsService::new();
        let request = GetEventProperties {};
        let result = service.handle_get_event_properties(request).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(!response.topic_namespace_location.is_empty());
        assert_eq!(
            response.topic_namespace_location[0],
            "http://www.onvif.org/event/topics"
        );
        assert_eq!(response.fixed_topic_set, Some(true));
    }

    #[tokio::test]
    async fn test_create_pull_point_subscription_not_supported() {
        let service = EventsService::new();
        let request = CreatePullPointSubscription::default();
        let result = service.handle_create_pull_point_subscription(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_pull_messages_not_supported() {
        let service = EventsService::new();
        let request = PullMessages::default();
        let result = service.handle_pull_messages(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_seek_not_supported() {
        let service = EventsService::new();
        let request = Seek::default();
        let result = service.handle_seek(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_set_synchronization_point_not_supported() {
        let service = EventsService::new();
        let request = SetSynchronizationPoint {};
        let result = service.handle_set_synchronization_point(request).await;
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[tokio::test]
    async fn test_get_service_capabilities_xml() {
        let service = EventsService::new();
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
    async fn test_get_event_properties_xml() {
        let service = EventsService::new();
        let xml = r#"<GetEventProperties/>"#;
        let result = service.handle_operation("GetEventProperties", xml).await;
        assert!(result.is_ok());
        let response_xml = result.unwrap();
        assert!(response_xml.contains("GetEventPropertiesResponse"));
        assert!(response_xml.contains("TopicNamespaceLocation"));
    }

    #[tokio::test]
    async fn test_create_pull_point_subscription_xml_not_supported() {
        let service = EventsService::new();
        let xml = r#"<CreatePullPointSubscription/>"#;
        let result = service
            .handle_operation("CreatePullPointSubscription", xml)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_xml() {
        let service = EventsService::new();
        let xml = "<GetServiceCapabilities><Broken";
        let result = service
            .handle_operation("GetServiceCapabilities", xml)
            .await;
        assert!(result.is_err());
    }
}
