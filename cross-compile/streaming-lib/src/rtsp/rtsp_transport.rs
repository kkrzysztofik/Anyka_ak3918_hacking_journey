use crate::rtsp::global_trait::Marshal;

use super::global_trait::Unmarshal;
use super::rtsp_utils::scanf;

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

impl Unmarshal for RtspTransport {
    fn unmarshal(raw_data: &str) -> Option<Self> {
        let mut rtsp_transport = RtspTransport::default();

        let param_parts: Vec<&str> = raw_data.split(';').collect();
        for part in param_parts {
            let kv: Vec<&str> = part.split('=').collect();
            match kv[0] {
                "RTP/AVP/TCP" => {
                    rtsp_transport.protocol_type = ProtocolType::TCP;
                }
                "RTP/AVP/UDP" | "RTP/AVP" => {
                    rtsp_transport.protocol_type = ProtocolType::UDP;
                }
                "unicast" => {
                    rtsp_transport.cast_type = CastType::Unicast;
                }
                "multicast" => {
                    rtsp_transport.cast_type = CastType::Multicast;
                }
                "mode" => {
                    rtsp_transport.transport_mod = Some(kv[1].to_string());
                }
                "client_port" => {
                    let ports = scanf!(kv[1], '-', u16, u16);

                    let mut client_ports: [u16; 2] = [0, 0];
                    if let Some(port) = ports.0 {
                        client_ports[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        client_ports[1] = port;
                    }

                    rtsp_transport.client_port = Some(client_ports);
                }
                "server_port" => {
                    let ports = scanf!(kv[1], '-', u16, u16);

                    let mut server_ports: [u16; 2] = [0, 0];
                    if let Some(port) = ports.0 {
                        server_ports[0] = port;
                    }
                    if let Some(port) = ports.1 {
                        server_ports[1] = port;
                    }

                    rtsp_transport.server_port = Some(server_ports);
                }
                "interleaved" => {
                    let vals = scanf!(kv[1], '-', u8, u8);

                    let mut interleaveds: [u8; 2] = [0, 0];
                    if let Some(val) = vals.0 {
                        interleaveds[0] = val;
                    }
                    if let Some(val) = vals.1 {
                        interleaveds[1] = val;
                    }

                    rtsp_transport.interleaved = Some(interleaveds);
                }
                "ssrc" => {
                    let ssrc_str = kv[1].trim();
                    let ssrc = if ssrc_str.starts_with("0x") || ssrc_str.starts_with("0X") {
                        u32::from_str_radix(&ssrc_str[2..], 16).ok()
                    } else {
                        ssrc_str.parse::<u32>().ok()
                    };
                    rtsp_transport.ssrc = ssrc;
                }

                _ => {}
            }
        }

        Some(rtsp_transport)
    }
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

    use crate::rtsp::global_trait::Marshal;
    use crate::rtsp::global_trait::Unmarshal;

    use super::{RtspTransport, CastType, ProtocolType};

    #[test]
    fn test_parse_transport() {
        let parser = RtspTransport::unmarshal(
            "RTP/AVP;unicast;client_port=8000-8001;server_port=9000-9001;ssrc=1234;interleaved=0-1;mode=record",
        ).unwrap();

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
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
    }

    #[test]
    fn test_unmarshal_rtp_avp_udp_explicit() {
        let transport = RtspTransport::unmarshal("RTP/AVP/UDP;unicast").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
    }

    #[test]
    fn test_unmarshal_rtp_avp_tcp() {
        let transport = RtspTransport::unmarshal("RTP/AVP/TCP;unicast").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::TCP);
    }

    // ============================================
    // Cast Type Tests
    // ============================================

    #[test]
    fn test_unmarshal_unicast() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast").unwrap();
        assert_eq!(transport.cast_type, CastType::Unicast);
    }

    #[test]
    fn test_unmarshal_multicast() {
        let transport = RtspTransport::unmarshal("RTP/AVP;multicast").unwrap();
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
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;server_port=6000-6001").unwrap();
        assert_eq!(transport.server_port, Some([6000, 6001]));
    }

    #[test]
    fn test_unmarshal_both_ports() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=5000-5001;server_port=6000-6001").unwrap();
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
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;ssrc=0x12345678").unwrap();
        assert_eq!(transport.ssrc, Some(0x12345678));
    }

    #[test]
    fn test_unmarshal_ssrc_decimal() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;ssrc=1234567890").unwrap();
        assert_eq!(transport.ssrc, Some(1234567890));
    }

    // ============================================
    // Mode Tests
    // ============================================

    #[test]
    fn test_unmarshal_mode_record() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;mode=record").unwrap();
        assert_eq!(transport.transport_mod, Some("record".to_string()));
    }

    #[test]
    fn test_unmarshal_mode_play() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;mode=play").unwrap();
        assert_eq!(transport.transport_mod, Some("play".to_string()));
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
    fn test_unmarshal_minimal() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast").unwrap();
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
        assert_eq!(transport.cast_type, CastType::Unicast);
        assert_eq!(transport.client_port, None);
        assert_eq!(transport.server_port, None);
        assert_eq!(transport.ssrc, None);
        assert_eq!(transport.interleaved, None);
        assert_eq!(transport.transport_mod, None);
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
        let original_str = "RTP/AVP;unicast";
        let transport = RtspTransport::unmarshal(original_str).unwrap();
        let marshaled = transport.marshal();
        // Marshal format may differ slightly, but should contain key components
        assert!(marshaled.contains("RTP/AVP"));
        assert!(marshaled.contains("unicast"));
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
        // Should return default transport
        assert!(transport.is_some());
    }

    #[test]
    fn test_unmarshal_unknown_parameters() {
        let transport = RtspTransport::unmarshal("RTP/AVP;unicast;unknown_param=value").unwrap();
        // Unknown parameters should be ignored
        assert_eq!(transport.protocol_type, ProtocolType::UDP);
        assert_eq!(transport.cast_type, CastType::Unicast);
    }

    #[test]
    fn test_unmarshal_malformed_port() {
        let _transport = RtspTransport::unmarshal("RTP/AVP;unicast;client_port=invalid").unwrap();
        // Malformed port should be ignored or set to default
        // Implementation may vary, but should not panic
        assert!(true);
    }
}
