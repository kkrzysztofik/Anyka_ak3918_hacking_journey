---
name: onvif-service-impl
description: |
  ONVIF 24.12 service implementation specialist for the onvif-rust project.
  Triggers on: "implement ONVIF", "add operation", "SOAP handler", "device service", "media service", "PTZ service", "imaging service", "ONVIF endpoint".
version: 1.0.0
---

# ONVIF Service Implementation

Implement ONVIF 24.12 compliant services in the onvif-rust project. Follow the established handler patterns and ensure full specification compliance.

## Service Architecture Overview

The onvif-rust project follows a layered architecture:

```
axum Router
    └── Service Router (e.g., /onvif/device_service)
        └── ServiceHandler trait
            └── handle_operation() → dispatches to handle_* methods
                └── Platform trait → hardware abstraction
```

## Handler Pattern

Each ONVIF service implements the `ServiceHandler` trait:

```rust
#[async_trait]
pub trait ServiceHandler: Send + Sync {
    async fn handle_operation(
        &self,
        action: &str,
        body: &str,
        auth_context: &AuthContext,
    ) -> Result<String, OnvifError>;

    fn service_name(&self) -> &'static str;
    fn supported_actions(&self) -> &[&'static str];
}
```

## Implementing a New Operation

### Step 1: Define the Handler Method

Follow the naming convention `handle_<operation_name>`:

```rust
impl DeviceService {
    /// Handles the GetSystemLog operation.
    ///
    /// ONVIF Spec: Device Management - GetSystemLog
    /// Returns system logs based on the log type requested.
    async fn handle_get_system_log(
        &self,
        body: &str,
        auth_context: &AuthContext,
    ) -> Result<String, OnvifError> {
        // 1. Validate authorization
        auth_context.require_admin()?;

        // 2. Parse request XML
        let request = parse_get_system_log_request(body)?;

        // 3. Validate input parameters
        validate_log_type(&request.log_type)?;

        // 4. Call platform layer
        let logs = self.platform.get_system_log(request.log_type).await?;

        // 5. Build response XML
        Ok(build_get_system_log_response(&logs))
    }
}
```

### Step 2: Add to Operation Dispatcher

Update `handle_operation()` in the `ServiceHandler` impl:

```rust
impl ServiceHandler for DeviceService {
    async fn handle_operation(
        &self,
        action: &str,
        body: &str,
        auth_context: &AuthContext,
    ) -> Result<String, OnvifError> {
        match action {
            "GetDeviceInformation" => self.handle_get_device_information(body, auth_context).await,
            "GetCapabilities" => self.handle_get_capabilities(body, auth_context).await,
            // Add new operation here:
            "GetSystemLog" => self.handle_get_system_log(body, auth_context).await,
            _ => Err(OnvifError::ActionNotSupported(action.to_string())),
        }
    }

    fn supported_actions(&self) -> &[&'static str] {
        &[
            "GetDeviceInformation",
            "GetCapabilities",
            "GetSystemLog",  // Add here too
        ]
    }
}
```

### Step 3: Define Types

Create request/response types in the service's `types.rs`:

```rust
// src/onvif/device/types.rs

/// Request for GetSystemLog operation
#[derive(Debug, Clone)]
pub struct GetSystemLogRequest {
    pub log_type: SystemLogType,
}

/// System log types per ONVIF spec
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SystemLogType {
    System,
    Access,
}

/// Response for GetSystemLog operation
#[derive(Debug, Clone)]
pub struct SystemLogResponse {
    pub log_data: String,
}
```

### Step 4: XML Parsing and Building

Create parser and builder functions:

```rust
// src/onvif/device/xml.rs

pub fn parse_get_system_log_request(body: &str) -> Result<GetSystemLogRequest, OnvifError> {
    let doc = roxmltree::Document::parse(body)
        .map_err(|e| OnvifError::InvalidRequest(format!("XML parse error: {}", e)))?;

    let log_type_node = doc
        .descendants()
        .find(|n| n.has_tag_name("LogType"))
        .ok_or_else(|| OnvifError::InvalidRequest("Missing LogType element".into()))?;

    let log_type = match log_type_node.text() {
        Some("System") => SystemLogType::System,
        Some("Access") => SystemLogType::Access,
        Some(other) => return Err(OnvifError::InvalidRequest(
            format!("Invalid log type: {}", other)
        )),
        None => return Err(OnvifError::InvalidRequest("Empty LogType".into())),
    };

    Ok(GetSystemLogRequest { log_type })
}

pub fn build_get_system_log_response(logs: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<tds:GetSystemLogResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
    xmlns:tt="http://www.onvif.org/ver10/schema">
    <tds:SystemLog>
        <tt:String>{}</tt:String>
    </tds:SystemLog>
</tds:GetSystemLogResponse>"#,
        xml_escape(logs)
    )
}
```

