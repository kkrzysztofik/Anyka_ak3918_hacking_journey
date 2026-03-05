//! Dispatcher action extraction & routing contract tests.
//!
//! This module tests the ONVIF service dispatcher including:
//! - Action extraction from various HTTP headers
//! - Service routing and handler dispatch
//! - Error response mapping to HTTP status codes and SOAP faults
//! - Response envelope structure
//!
//! ## Test Groups
//!
//! - Group 1: Action extraction precedence
//! - Group 2: Routing contract
//! - Group 3: Error mapping contract
//! - Group 4: Response envelope contract

use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode, header},
    response::Response,
};
use onvif_rust::onvif::dispatcher::{ServiceDispatcher, ServiceHandler};
use onvif_rust::onvif::error::OnvifError;
use std::sync::Arc;

use async_trait::async_trait;

// ============================================================================
// Test Handler Implementation
// ============================================================================

/// A simple test handler for contract tests.
struct TestHandler;

#[async_trait]
impl ServiceHandler for TestHandler {
    async fn handle_operation(&self, action: &str, _body: &str) -> Result<String, OnvifError> {
        match action {
            "GetTest" => Ok("<GetTestResponse/>".to_string()),
            "GetDeviceInformation" => Ok(
                "<GetDeviceInformationResponse><Manufacturer>Test</Manufacturer></GetDeviceInformationResponse>"
                    .to_string(),
            ),
            _ => Err(OnvifError::ActionNotSupported(action.to_string())),
        }
    }

    fn service_name(&self) -> &str {
        "test"
    }
}

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a minimal SOAP envelope for testing.
fn create_soap_envelope(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>{}</s:Body>
</s:Envelope>"#,
        body
    )
}

/// Extract body from response for inspection.
async fn get_response_body(response: Response) -> String {
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    String::from_utf8(body.to_vec()).unwrap()
}

// ============================================================================
// Group 1: Action extraction precedence
// ============================================================================

/// Test that SOAPAction header is preferred over Content-Type action parameter.
#[tokio::test]
async fn test_contract_action_from_soapaction_header_preferred() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // SOAPAction header present
    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "\"http://www.onvif.org/ver10/test/GetTest\"")
        .header(
            header::CONTENT_TYPE,
            "application/soap+xml; action=\"WrongAction\"",
        )
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    // Should succeed with GetTest from SOAPAction header, not WrongAction from Content-Type
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "SOAPAction header should be preferred over Content-Type action"
    );
}

/// Test that action is extracted from Content-Type header's action parameter.
#[tokio::test]
async fn test_contract_action_from_content_type_action_param() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // No SOAPAction, but Content-Type has action parameter
    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header(
            header::CONTENT_TYPE,
            "application/soap+xml; charset=utf-8; action=\"GetTest\"",
        )
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should extract action from Content-Type action parameter"
    );
}

/// Test fallback to body first element when no headers provide action.
#[tokio::test]
async fn test_contract_action_from_body_first_element_fallback() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // No SOAPAction, no Content-Type action, action in body
    let soap_body = create_soap_envelope(
        "<GetDeviceInformation xmlns=\"http://www.onvif.org/ver10/device/wsdl\"/>",
    );

    let request = Request::builder()
        .method(Method::POST)
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    // Should extract GetDeviceInformation from body first element
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should fallback to action from body first element"
    );
}

/// Test that action is extracted from last segment of URI.
#[tokio::test]
async fn test_contract_action_from_uri_extracts_last_segment() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // Full URI in SOAPAction - should extract last segment
    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header(
            "SOAPAction",
            "http://www.onvif.org/ver10/device/wsdl/GetTest",
        )
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should extract GetTest from URI's last segment"
    );

    let body = get_response_body(response).await;
    assert!(
        body.contains("GetTestResponse"),
        "Response should contain expected element"
    );
}

/// Test that quotes are stripped from action string.
#[tokio::test]
async fn test_contract_action_strips_quotes() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // Action with quotes
    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "\"GetTest\"")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Quotes should be stripped from action"
    );
}

/// Test that missing action returns well-formed error response.
#[tokio::test]
async fn test_contract_missing_action_returns_wellformed_error() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // No SOAPAction, no Content-Type action, no action in body (empty body element)
    let soap_body = create_soap_envelope("<EmptyBody/>");

    let request = Request::builder()
        .method(Method::POST)
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    // Should return error for missing action
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = get_response_body(response).await;
    assert!(
        body.contains("Envelope"),
        "Error response should be well-formed SOAP envelope"
    );
    assert!(
        body.contains("Fault"),
        "Error response should contain Fault element"
    );
}

// ============================================================================
// Group 2: Routing contract
// ============================================================================

