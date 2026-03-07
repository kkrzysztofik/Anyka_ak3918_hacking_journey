//! Events Service types.
//!
//! This module defines the request and response types for Events Service operations
//! as defined in the ONVIF Events Service WSDL specification.

use serde::{Deserialize, Serialize};

pub use crate::onvif::common::GetServiceCapabilities;
pub type GetServiceCapabilitiesResponse =
    crate::onvif::common::SharedGetServiceCapabilitiesResponse<EventsServiceCapabilities>;

// ============================================================================
// Service Capabilities
// ============================================================================

/// Events Service capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "EventsServiceCapabilities", rename_all = "PascalCase")]
pub struct EventsServiceCapabilities {
    /// XAddr for the service.
    #[serde(rename = "XAddr", default, skip_serializing_if = "Option::is_none")]
    pub x_addr: Option<String>,

    /// WSSubscriptionPolicy support.
    #[serde(
        rename = "WSSubscriptionPolicySupport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ws_subscription_policy_support: Option<bool>,

    /// WSPullPoint support.
    #[serde(
        rename = "WSPullPointSupport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ws_pull_point_support: Option<bool>,

    /// WSPausableSubscriptionManagerInterface support.
    #[serde(
        rename = "WSPausableSubscriptionManagerInterfaceSupport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub ws_pausable_subscription_manager_interface_support: Option<bool>,

    /// MaxNotificationProducers.
    #[serde(
        rename = "MaxNotificationProducers",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_notification_producers: Option<i32>,

    /// MaxPullPoints.
    #[serde(
        rename = "MaxPullPoints",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_pull_points: Option<i32>,

    /// PersistentNotificationStorage.
    #[serde(
        rename = "PersistentNotificationStorage",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub persistent_notification_storage: Option<bool>,
}

// ============================================================================
// Event Properties
// ============================================================================

/// GetEventProperties request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetEventProperties", rename_all = "PascalCase")]
pub struct GetEventProperties {}

/// GetEventProperties response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetEventPropertiesResponse", rename_all = "PascalCase")]
pub struct GetEventPropertiesResponse {
    /// Topic Namespace Location.
    #[serde(
        rename = "TopicNamespaceLocation",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub topic_namespace_location: Vec<String>,

    /// FixedTopicSet.
    #[serde(
        rename = "FixedTopicSet",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub fixed_topic_set: Option<bool>,

    /// Topic expression dialects.
    #[serde(
        rename = "TopicExpressionDialect",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub topic_expression_dialect: Vec<String>,

    /// Message content filter dialects.
    #[serde(
        rename = "MessageContentFilterDialect",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub message_content_filter_dialect: Vec<String>,

    /// Production limit.
    #[serde(
        rename = "ProductionLimit",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub production_limit: Option<i32>,

    /// Production node.
    #[serde(
        rename = "ProductionNode",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub production_node: Option<String>,
}

// ============================================================================
// Subscriptions
// ============================================================================

/// CreatePullPointSubscription request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreatePullPointSubscription", rename_all = "PascalCase")]
pub struct CreatePullPointSubscription {
    /// Filter for the subscription.
    #[serde(rename = "Filter", default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<TopicFilter>,

    /// Initial termination time.
    #[serde(
        rename = "InitialTerminationTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub initial_termination_time: Option<String>,
}

/// Topic filter.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "TopicFilter", rename_all = "PascalCase")]
pub struct TopicFilter {
    /// Topic expression.
    #[serde(
        rename = "TopicExpression",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub topic_expression: Option<TopicExpression>,
}

/// Topic expression.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "TopicExpression", rename_all = "PascalCase")]
pub struct TopicExpression {
    /// Dialect (XML attribute, not child element).
    #[serde(rename = "@Dialect", default, skip_serializing_if = "Option::is_none")]
    pub dialect: Option<String>,

    /// Content (text).
    #[serde(rename = "$value", default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// CreatePullPointSubscription response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(
    rename = "CreatePullPointSubscriptionResponse",
    rename_all = "PascalCase"
)]
pub struct CreatePullPointSubscriptionResponse {
    /// Subscription reference.
    #[serde(
        rename = "SubscriptionReference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub subscription_reference: Option<SubscriptionReference>,

    /// Current time.
    #[serde(
        rename = "CurrentTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub current_time: Option<String>,

    /// Termination time.
    #[serde(
        rename = "TerminationTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub termination_time: Option<String>,
}

/// Subscription reference.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SubscriptionReference", rename_all = "PascalCase")]
pub struct SubscriptionReference {
    /// Address.
    #[serde(rename = "Address", default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

// ============================================================================
// Pull Messages
// ============================================================================

/// PullMessages request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "PullMessages", rename_all = "PascalCase")]
pub struct PullMessages {
    /// Timeout in seconds.
    #[serde(rename = "Timeout", default, skip_serializing_if = "Option::is_none")]
    pub timeout: Option<String>,

