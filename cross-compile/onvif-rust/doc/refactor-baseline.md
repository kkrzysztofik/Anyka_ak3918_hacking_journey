# ONVIF Refactoring Baseline Documentation

**Epic:** anyka-dev-11a  
**Task:** Task 7 - Write baseline documentation  
**Document Purpose:** Establish authoritative behavior contracts for the ONVIF refactoring effort

---

## 1. Overview

### 1.1 Purpose of This Baseline

This document serves as the **authoritative reference** for the ONVIF service refactoring effort. It captures the behavior contracts established by the test suite, ensuring that refactoring activities preserve existing functionality while enabling future improvements.

### 1.2 Scope of the Refactoring (Epic anyka-dev-11a)

The refactoring covers the core ONVIF infrastructure components:

- **SOAP Parsing** (`src/onvif/soap.rs`): Envelope parsing, namespace extraction, WS-Security extraction
- **Service Dispatcher** (`src/onvif/dispatcher.rs`): Action extraction, routing, authentication, error mapping
- **Authentication** (`src/onvif/ws_security.rs`, `src/onvif/auth_requirements.rs`): Basic Auth, WS-Security, auth level enforcement
- **Error Handling** (`src/onvif/error.rs`): HTTP status mapping, SOAP fault generation

### 1.3 How to Use This Document During Refactoring

1. **Before any refactor step**: Run the full test suite to establish a baseline
2. **After each change**: Run tests again to detect any behavior changes
3. **If tests fail**: Compare against this document to determine if the change is acceptable or a regression
4. **Document exceptions**: Any new behavior must be added to this document as a contract update

---

## 2. Behavior Contract: SOAP Parsing

**Module:** `src/onvif/soap.rs`  
**Public API:** `parse_soap_request(xml: &str) -> Result<RawSoapEnvelope, SoapParseError>`

### 2.1 Input → Output Contract

| Input | Output |
|-------|--------|
| Raw XML string (valid SOAP 1.2 envelope) | `RawSoapEnvelope { header: Option<SoapHeader>, body_xml: String, action: Option<String> }` |
| Invalid/malformed XML | `SoapParseError` variant |

### 2.2 Namespace Validation Rules

- **Required**: SOAP 1.2 namespace (`http://www.w3.org/2003/05/soap-envelope`)
- **Rejection**: SOAP 1.1 namespace (`http://schemas.xmlsoap.org/soap/envelope/`) returns `SoapParseError::InvalidStructure` or `SoapParseError::MissingEnvelope`
- **Test:** `test_contract_reject_wrong_soap_namespace` (`tests/onvif/soap_contract.rs:175`)

### 2.3 WS-Security Extraction Rules

When present in the SOAP Header, the following elements are extracted:

| Element | Extraction |
|---------|------------|
| `wsse:UsernameToken` | `UsernameToken` struct with username, password, nonce, created |
| Password type | Stored as enum: `PasswordDigest` or `PasswordText` |
| Nonce encoding | Stored if present (`wsse:Nonce`) |

- **Test:** `test_contract_parse_ws_security_digest_succeeds` (`soap_contract.rs:70`)
- **Test:** `test_contract_parse_ws_security_plaintext_succeeds` (`soap_contract.rs:90`)

### 2.4 Body Reconstruction Rules

- Body XML is reconstructed from content between `<s:Body>` and `</s:Body>`
- Current implementation uses `local_name()` which strips QName prefixes (known bug: anyka-dev-2hh)
- Body must contain content; self-closing `<s:Body/>` returns `SoapParseError::MissingBody`
- **Test:** `test_contract_parse_empty_body_element_rejected` (`soap_contract.rs:113`)

### 2.5 Test References