/// Test that request is dispatched to registered service.
#[tokio::test]
async fn test_contract_dispatch_to_registered_service_succeeds() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Should dispatch to registered service"
    );

    let body = get_response_body(response).await;
    assert!(
        body.contains("GetTestResponse"),
        "Response should contain handler result"
    );
}

/// Test that dispatch to unregistered service returns ActionNotSupported error.
#[tokio::test]
async fn test_contract_dispatch_to_unregistered_service_returns_action_not_supported() {
    let dispatcher = ServiceDispatcher::new();
    // No services registered

    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("unknown_service", request).await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Unregistered service should return BAD_REQUEST"
    );

    let body = get_response_body(response).await;
    assert!(
        body.contains("ActionNotSupported") || body.contains("not available"),
        "Should return ActionNotSupported error"
    );
}

/// Test that service name lookup is case insensitive.
#[tokio::test]
async fn test_contract_service_name_case_insensitive() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    let soap_body = create_soap_envelope("<GetTest/>");

    // Try with uppercase service name
    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("TEST", request).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Service name lookup should be case insensitive"
    );
}

// ============================================================================
// Group 3: Error mapping contract
// ============================================================================

/// Test that well-formed error returns 400.
#[tokio::test]
async fn test_contract_error_response_wellformed_returns_400() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    // Invalid XML - no proper SOAP envelope
    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from("<invalid>"))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Well-formed error should return 400"
    );
}

/// Test that ActionNotSupported error returns 400.
#[tokio::test]
async fn test_contract_error_response_action_not_supported_returns_400() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    let soap_body = create_soap_envelope("<UnsupportedAction/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "UnsupportedAction")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "ActionNotSupported should return 400"
    );

    let body = get_response_body(response).await;
    assert!(
        body.contains("ActionNotSupported"),
        "Should contain ActionNotSupported fault"
    );
}

/// Test that NotAuthorized error returns 401.
#[tokio::test]
async fn test_contract_error_response_not_authorized_returns_401() {
    // Test the OnvifError directly
    let error = OnvifError::NotAuthorized("Test unauthorized".to_string());
    let status = error.http_status();

    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "NotAuthorized should return 401"
    );
}

/// Test that internal error returns 500.
#[tokio::test]
async fn test_contract_error_response_internal_returns_500() {
    // Test the OnvifError directly
    let error = OnvifError::Internal("Internal error".to_string());
    let status = error.http_status();

    assert_eq!(
        status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal error should return 500"
    );
}

/// Test SOAP fault XML structure.
#[tokio::test]
async fn test_contract_soap_fault_xml_structure() {
    // Test the OnvifError SOAP fault structure
    let error = OnvifError::ActionNotSupported("TestAction".to_string());
    let fault_xml = error.to_soap_fault();

    // Verify SOAP fault structure
    assert!(
        fault_xml.contains("Envelope"),
        "Fault should contain Envelope"
    );
    assert!(
        fault_xml.contains("Fault"),
        "Fault should contain Fault element"
    );
    assert!(
        fault_xml.contains("s:Sender"),
        "Fault should contain sender code"
    );
    assert!(
        fault_xml.contains("ActionNotSupported"),
        "Fault should contain ActionNotSupported subcode"
    );
    assert!(
        fault_xml.contains("TestAction"),
        "Fault should contain action name"
    );
}

// ============================================================================
// Group 4: Response envelope contract
// ============================================================================

/// Test that successful response returns 200.
#[tokio::test]
async fn test_contract_success_response_is_200() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Success should return 200"
    );
}

/// Test that success response has correct Content-Type.
#[tokio::test]
async fn test_contract_success_response_content_type_is_soap_xml() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;

    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap())
        .unwrap_or("");

    assert!(
        content_type.contains("application/soap+xml"),
        "Content-Type should be application/soap+xml"
    );
}

/// Test that success response body is a SOAP envelope.
#[tokio::test]
async fn test_contract_success_response_body_is_soap_envelope() {
    let dispatcher = ServiceDispatcher::new();
    dispatcher.register_service("test", Arc::new(TestHandler));

    let soap_body = create_soap_envelope("<GetTest/>");

    let request = Request::builder()
        .method(Method::POST)
        .header("SOAPAction", "GetTest")
        .header(header::CONTENT_TYPE, "application/soap+xml")
        .body(Body::from(soap_body))
        .unwrap();

    let response = dispatcher.dispatch("test", request).await;
    let body = get_response_body(response).await;

    // Verify SOAP envelope structure
    assert!(
        body.contains("Envelope"),
        "Response should contain Envelope element"
    );
    assert!(
        body.contains("Body"),
        "Response should contain Body element"
    );
    assert!(
        body.contains("GetTestResponse"),
        "Response should contain response data"
    );
    // Verify SOAP 1.2 namespace
    assert!(
        body.contains("http://www.w3.org/2003/05/soap-envelope"),
        "Response should use SOAP 1.2 namespace"
    );
}
