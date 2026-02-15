# ONVIF Handler Patterns Reference

## Complete Handler Template

```rust
impl MediaService {
    /// Handles the GetProfiles operation.
    ///
    /// ONVIF Spec: Media Service - GetProfiles
    /// Returns all configured media profiles.
    ///
    /// # Authorization
    /// Requires User level or higher.
    ///
    /// # Errors
    /// - `OnvifError::NotAuthorized` - Insufficient permissions
    /// - `OnvifError::Platform` - Hardware communication failure
    async fn handle_get_profiles(
        &self,
        _body: &str,
        auth_context: &AuthContext,
    ) -> Result<String, OnvifError> {
        // Authorization check
        auth_context.require_user()?;

        // Get data from platform or state
        let profiles = self.profiles.read().await;

        // Build response
        let mut response = String::from(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<trt:GetProfilesResponse xmlns:trt="http://www.onvif.org/ver10/media/wsdl"
    xmlns:tt="http://www.onvif.org/ver10/schema">"#
        );

        for profile in profiles.values() {
            response.push_str(&format!(
                r#"
    <trt:Profiles token="{}" fixed="{}">
        <tt:Name>{}</tt:Name>
        <tt:VideoSourceConfiguration token="{}">
            <tt:Name>{}</tt:Name>
            <tt:UseCount>1</tt:UseCount>
            <tt:SourceToken>{}</tt:SourceToken>
        </tt:VideoSourceConfiguration>
    </trt:Profiles>"#,
                xml_escape(&profile.token),
                profile.fixed,
                xml_escape(&profile.name),
                xml_escape(&profile.video_source_config.token),
                xml_escape(&profile.video_source_config.name),
                xml_escape(&profile.video_source_config.source_token),
            ));
        }

        response.push_str("\n</trt:GetProfilesResponse>");
        Ok(response)
    }
}
```

## Request Parsing Patterns

### Simple Element Extraction
```rust
fn parse_token_request(body: &str) -> Result<String, OnvifError> {
    let doc = roxmltree::Document::parse(body)
        .map_err(|e| OnvifError::InvalidRequest(format!("XML error: {}", e)))?;

    doc.descendants()
        .find(|n| n.has_tag_name("ProfileToken"))
        .and_then(|n| n.text())
        .map(String::from)
        .ok_or_else(|| OnvifError::InvalidRequest("Missing ProfileToken".into()))
}
```

### Complex Nested Parsing
```rust
fn parse_create_profile_request(body: &str) -> Result<CreateProfileRequest, OnvifError> {
    let doc = roxmltree::Document::parse(body)?;

    let name = doc.descendants()
        .find(|n| n.has_tag_name("Name"))
        .and_then(|n| n.text())
        .map(String::from)
        .ok_or_else(|| OnvifError::InvalidRequest("Missing Name".into()))?;

    let token = doc.descendants()
        .find(|n| n.has_tag_name("Token"))
        .and_then(|n| n.text())
        .map(String::from);

    Ok(CreateProfileRequest { name, token })
}
```

### Attribute Extraction
```rust
fn parse_configuration_options(node: roxmltree::Node) -> ConfigOptions {
    let min = node.attribute("Min")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let max = node.attribute("Max")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    ConfigOptions { min, max }
}
```

## Response Building Patterns

### Using format! for Simple Responses
```rust
fn build_simple_response(value: &str) -> String {
    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<tds:GetHostnameResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
    <tds:HostnameInformation>
        <tt:Name>{}</tt:Name>
    </tds:HostnameInformation>
</tds:GetHostnameResponse>"#,
        xml_escape(value)
    )
}
```