| Test Function | Behavior Guarded |
|---------------|------------------|
| `test_contract_parse_minimal_envelope_succeeds` | Minimal valid envelope parses |
| `test_contract_parse_prefixed_body_succeeds` | Prefixed body elements parse |
| `test_contract_parse_nested_prefixed_body_succeeds` | Nested prefixed elements parse |
| `test_contract_parse_ws_security_digest_succeeds` | WS-Security digest extraction |
| `test_contract_parse_ws_security_plaintext_succeeds` | WS-Security plaintext extraction |
| `test_contract_parse_body_with_attributes_succeeds` | Body with attributes preserved |
| `test_contract_reject_missing_envelope` | Missing envelope rejected |
| `test_contract_reject_missing_body` | Missing body rejected |
| `test_contract_reject_wrong_soap_namespace` | Wrong namespace rejected |
| `test_contract_reject_envelope_not_root` | Non-root envelope rejected |
| `test_contract_accepts_no_namespace_on_inner_elements` | Inner elements without ns accepted |
| `test_contract_reject_empty_input` | Empty input rejected |

---

## 3. Behavior Contract: Action Resolution

**Module:** `src/onvif/dispatcher.rs`  
**Public API:** Action extracted during dispatch, used to route to service handlers

### 3.1 Precedence Order

The dispatcher resolves action from these sources (in order):

1. **SOAPAction HTTP header** (highest priority)
2. **Content-Type header action parameter** (`application/soap+xml; action="..."`)
3. **Body first element name** (fallback)

If none provide an action, returns well-formed error response.

### 3.2 URI Normalization

- Full ONVIF URIs like `http://www.onvif.org/ver10/device/wsdl/GetDeviceInformation`
- Extract last path segment: `GetDeviceInformation`
- **Test:** `test_contract_action_from_uri_extracts_last_segment` (`dispatcher_contract.rs:165`)

### 3.3 Quote Stripping

- Actions in headers may be quoted: `"GetDeviceInformation"`
- Quotes are stripped before dispatch
- **Test:** `test_contract_action_strips_quotes` (`dispatcher_contract.rs:199`)

### 3.4 Empty Action Handling

- No valid action found → returns 400 Bad Request with SOAP Fault
- Response is well-formed SOAP envelope
- **Test:** `test_contract_missing_action_returns_wellformed_error` (`dispatcher_contract.rs:224`)

### 3.5 Test References

| Test Function | Behavior Guarded |
|---------------|------------------|
| `test_contract_action_from_soapaction_header_preferred` | SOAPAction > Content-Type |
| `test_contract_action_from_content_type_action_param` | Content-Type action param |
| `test_contract_action_from_body_first_element_fallback` | Body first element fallback |
| `test_contract_action_from_uri_extracts_last_segment` | URI normalization |
| `test_contract_action_strips_quotes` | Quote stripping |
| `test_contract_missing_action_returns_wellformed_error` | Empty action error |

---

## 4. Behavior Contract: Routing

**Module:** `src/onvif/dispatcher.rs`

### 4.1 Service Name Matching

- Service name lookup is **case-insensitive**
- `"device"`, `"Device"`, `"DEVICE"` all match the registered `device` service
- **Test:** `test_contract_service_name_case_insensitive` (`dispatcher_contract.rs:319`)

### 4.2 Unknown Service Handling

When a request is dispatched to an unregistered service:

- Returns HTTP 400 Bad Request
- Response body contains SOAP Fault with `ActionNotSupported` or "not available" message
- **Test:** `test_contract_dispatch_to_unregistered_service_returns_action_not_supported` (`dispatcher_contract.rs:289`)

### 4.3 Handler Dispatch Flow

```
Request → Extract Action (precedence) → Lookup Service (case-insensitive) 
  → Get Handler → Call handle_operation(action, body_xml) → Build Response
```

### 4.4 Test References

| Test Function | Behavior Guarded |
|---------------|------------------|
| `test_contract_dispatch_to_registered_service_succeeds` | Registered service dispatch |
| `test_contract_dispatch_to_unregistered_service_returns_action_not_supported` | Unknown service error |
| `test_contract_service_name_case_insensitive` | Case-insensitive lookup |

---

## 5. Behavior Contract: Error Mapping

**Module:** `src/onvif/error.rs`  
**Public API:** `OnvifError::http_status()` and `OnvifError::to_soap_fault()`

### 5.1 Error → HTTP Status Code Mapping

| `OnvifError` Variant | HTTP Status | Notes |
|---------------------|-------------|-------|
| `NotAuthorized` | 401 Unauthorized | Authentication failed |
| `ActionNotSupported` | 400 Bad Request | Unknown operation |
| `InvalidArguments` | 400 Bad Request | Bad request |
| `Internal` | 500 Internal Server Error | Server error |
| `SoapParseError` | 400 Bad Request | Malformed request |

