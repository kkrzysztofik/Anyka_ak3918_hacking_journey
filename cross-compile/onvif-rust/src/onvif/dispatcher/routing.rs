//! Routing module for the dispatcher.
//!
//! This module contains the main dispatch logic for routing SOAP requests
//! to appropriate service handlers.

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::Request,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use parking_lot::RwLock;

use crate::onvif::error::OnvifError;
use crate::onvif::soap::{build_soap_response, parse_soap_request};
use crate::utils::validation::SecurityValidator;

use crate::onvif::dispatcher::auth as auth_mod;
use crate::onvif::dispatcher::request_parse::{
    extract_action, extract_soap_action, read_and_parse_request,
};
use crate::onvif::dispatcher::response::{self as response_mod, error_response};
use crate::onvif::dispatcher::{AuthContext, ServiceHandler};

// Re-export ServiceDispatcher from mod.rs
use crate::onvif::dispatcher::ServiceDispatcher;

impl ServiceDispatcher {
    /// Create a new empty dispatcher.
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a service handler.
    pub fn register_service(&self, service_name: &str, handler: Arc<dyn ServiceHandler>) {
        let mut handlers = self.handlers.write();
        handlers.insert(service_name.to_lowercase(), handler);
        tracing::debug!("Registered service handler: {}", service_name);
    }

    /// Dispatch a request to the appropriate service handler.
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

        // CRIT-002: Validate XML security before parsing to prevent XXE, XML bombs, etc.
        let security_validator = SecurityValidator::default();
        if let Err(e) = security_validator.check_xml_security(body_str) {
            tracing::warn!("XML security validation failed: {}", e);
            return error_response(OnvifError::WellFormed(format!(
                "XML security validation failed: {}",
                e
            )));
        }

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
        tracing::debug!(
            "Action extraction: soap_action_header={:?}, envelope_action={:?}",
            soap_action,
            envelope.action
        );
        let action = soap_action
            .clone()
            .or(envelope.action.clone())
            .unwrap_or_default();

        if action.is_empty() {
            tracing::warn!(
                "Missing SOAP action in request (header={:?}, body={:?})",
                soap_action,
                envelope.action
            );
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
                let response_xml = build_soap_response(&response_body);
                (
                    StatusCode::OK,
                    [(header::CONTENT_TYPE, "application/soap+xml; charset=utf-8")],
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

    /// Dispatch a request with authentication.
    pub async fn dispatch_with_auth(
        &self,
        service: &str,
        request: Request<Body>,
        auth_ctx: &AuthContext,
    ) -> Response {
        // Extract SOAP action from header
        let soap_action = extract_soap_action(&request);

        // Check Basic Auth existence and validity (before consuming body)
        let basic_auth_result = if auth_ctx.auth_enabled {
            auth_mod::verify_basic_auth_self(self, &request, auth_ctx)
        } else {
            Ok(None)
        };

        // Read and parse request body
        let envelope = match read_and_parse_request(request).await {
            Ok(env) => env,
            Err(response) => return *response,
        };

        // Determine action (prefer header, fallback to body)
        let action = match extract_action(soap_action, &envelope) {
            Ok(action) => action,
            Err(response) => return *response,
        };

        tracing::debug!(
            "Dispatching {} to service '{}' (auth_enabled: {})",
            action,
            service,
            auth_ctx.auth_enabled
        );

        // Find handler
        let handler = match self.find_handler(service) {
            Ok(handler) => handler,
            Err(response) => return *response,
        };

        // Check authentication if enabled
        if let Err(response) = auth_mod::check_authentication_self(
            self,
            &action,
            &handler,
            &basic_auth_result,
            &envelope,
            auth_ctx,
        )
        .await
        {
            return *response;
        }

        // Handle the operation
        response_mod::handle_operation_self(&handler, &action, &envelope.body_xml).await
    }

    /// Find handler for the given service.
    fn find_handler(&self, service: &str) -> Result<Arc<dyn ServiceHandler>, Box<Response>> {
        let handler = {
            let handlers = self.handlers.read();
            handlers.get(&service.to_lowercase()).cloned()
        };

        match handler {
            Some(h) => Ok(h),
            None => {
                tracing::warn!("No handler registered for service: {}", service);
                Err(Box::new(error_response(OnvifError::ActionNotSupported(
                    format!("Service '{}' not available", service),
                ))))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::auth_requirements::AuthLevel;
    use async_trait::async_trait;
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

    #[test]
    fn test_required_auth_level_default() {
        // TestHandler uses the default implementation which looks up from auth_requirements
        let handler = TestHandler;

        // Since TestHandler's service_name is "Test" and there's no entry for it,
        // the default should be Administrator (fail-secure)
        assert_eq!(
            handler.required_auth_level("GetTest"),
            AuthLevel::Administrator
        );
    }

    // Custom handler that overrides required_auth_level
    struct CustomAuthHandler;

    #[async_trait]
    impl ServiceHandler for CustomAuthHandler {
        async fn handle_operation(&self, _action: &str, _body: &str) -> Result<String, OnvifError> {
            Ok("<Response/>".to_string())
        }

        fn service_name(&self) -> &str {
            "custom"
        }

        fn required_auth_level(&self, action: &str) -> AuthLevel {
            match action {
                "PublicOp" => AuthLevel::Anonymous,
                "AdminOp" => AuthLevel::Administrator,
                _ => AuthLevel::User,
            }
        }
    }

    #[test]
    fn test_required_auth_level_custom_override() {
        let handler = CustomAuthHandler;

        assert_eq!(
            handler.required_auth_level("PublicOp"),
            AuthLevel::Anonymous
        );
        assert_eq!(
            handler.required_auth_level("AdminOp"),
            AuthLevel::Administrator
        );
        assert_eq!(handler.required_auth_level("UnknownOp"), AuthLevel::User);
    }
}
