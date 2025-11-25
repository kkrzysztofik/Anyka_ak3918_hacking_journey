use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};
use std::io::Cursor;

/// Device information structure matching ONVIF specification
#[derive(Debug, Clone)]
pub struct DeviceInformation {
    pub manufacturer: String,
    pub model: String,
    pub firmware_version: String,
    pub serial_number: String,
    pub hardware_id: String,
}

impl Default for DeviceInformation {
    fn default() -> Self {
        Self {
            manufacturer: "Anyka".to_string(),
            model: "AK3918 Camera".to_string(),
            firmware_version: "1.0.0".to_string(),
            serial_number: "AK3918-001".to_string(),
            hardware_id: "1.0".to_string(),
        }
    }
}

/// Parse SOAP request to detect ONVIF operation
pub fn parse_soap_operation(xml_body: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml_body);
    reader.trim_text(true);

    let mut buf = Vec::new();
    let mut in_body = false;
    let mut operation_name: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                match e.name().as_ref() {
                    b"Body" | b"s:Body" => {
                        in_body = true;
                    }
                    _ if in_body => {
                        // Extract operation name from body element
                        let name = e.name();
                        let name_str = String::from_utf8_lossy(name.as_ref());
                        // Remove namespace prefix if present (e.g., "tds:GetDeviceInformation" -> "GetDeviceInformation")
                        if let Some(colon_pos) = name_str.find(':') {
                            operation_name = Some(name_str[colon_pos + 1..].to_string());
                        } else {
                            operation_name = Some(name_str.to_string());
                        }
                        break;
                    }
                    _ => {}
                }
            }
            Ok(Event::End(e)) => {
                if matches!(e.name().as_ref(), b"Body" | b"s:Body") {
                    break;
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parsing error: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    operation_name
}

/// Generate SOAP response for GetDeviceInformation
pub fn generate_get_device_information_response(device_info: &DeviceInformation) -> String {
    let mut writer = Writer::new_with_indent(Cursor::new(Vec::new()), b' ', 2);

    // Write XML declaration
    let decl = BytesDecl::new("1.0", Some("UTF-8"), None);
    writer
        .write_event(Event::Decl(decl))
        .expect("Failed to write XML declaration");

    // Write Envelope start with namespaces
    let mut envelope_start = BytesStart::new("s:Envelope");
    envelope_start.push_attribute(("xmlns:s", "http://www.w3.org/2003/05/soap-envelope"));
    envelope_start.push_attribute(("xmlns:tds", "http://www.onvif.org/ver10/device/wsdl"));
    writer
        .write_event(Event::Start(envelope_start))
        .expect("Failed to write Envelope start");

    // Write Body start
    let body_start = BytesStart::new("s:Body");
    writer
        .write_event(Event::Start(body_start))
        .expect("Failed to write Body start");

    // Write GetDeviceInformationResponse start
    let response_start = BytesStart::new("tds:GetDeviceInformationResponse");
    writer
        .write_event(Event::Start(response_start))
        .expect("Failed to write GetDeviceInformationResponse start");

    // Write Manufacturer
    let manufacturer_start = BytesStart::new("tds:Manufacturer");
    writer
        .write_event(Event::Start(manufacturer_start))
        .expect("Failed to write Manufacturer start");
    writer
        .write_event(Event::Text(BytesText::new(&device_info.manufacturer)))
        .expect("Failed to write Manufacturer text");
    writer
        .write_event(Event::End(BytesEnd::new("tds:Manufacturer")))
        .expect("Failed to write Manufacturer end");

    // Write Model
    let model_start = BytesStart::new("tds:Model");
    writer
        .write_event(Event::Start(model_start))
        .expect("Failed to write Model start");
    writer
        .write_event(Event::Text(BytesText::new(&device_info.model)))
        .expect("Failed to write Model text");
    writer
        .write_event(Event::End(BytesEnd::new("tds:Model")))
        .expect("Failed to write Model end");

    // Write FirmwareVersion
    let firmware_start = BytesStart::new("tds:FirmwareVersion");
    writer
        .write_event(Event::Start(firmware_start))
        .expect("Failed to write FirmwareVersion start");
    writer
        .write_event(Event::Text(BytesText::new(&device_info.firmware_version)))
        .expect("Failed to write FirmwareVersion text");
    writer
        .write_event(Event::End(BytesEnd::new("tds:FirmwareVersion")))
        .expect("Failed to write FirmwareVersion end");

    // Write SerialNumber
    let serial_start = BytesStart::new("tds:SerialNumber");
    writer
        .write_event(Event::Start(serial_start))
        .expect("Failed to write SerialNumber start");
    writer
        .write_event(Event::Text(BytesText::new(&device_info.serial_number)))
        .expect("Failed to write SerialNumber text");
    writer
        .write_event(Event::End(BytesEnd::new("tds:SerialNumber")))
        .expect("Failed to write SerialNumber end");

    // Write HardwareId
    let hardware_start = BytesStart::new("tds:HardwareId");
    writer
        .write_event(Event::Start(hardware_start))
        .expect("Failed to write HardwareId start");
    writer
        .write_event(Event::Text(BytesText::new(&device_info.hardware_id)))
        .expect("Failed to write HardwareId text");
    writer
        .write_event(Event::End(BytesEnd::new("tds:HardwareId")))
        .expect("Failed to write HardwareId end");

    // Write GetDeviceInformationResponse end
    writer
        .write_event(Event::End(BytesEnd::new(
            "tds:GetDeviceInformationResponse",
        )))
        .expect("Failed to write GetDeviceInformationResponse end");

    // Write Body end
    writer
        .write_event(Event::End(BytesEnd::new("s:Body")))
        .expect("Failed to write Body end");

    // Write Envelope end
    writer
        .write_event(Event::End(BytesEnd::new("s:Envelope")))
        .expect("Failed to write Envelope end");

    // Extract the written XML
    let result = writer.into_inner().into_inner();
    String::from_utf8(result).expect("Failed to convert XML bytes to string")
}

/// Generate SOAP fault response
pub fn generate_soap_fault(code: &str, reason: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope">
  <s:Body>
    <s:Fault>
      <s:Code>
        <s:Value>s:{}</s:Value>
      </s:Code>
      <s:Reason>
        <s:Text>{}</s:Text>
      </s:Reason>
    </s:Fault>
  </s:Body>
</s:Envelope>"#,
        escape_xml(code),
        escape_xml(reason)
    )
}

/// Escape XML special characters
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get_device_information() {
        let soap_request = r#"<?xml version="1.0" encoding="UTF-8"?>
<s:Envelope xmlns:s="http://www.w3.org/2003/05/soap-envelope" xmlns:tds="http://www.onvif.org/ver10/device/wsdl">
  <s:Body>
    <tds:GetDeviceInformation/>
  </s:Body>
</s:Envelope>"#;

        let operation = parse_soap_operation(soap_request);
        assert_eq!(operation, Some("GetDeviceInformation".to_string()));
    }

    #[test]
    fn test_generate_get_device_information_response() {
        let device_info = DeviceInformation::default();
        let response = generate_get_device_information_response(&device_info);

        assert!(response.contains("GetDeviceInformationResponse"));
        assert!(response.contains("Anyka"));
        assert!(response.contains("AK3918 Camera"));
        assert!(response.contains("1.0.0"));
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("test"), "test");
        assert_eq!(escape_xml("a<b>c"), "a&lt;b&gt;c");
        assert_eq!(escape_xml("a&b"), "a&amp;b");
        assert_eq!(escape_xml(r#"a"b'c"#), "a&quot;b&apos;c");
    }
}