### 5.2 Error → SOAP Fault Mapping

| `OnvifError` Variant | SOAP Fault Code | SOAP Fault Subcode |
|---------------------|-----------------|-------------------|
| `NotAuthorized` | `s:Sender` | `ter:NotAuthorized` |
| `ActionNotSupported` | `s:Sender` | `ter:ActionNotSupported` |
| `InvalidArguments` | `s:Sender` | `ter:InvalidArgs` |
| `Internal` | `s:Receiver` | (internal error) |

### 5.3 Test References

| Test Function | Behavior Guarded |
|---------------|------------------|
| `test_contract_error_response_wellformed_returns_400` | Parse error → 400 |
| `test_contract_error_response_action_not_supported_returns_400` | ActionNotSupported → 400 |
| `test_contract_error_response_not_authorized_returns_401` | NotAuthorized → 401 |
| `test_contract_error_response_internal_returns_500` | Internal → 500 |
| `test_contract_soap_fault_xml_structure` | Fault XML structure |

---

## 6. Behavior Contract: Authentication

**Module:** `src/onvif/dispatcher.rs`, `src/onvif/auth_requirements.rs`

### 6.1 Auth Disabled Mode

When authentication is disabled (`auth_enabled = false`):

- **All operations pass** without credential checks
- **Test:** `test_contract_auth_disabled_bypasses_all_checks` (`auth_contract.rs:345`)

### 6.2 Auth Enabled Flow

When authentication is enabled, credentials are checked in this order:

1. **WS-Security** header (UsernameToken with password digest or plaintext)
2. **HTTP Basic Auth** header
3. If neither → 401 Unauthorized

If **both** WS-Security and Basic Auth are present → reject with 401

### 6.3 Auth Level Hierarchy

| Level | Access |
|-------|--------|
| `Anonymous` | No auth required |
| `User` | Basic read operations |
| `Operator` | Configuration changes |
| `Administrator` | User management, system settings |

**Hierarchy:** Anonymous ⊂ User ⊂ Operator ⊂ Administrator

Higher levels include all permissions of lower levels.

### 6.4 Fail-Secure Default

- **Unknown operations** require `Administrator` level
- This prevents information leakage about available operations
- **Test:** `test_contract_unknown_operation_requires_admin` (`auth_contract.rs:331`)

### 6.5 Test References

| Test Function | Behavior Guarded |
|---------------|------------------|
| `test_contract_anonymous_operation_succeeds_without_credentials` | Anonymous operations |
| `test_contract_user_level_operation_requires_authentication` | User-level requires auth |
| `test_contract_operator_level_rejects_user_role` | Operator rejects User |
| `test_contract_admin_level_rejects_operator_role` | Admin rejects Operator |
| `test_contract_unknown_operation_requires_admin` | Unknown → Admin |
| `test_contract_auth_disabled_bypasses_all_checks` | Auth disabled bypass |
| `test_contract_missing_credentials_returns_not_authorized` | No credentials → 401 |
| `test_contract_invalid_password_returns_not_authorized` | Wrong password → 401 |
| `test_contract_basic_auth_invalid_base64_returns_not_authorized` | Invalid Base64 → 401 |
| `test_contract_basic_auth_missing_colon_returns_not_authorized` | Malformed creds → 401 |
| `test_contract_ws_security_digest_token_extracted` | WS-Security digest |
| `test_contract_ws_security_plaintext_token_extracted` | WS-Security plaintext |
| `test_contract_ws_security_missing_token_returns_error` | Missing token → 401 |

---

## 7. Known Bugs (XFAIL)

This section documents known bugs that should **NOT** be fixed during refactoring unless explicitly listed as exceptions in Section 8.

---

### 7.1 Bug ID: anyka-dev-2sx

**Title:** Namespace extraction captures first xmlns attribute instead of SOAP envelope namespace

**Location:** `src/onvif/soap.rs:427-441` - `extract_envelope_namespace()`

**Description:**
The current implementation captures the **first** xmlns declaration found in the Envelope element, rather than matching by namespace URI value. This causes issues when non-SOAP xmlns declarations appear before the SOAP namespace.

