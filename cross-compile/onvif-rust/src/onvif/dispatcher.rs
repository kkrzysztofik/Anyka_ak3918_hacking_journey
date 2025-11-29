//! Service dispatcher for routing SOAP requests to handlers.
//!
//! The dispatcher is responsible for:
//!
//! - Extracting the SOAP action from request headers or body
//! - Routing requests to the appropriate service handler
//! - Managing service registration
//!
//! # Architecture
//!
//! ```text
//! Request → Extract Action → Find Handler → Execute → Response
//!              ↓                   ↓
//!         SOAPAction header    HashMap<service, Handler>
//!         or Body first elem
//! ```
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::dispatcher::{ServiceDispatcher, ServiceHandler};
//!
//! let mut dispatcher = ServiceDispatcher::new();
//! dispatcher.register_service("device", DeviceServiceHandler::new());
//!
//! let response = dispatcher.dispatch("device", request).await;
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use parking_lot::RwLock;

use super::error::OnvifError;
use super::soap::parse_soap_request;

/// Trait for service handlers that process SOAP operations.
///
/// Each ONVIF service (Device, Media, PTZ, Imaging) implements this trait
/// to handle incoming SOAP requests.
///
/// # Example
///
/// ```ignore
/// use onvif_rust::onvif::dispatcher::ServiceHandler;
///
/// struct DeviceServiceHandler;
///
/// #[async_trait]
/// impl ServiceHandler for DeviceServiceHandler {
///     async fn handle_operation(&self, action: &str, body: &str) -> Result<String, OnvifError> {
///         match action {
///             "GetDeviceInformation" => Ok(self.get_device_information()),
///             _ => Err(OnvifError::ActionNotSupported(action.to_string())),
///         }
///     }
///
///     fn service_name(&self) -> &str {
///         "Device"
///     }
///
///     fn supported_actions(&self) -> Vec<&str> {
///         vec!["GetDeviceInformation", "GetCapabilities", "GetScopes"]
///     }
/// }
/// ```
#[async_trait]
pub trait ServiceHandler: Send + Sync {
    /// Handle a SOAP operation.
    ///
    /// # Arguments
    ///
    /// * `action` - The SOAP action/operation name (e.g., "GetDeviceInformation")
    /// * `body_xml` - The raw SOAP body XML content
    ///
    /// # Returns
    ///
    /// The response body XML on success, or an `OnvifError` on failure.
    async fn handle_operation(&self, action: &str, body_xml: &str) -> Result<String, OnvifError>;

    /// Get the service name for logging and debugging.
    fn service_name(&self) -> &str;

    /// Get the list of actions supported by this service.
    fn supported_actions(&self) -> Vec<&str>;
}

/// Service dispatcher for routing SOAP requests to handlers.
pub struct ServiceDispatcher {
    /// Registered service handlers.
    handlers: RwLock<HashMap<String, Arc<dyn ServiceHandler>>>,
}

impl ServiceDispatcher {
    /// Create a new empty dispatcher.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a service handler.
    ///
    /// # Arguments
    ///
    /// * `service_name` - The service identifier (e.g., "device", "media")
    /// * `handler` - The handler implementation
    pub fn register_service(&self, service_name: &str, handler: Arc<dyn ServiceHandler>) {
        let mut handlers = self.handlers.write();
        handlers.insert(service_name.to_lowercase(), handler);
        tracing::debug!("Registered service handler: {}", service_name);
    }

    /// Dispatch a request to the appropriate service handler.
    ///
    /// # Arguments
    ///
    /// * `service` - The service to dispatch to (e.g., "device", "media")
    /// * `request` - The HTTP request
    ///
    /// # Returns
    ///
    /// An HTTP response with the SOAP response or fault.
    pub async fn dispatch(&self, service: &str, request: Request<Body>) -> Response {
        // Extract SOAP action from header
        let soap_action = extract_soap_action(&request);

        // Read body
        let body_bytes = match axum::body::to_bytes(request.into_body(), 1024 * 1024).await {
            Ok(bytes) => bytes,
            Err(e) => {
                tracing::error!("Failed to read request body: {}", e);
                return error_response(OnvifError::WellFormed(format!(
                    "Failed to read request body: {}",
                    e
                )));
            }
        };

        let body_str = match std::str::from_utf8(&body_bytes) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Invalid UTF-8 in request body: {}", e);
                return error_response(OnvifError::WellFormed(format!(
                    "Invalid UTF-8 in request body: {}",
                    e
                )));
            }
        };

        // Parse SOAP envelope
        let envelope = match parse_soap_request(body_str) {
            Ok(env) => env,
            Err(e) => {
                tracing::error!("Failed to parse SOAP envelope: {}", e);
                return error_response(OnvifError::WellFormed(format!(
                    "Failed to parse SOAP envelope: {}",
                    e
                )));
            }
        };

        // Determine action (prefer header, fallback to body)
        let action = soap_action.or(envelope.action.clone()).unwrap_or_default();

        if action.is_empty() {
            tracing::warn!("Missing SOAP action in request");
            return error_response(OnvifError::WellFormed(
                "Missing SOAP action in request".to_string(),
            ));
        }

        tracing::debug!("Dispatching {} to service '{}'", action, service);

        // Find handler
        let handler = {
            let handlers = self.handlers.read();
            handlers.get(&service.to_lowercase()).cloned()
        };

        let handler = match handler {
            Some(h) => h,
            None => {
                tracing::warn!("No handler registered for service: {}", service);
                return error_response(OnvifError::ActionNotSupported(format!(
                    "Service '{}' not available",
                    service
                )));
            }
        };

        // Handle the operation
        match handler.handle_operation(&action, &envelope.body_xml).await {
            Ok(response_body) => {
                let response_xml = super::soap::build_soap_response(&response_body);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
                    response_xml,
                )
                    .into_response()
            }
            Err(e) => {
                tracing::warn!("Operation {} failed: {:?}", action, e);
                error_response(e)
            }
        }
    }

    /// Check if a service is registered.
    pub fn has_service(&self, service: &str) -> bool {
        let handlers = self.handlers.read();
        handlers.contains_key(&service.to_lowercase())
    }

    /// Get the list of registered services.
    pub fn services(&self) -> Vec<String> {
        let handlers = self.handlers.read();
        handlers.keys().cloned().collect()
    }
}

