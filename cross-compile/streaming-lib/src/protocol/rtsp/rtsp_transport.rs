use crate::protocol::rtsp::global_trait::Marshal;

use super::global_trait::Unmarshal;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum CastType {
    Multicast,
    #[default]
    Unicast,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ProtocolType {
    #[default]
    TCP,
    UDP,
}
#[derive(Debug, Clone, Default)]
pub struct RtspTransport {
    pub cast_type: CastType,
    pub protocol_type: ProtocolType,
    pub interleaved: Option<[u8; 2]>,
    pub transport_mod: Option<String>,
    pub client_port: Option<[u16; 2]>,
    pub server_port: Option<[u16; 2]>,
    pub ssrc: Option<u32>,
}

impl RtspTransport {
    /// Parse a key=value pair and update the transport state.
    fn parse_key_value(&mut self, key: &str, val: &str) {
        match key {
            "mode" => {
                let mode = val
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim()
                    .to_ascii_lowercase();
                self.transport_mod = Some(mode);
            }
            "client_port" => {
                if let Some(ports) = parse_port_pair(val) {
                    self.client_port = Some(ports);
                }
            }
            "server_port" => {
                if let Some(ports) = parse_port_pair(val) {
                    self.server_port = Some(ports);
                }
            }
            "interleaved" => {
                if let Some(chs) = parse_u8_pair(val) {
                    self.interleaved = Some(chs);
                }
            }
            "ssrc" => {
                self.ssrc = parse_ssrc(val);
            }
            _ => {}
        }
    }

    /// Parse a standalone token and update protocol/cast type.
    fn parse_token(&mut self, token_upper: &str, token_lower: &str) {
        match token_upper {
            "RTP/AVP/TCP" => self.protocol_type = ProtocolType::TCP,
            "RTP/AVP/UDP" | "RTP/AVP" => self.protocol_type = ProtocolType::UDP,
            _ => {}
        }

        match token_lower {
            "unicast" => self.cast_type = CastType::Unicast,
            "multicast" => self.cast_type = CastType::Multicast,
            _ => {}
        }
    }

    /// Validate required transport parameters per RFC 2326 §12.39.
    fn validate(&self) -> Result<(), String> {
        match self.protocol_type {
            ProtocolType::TCP if self.interleaved.is_none() => {
                Err("TCP transport missing required 'interleaved' parameter".to_string())
            }
            ProtocolType::UDP if self.client_port.is_none() => {
                Err("UDP transport missing required 'client_port' parameter".to_string())
            }
            _ => Ok(()),
        }
    }
}

impl Unmarshal for RtspTransport {
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut rtsp_transport = RtspTransport::default();

        for part in raw_data.split(';') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }

            if let Some((raw_key, raw_val)) = part.split_once('=') {
                let key = raw_key.trim().to_ascii_lowercase();
                let val = raw_val.trim();
                rtsp_transport.parse_key_value(key.as_str(), val);
                continue;
            }

            let token_upper = part.to_ascii_uppercase();
            let token_lower = part.to_ascii_lowercase();
            rtsp_transport.parse_token(token_upper.as_str(), token_lower.as_str());
        }

        rtsp_transport.validate()?;
        Ok(rtsp_transport)
    }
}

fn parse_port_pair(val: &str) -> Option<[u16; 2]> {
    let val = val.trim();
    if val.is_empty() {
        return None;
    }

    if let Some((a, b)) = val.split_once('-') {
        let first = a.trim().parse::<u16>().ok()?;
        let second = b
            .trim()
            .parse::<u16>()
            .ok()
            .unwrap_or_else(|| if first < u16::MAX { first + 1 } else { first });
        return Some([first, second]);
    }

    let first = val.parse::<u16>().ok()?;
    let second = if first < u16::MAX { first + 1 } else { first };
    Some([first, second])
}

fn parse_u8_pair(val: &str) -> Option<[u8; 2]> {
    let val = val.trim();
    if val.is_empty() {
        return None;
    }

    if let Some((a, b)) = val.split_once('-') {
        let first = a.trim().parse::<u8>().ok()?;
        let second = b
            .trim()
            .parse::<u8>()
            .ok()
            .unwrap_or_else(|| first.saturating_add(1));
        return Some([first, second]);
    }

    let first = val.parse::<u8>().ok()?;
    Some([first, first.saturating_add(1)])
}