**Current Behavior:**
```rust
// Line 434-439: Captures first xmlns value
let value = String::from_utf8_lossy(&attr.value).to_string();
state.envelope_namespace = Some(value);
break;  // BUG: breaks on first xmlns, not SOAP envelope
```

**Correct Behavior:**
Should match xmlns attribute where value equals `SOAP_ENVELOPE_NS` (`http://www.w3.org/2003/05/soap-envelope`).

**Test References:**
- `test_xfail_2sx_namespace_captures_envelope_uri_not_first_xmlns` (`soap_contract.rs:260`) - Documents correct behavior (currently ignored)
- `test_regression_2sx_documents_current_first_xmlns_behavior` (`soap_contract.rs:280`) - Documents current buggy behavior

**Remediation Plan:**
Modify `extract_envelope_namespace()` to filter by value matching `SOAP_ENVELOPE_NS` constant before storing.

---

### 7.2 Bug ID: anyka-dev-2hh

**Title:** QName prefix dropping in body XML reconstruction

**Location:** Multiple locations in `src/onvif/soap.rs`:
- Line 236: `e.local_name()` in start event
- Line 240: `e.local_name()` in end event  
- Line 244: `e.local_name()` in empty event
- Lines 558-559: `append_body_start_tag()` - uses `name` parameter (already stripped)
- Lines 618-620: `append_body_end_tag()` - uses stripped `name`

**Description:**
The parser uses `quick_xml::events::BytesStart::local_name()` which returns only the local part of a QName, dropping any namespace prefix. This causes `tds:GetDeviceInformation` to become `GetDeviceInformation` in the reconstructed body XML.

**Current Behavior:**
```rust
// Line 236: Uses local_name() which strips prefix
let name = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
```

**Correct Behavior:**
Should use `e.name()` to preserve the full QName including prefix.

**Test References:**
- `test_xfail_2hh_body_xml_preserves_qname_prefixes` (`soap_contract.rs:306`) - Documents correct behavior (currently ignored)
- `test_xfail_2hh_closing_tags_preserve_qname_prefixes` (`soap_contract.rs:322`) - Documents correct behavior (currently ignored)
- `test_regression_2hh_documents_current_local_name_behavior` (`soap_contract.rs:338`) - Documents current behavior

**Remediation Plan:**
Replace all `local_name()` calls with `name()` calls that return the full qualified name, then update reconstruction functions to handle prefixes correctly.

---

### 7.3 Bug ID: anyka-dev-2h2

**Title:** Username enumeration via authentication error messages

**Location:** `src/onvif/dispatcher.rs`:
- Line 677: `.ok_or_else(|| OnvifError::NotAuthorized(format!("User '{}' not found", username)))?;`
- Line 838: `.ok_or_else(|| OnvifError::NotAuthorized(format!("User '{}' not found", username)))?;`
- Line 964: `OnvifError::NotAuthorized(format!("User '{}' not found", user))`

**Description:**
Error messages reveal whether a username exists in the system. An attacker can distinguish between valid and invalid usernames by observing different error messages.

**Current Behavior:**
```
# Existing user, wrong password:
"User 'admin' not found"  (reveals admin exists)

# Non-existing user:
"User 'attacker' not found"  (reveals attacker doesn't exist)
```

**Correct Behavior:**
All authentication failures should return identical generic message: `"Invalid credentials"` - same message whether user exists or not.

**Test References:**
- `test_xfail_2h2_auth_error_does_not_reveal_username_existence` (`auth_contract.rs:665`) - Documents correct behavior (currently ignored)
- `test_regression_2h2_documents_username_in_error_currently` (`auth_contract.rs:594`) - Documents current behavior

**Remediation Plan:**
Replace all username-specific error messages with generic "Invalid credentials" message.

---

## 8. Allowed Changes for Refactoring

### 8.1 What CAN Change

- **Internal module structure**: Moving code between modules/files
- **File organization**: Restructuring directory layout
- **Internal implementation details**: Algorithm changes that preserve contracts
- **Performance optimizations**: As long as performance baselines are met

### 8.2 What MUST NOT Change (Without Explicit Approval)