impl Default for ServiceDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract the SOAPAction from request headers.
///
/// The SOAPAction can be in the `SOAPAction` header or in the
/// `action` parameter of the `Content-Type` header.
fn extract_soap_action(request: &Request<Body>) -> Option<String> {
    // Try SOAPAction header first
    if let Some(action) = request.headers().get("SOAPAction")
        && let Ok(action_str) = action.to_str()
    {
        // Remove quotes if present
        let action = action_str.trim_matches('"');
        // Extract action name from URI if needed
        if let Some(pos) = action.rfind('/') {
            return Some(action[pos + 1..].to_string());
        }
        return Some(action.to_string());
    }

    // Try Content-Type header with action parameter
    if let Some(content_type) = request.headers().get(header::CONTENT_TYPE)
        && let Ok(ct_str) = content_type.to_str()
    {
        // Look for action= parameter
        for part in ct_str.split(';') {
            let part = part.trim();
            if let Some(action) = part.strip_prefix("action=") {
                let action = action.trim_matches('"');
                if let Some(pos) = action.rfind('/') {
                    return Some(action[pos + 1..].to_string());
                }
                return Some(action.to_string());
            }
        }
    }

    None
}

/// Build an error response from an OnvifError.
fn error_response(error: OnvifError) -> Response {
    let status = error.http_status();
    let fault_xml = error.to_soap_fault();

    (
        status,
        [(header::CONTENT_TYPE, "text/xml; charset=utf-8")],
        fault_xml,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;

    #[test]
    fn test_dispatcher_new() {
        let dispatcher = ServiceDispatcher::new();
        assert!(dispatcher.services().is_empty());
    }

    struct TestHandler;

    #[async_trait]
    impl ServiceHandler for TestHandler {
        async fn handle_operation(&self, action: &str, _body: &str) -> Result<String, OnvifError> {
            if action == "GetTest" {
                Ok("<TestResponse/>".to_string())
            } else {
                Err(OnvifError::ActionNotSupported(action.to_string()))
            }
        }

        fn service_name(&self) -> &str {
            "Test"
        }

        fn supported_actions(&self) -> Vec<&str> {
            vec!["GetTest"]
        }
    }

    #[test]
    fn test_register_service() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        assert!(dispatcher.has_service("test"));
        assert!(dispatcher.has_service("TEST")); // Case insensitive
        assert!(!dispatcher.has_service("unknown"));
    }

    #[test]
    fn test_services_list() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));
        dispatcher.register_service("device", Arc::new(TestHandler));

        let services = dispatcher.services();
        assert_eq!(services.len(), 2);
        assert!(services.contains(&"test".to_string()));
        assert!(services.contains(&"device".to_string()));
    }

    #[test]
    fn test_extract_soap_action_from_header() {
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "SOAPAction",
                "\"http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation\"",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_from_content_type() {
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "Content-Type",
                "application/soap+xml; charset=utf-8; action=\"GetDeviceInformation\"",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_none() {
        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "text/xml")
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert!(action.is_none());
    }

    #[test]
    fn test_extract_soap_action_strips_quotes() {
        // Test with double quotes
        let request = HttpRequest::builder()
            .method("POST")
            .header("SOAPAction", "\"GetDeviceInformation\"")
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_extract_soap_action_extracts_last_segment() {
        // Full URI should extract just the action name
        let request = HttpRequest::builder()
            .method("POST")
            .header(
                "SOAPAction",
                "http://www.onvif.org/ver10/device/wsdl/GetCapabilities",
            )
            .body(Body::empty())
            .unwrap();

        let action = extract_soap_action(&request);
        assert_eq!(action, Some("GetCapabilities".to_string()));
    }

    #[tokio::test]
    async fn test_dispatch_no_handler() {
        let dispatcher = ServiceDispatcher::new();

        // Create a simple SOAP request
        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><GetTest/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("nonexistent", request).await;

        // Should return error for missing service
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_with_handler() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><GetTest/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_dispatch_action_not_supported() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body><UnsupportedAction/></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "UnsupportedAction")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for unsupported action
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_invalid_utf8() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        // Invalid UTF-8 bytes
        let invalid_bytes = vec![0xff, 0xfe, 0x00, 0x01];

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(invalid_bytes))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for invalid UTF-8
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_invalid_soap() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        let invalid_soap = "not valid xml at all";

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .header("SOAPAction", "GetTest")
            .body(Body::from(invalid_soap))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for invalid SOAP
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_dispatch_missing_action() {
        let dispatcher = ServiceDispatcher::new();
        dispatcher.register_service("test", Arc::new(TestHandler));

        // SOAP without action in body and no header
        let soap_body = r#"<?xml version="1.0"?>
            <s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
                <s:Body></s:Body>
            </s:Envelope>"#;

        let request = HttpRequest::builder()
            .method("POST")
            .header("Content-Type", "application/soap+xml")
            .body(Body::from(soap_body))
            .unwrap();

        let response = dispatcher.dispatch("test", request).await;

        // Should return error for missing action
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }
}