## Authentication Patterns

Use the `AuthContext` for authorization:

```rust
// No auth required (public operations)
async fn handle_get_wsdl_url(&self, _body: &str, _auth: &AuthContext) -> Result<String, OnvifError> {
    // Implementation
}

// User-level access
async fn handle_get_profiles(&self, body: &str, auth: &AuthContext) -> Result<String, OnvifError> {
    auth.require_user()?;  // User or higher
    // Implementation
}

// Operator-level access
async fn handle_create_profile(&self, body: &str, auth: &AuthContext) -> Result<String, OnvifError> {
    auth.require_operator()?;  // Operator or higher
    // Implementation
}

// Admin-only access
async fn handle_set_user(&self, body: &str, auth: &AuthContext) -> Result<String, OnvifError> {
    auth.require_admin()?;  // Admin only
    // Implementation
}
```

## Error Handling

Return appropriate ONVIF faults:

```rust
use crate::onvif::faults::{OnvifError, OnvifFault};

// Input validation errors
if name.is_empty() {
    return Err(OnvifError::InvalidArgs("Name cannot be empty".into()));
}

// Authorization errors (handled by auth.require_*)
// Returns: ter:NotAuthorized

// Resource not found
if profile.is_none() {
    return Err(OnvifError::NoProfile(profile_token.clone()));
}

// Operation not supported
if !self.capabilities.supports_ptz {
    return Err(OnvifError::ActionNotSupported("PTZ".into()));
}

// Platform/hardware errors
let result = self.platform.set_brightness(level).await
    .map_err(|e| OnvifError::Platform(e))?;
```

## Platform Integration

Call the platform layer for hardware operations:

```rust
// Platform trait method call
let device_info = self.platform.get_device_info().await?;

// With fallback to config
let device_info = match self.platform.get_device_info().await {
    Ok(info) => info,
    Err(e) => {
        tracing::warn!("Platform error, using config fallback: {}", e);
        self.device_info_from_config()
    }
};
```

## Testing New Operations

Write comprehensive unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_service() -> DeviceService {
        let mut mock = MockPlatform::new();
        mock.expect_get_system_log()
            .returning(|_| Ok("Test log content".to_string()));

        DeviceService::with_config_and_platform(
            test_config(),
            Arc::new(mock),
        )
    }

    #[tokio::test]
    async fn test_get_system_log_success() {
        let service = create_test_service();
        let auth = AuthContext::admin();

        let result = service
            .handle_get_system_log("<GetSystemLog><LogType>System</LogType></GetSystemLog>", &auth)
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.contains("GetSystemLogResponse"));
        assert!(response.contains("Test log content"));
    }

    #[tokio::test]
    async fn test_get_system_log_unauthorized() {
        let service = create_test_service();
        let auth = AuthContext::user();  // Not admin

        let result = service
            .handle_get_system_log("<GetSystemLog><LogType>System</LogType></GetSystemLog>", &auth)
            .await;

        assert!(matches!(result, Err(OnvifError::NotAuthorized)));
    }

    #[tokio::test]
    async fn test_get_system_log_invalid_type() {
        let service = create_test_service();
        let auth = AuthContext::admin();

        let result = service
            .handle_get_system_log("<GetSystemLog><LogType>Invalid</LogType></GetSystemLog>", &auth)
            .await;

        assert!(matches!(result, Err(OnvifError::InvalidRequest(_))));
    }
}
```

## ONVIF Namespaces

Use correct namespaces per service:

```rust
// Device Management
const TDS_NS: &str = "http://www.onvif.org/ver10/device/wsdl";

// Media
const TRT_NS: &str = "http://www.onvif.org/ver10/media/wsdl";

// PTZ
const TPT_NS: &str = "http://www.onvif.org/ver20/ptz/wsdl";

// Imaging
const TIM_NS: &str = "http://www.onvif.org/ver20/imaging/wsdl";

// Common schema types
const TT_NS: &str = "http://www.onvif.org/ver10/schema";
```

## Reference

For handler code patterns and XML templates, see `references/handler-patterns.md`.