- **Public API signatures**: Function/method signatures used externally
- **Observable HTTP behavior**: Status codes, headers, response format
- **SOAP envelope structure**: Element names, namespace declarations
- **Error message formats** (except bugs documented in Section 7)
- **Authentication flow**: Unless fixing bugs in Section 7

### 8.3 Exception List: Bugs That MAY Be Fixed

The following known bugs **MAY** be fixed during refactoring:

| Bug ID | Description |
|--------|-------------|
| anyka-dev-2sx | Namespace extraction fix |
| anyka-dev-2hh | QName prefix preservation |
| anyka-dev-2h2 | Username enumeration prevention |

**Note:** Fixing these bugs changes observable behavior. After fixing, update Section 7 to reflect the corrected behavior and add tests verifying the fix.

---

## 9. Performance Baselines

The following performance targets must be maintained during refactoring:

| Operation | Target | Test Reference |
|-----------|--------|----------------|
| `parse_soap_request()` | <1ms per call | See note below |
| `build_soap_response()` | <500µs per call | See note below |
| `build_soap_fault()` | <500µs per call | See note below |
| Parse/build cycle | <2ms per iteration | See note below |
| Full test suite | <15s total | `cargo test` |

**Note on Performance Tests:** Dedicated performance benchmark tests have been deferred to a future task. The targets above represent expected behavior based on the existing implementation and are provided as guidance for refactoring. The full test suite execution time (<15s) serves as a coarse performance regression gate. Future work may add `criterion`-based benchmarks in `benches/` if stricter validation is needed.

---

## 10. Contract Test Matrix

### 10.1 SOAP Contract Tests

| Test | Guards | Location |
|------|--------|----------|
| `test_contract_parse_minimal_envelope_succeeds` | SOAP envelope parsing | `tests/onvif/soap_contract.rs:28` |
| `test_contract_parse_prefixed_body_succeeds` | Prefixed body parsing | `tests/onvif/soap_contract.rs:41` |
| `test_contract_parse_nested_prefixed_body_succeeds` | Nested prefixed elements | `tests/onvif/soap_contract.rs:54` |
| `test_contract_parse_ws_security_digest_succeeds` | WS-Security digest extraction | `tests/onvif/soap_contract.rs:70` |
| `test_contract_parse_ws_security_plaintext_succeeds` | WS-Security plaintext extraction | `tests/onvif/soap_contract.rs:90` |
| `test_contract_parse_empty_body_element_rejected` | Empty body rejection | `tests/onvif/soap_contract.rs:113` |
| `test_contract_parse_body_with_attributes_succeeds` | Body with attributes | `tests/onvif/soap_contract.rs:127` |
| `test_contract_reject_missing_envelope` | Missing envelope rejection | `tests/onvif/soap_contract.rs:147` |
| `test_contract_reject_missing_body` | Missing body rejection | `tests/onvif/soap_contract.rs:161` |
| `test_contract_reject_wrong_soap_namespace` | Wrong namespace rejection | `tests/onvif/soap_contract.rs:175` |
| `test_contract_reject_envelope_not_root` | Non-root envelope rejection | `tests/onvif/soap_contract.rs:193` |
| `test_contract_accepts_no_namespace_on_inner_elements` | Inner elements without ns | `tests/onvif/soap_contract.rs:214` |
| `test_contract_reject_empty_input` | Empty input rejection | `tests/onvif/soap_contract.rs:231` |
| `test_xfail_2sx_namespace_captures_envelope_uri_not_first_xmlns` | anyka-dev-2sx correct behavior | `tests/onvif/soap_contract.rs:260` |
| `test_regression_2sx_documents_current_first_xmlns_behavior` | anyka-dev-2sx current behavior | `tests/onvif/soap_contract.rs:280` |
| `test_xfail_2hh_body_xml_preserves_qname_prefixes` | anyka-dev-2hh correct behavior | `tests/onvif/soap_contract.rs:306` |
| `test_xfail_2hh_closing_tags_preserve_qname_prefixes` | anyka-dev-2hh correct behavior | `tests/onvif/soap_contract.rs:322` |
| `test_regression_2hh_documents_current_local_name_behavior` | anyka-dev-2hh current behavior | `tests/onvif/soap_contract.rs:338` |
| `test_contract_build_soap_response_contains_envelope` | Response envelope structure | `tests/onvif/soap_contract.rs:362` |
| `test_contract_build_soap_response_contains_all_namespaces` | Response namespaces | `tests/onvif/soap_contract.rs:382` |
| `test_contract_build_soap_fault_contains_code_subcode_reason` | Fault structure | `tests/onvif/soap_contract.rs:405` |

