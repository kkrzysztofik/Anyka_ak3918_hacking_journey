//! Dispatch helpers to reduce boilerplate in ONVIF service handlers.
//!
//! This module provides helper functions for dispatching SOAP requests
//! to handlers, reducing the boilerplate needed in each service implementation.
//!
//! # Response contract
//!
//! [`dispatch_sync`] and [`dispatch_async`] return **SOAP body XML only** (the serialized
//! operation response element). The HTTP/SOAP layer wraps that with
//! [`crate::onvif::soap::build_soap_response`] exactly once in
//! [`crate::onvif::dispatcher::routing`] / [`crate::onvif::dispatcher::response`].
//! Do not call `build_soap_response` on the return value of these helpers.
//!
//! # Usage
//!
//! These helpers combine XML parsing, validation, and response serialization
//! into a single function call.
//!
//! ## Sync Dispatch
//!
//! For handlers that don't need async:
//!
//! ```
//! use onvif_rust::onvif::common::dispatch_sync;
//! use onvif_rust::onvif::error::OnvifResult;
//!
//! #[derive(serde::Deserialize)]
//! struct GetDeviceInformation;
//!
//! #[derive(serde::Serialize)]
//! struct GetDeviceInformationResponse {
//!     manufacturer: String,
//!     model: String,
//! }
//!
//! fn handle_get_device_info(_req: GetDeviceInformation) -> OnvifResult<GetDeviceInformationResponse> {
//!     Ok(GetDeviceInformationResponse {
//!         manufacturer: "Anyka".to_string(),
//!         model: "AK3918".to_string(),
//!     })
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let body_xml = r#"<GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#;
//! let body_xml_out = dispatch_sync(body_xml, handle_get_device_info)?;
//! assert!(body_xml_out.contains("Anyka"));
//! # Ok(())
//! # }
//! ```
//!
//! ## Async Dispatch
//!
//! For handlers that need async (most real handlers):
//!
//! ```
//! use onvif_rust::onvif::common::dispatch_async;
//! use onvif_rust::onvif::error::OnvifResult;
//! use tokio;
//!
//! #[derive(serde::Deserialize)]
//! struct GetProfiles;
//!
//! #[derive(serde::Serialize)]
//! struct GetProfilesResponse {
//!     profiles: Vec<Profile>,
//! }
//!
//! #[derive(serde::Serialize)]
//! struct Profile {
//!     token: String,
//! }
//!
//! async fn handle_get_profiles(_req: GetProfiles) -> OnvifResult<GetProfilesResponse> {
//!     Ok(GetProfilesResponse {
//!         profiles: vec![Profile { token: "Main".to_string() }],
//!     })
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let body_xml = r#"<GetProfiles xmlns="http://www.onvif.org/ver10/media/wsdl"/>"#;
//! let body_xml_out: String = tokio::runtime::Runtime::new()?.block_on(async {
//!     dispatch_async(body_xml, handle_get_profiles).await
//! })?;
//! assert!(body_xml_out.contains("Main"));
//! # Ok(())
//! # }
//! ```

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::onvif::dispatcher::parse_body;
use crate::onvif::error::{OnvifError, OnvifResult};