    /// Maximum number of messages.
    #[serde(
        rename = "MaxMessageCount",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub max_message_count: Option<i32>,
}

/// PullMessages response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "PullMessagesResponse", rename_all = "PascalCase")]
pub struct PullMessagesResponse {
    /// Current time.
    #[serde(
        rename = "CurrentTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub current_time: Option<String>,

    /// Termination time.
    #[serde(
        rename = "TerminationTime",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub termination_time: Option<String>,

    /// Messages.
    #[serde(
        rename = "NotificationMessage",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub notification_message: Vec<NotificationMessage>,
}

/// Notification message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "NotificationMessage", rename_all = "PascalCase")]
pub struct NotificationMessage {
    /// Topic.
    #[serde(rename = "Topic", default, skip_serializing_if = "Option::is_none")]
    pub topic: Option<TopicExpression>,

    /// Producer reference.
    #[serde(
        rename = "ProducerReference",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub producer_reference: Option<String>,

    /// Message number.
    #[serde(
        rename = "MessageNumber",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub message_number: Option<u64>,

    /// Message.
    #[serde(rename = "Message", default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// ============================================================================
// Seek
// ============================================================================

/// Seek request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Seek", rename_all = "PascalCase")]
pub struct Seek {
    /// UtcTime.
    #[serde(rename = "UtcTime", default, skip_serializing_if = "Option::is_none")]
    pub utc_time: Option<String>,

    /// ResetPointer.
    #[serde(
        rename = "ResetPointer",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub reset_pointer: Option<bool>,
}

/// Seek response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SeekResponse", rename_all = "PascalCase")]
pub struct SeekResponse {}

// ============================================================================
// SetSynchronizationPoint
// ============================================================================

/// SetSynchronizationPoint request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetSynchronizationPoint", rename_all = "PascalCase")]
pub struct SetSynchronizationPoint {}

/// SetSynchronizationPoint response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SetSynchronizationPointResponse", rename_all = "PascalCase")]
pub struct SetSynchronizationPointResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_capabilities_default() {
        let req = GetServiceCapabilities {};
        // Empty struct - just check it can be created
        let _ = req;
    }

    #[test]
    fn test_get_event_properties_default() {
        let req = GetEventProperties {};
        // Empty struct - just check it can be created
        let _ = req;
    }

    #[test]
    fn test_events_service_capabilities_defaults() {
        let caps = EventsServiceCapabilities::default();
        assert_eq!(caps.x_addr, None);
        assert_eq!(caps.ws_subscription_policy_support, None);
        assert_eq!(caps.ws_pull_point_support, None);
    }

    #[test]
    fn test_create_pull_point_subscription_default() {
        let req = CreatePullPointSubscription::default();
        assert!(req.filter.is_none());
        assert!(req.initial_termination_time.is_none());
    }

    #[test]
    fn test_pull_messages_default() {
        let req = PullMessages::default();
        assert!(req.timeout.is_none());
        assert!(req.max_message_count.is_none());
    }

    #[test]
    fn test_pull_messages_response_empty() {
        let resp = PullMessagesResponse::default();
        assert!(resp.notification_message.is_empty());
    }

    #[test]
    fn test_seek_default() {
        let req = Seek::default();
        assert!(req.utc_time.is_none());
        assert!(req.reset_pointer.is_none());
    }

    #[test]
    fn test_set_synchronization_point_default() {
        let req = SetSynchronizationPoint {};
        // Empty struct - just check it can be created
        let _ = req;
    }

    #[test]
    fn test_notification_message_empty() {
        let msg = NotificationMessage::default();
        assert!(msg.topic.is_none());
        assert!(msg.message_number.is_none());
    }

    #[test]
    fn test_topic_expression_dialect_is_xml_attribute() {
        let expr = TopicExpression {
            dialect: Some("http://www.onvif.org/ver10/tev/topicExpression/ConcreteSet".into()),
            content: Some("tns1:RuleEngine".into()),
        };
        let xml = quick_xml::se::to_string(&expr).expect("serialize TopicExpression");
        // Dialect must be an XML attribute (@Dialect), not a child element.
        // Correct:   <TopicExpression Dialect="...">content</TopicExpression>
        // Incorrect: <TopicExpression><Dialect>...</Dialect>content</TopicExpression>
        assert!(
            xml.contains("Dialect=\""),
            "Dialect should be an XML attribute, got: {xml}"
        );
        assert!(
            !xml.contains("<Dialect>"),
            "Dialect must not be a child element, got: {xml}"
        );
    }
}