### 10.2 Dispatcher Contract Tests

| Test | Guards | Location |
|------|--------|----------|
| `test_contract_action_from_soapaction_header_preferred` | SOAPAction precedence | `tests/onvif/dispatcher_contract.rs:82` |
| `test_contract_action_from_content_type_action_param` | Content-Type action param | `tests/onvif/dispatcher_contract.rs:111` |
| `test_contract_action_from_body_first_element_fallback` | Body first element fallback | `tests/onvif/dispatcher_contract.rs:138` |
| `test_contract_action_from_uri_extracts_last_segment` | URI normalization | `tests/onvif/dispatcher_contract.rs:165` |
| `test_contract_action_strips_quotes` | Quote stripping | `tests/onvif/dispatcher_contract.rs:199` |
| `test_contract_missing_action_returns_wellformed_error` | Empty action error | `tests/onvif/dispatcher_contract.rs:224` |
| `test_contract_dispatch_to_registered_service_succeeds` | Registered service dispatch | `tests/onvif/dispatcher_contract.rs:259` |
| `test_contract_dispatch_to_unregistered_service_returns_action_not_supported` | Unknown service handling | `tests/onvif/dispatcher_contract.rs:289` |
| `test_contract_service_name_case_insensitive` | Case-insensitive lookup | `tests/onvif/dispatcher_contract.rs:319` |
| `test_contract_error_response_wellformed_returns_400` | Parse error → 400 | `tests/onvif/dispatcher_contract.rs:348` |
| `test_contract_error_response_action_not_supported_returns_400` | ActionNotSupported → 400 | `tests/onvif/dispatcher_contract.rs:371` |
| `test_contract_error_response_not_authorized_returns_401` | NotAuthorized → 401 | `tests/onvif/dispatcher_contract.rs:401` |
| `test_contract_error_response_internal_returns_500` | Internal → 500 | `tests/onvif/dispatcher_contract.rs:415` |
| `test_contract_soap_fault_xml_structure` | Fault XML structure | `tests/onvif/dispatcher_contract.rs:429` |
| `test_contract_success_response_is_200` | Success → 200 | `tests/onvif/dispatcher_contract.rs:463` |
| `test_contract_success_response_content_type_is_soap_xml` | Content-Type header | `tests/onvif/dispatcher_contract.rs:487` |
| `test_contract_success_response_body_is_soap_envelope` | Response envelope | `tests/onvif/dispatcher_contract.rs:516` |

### 10.3 Authentication Contract Tests

| Test | Guards | Location |
|------|--------|----------|
| `test_contract_anonymous_operation_succeeds_without_credentials` | Anonymous access | `tests/onvif/auth_contract.rs:103` |
| `test_contract_user_level_operation_requires_authentication` | User-level requires auth | `tests/onvif/auth_contract.rs:163` |
| `test_contract_operator_level_rejects_user_role` | Operator rejects User | `tests/onvif/auth_contract.rs:218` |
| `test_contract_admin_level_rejects_operator_role` | Admin rejects Operator | `tests/onvif/auth_contract.rs:274` |
| `test_contract_unknown_operation_requires_admin` | Unknown → Admin | `tests/onvif/auth_contract.rs:331` |
| `test_contract_auth_disabled_bypasses_all_checks` | Auth disabled bypass | `tests/onvif/auth_contract.rs:345` |
| `test_contract_missing_credentials_returns_not_authorized` | No credentials → 401 | `tests/onvif/auth_contract.rs:402` |
| `test_contract_invalid_password_returns_not_authorized` | Wrong password → 401 | `tests/onvif/auth_contract.rs:441` |
| `test_contract_basic_auth_invalid_base64_returns_not_authorized` | Invalid Base64 → 401 | `tests/onvif/auth_contract.rs:486` |
| `test_contract_basic_auth_missing_colon_returns_not_authorized` | Malformed creds → 401 | `tests/onvif/auth_contract.rs:530` |
| `test_regression_2h2_documents_username_in_error_currently` | anyka-dev-2h2 current | `tests/onvif/auth_contract.rs:594` |
| `test_xfail_2h2_auth_error_does_not_reveal_username_existence` | anyka-dev-2h2 correct | `tests/onvif/auth_contract.rs:665` |
| `test_contract_ws_security_digest_token_extracted` | WS-Security digest | `tests/onvif/auth_contract.rs:730` |
| `test_contract_ws_security_plaintext_token_extracted` | WS-Security plaintext | `tests/onvif/auth_contract.rs:775` |
| `test_contract_ws_security_missing_token_returns_error` | Missing token → 401 | `tests/onvif/auth_contract.rs:821` |