### Using String Builder for Complex Responses
```rust
fn build_capabilities_response(caps: &Capabilities) -> String {
    let mut xml = String::with_capacity(4096);

    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<tds:GetCapabilitiesResponse xmlns:tds="http://www.onvif.org/ver10/device/wsdl"
    xmlns:tt="http://www.onvif.org/ver10/schema">
    <tds:Capabilities>"#);

    if let Some(device) = &caps.device {
        xml.push_str(&format!(r#"
        <tt:Device>
            <tt:XAddr>{}</tt:XAddr>
            <tt:Network>
                <tt:IPFilter>{}</tt:IPFilter>
            </tt:Network>
        </tt:Device>"#,
            xml_escape(&device.xaddr),
            device.ip_filter
        ));
    }

    xml.push_str("\n    </tds:Capabilities>\n</tds:GetCapabilitiesResponse>");
    xml
}
```

## XML Escaping

Always escape user-provided content:

```rust
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
```

## Validation Patterns

### Hostname Validation
```rust
fn validate_hostname(name: &str) -> Result<(), OnvifError> {
    if name.is_empty() {
        return Err(OnvifError::InvalidArgs("Hostname cannot be empty".into()));
    }
    if name.len() > 63 {
        return Err(OnvifError::InvalidArgs("Hostname too long".into()));
    }
    if name.starts_with('-') || name.ends_with('-') {
        return Err(OnvifError::InvalidArgs("Invalid hostname format".into()));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(OnvifError::InvalidArgs("Invalid characters in hostname".into()));
    }
    Ok(())
}
```

### IP Address Validation
```rust
fn validate_ip_address(ip: &str) -> Result<std::net::IpAddr, OnvifError> {
    ip.parse()
        .map_err(|_| OnvifError::InvalidArgs(format!("Invalid IP address: {}", ip)))
}
```

### Token Validation
```rust
fn validate_token(token: &str) -> Result<(), OnvifError> {
    if token.is_empty() {
        return Err(OnvifError::InvalidArgs("Token cannot be empty".into()));
    }
    if token.len() > 64 {
        return Err(OnvifError::InvalidArgs("Token too long".into()));
    }
    Ok(())
}
```

## State Management Patterns

### Read-Only Access
```rust
async fn handle_get_profiles(&self, ...) -> Result<String, OnvifError> {
    let profiles = self.profiles.read().await;
    // Use profiles...
}
```

### Write Access with Validation
```rust
async fn handle_create_profile(&self, body: &str, auth: &AuthContext) -> Result<String, OnvifError> {
    auth.require_operator()?;

    let request = parse_create_profile_request(body)?;

    let mut profiles = self.profiles.write().await;

    // Validate before modification
    if profiles.len() >= MAX_PROFILES {
        return Err(OnvifError::MaxProfiles);
    }
    if profiles.contains_key(&request.token) {
        return Err(OnvifError::ProfileExists);
    }

    // Create and insert
    let profile = MediaProfile::new(request.name, request.token);
    profiles.insert(profile.token.clone(), profile.clone());

    Ok(build_create_profile_response(&profile))
}
```

## Error Response Format

ONVIF errors are SOAP faults:

```rust
fn build_fault_response(error: &OnvifError) -> String {
    let (code, subcode, reason) = match error {
        OnvifError::NotAuthorized => (
            "s:Sender",
            "ter:NotAuthorized",
            "Not authorized to perform this operation"
        ),
        OnvifError::InvalidArgs(msg) => (
            "s:Sender",
            "ter:InvalidArgVal",
            msg.as_str()
        ),
        OnvifError::ActionNotSupported(action) => (
            "s:Sender",
            "ter:ActionNotSupported",
            &format!("Action {} not supported", action)
        ),
        _ => (
            "s:Receiver",
            "ter:Action",
            "Internal error"
        ),
    };

    format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
    <s:Body>
        <s:Fault>
            <s:Code>
                <s:Value>{}</s:Value>
                <s:Subcode>
                    <s:Value xmlns:ter="http://www.onvif.org/ver10/error">{}</s:Value>
                </s:Subcode>
            </s:Code>
            <s:Reason>
                <s:Text xml:lang="en">{}</s:Text>
            </s:Reason>
        </s:Fault>
    </s:Body>
</s:Envelope>"#,
        code, subcode, xml_escape(reason)
    )
}
```