fn parse_ssrc(val: &str) -> Option<u32> {
    let ssrc_str = val.trim();
    if ssrc_str.is_empty() {
        return None;
    }

    if let Some(rest) = ssrc_str
        .strip_prefix("0x")
        .or_else(|| ssrc_str.strip_prefix("0X"))
    {
        return u32::from_str_radix(rest, 16).ok();
    }

    if let Ok(decimal) = ssrc_str.parse::<u32>() {
        return Some(decimal);
    }

    if ssrc_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return u32::from_str_radix(ssrc_str, 16).ok();
    }

    None
}

impl Marshal for RtspTransport {
    fn marshal(&self) -> String {
        let protocol_type = match self.protocol_type {
            ProtocolType::TCP => "RTP/AVP/TCP",
            ProtocolType::UDP => "RTP/AVP/UDP",
        };

        let cast_type = match self.cast_type {
            CastType::Multicast => "multicast",
            CastType::Unicast => "unicast",
        };

        let client_port = if let Some(client_ports) = self.client_port {
            format!("client_port={}-{};", client_ports[0], client_ports[1])
        } else {
            String::from("")
        };

        let server_port = if let Some(server_ports) = self.server_port {
            format!("server_port={}-{};", server_ports[0], server_ports[1])
        } else {
            String::from("")
        };

        let interleaved = if let Some(interleaveds) = self.interleaved {
            format!("interleaved={}-{};", interleaveds[0], interleaveds[1])
        } else {
            String::from("")
        };

        let ssrc = if let Some(ssrc) = self.ssrc {
            format!("ssrc={ssrc};")
        } else {
            String::from("")
        };

        let mode = if let Some(mode) = &self.transport_mod {
            format!("mode={mode}")
        } else {
            String::from("")
        };

        format!("{protocol_type};{cast_type};{client_port}{server_port}{interleaved}{ssrc}{mode}")
    }
}

#[cfg(test)]
mod tests {

    use crate::protocol::rtsp::global_trait::Marshal;
    use crate::protocol::rtsp::global_trait::Unmarshal;

    use super::{CastType, ProtocolType, RtspTransport};

    #[test]
    fn test_parse_transport() {
        let parser = RtspTransport::unmarshal(
            "RTP/AVP;unicast;client_port=8000-8001;server_port=9000-9001;ssrc=1234;interleaved=0-1;mode=record",
        )
        .unwrap();

        assert_eq!(parser.cast_type, CastType::Unicast);
        assert_eq!(parser.protocol_type, ProtocolType::UDP);
        assert_eq!(parser.interleaved.unwrap(), [0, 1]);
        assert_eq!(parser.transport_mod.unwrap(), "record".to_string());
        assert_eq!(parser.client_port.unwrap(), [8000, 8001]);
        assert_eq!(parser.server_port.unwrap(), [9000, 9001]);
        assert_eq!(parser.ssrc.unwrap(), 1234);
    }

    // ============================================
    // Protocol Type Tests
    // ============================================