---

## 11. How to Use This Baseline During Refactoring

### 11.1 Running the Full Test Suite

Before any refactoring step, run the complete test suite:

```bash
cd cross-compile/onvif-rust
../../toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu
```

Expected result: **All tests pass** (except documented xfail tests)

### 11.2 Detecting Behavior Changes

After each refactoring change:

1. **Re-run the test suite**
2. **Compare results** against baseline
3. **Analyze any failures**:
   - If test fails due to a **known bug** (Section 7) → Acceptable if fixing that bug
   - If test fails for other reason → **REGRESSION** - do not proceed
   - Document new failures in issue tracker before continuing

### 11.3 Documenting Contract Changes

If a refactoring requires changing a contract:

1. **Create an issue** describing the desired behavior change
2. **Update this document** with new contract details
3. **Add or modify tests** to verify new behavior
4. **Run full test suite** to ensure no regressions

### 11.4 When to Update This Document

Update this baseline when:

- A bug from Section 7 is fixed
- A new contract is established via tests
- Performance baselines change significantly
- New behavior is intentionally introduced

---

## Appendix A: Error Type Reference

### A.1 SoapParseError Variants

| Variant | Cause |
|---------|-------|
| `MissingEnvelope` | No `<s:Envelope>` root element |
| `MissingBody` | No `<s:Body>` element or empty body |
| `InvalidStructure` | Wrong element order or invalid XML structure |
| `XmlError` | Malformed XML (parse error) |

### A.2 OnvifError Variants

| Variant | HTTP Status | SOAP Fault Code |
|---------|-------------|-----------------|
| `NotAuthorized` | 401 | s:Sender / ter:NotAuthorized |
| `ActionNotSupported` | 400 | s:Sender / ter:ActionNotSupported |
| `InvalidArguments` | 400 | s:Sender / ter:InvalidArgs |
| `Internal` | 500 | s:Receiver |
| `NotFound` | 404 | s:Sender / ter:NotFound |

---

## Appendix B: Test Fixtures Reference

Test fixtures are located in `tests/fixtures/soap/envelopes.rs`:

| Constant | Description |
|----------|-------------|
| `MINIMAL_GET_DEVICE_INFO` | Minimal valid SOAP envelope |
| `PREFIXED_BODY_GET_DEVICE_INFO` | Body with tds: prefix |
| `NESTED_BODY_WITH_PREFIXES` | Nested elements with prefixes |
| `FULL_WS_SECURITY_DIGEST` | WS-Security with digest auth |
| `FULL_WS_SECURITY_PLAINTEXT` | WS-Security with plaintext |
| `EMPTY_BODY_ELEMENT` | Self-closing Body element |
| `BODY_WITH_ATTRIBUTES` | Body with XML attributes |
| `MISSING_ENVELOPE` | No Envelope element |
| `MISSING_BODY` | No Body element |
| `WRONG_SOAP_NAMESPACE` | SOAP 1.1 namespace |
| `ENVELOPE_NOT_ROOT` | Envelope not as root |
| `NO_NAMESPACE_DECLARATION` | Inner elements without ns |
| `EMPTY_STRING` | Empty input |
| `NON_SOAP_XMLNS_FIRST` | Non-SOAP xmlns before SOAP |

---

**Document Version:** 1.0  
**Last Updated:** 2026-03-04  
**Maintained By:** ONVIF Implementation Team
