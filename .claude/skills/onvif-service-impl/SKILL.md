---
name: onvif-service-impl
description: Use when implementing or extending ONVIF 24.12 service operations in onvif-rust (SOAP handlers, device/media/ptz/imaging services, dispatch_sync/dispatch_async, OnvifError faults, ops modules).
version: 2.0.0
---

# ONVIF Service Implementation

Implement ONVIF 24.12 compliant operations in `cross-compile/onvif-rust`. Follow the real handler patterns below — verify against the code, not older skill drafts.

## Architecture

```
axum Router
  └── ServiceDispatcher → extracts SOAP action
        └── ServiceHandler trait (handle_operation)
              └── dispatch_sync / dispatch_async helpers
                    └── ops/ functions (typed Req → typed Resp)
                          └── Platform trait → hardware
```

Auth is enforced **upstream** by the dispatcher's `auth_requirements.rs` (`get_required_level(service, action)`). Handlers do not receive an `AuthContext`.

## ServiceHandler Trait

`src/onvif/dispatcher/mod.rs`:

```rust
#[async_trait]
pub trait ServiceHandler: Send + Sync {
    async fn handle_operation(&self, action: &str, body_xml: &str) -> Result<String, OnvifError>;
    fn service_name(&self) -> &str;
    fn required_auth_level(&self, action: &str) -> AuthLevel {
        get_required_level(self.service_name(), action)
    }
}
```

There is **no `supported_actions()`** and **no `auth_context` parameter**. `handle_operation` returns the serialized response body XML fragment (not a full SOAP envelope).

## Adding a New Operation

### Step 1: Handler method (typed) in the service

Public typed handlers on the service (e.g. `DeviceService`) delegate to `ops/`:

```rust
// src/onvif/device/service.rs
pub async fn handle_get_device_information(
    &self,
    _request: GetDeviceInformation,
) -> Result<GetDeviceInformationResponse, OnvifError> {
    system_ops::handle_get_device_information(&self.platform, &self.store.config).await
}
```

### Step 2: Wire it into `handle_operation` with a dispatch helper

`src/onvif/common/dispatch.rs` provides two helpers that combine (1) parse XML body → typed `Req`, (2) call handler, (3) serialize `Resp` → body XML:

```rust
// Sync handlers
"GetCapabilities" => dispatch_sync(body_xml, |request: GetCapabilities| {
    system_ops::handle_get_capabilities(&config, request)
}),

// Async handlers (most real handlers)
"GetDeviceInformation" => dispatch_async(body_xml, |_req: GetDeviceInformation| {
    let platform = platform.clone();
    let config = config.clone();
    async move { system_ops::handle_get_device_information(&platform, &config).await }
}).await,

_ => Err(OnvifError::ActionNotSupported(action.to_string())),
```

Signatures:

```rust
pub fn dispatch_sync<Req, Resp>(body_xml: &str, handler: impl FnOnce(Req) -> OnvifResult<Resp>) -> OnvifResult<String>
pub async fn dispatch_async<Req, Resp, F, Fut>(body_xml: &str, handler: F) -> OnvifResult<String>
where Req: DeserializeOwned, Resp: Serialize, F: FnOnce(Req) -> Fut, Fut: Future<Output = OnvifResult<Resp>>
```

Request/response types use `serde` derive (deserialize via `parse_body`, serialize via `quick_xml`).

### Step 3: Implement the operation in the right `ops/` module

Device ops are split by domain under `src/onvif/device/ops/`: `system.rs` (device info, capabilities, date/time), `network.rs`, `discovery.rs` (scopes), `users.rs`. Media/PTZ/Imaging keep handlers in their service or local modules.

## OnvifError Enum

`src/onvif/error/mod.rs`. **These are the only variants** — older names like `InvalidRequest`, `NoProfile`, `InvalidArgs` do NOT exist:

```rust
pub enum OnvifError {
    ActionNotSupported(String),      // EC-001
    WellFormed(String),              // EC-002 malformed/missing XML
    InvalidArgVal { subcode: String, reason: String },  // EC-003/EC-006
    HardwareFailure(String),         // EC-005
    NotAuthorized(String),           // EC-011
    MaxUsers,                        // EC-013
    ConfigurationConflict(String),   // EC-015
    Internal(String),
    NotFound(String),
}
```

Constructor helpers:

```rust
OnvifError::invalid_arg("OutOfRange", "Value must be between 0 and 100");
OnvifError::missing_arg("ProfileToken");
OnvifError::out_of_range("Brightness", 0, 100);
```

XML parse failures map to `WellFormed`; missing resources to `NotFound`; unknown actions to `ActionNotSupported`.

## Namespaces

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

## Testing

Unit tests live inline in `#[cfg(test)] mod tests`. Auth levels are enforced in `auth_requirements.rs`, not via a per-handler `require_*()` API:

```rust
#[tokio::test]
async fn test_service_handler_unknown_action_device() {
    let service = create_test_service();
    let result = service.handle_operation("UnknownAction", "<test/>").await;
    assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
}

#[tokio::test]
async fn test_service_handler_invalid_xml() {
    let service = create_test_service();
    let result = service.handle_operation("GetDeviceInformation", "<InvalidXml><Broken").await;
    assert!(matches!(result, Err(OnvifError::WellFormed(_))));
}
```

Test names follow `test_<thing>_<scenario>`. Run host-side with `$CARGO test --target x86_64-unknown-linux-gnu` after `source ./setenv.sh` (see `anyka-embedded-build` skill).
