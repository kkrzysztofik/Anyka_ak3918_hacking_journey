//! Dispatch helpers to reduce boilerplate in ONVIF service handlers.
//!
//! This module provides helper functions for dispatching SOAP requests
//! to handlers, reducing the boilerplate needed in each service implementation.
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
//! use onvif_rust::onvif::common::dispatch::dispatch_sync;
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
//! let body_xml = r#"<GetDeviceInformation xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#;
//! let response = dispatch_sync(body_xml, handle_get_device_info).unwrap();
//! assert!(response.contains("Anyka"));
//! ```
//!
//! ## Async Dispatch
//!
//! For handlers that need async (most real handlers):
//!
//! ```
//! use onvif_rust::onvif::common::dispatch::dispatch_async;
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
//! let body_xml = r#"<GetProfiles xmlns="http://www.onvif.org/ver10/media/wsdl"/>"#;
//! let response = tokio::runtime::Runtime::new().unwrap().block_on(
//!     dispatch_async(body_xml, handle_get_profiles)
//! ).unwrap();
//! assert!(response.contains("Main"));
//! ```

use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::onvif::dispatcher::parse_body;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::soap::build_soap_response;

// Test types made public for module testing
#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
pub struct TestRequestDispatch {
    pub value: Option<String>,
}

#[allow(dead_code)]
#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
pub struct TestResponseDispatch {
    pub result: String,
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
/// A `Result` containing the serialized XML response string, or an `OnvifError`.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::dispatch::dispatch_sync;
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
/// let xml = r#"<GetHostname xmlns="http://www.onvif.org/ver10/device/wsdl"/>"#;
/// let result = dispatch_sync(xml, handle);
/// assert!(result.is_ok());
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
    let response_xml = quick_xml::se::to_string(&response)
        .map_err(|e| OnvifError::Internal(format!("Failed to serialize response: {}", e)))?;

    // Wrap in SOAP response envelope
    Ok(build_soap_response(&response_xml))
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
/// A `Result` containing the serialized XML response string, or an `OnvifError`.
///
/// # Example
///
/// ```
/// use onvif_rust::onvif::common::dispatch::dispatch_async;
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
/// let xml = r#"<GetProfiles xmlns="http://www.onvif.org/ver10/media/wsdl"/>"#;
/// let result = tokio::runtime::Runtime::new().unwrap().block_on(
///     dispatch_async(xml, handle)
/// );
/// assert!(result.is_ok());
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
    let response_xml = quick_xml::se::to_string(&response)
        .map_err(|e| OnvifError::Internal(format!("Failed to serialize response: {}", e)))?;

    // Wrap in SOAP response envelope
    Ok(build_soap_response(&response_xml))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Check it's wrapped in SOAP envelope
        assert!(response.contains("Envelope"));
        assert!(response.contains("Body"));
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
        assert!(err.to_string().contains("serialize"));
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
        assert!(err.to_string().contains("serialize"));
    }

    // Test SOAP envelope structure

    #[test]
    fn test_dispatch_sync_soap_envelope_structure() {
        #[derive(serde::Serialize)]
        struct SimpleResponse {
            #[serde(rename = "Value")]
            value: i32,
        }

        fn handler(_req: TestRequest) -> OnvifResult<SimpleResponse> {
            Ok(SimpleResponse { value: 42 })
        }

        let xml = r#"<TestRequest xmlns="http://test.example.com/"/>"#;
        let result = dispatch_sync(xml, handler).unwrap();

        // Check SOAP envelope structure (uses "s:" namespace prefix)
        assert!(result.contains("xmlns:s="));
        assert!(result.contains("s:Envelope"));
        assert!(result.contains("s:Body"));
        // Check our response is inside the body
        assert!(result.contains("<Value>42</Value>"));
    }

    #[tokio::test]
    async fn test_dispatch_async_soap_envelope_structure() {
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
        let result = dispatch_async(xml, handler).await.unwrap();

        assert!(result.contains("s:Envelope"));
        // Vec<String> serializes as repeated element tags
        assert!(result.contains("<Items>a</Items>"));
        assert!(result.contains("<Items>b</Items>"));
    }
}