    #[test]
    fn test_unmarshal_rtp_avp_udp() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
    }

    #[test]
    fn test_unmarshal_rtp_avp_udp_explicit() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP/UDP;unicast;client_port=5000-5001").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
    }

    #[test]
    fn test_unmarshal_rtp_avp_tcp() {
        let transport = RtspTransport::unmarshal("RTP/AVP/TCP;unicast;interleaved=0-1").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::TCP);
    }

    // ============================================
    // Cast Type Tests
    // ============================================

    #[test]
    fn test_unmarshal_unicast() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001").unwrap();
        assert_eq!(transport.cast_type, CastType::Unicast);
    }

    #[test]
    fn test_unmarshal_multicast() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;multicast;client_port=5000-5001").unwrap();
        assert_eq!(transport.cast_type, CastType::Multicast);
    }

    // ============================================
    // Port Tests
    // ============================================

    #[test]
    fn test_unmarshal_client_port() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001").unwrap();
        assert_eq!(transport.client_port, Some([5000, 5001]));
    }

    #[test]
    fn test_unmarshal_server_port() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;server_port=6000-6001")
                .unwrap();
        assert_eq!(transport.server_port, Some([6000, 6001]));
    }

    #[test]
    fn test_unmarshal_both_ports() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;server_port=6000-6001")
                .unwrap();
        assert_eq!(transport.client_port, Some([5000, 5001]));
        assert_eq!(transport.server_port, Some([6000, 6001]));
    }

    // ============================================
    // Interleaved Tests
    // ============================================

    #[test]
    fn test_unmarshal_interleaved() {
        let transport = RtspTransport::unmarshal("RTP/AVP/TCP;unicast;interleaved=0-1").unwrap();
        assert_eq!(transport.interleaved, Some([0, 1]));
    }

    #[test]
    fn test_unmarshal_interleaved_different_channels() {
        let transport = RtspTransport::unmarshal("RTP/AVP/TCP;unicast;interleaved=2-3").unwrap();
        assert_eq!(transport.interleaved, Some([2, 3]));
    }

    // ============================================
    // SSRC Tests
    // ============================================

    #[test]
    fn test_unmarshal_ssrc() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;ssrc=0x12345678")
                .unwrap();
        assert_eq!(transport.ssrc, Some(0x12345678));
    }

    #[test]
    fn test_unmarshal_ssrc_decimal() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;ssrc=1234567890")
                .unwrap();
        assert_eq!(transport.ssrc, Some(1234567890));
    }

    // ============================================
    // Mode Tests
    // ============================================

    #[test]
    fn test_unmarshal_mode_record() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;mode=record").unwrap();
        assert_eq!(transport.transport_mod, Some("record".to_string()));
    }

    #[test]
    fn test_unmarshal_mode_play() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;mode=play").unwrap();
        assert_eq!(transport.transport_mod, Some("play".to_string()));
    }

    #[test]
    fn test_unmarshal_case_insensitive_tokens_and_mode_play() {
        let transport =
            RtspTransport::unmarshal("rtp/avp/tcp;UNICAST;interleaved=0-1;mode=PLAY").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::TCP);
        assert_eq!(transport.cast_type, CastType::Unicast);
        assert_eq!(transport.interleaved, Some([0, 1]));
        assert_eq!(transport.transport_mod, Some("play".to_string()));
    }

    #[test]
    fn test_unmarshal_client_port_single_infers_rtcp() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000").unwrap();
        assert_eq!(transport.client_port, Some([5000, 5001]));
    }

    #[test]
    fn test_unmarshal_interleaved_single_infers_pair() {
        let transport = RtspTransport::unmarshal("RTP/AVP/TCP;unicast;interleaved=10").unwrap();
        assert_eq!(transport.interleaved, Some([10, 11]));
    }

    #[test]
    fn test_unmarshal_ssrc_hex_without_prefix() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;ssrc=ABCDEF00")
                .unwrap();
        assert_eq!(transport.ssrc, Some(0xABCDEF00));
    }

    #[test]
    fn test_unmarshal_transport_with_spaces() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP ; unicast ; client_port = 5000 - 5001 ").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
        assert_eq!(transport.cast_type, CastType::Unicast);
        assert_eq!(transport.client_port, Some([5000, 5001]));
    }

    // ============================================
    // Combined Tests
    // ============================================

    #[test]
    fn test_unmarshal_all_fields() {
        let transport = RtspTransport::unmarshal(
            "RTP/AVP/TCP;multicast;client_port=8000-8001;server_port=9000-9001;ssrc=0xABCDEF00;interleaved=4-5;mode=record"
        ).unwrap();

        assert_eq!(transport.protocol_type, ProtocolType::TCP);
        assert_eq!(transport.cast_type, CastType::Multicast);
        assert_eq!(transport.client_port, Some([8000, 8001]));
        assert_eq!(transport.server_port, Some([9000, 9001]));
        assert_eq!(transport.ssrc, Some(0xABCDEF00));
        assert_eq!(transport.interleaved, Some([4, 5]));
        assert_eq!(transport.transport_mod, Some("record".to_string()));
    }

    #[test]
    fn test_unmarshal_minimal_udp_without_client_port_rejected() {
        let result = RtspTransport::unmarshal("RTP/AVP;unicast");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("client_port"),
            "should mention missing client_port"
        );
    }

    #[test]
    fn test_unmarshal_minimal_tcp_without_interleaved_rejected() {
        let result = RtspTransport::unmarshal("RTP/AVP/TCP;unicast");
        assert!(result.is_err());
        assert!(
            result.unwrap_err().contains("interleaved"),
            "should mention missing interleaved"
        );
    }

    // ============================================
    // Marshal Tests
    // ============================================

    #[test]
    fn test_marshal_udp_unicast() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::UDP,
            cast_type: CastType::Unicast,
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("RTP/AVP/UDP"));
        assert!(result.contains("unicast"));
    }

    #[test]
    fn test_marshal_tcp_unicast() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::TCP,
            cast_type: CastType::Unicast,
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("RTP/AVP/TCP"));
        assert!(result.contains("unicast"));
    }

    #[test]
    fn test_marshal_multicast() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::UDP,
            cast_type: CastType::Multicast,
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("multicast"));
    }

    #[test]
    fn test_marshal_with_ports() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::UDP,
            cast_type: CastType::Unicast,
            client_port: Some([5000, 5001]),
            server_port: Some([6000, 6001]),
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("client_port=5000-5001"));
        assert!(result.contains("server_port=6000-6001"));
    }

    #[test]
    fn test_marshal_with_interleaved() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::TCP,
            cast_type: CastType::Unicast,
            interleaved: Some([0, 1]),
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("interleaved=0-1"));
    }

    #[test]
    fn test_marshal_with_ssrc() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::UDP,
            cast_type: CastType::Unicast,
            ssrc: Some(0x12345678),
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("ssrc=305419896")); // 0x12345678 in decimal
    }

    #[test]
    fn test_marshal_with_mode() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::UDP,
            cast_type: CastType::Unicast,
            transport_mod: Some("record".to_string()),
            ..Default::default()
        };
        let result = transport.marshal();
        assert!(result.contains("mode=record"));
    }

    // ============================================
    // Round-trip Tests (Unmarshal -> Marshal)
    // ============================================

    #[test]
    fn test_rtsp_transport_roundtrip_basic() {
        let original_str = "RTP/AVP;unicast;client_port=5000-5001";
        let transport = RtspTransport::unmarshal(original_str).unwrap();
        let marshaled = transport.marshal();
        // Marshal format may differ slightly, but should contain key components
        assert!(marshaled.contains("RTP/AVP"));
        assert!(marshaled.contains("unicast"));
        assert!(marshaled.contains("client_port=5000-5001"));
    }

    #[test]
    fn test_rtsp_transport_roundtrip_full() {
        let original_str = "RTP/AVP/TCP;unicast;client_port=8000-8001;server_port=9000-9001;ssrc=1234;interleaved=0-1;mode=record";
        let transport = RtspTransport::unmarshal(original_str).unwrap();
        let marshaled = transport.marshal();

        // Verify all components are present
        assert!(marshaled.contains("RTP/AVP/TCP"));
        assert!(marshaled.contains("unicast"));
        assert!(marshaled.contains("client_port=8000-8001"));
        assert!(marshaled.contains("server_port=9000-9001"));
        assert!(marshaled.contains("ssrc=1234"));
        assert!(marshaled.contains("interleaved=0-1"));
        assert!(marshaled.contains("mode=record"));
    }

    // ============================================
    // Edge Cases
    // ============================================

    #[test]
    fn test_unmarshal_empty_string() {
        let transport = RtspTransport::unmarshal("");
        // Empty string defaults to UDP but lacks client_port, so validation rejects it
        assert!(transport.is_err());
    }

    #[test]
    fn test_unmarshal_unknown_parameters() {
        let transport =
            RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;unknown_param=value")
                .unwrap();
        // Unknown parameters should be ignored
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
        assert_eq!(transport.cast_type, CastType::Unicast);
    }

    #[test]
    fn test_unmarshal_malformed_port() {
        // Malformed client_port parses as None, which now triggers validation error for UDP
        let result = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=invalid");
        assert!(result.is_err());
    }
}