// Test types for module testing - crate-internal only
#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
pub(crate) struct TestRequestDispatch {
    pub(crate) value: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
pub(crate) struct TestResponseDispatch {
    pub(crate) result: String,
}

/// Synchronous dispatch helper for SOAP request handling.
///
/// This function combines:
/// 1. Parsing the XML body into the request type
/// 2. Calling the handler function
/// 3. Serializing the response to XML
///
/// # Arguments
///
/// * `body_xml` - The raw SOAP body XML content
/// * `handler` - A synchronous function that takes the deserialized request
///   and returns a result with the response type
///
/// # Type Parameters
///
/// * `Req` - The request type to deserialize from XML
/// * `Resp` - The response type to serialize to XML
///
/// # Returns
///
/// A `Result` containing the serialized **SOAP body** XML (response element only), or an `OnvifError`.
///
/// # Errors
///
/// Returns an error if:
/// - The request XML cannot be parsed into the expected `Req` type
///   (`OnvifError::WellFormed`).
/// - The handler function returns an error (propagated as-is).
/// - The response cannot be serialized to XML (`OnvifError::Internal`).
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::dispatch_sync;
/// use onvif_rust::onvif::error::OnvifResult;
///
/// #[derive(serde::Deserialize)]
/// struct GetHostname;
///
/// #[derive(serde::Serialize)]
/// struct GetHostnameResponse {
///     hostname: String,
/// }
///
/// fn handle(req: GetHostname) -> OnvifResult<GetHostnameResponse> {
///     Ok(GetHostnameResponse { hostname: "camera".to_string() })
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let xml = r#"<GetHostname xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#;
/// let body_xml_out = dispatch_sync(xml, handle)?;
/// assert!(body_xml_out.contains("camera"));
/// # Ok(())
/// # }
/// ```
pub fn dispatch_sync<Req, Resp>(
    body_xml: &str,
    handler: impl FnOnce(Req) -> OnvifResult<Resp>,
) -> OnvifResult<String>
where
    Req: DeserializeOwned,
    Resp: Serialize,
{
    // Parse the request from XML
    let request = parse_body::<Req>(body_xml)?;

    // Call the handler
    let response = handler(request)?;

    // Serialize the response to XML
    let response_xml = quick_xml::se::to_string(&response).map_err(|e| {
        tracing::error!("SOAP response serialization failed: {}", e);
        OnvifError::Internal("Internal processing error".to_string())
    })?;

    Ok(response_xml)
}

/// Asynchronous dispatch helper for SOAP request handling.
///
/// This function combines:
/// 1. Parsing the XML body into the request type
/// 2. Calling the async handler function
/// 3. Serializing the response to XML
///
/// # Arguments
///
/// * `body_xml` - The raw SOAP body XML content
/// * `handler` - An async function that takes the deserialized request
///   and returns a future that resolves to a result with the response type
///
/// # Type Parameters
///
/// * `Req` - The request type to deserialize from XML
/// * `Resp` - The response type to serialize to XML
/// * `F` - The async handler function type
/// * `Fut` - The future type returned by the handler
///
/// # Returns
///
/// A `Result` containing the serialized **SOAP body** XML (response element only), or an `OnvifError`.
///
/// # Errors
///
/// Returns an error if:
/// - The request XML cannot be parsed into the expected `Req` type
///   (`OnvifError::WellFormed`).
/// - The async handler returns an error (propagated as-is).
/// - The response cannot be serialized to XML (`OnvifError::Internal`).
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::dispatch_async;
/// use onvif_rust::onvif::error::OnvifResult;
/// use tokio;
///
/// #[derive(serde::Deserialize)]
/// struct GetProfiles;
///
/// #[derive(serde::Serialize)]
/// struct Profile {
///     token: String,
/// }
///
/// #[derive(serde::Serialize)]
/// struct GetProfilesResponse {
///     profiles: Vec<Profile>,
/// }
///
/// async fn handle(req: GetProfiles) -> OnvifResult<GetProfilesResponse> {
///     Ok(GetProfilesResponse {
///         profiles: vec![Profile { token: "main".to_string() }],
///     })
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let xml = r#"<GetProfiles xmlns="http://www.onvif.org/ver10/media/wsdl"/>"#;
/// let body_xml_out: String = tokio::runtime::Runtime::new()?.block_on(async {
///     dispatch_async(xml, handle).await
/// })?;
/// assert!(body_xml_out.contains("main"));
/// # Ok(())
/// # }
/// ```
pub async fn dispatch_async<Req, Resp, F, Fut>(body_xml: &str, handler: F) -> OnvifResult<String>
where
    Req: DeserializeOwned,
    Resp: Serialize,
    F: FnOnce(Req) -> Fut,
    Fut: std::future::Future<Output = OnvifResult<Resp>>,
{
    // Parse the request from XML
    let request = parse_body::<Req>(body_xml)?;

    // Call the async handler
    let response = handler(request).await?;

    // Serialize the response to XML
    let response_xml = quick_xml::se::to_string(&response).map_err(|e| {
        tracing::error!("SOAP response serialization failed: {}", e);
        OnvifError::Internal("Internal processing error".to_string())
    })?;

    Ok(response_xml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::soap::build_soap_response;

    // Test request/response types
    #[derive(serde::Deserialize, Debug, PartialEq)]
    struct TestRequest {
        #[serde(rename = "Namespace", default)]
        namespace: Option<String>,
    }

    #[derive(serde::Serialize, Debug, PartialEq)]
    struct TestResponse {
        #[serde(rename = "Result")]
        result: String,
    }

    // Test dispatch_sync

    #[test]
    fn test_dispatch_sync_success() {
        fn handler(req: TestRequest) -> OnvifResult<TestResponse> {
            Ok(TestResponse {
                result: format!("handled: {:?}", req.namespace),
            })
        }

        let xml = r#"<TestRequest xmlns="http://test.example.com/"><Namespace>test-ns</Namespace></TestRequest>"#;
        let result = dispatch_sync(xml, handler);

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("handled"));
        assert!(response.contains("test-ns"));
        assert!(
            !response.contains("Envelope"),
            "dispatch returns body fragment only"
        );
    }

    #[test]
    fn test_dispatch_sync_invalid_xml() {
        fn handler(_req: TestRequest) -> OnvifResult<TestResponse> {
            Ok(TestResponse {
                result: "should not reach".to_string(),
            })
        }

        let xml = "not valid xml at all";
        let result = dispatch_sync(xml, handler);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Invalid request XML")
                || err.to_string().contains("WellFormed")
        );
    }

