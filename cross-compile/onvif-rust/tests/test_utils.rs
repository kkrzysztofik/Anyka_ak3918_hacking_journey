//! Test utilities for ONVIF type serialization tests.
//!
//! Provides helper functions to extract SOAP body content from real captured
//! ONVIF request/response XML files and strip namespace prefixes for type parsing.

use regex::Regex;

/// Extract the SOAP body content from a full SOAP envelope XML.
///
/// This function extracts the content between `<s:Body>` and `</s:Body>` tags,
/// stripping the outer envelope for direct type parsing.
///
/// # Arguments
///
/// * `xml` - The full SOAP envelope XML string
///
/// # Returns
///
/// The inner body content as a string, or the original XML if no body is found.
pub fn extract_soap_body(xml: &str) -> String {
    // Find content between <s:Body> and </s:Body>
    let body_start_re = Regex::new(r"<s:Body[^>]*>").unwrap();
    let body_end = "</s:Body>";

    if let Some(start_match) = body_start_re.find(xml) {
        let after_start = &xml[start_match.end()..];
        if let Some(end_pos) = after_start.find(body_end) {
            return after_start[..end_pos].trim().to_string();
        }
    }

    xml.to_string()
}

/// Strip common ONVIF namespace prefixes from XML.
///
/// Removes the following prefixes:
/// - `tt:` - ONVIF schema namespace
/// - `tds:` - Device service namespace
/// - `trt:` - Media service namespace
/// - `tptz:` - PTZ service namespace
/// - `timg:` - Imaging service namespace
/// - `tan:` - Analytics service namespace
///
/// # Arguments
///
/// * `xml` - XML string with namespace prefixes
///
/// # Returns
///
/// XML string with namespace prefixes removed.
pub fn strip_namespace_prefixes(xml: &str) -> String {
    // List of prefixes to remove (order matters - longer prefixes first)
    // Include SOAP s: and ter: prefixes for fault parsing
    let prefixes = [
        "timg:", "tptz:", "tds:", "trt:", "tan:", "ter:", "tt:", "s:",
    ];

    let mut result = xml.to_string();

    for prefix in prefixes {
        // Remove opening tags: <prefix:TagName -> <TagName
        let open_pattern = format!("<{}", prefix);
        result = result.replace(&open_pattern, "<");

        // Remove closing tags: </prefix:TagName -> </TagName
        let close_pattern = format!("</{}", prefix);
        result = result.replace(&close_pattern, "</");
    }

    // Also remove xmlns default namespace attributes which cause parsing issues
    // Pattern: xmlns="http://..."
    let xmlns_re = Regex::new(r#"\s+xmlns="[^"]*""#).unwrap();
    result = xmlns_re.replace_all(&result, "").to_string();

    result
}

/// Extract and clean SOAP body content for type deserialization.
///
/// This is a convenience function that combines `extract_soap_body` and
/// `strip_namespace_prefixes` for common test usage.
///
/// # Arguments
///
/// * `xml` - The full SOAP envelope XML string
///
/// # Returns
///
/// Cleaned body content ready for type deserialization.
pub fn extract_and_clean_body(xml: &str) -> String {
    let body = extract_soap_body(xml);
    strip_namespace_prefixes(&body)
}

/// Extract the inner content of a specific response element.
///
/// For example, extracting the content inside `<GetDeviceInformationResponse>...</GetDeviceInformationResponse>`.
/// Also handles self-closing tags like `<ContinuousMoveResponse />`.
///
/// # Arguments
///
/// * `xml` - XML string containing the response
/// * `element_name` - Name of the response element (without namespace prefix)
///
/// # Returns
///
/// The response element with its content, or None if not found.
pub fn extract_response_element(xml: &str, element_name: &str) -> Option<String> {
    let cleaned = extract_and_clean_body(xml);

    // Find the opening tag
    let open_tag = format!("<{}", element_name);
    let close_tag = format!("</{}>", element_name);

    if let Some(start_pos) = cleaned.find(&open_tag) {
        let after_open = &cleaned[start_pos..];

        // Check for self-closing tag first: <ElementName ... />
        // Find the first > after the opening tag
        if let Some(tag_end) = after_open.find('>') {
            let tag_content = &after_open[..tag_end];
            // Check if it ends with / (self-closing)
            if tag_content.trim_end().ends_with('/') {
                // Return the full self-closing element with normalized format
                return Some(format!("<{} />", element_name));
            }
        }

        // Otherwise, find the closing tag
        if let Some(close_pos) = after_open.find(&close_tag) {
            let end_pos = close_pos + close_tag.len();
            return Some(after_open[..end_pos].to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_soap_body() {
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformationResponse>
      <tds:Manufacturer>Test</tds:Manufacturer>
    </tds:GetDeviceInformationResponse>
  </s:Body>
</s:Envelope>"#;

        let body = extract_soap_body(xml);
        assert!(body.contains("GetDeviceInformationResponse"));
        assert!(!body.contains("Envelope"));
    }

    #[test]
    fn test_strip_namespace_prefixes() {
        let xml = r#"<tds:GetDeviceInformationResponse>
  <tds:Manufacturer>Test</tds:Manufacturer>
  <tt:Resolution>
    <tt:Width>1920</tt:Width>
  </tt:Resolution>
</tds:GetDeviceInformationResponse>"#;

        let stripped = strip_namespace_prefixes(xml);
        assert!(stripped.contains("<GetDeviceInformationResponse>"));
        assert!(stripped.contains("<Manufacturer>"));
        assert!(stripped.contains("<Resolution>"));
        assert!(stripped.contains("<Width>"));
        assert!(!stripped.contains("tds:"));
        assert!(!stripped.contains("tt:"));
    }

    #[test]
    fn test_extract_and_clean_body() {
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformationResponse>
      <tds:Manufacturer>Test</tds:Manufacturer>
    </tds:GetDeviceInformationResponse>
  </s:Body>
</s:Envelope>"#;

        let cleaned = extract_and_clean_body(xml);
        assert!(cleaned.contains("<GetDeviceInformationResponse>"));
        assert!(cleaned.contains("<Manufacturer>Test</Manufacturer>"));
        assert!(!cleaned.contains("tds:"));
    }

    #[test]
    fn test_extract_response_element() {
        let xml = r#"<?xml version="1.0"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <tds:GetDeviceInformationResponse>
      <tds:Manufacturer>Test</tds:Manufacturer>
    </tds:GetDeviceInformationResponse>
  </s:Body>
</s:Envelope>"#;

        let element = extract_response_element(xml, "GetDeviceInformationResponse");
        assert!(element.is_some());
        let el = element.unwrap();
        assert!(el.starts_with("<GetDeviceInformationResponse>"));
        assert!(el.ends_with("</GetDeviceInformationResponse>"));
        assert!(el.contains("<Manufacturer>Test</Manufacturer>"));
    }
}