    #[test]
    fn test_dispatch_sync_serialize_error() {
        // Request that can be parsed
        #[derive(serde::Deserialize)]
        struct SimpleRequest;

        // Response that cannot be serialized (contains non-serializable type)
        #[derive(serde::Serialize)]
        struct BadResponse {
            #[serde(serialize_with = "serialize_fails")]
            bad_field: (),
        }

        fn serialize_fails<S>(_value: &(), _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom(
                "intentional serialization failure",
            ))
        }

        fn handler(_req: SimpleRequest) -> OnvifResult<BadResponse> {
            Ok(BadResponse { bad_field: () })
        }

        let xml = r#"<SimpleRequest xmlns="http://test.example.com/"/>"#;
        let result = dispatch_sync(xml, handler);

        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error message is generic to avoid leaking serializer internals to clients
        assert!(err.to_string().contains("Internal processing error"));
    }

    // Test dispatch_async

    #[tokio::test]
    async fn test_dispatch_async_success() {
        async fn handler(req: TestRequest) -> OnvifResult<TestResponse> {
            Ok(TestResponse {
                result: format!("async handled: {:?}", req.namespace),
            })
        }

        let xml = r#"<TestRequest xmlns="http://test.example.com/"><Namespace>async-test</Namespace></TestRequest>"#;
        let result = dispatch_async(xml, handler).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("async handled"));
        assert!(response.contains("async-test"));
    }

    #[tokio::test]
    async fn test_dispatch_async_invalid_xml() {
        async fn handler(_req: TestRequest) -> OnvifResult<TestResponse> {
            Ok(TestResponse {
                result: "should not reach".to_string(),
            })
        }

        let xml = "not valid xml";
        let result = dispatch_async(xml, handler).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dispatch_async_handler_error() {
        // Request that parses successfully
        #[derive(serde::Deserialize)]
        struct RequestWithData {
            value: i32,
        }

        // Handler that returns an error
        async fn handler(req: RequestWithData) -> OnvifResult<TestResponse> {
            if req.value < 0 {
                Err(OnvifError::invalid_arg(
                    "NegativeValue",
                    "Value must be non-negative",
                ))
            } else {
                Ok(TestResponse {
                    result: "ok".to_string(),
                })
            }
        }

        let xml = r#"<RequestWithData xmlns="http://test.example.com/"><value>-5</value></RequestWithData>"#;
        let result = dispatch_async(xml, handler).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("NegativeValue"));
    }

    #[tokio::test]
    async fn test_dispatch_async_serialize_error() {
        #[derive(serde::Deserialize)]
        struct SimpleRequest;

        #[derive(serde::Serialize)]
        struct BadResponse {
            #[serde(serialize_with = "serialize_fails")]
            field: (),
        }

        fn serialize_fails<S>(_value: &(), _serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            Err(serde::ser::Error::custom("async serialization failure"))
        }

        async fn handler(_req: SimpleRequest) -> OnvifResult<BadResponse> {
            Ok(BadResponse { field: () })
        }

        let xml = r#"<SimpleRequest xmlns="http://test.example.com/"/>"#;
        let result = dispatch_async(xml, handler).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        // Error message is generic to avoid leaking serializer internals to clients
        assert!(err.to_string().contains("Internal processing error"));
    }

    // Body XML + single envelope wrap (as the dispatcher does)

    #[test]
    fn test_dispatch_sync_body_then_wrapped_envelope() {
        #[derive(serde::Serialize)]
        struct SimpleResponse {
            #[serde(rename = "Value")]
            value: i32,
        }

        fn handler(_req: TestRequest) -> OnvifResult<SimpleResponse> {
            Ok(SimpleResponse { value: 42 })
        }

        let xml = r#"<TestRequest xmlns="http://test.example.com/"/>"#;
        let body = dispatch_sync(xml, handler).unwrap();
        assert!(body.contains("<Value>42</Value>"));
        assert!(!body.contains("Envelope"));

        let soap = build_soap_response(&body);
        assert!(soap.contains("xmlns:s="));
        assert!(soap.contains("s:Envelope"));
        assert!(soap.contains("s:Body"));
        assert!(soap.contains("<Value>42</Value>"));
    }

    #[tokio::test]
    async fn test_dispatch_async_body_then_wrapped_envelope() {
        #[derive(serde::Serialize)]
        struct AsyncResponse {
            #[serde(rename = "Items")]
            items: Vec<String>,
        }

        async fn handler(_req: TestRequest) -> OnvifResult<AsyncResponse> {
            Ok(AsyncResponse {
                items: vec!["a".to_string(), "b".to_string()],
            })
        }

        let xml = r#"<TestRequest xmlns="http://test.example.com/"/>"#;
        let body = dispatch_async(xml, handler).await.unwrap();
        assert!(!body.contains("Envelope"));
        assert!(body.contains("<Items>a</Items>"));
        assert!(body.contains("<Items>b</Items>"));

        let soap = build_soap_response(&body);
        assert!(soap.contains("s:Envelope"));
    }
}
