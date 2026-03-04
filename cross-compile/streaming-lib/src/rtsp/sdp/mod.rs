pub mod fmtp;
pub mod rtpmap;

use crate::rtsp::global_trait::{Marshal, Unmarshal};
use rtpmap::RtpMap;
use std::collections::HashMap;

use self::fmtp::Fmtp;

#[derive(Debug, Clone, Default)]
pub struct Bandwidth {
    b_type: String,
    bandwidth: u16,
}

impl Unmarshal for Bandwidth {
    //   b=AS:284\r\n\
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut sdp_bandwidth = Bandwidth::default();

        let parameters: Vec<&str> = raw_data.split(':').collect();
        if let Some(t) = parameters.first() {
            sdp_bandwidth.b_type = t.to_string();
        }

        if let Some(bandwidth) = parameters.get(1)
            && let Ok(bandwidth) = bandwidth.parse::<u16>()
        {
            sdp_bandwidth.bandwidth = bandwidth;
        }

        Ok(sdp_bandwidth)
    }
}

impl Marshal for Bandwidth {
    fn marshal(&self) -> String {
        format!("{}:{}\r\n", self.b_type, self.bandwidth)
    }
}

/*
v=0
o=- 946685052188730 1 IN IP4 0.0.0.0
s=RTSP/RTP Server
i=playback/robot=040082d087c335e3bd2b/camera=head/timerang1=1533620879-1533620898
t=0 0
a=tool:vlc 0.9.8a
a=type:broadcast
a=control:*
a=range:npt=0-
m=video 20003 RTP/AVP 97
b=RR:0
a=rtpmap:97 H264/90000
a=fmtp:97 profile-level-id=42C01E;packetization-mode=1;sprop-parameter-sets=Z0LAHtkDxWhAAAADAEAAAAwDxYuSAAAAAQ==,aMuMsgAAAAE=
a=control:track1
m=audio 11704 RTP/AVP 96 97 98 0 8 18 101 99 100 */

#[derive(Default, Debug, Clone)]
pub struct SdpMediaInfo {
    pub media_type: String,
    port: usize,
    protocol: String,
    fmts: Vec<u8>,
    bandwidth: Option<Bandwidth>,
    pub rtpmap: RtpMap,
    pub fmtp: Option<fmtp::Fmtp>,
    pub attributes: HashMap<String, String>,
}

// impl std::fmt::Debug for dyn TMsgConverter {
//     fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
//         write!(fmt, "S2 {{ member: {:?} }}", self.member)
//     }
// }

// impl Default for SdpMediaInfo {
//     fn default() -> Self {
//         Self {
//             fmtp: Box::new(fmtp::UnknownFmtpSdp::default()),
//             ..Default::default()
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct Sdp {
    pub raw_string: String,
    version: u16,
    origin: String,
    session: String,
    connection: String,
    timing: String,
    pub medias: Vec<SdpMediaInfo>,
    attributes: HashMap<String, String>,
}

impl Default for Sdp {
    fn default() -> Self {
        Self {
            raw_string: String::new(),
            version: 0,
            origin: format!("- {} 0 IN IP4 127.0.0.1", Self::ntp_session_id()),
            session: "No Name".to_string(),
            connection: "IN IP4 0.0.0.0".to_string(),
            timing: "0 0".to_string(),
            medias: Vec::new(),
            attributes: HashMap::new(),
        }
    }
}

impl Sdp {
    /// Generate a unique session-id (NTP timestamp in seconds) per RFC 4566 §5.2.
    /// Uses the NTP epoch (January 1, 1900) as the base.
    fn ntp_session_id() -> u64 {
        const NTP_UNIX_DIFF: u64 = 2_208_988_800;
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() + NTP_UNIX_DIFF)
    }
}

impl Unmarshal for SdpMediaInfo {
    //m=audio 11704 RTP/AVP 96 97 98 0 8 18 101 99 100 */
    //m=video 20003 RTP/AVP 97
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut sdp_media = SdpMediaInfo::default();
        let parameters: Vec<&str> = raw_data.split(' ').collect();

        if let Some(para_0) = parameters.first() {
            sdp_media.media_type = para_0.to_string();
        }

        if let Some(para_1) = parameters.get(1)
            && let Ok(port) = para_1.parse::<usize>()
        {
            sdp_media.port = port;
        }

        if let Some(para_2) = parameters.get(2) {
            sdp_media.protocol = para_2.to_string();
        }

        let mut cur_param_idx = 3;

        while let Some(fmt_str) = parameters.get(cur_param_idx) {
            if let Ok(fmt) = fmt_str.parse::<u8>() {
                sdp_media.fmts.push(fmt);
            }
            cur_param_idx += 1;
        }

        Ok(sdp_media)
    }
}

// m=video 0 RTP/AVP 96\r\n\
// b=AS:284\r\n\
// a=rtpmap:96 H264/90000\r\n\
// a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
// a=control:streamid=0\r\n\
// m=audio 0 RTP/AVP 97\r\n\
// b=AS:128\r\n\
// a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
// a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
// a=control:streamid=1\r\n"

impl Marshal for SdpMediaInfo {
    fn marshal(&self) -> String {
        let fmts_str = self
            .fmts
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<String>>()
            .join(" ");

        let bandwidth = if let Some(bandwidth) = &self.bandwidth {
            format!("b={}", bandwidth.marshal())
        } else {
            String::from("")
        };

        let mut sdp_media_info = format!(
            "m={} {} {} {}\r\n{}a=rtpmap:{}",
            self.media_type,
            self.port,
            self.protocol,
            fmts_str,
            bandwidth,
            self.rtpmap.marshal()
        );

        if let Some(fmtp) = &self.fmtp {
            sdp_media_info = format!("{}a=fmtp:{}", sdp_media_info, fmtp.marshal());
        }

        for (k, v) in &self.attributes {
            if v.is_empty() {
                sdp_media_info = format!("{sdp_media_info}a={k}\r\n");
            } else {
                sdp_media_info = format!("{sdp_media_info}a={k}:{v}\r\n");
            }
        }

        sdp_media_info
    }
}

impl Sdp {
    /// Parses a single SDP line and updates the Sdp struct
    fn parse_line(&mut self, line: &str) {
        let kv: Vec<&str> = line.trim().splitn(2, '=').collect();
        if kv.len() < 2 {
            tracing::error!(line = %line, "sdp_parse_error");
            return;
        }

        match kv[0] {
            "v" => self.parse_version(kv[1]),
            "o" => self.origin = kv[1].to_string(),
            "s" => self.session = kv[1].to_string(),
            "c" => self.connection = kv[1].to_string(),
            "t" => self.timing = kv[1].to_string(),
            "m" => self.parse_media(kv[1]),
            "b" => self.parse_bandwidth(kv[1]),
            "a" => self.parse_attribute(kv[1]),
            _ => tracing::info!(line = %line, "sdp_not_parsed"),
        }
    }

    fn parse_version(&mut self, value: &str) {
        if let Ok(version) = value.parse::<u16>() {
            self.version = version;
        }
    }

    fn parse_media(&mut self, value: &str) {
        if let Ok(sdp_media) = SdpMediaInfo::unmarshal(value) {
            self.medias.push(sdp_media);
        }
    }

    fn parse_bandwidth(&mut self, value: &str) {
        if let Some(cur_media) = self.medias.last_mut()
            && let Ok(bandwidth) = Bandwidth::unmarshal(value)
        {
            cur_media.bandwidth = Some(bandwidth);
        }
    }

    fn parse_attribute(&mut self, value: &str) {
        let attribute: Vec<&str> = value.splitn(2, ':').collect();
        let attr_name = attribute[0];
        let attr_value = attribute.get(1).copied().unwrap_or("");

        if let Some(cur_media) = self.medias.last_mut() {
            if !Self::try_parse_media_attribute(cur_media, attr_name, attr_value) {
                cur_media
                    .attributes
                    .insert(attr_name.to_string(), attr_value.to_string());
            }
        } else {
            self.attributes
                .insert(attr_name.to_string(), attr_value.to_string());
        }
    }

    /// Attempts to parse media-specific attributes (rtpmap, fmtp)
    /// Returns true if the attribute was handled, false otherwise
    fn try_parse_media_attribute(
        cur_media: &mut SdpMediaInfo,
        attr_name: &str,
        attr_value: &str,
    ) -> bool {
        if attr_value.is_empty() {
            return false;
        }

        match attr_name {
            "rtpmap" => {
                if let Ok(rtpmap) = RtpMap::unmarshal(attr_value) {
                    cur_media.rtpmap = rtpmap;
                    return true;
                }
            }
            "fmtp" => {
                if let Ok(fmtp) = Fmtp::new(&cur_media.rtpmap.encoding_name, attr_value) {
                    cur_media.fmtp = Some(fmtp);
                }
                return true;
            }
            _ => {}
        }
        false
    }
}

impl Unmarshal for Sdp {
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut sdp = Sdp {
            raw_string: raw_data.to_string(),
            ..Default::default()
        };

        let lines: Vec<&str> = raw_data.split(['\r', '\n']).collect();
        for line in lines {
            if !line.is_empty() {
                sdp.parse_line(line);
            }
        }

        Ok(sdp)
    }
}

// v=0\r\n\
// o=- 0 0 IN IP4 127.0.0.1\r\n\
// s=No Name\r\n\
// c=IN IP4 127.0.0.1\r\n\
// t=0 0\r\n\
// a=tool:libavformat 58.76.100\r\n\

impl Marshal for Sdp {
    fn marshal(&self) -> String {
        let mut sdp_str = format!(
            "v={}\r\no={}\r\ns={}\r\nc={}\r\nt={}\r\n",
            self.version, self.origin, self.session, self.connection, self.timing
        );

        for (k, v) in &self.attributes {
            if v.is_empty() {
                sdp_str = format!("{sdp_str}a={k}\r\n");
            } else {
                sdp_str = format!("{sdp_str}a={k}:{v}\r\n");
            }
        }

        for media_info in &self.medias {
            sdp_str = format!("{}{}", sdp_str, media_info.marshal());
        }

        sdp_str
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::rtsp::global_trait::{Marshal, Unmarshal};

    use super::Sdp;
    use super::rtpmap;

    #[test]
    fn test_parse_sdp() {
        let data2 = "ANNOUNCE rtsp://127.0.0.1:5544/stream RTSP/1.0\r\n\
        Content-Type: application/sdp\r\n\
        CSeq: 2\r\n\
        User-Agent: Lavf58.76.100\r\n\
        Content-Length: 500\r\n\
        \r\n\
        v=0\r\n\
        o=- 0 0 IN IP4 127.0.0.1\r\n\
        s=No Name\r\n\
        c=IN IP4 127.0.0.1\r\n\
        t=0 0\r\n\
        a=tool:libavformat 58.76.100\r\n\
        m=video 0 RTP/AVP 96\r\n\
        b=AS:284\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
        a=control:streamid=0\r\n\
        m=audio 0 RTP/AVP 97\r\n\
        b=AS:128\r\n\
        a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
        a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
        a=control:streamid=1\r\n";

        // v=0：SDP版本号，通常为0。
        // o=- 0 0 IN IP4 127.0.0.1：会话的所有者和会话ID，以及会话开始时间和会话结束时间的信息。
        // s=No Name：会话名称或标题。
        // c=IN IP4 127.0.0.1：表示会话数据传输的地址类型(IPv4)和地址(127.0.0.1)。
        // t=0 0：会话时间，包括会话开始时间和结束时间，这里的值都是0，表示会话没有预定义的结束时间。
        // a=tool:libavformat 58.76.100：会话所使用的工具或软件名称和版本号。

        // m=video 0 RTP/AVP 96：媒体类型(video或audio)、媒体格式(RTP/AVP)、媒体格式编号(96)和媒体流的传输地址。
        // b=AS:284：视频流所使用的带宽大小。
        // a=rtpmap:96 H264/90000：视频流所使用的编码方式(H.264)和时钟频率(90000)。
        // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E：视频流的格式参数，如分片方式、SPS和PPS等。
        // a=control:streamid=0：指定视频流的流ID。

        // m=audio 0 RTP/AVP 97：媒体类型(audio)、媒体格式(RTP/AVP)、媒体格式编号(97)和媒体流的传输地址。
        // b=AS:128：音频流所使用的带宽大小。
        // a=rtpmap:97 MPEG4-GENERIC/48000/2：音频流所使用的编码方式(MPEG4-GENERIC)、采样率(48000Hz)、和通道数(2)。
        // a=rtpmap:97 MPEG4-GENERIC/48000/2：音频流所使用的编码方式(MPEG4-GENERIC)、采样率(48000Hz)、和通道数(2)。
        // a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500：音频流的格式参数，如编码方式、采样长度、索引长度等。
        // a=control:streamid=1：指定音频流的流ID。

        if let Ok(sdp) = Sdp::unmarshal(data2) {
            println!("sdp : {sdp:?}");

            println!("sdp str : {}", sdp.marshal());
        }
    }

    #[test]
    fn test_marshal_attribute_without_value() {
        let data = "v=0\r\n\
        o=- 0 0 IN IP4 0.0.0.0\r\n\
        s=Test\r\n\
        c=IN IP4 0.0.0.0\r\n\
        t=0 0\r\n\
        a=sendonly\r\n\
        m=video 0 RTP/AVP 96\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=control:trackID=0\r\n";

        let sdp = Sdp::unmarshal(data).expect("SDP should parse");
        let marshaled = sdp.marshal();

        assert!(marshaled.contains("a=sendonly\r\n"));
        assert!(!marshaled.contains("a=sendonly:\r\n"));
    }
    #[test]
    fn test_str() {
        //let fmts: Vec<u8> = vec![5];
        //// fmts.push(6);
        //let fmts_str = fmts
        //    .iter()
        //    .map(|b| b.to_string())
        //    .collect::<Vec<String>>()
        //    .join(" ");

        //println!("=={fmts_str}==");
    }

    // ========== Bandwidth Tests ==========

    #[test]
    fn test_bandwidth_unmarshal() {
        use super::Bandwidth;
        let bw = Bandwidth::unmarshal("AS:284").unwrap();
        assert_eq!(bw.b_type, "AS");
        assert_eq!(bw.bandwidth, 284);
    }

    #[test]
    fn test_bandwidth_unmarshal_rr() {
        use super::Bandwidth;
        let bw = Bandwidth::unmarshal("RR:0").unwrap();
        assert_eq!(bw.b_type, "RR");
        assert_eq!(bw.bandwidth, 0);
    }

    #[test]
    fn test_bandwidth_marshal() {
        use super::Bandwidth;
        let bw = Bandwidth {
            b_type: "AS".to_string(),
            bandwidth: 128,
        };
        assert_eq!(bw.marshal(), "AS:128\r\n");
    }

    #[test]
    fn test_bandwidth_unmarshal_invalid_number() {
        use super::Bandwidth;
        let bw = Bandwidth::unmarshal("AS:abc").unwrap();
        assert_eq!(bw.b_type, "AS");
        assert_eq!(bw.bandwidth, 0); // default when parse fails
    }

    #[test]
    fn test_bandwidth_unmarshal_no_colon() {
        use super::Bandwidth;
        let bw = Bandwidth::unmarshal("AS").unwrap();
        assert_eq!(bw.b_type, "AS");
        assert_eq!(bw.bandwidth, 0);
    }

    // ========== SdpMediaInfo Unmarshal Tests ==========

    #[test]
    fn test_sdp_media_info_unmarshal_video() {
        use super::SdpMediaInfo;
        let mi = SdpMediaInfo::unmarshal("video 0 RTP/AVP 96").unwrap();
        assert_eq!(mi.media_type, "video");
        assert_eq!(mi.port, 0);
        assert_eq!(mi.protocol, "RTP/AVP");
        assert_eq!(mi.fmts, vec![96]);
    }

    #[test]
    fn test_sdp_media_info_unmarshal_audio_multiple_fmts() {
        use super::SdpMediaInfo;
        let mi = SdpMediaInfo::unmarshal("audio 11704 RTP/AVP 96 97 98 0 8").unwrap();
        assert_eq!(mi.media_type, "audio");
        assert_eq!(mi.port, 11704);
        assert_eq!(mi.protocol, "RTP/AVP");
        assert_eq!(mi.fmts, vec![96, 97, 98, 0, 8]);
    }

    #[test]
    fn test_sdp_media_info_unmarshal_no_fmts() {
        use super::SdpMediaInfo;
        let mi = SdpMediaInfo::unmarshal("video 0 RTP/AVP").unwrap();
        assert_eq!(mi.media_type, "video");
        assert_eq!(mi.protocol, "RTP/AVP");
        assert!(mi.fmts.is_empty());
    }

    // ========== SdpMediaInfo Marshal Tests ==========

    #[test]
    fn test_sdp_media_info_marshal_basic() {
        use super::SdpMediaInfo;
        use rtpmap::RtpMap;

        let mi = SdpMediaInfo {
            media_type: "video".to_string(),
            port: 0,
            protocol: "RTP/AVP".to_string(),
            fmts: vec![96],
            bandwidth: None,
            rtpmap: RtpMap {
                payload_type: 96,
                encoding_name: "H264".to_string(),
                clock_rate: 90000,
                encoding_param: String::new(),
            },
            fmtp: None,
            attributes: HashMap::new(),
        };
        let marshaled = mi.marshal();
        assert!(marshaled.contains("m=video 0 RTP/AVP 96"));
        assert!(marshaled.contains("a=rtpmap:"));
    }

    #[test]
    fn test_sdp_media_info_marshal_with_bandwidth() {
        use super::{Bandwidth, SdpMediaInfo};
        use rtpmap::RtpMap;

        let mi = SdpMediaInfo {
            media_type: "video".to_string(),
            port: 0,
            protocol: "RTP/AVP".to_string(),
            fmts: vec![96],
            bandwidth: Some(Bandwidth {
                b_type: "AS".to_string(),
                bandwidth: 284,
            }),
            rtpmap: RtpMap::default(),
            fmtp: None,
            attributes: HashMap::new(),
        };
        let marshaled = mi.marshal();
        assert!(marshaled.contains("b=AS:284"));
    }

    #[test]
    fn test_sdp_media_info_marshal_with_attributes() {
        use super::SdpMediaInfo;
        use rtpmap::RtpMap;

        let mut attrs = HashMap::new();
        attrs.insert("control".to_string(), "streamid=0".to_string());
        let mi = SdpMediaInfo {
            media_type: "video".to_string(),
            port: 0,
            protocol: "RTP/AVP".to_string(),
            fmts: vec![96],
            bandwidth: None,
            rtpmap: RtpMap::default(),
            fmtp: None,
            attributes: attrs,
        };
        let marshaled = mi.marshal();
        assert!(marshaled.contains("a=control:streamid=0\r\n"));
    }

    #[test]
    fn test_sdp_media_info_marshal_attribute_no_value() {
        use super::SdpMediaInfo;
        use rtpmap::RtpMap;

        let mut attrs = HashMap::new();
        attrs.insert("recvonly".to_string(), String::new());
        let mi = SdpMediaInfo {
            media_type: "audio".to_string(),
            port: 0,
            protocol: "RTP/AVP".to_string(),
            fmts: vec![97],
            bandwidth: None,
            rtpmap: RtpMap::default(),
            fmtp: None,
            attributes: attrs,
        };
        let marshaled = mi.marshal();
        assert!(marshaled.contains("a=recvonly\r\n"));
        assert!(!marshaled.contains("a=recvonly:\r\n"));
    }

    // ========== Sdp Unmarshal Tests ==========

    #[test]
    fn test_sdp_unmarshal_video_only() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 127.0.0.1\r\n\
            s=Test\r\n\
            c=IN IP4 0.0.0.0\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n\
            a=control:trackID=0\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.version, 0);
        assert_eq!(sdp.medias.len(), 1);
        assert_eq!(sdp.medias[0].media_type, "video");
        assert_eq!(sdp.medias[0].rtpmap.encoding_name, "H264");
        assert_eq!(sdp.medias[0].rtpmap.clock_rate, 90000);
    }

    #[test]
    fn test_sdp_unmarshal_with_bandwidth() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            b=AS:284\r\n\
            a=rtpmap:96 H264/90000\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.medias[0].bandwidth.is_some());
        let bw = sdp.medias[0].bandwidth.as_ref().unwrap();
        assert_eq!(bw.b_type, "AS");
        assert_eq!(bw.bandwidth, 284);
    }

    #[test]
    fn test_sdp_unmarshal_multiple_medias() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 127.0.0.1\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n\
            m=audio 0 RTP/AVP 97\r\n\
            a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.medias.len(), 2);
        assert_eq!(sdp.medias[0].media_type, "video");
        assert_eq!(sdp.medias[1].media_type, "audio");
        assert_eq!(sdp.medias[1].rtpmap.encoding_name, "MPEG4-GENERIC");
    }

    #[test]
    fn test_sdp_unmarshal_session_attributes() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            a=tool:libavformat 58.76.100\r\n\
            a=type:broadcast\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(
            sdp.attributes.get("tool"),
            Some(&"libavformat 58.76.100".to_string())
        );
        assert_eq!(sdp.attributes.get("type"), Some(&"broadcast".to_string()));
    }

    #[test]
    fn test_sdp_unmarshal_session_attribute_no_value() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            a=sendonly\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.attributes.get("sendonly"), Some(&"".to_string()));
    }

    #[test]
    fn test_sdp_unmarshal_preserves_origin() {
        let data = "v=0\r\n\
            o=- 946685052188730 1 IN IP4 0.0.0.0\r\n\
            s=RTSP/RTP Server\r\n\
            t=0 0\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.origin.contains("946685052188730"));
    }

    #[test]
    fn test_sdp_unmarshal_preserves_session() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=My Custom Session\r\n\
            t=0 0\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.session, "My Custom Session");
    }

    #[test]
    fn test_sdp_unmarshal_preserves_connection() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=Test\r\n\
            c=IN IP4 192.168.1.100\r\n\
            t=0 0\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.connection, "IN IP4 192.168.1.100");
    }

    #[test]
    fn test_sdp_unmarshal_preserves_raw_string() {
        let data = "v=0\r\ns=Test\r\n";
        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.raw_string, data);
    }

    // ========== Sdp Marshal Tests ==========

    #[test]
    fn test_sdp_marshal_basic() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 127.0.0.1\r\n\
            s=Test\r\n\
            c=IN IP4 0.0.0.0\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        let marshaled = sdp.marshal();

        assert!(marshaled.contains("v=0\r\n"));
        assert!(marshaled.contains("s=Test\r\n"));
        assert!(marshaled.contains("t=0 0\r\n"));
        assert!(marshaled.contains("m=video 0 RTP/AVP 96\r\n"));
    }

    #[test]
    fn test_sdp_marshal_session_attribute_with_value() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            a=tool:mylib\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        let marshaled = sdp.marshal();
        assert!(marshaled.contains("a=tool:mylib\r\n"));
    }

    #[test]
    fn test_sdp_marshal_session_attribute_no_value() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 0.0.0.0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            a=recvonly\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        let marshaled = sdp.marshal();
        assert!(marshaled.contains("a=recvonly\r\n"));
        assert!(!marshaled.contains("a=recvonly:\r\n"));
    }

    // ========== Sdp Default Tests ==========

    #[test]
    fn test_sdp_default() {
        let sdp = Sdp::default();
        assert_eq!(sdp.version, 0);
        assert_eq!(sdp.session, "No Name");
        assert_eq!(sdp.connection, "IN IP4 0.0.0.0");
        assert_eq!(sdp.timing, "0 0");
        assert!(sdp.medias.is_empty());
        assert!(sdp.origin.contains("IN IP4 127.0.0.1"));
    }

    // ========== Sdp parse_line edge cases ==========

    #[test]
    fn test_sdp_parse_line_invalid_no_equals() {
        // Lines without '=' should be silently ignored
        let data = "v=0\r\ninvalidline\r\ns=Test\r\nt=0 0\r\n";
        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.session, "Test");
    }

    #[test]
    fn test_sdp_parse_bandwidth_before_media_ignored() {
        // b= lines before any m= line should be ignored
        let data = "v=0\r\nb=AS:128\r\nm=video 0 RTP/AVP 96\r\na=rtpmap:96 H264/90000\r\n";
        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.medias[0].bandwidth.is_none());
    }

    #[test]
    fn test_sdp_media_attribute_control() {
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n\
            a=control:streamid=0\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(
            sdp.medias[0].attributes.get("control"),
            Some(&"streamid=0".to_string())
        );
    }

    #[test]
    fn test_sdp_media_attribute_with_fmtp() {
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n\
            a=fmtp:96 packetization-mode=1; profile-level-id=64001E\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.medias[0].fmtp.is_some());
    }

    #[test]
    fn test_sdp_unmarshal_version_non_zero() {
        let data = "v=1\r\ns=Test\r\nt=0 0\r\n";
        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.version, 1);
    }

    #[test]
    fn test_sdp_unmarshal_invalid_version() {
        let data = "v=abc\r\ns=Test\r\nt=0 0\r\n";
        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.version, 0); // default when parse fails
    }

    // ========== Bandwidth Default Test ==========

    #[test]
    fn test_bandwidth_default() {
        use super::Bandwidth;
        let bw = Bandwidth::default();
        assert_eq!(bw.b_type, "");
        assert_eq!(bw.bandwidth, 0);
    }

    // ========== Additional parse_line edge cases ==========

    #[test]
    fn test_sdp_parse_line_unknown_type_ignored() {
        // 'i' (session info) and 'z' (time zone) lines hit the `_` arm
        let data = "v=0\r\ni=Session Description\r\ns=Test\r\nt=0 0\r\nz=2882844526 -1h\r\n";
        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(sdp.session, "Test");
        assert_eq!(sdp.version, 0);
    }

    #[test]
    fn test_sdp_media_info_unmarshal_invalid_port() {
        use super::SdpMediaInfo;
        let mi = SdpMediaInfo::unmarshal("video notaport RTP/AVP 96").unwrap();
        assert_eq!(mi.media_type, "video");
        assert_eq!(mi.port, 0); // default when parse fails
        assert_eq!(mi.protocol, "RTP/AVP");
        assert_eq!(mi.fmts, vec![96]);
    }

    #[test]
    fn test_sdp_media_info_unmarshal_empty() {
        use super::SdpMediaInfo;
        let mi = SdpMediaInfo::unmarshal("").unwrap();
        assert_eq!(mi.media_type, "");
        assert_eq!(mi.port, 0);
    }

    #[test]
    fn test_sdp_try_parse_media_attribute_empty_value() {
        // When attr_value is empty, try_parse_media_attribute returns false
        // and the attribute is stored as a regular attribute
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H264/90000\r\n\
            a=recvonly\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        // 'recvonly' has no value, so it's stored as regular attribute
        assert_eq!(
            sdp.medias[0].attributes.get("recvonly"),
            Some(&"".to_string())
        );
    }

    #[test]
    fn test_sdp_media_info_marshal_with_fmtp() {
        use super::{SdpMediaInfo, fmtp};
        use rtpmap::RtpMap;

        let fmtp_obj =
            fmtp::Fmtp::new("h264", "96 packetization-mode=1; profile-level-id=640016").unwrap();

        let mi = SdpMediaInfo {
            media_type: "video".to_string(),
            port: 0,
            protocol: "RTP/AVP".to_string(),
            fmts: vec![96],
            bandwidth: None,
            rtpmap: RtpMap {
                payload_type: 96,
                encoding_name: "H264".to_string(),
                clock_rate: 90000,
                encoding_param: String::new(),
            },
            fmtp: Some(fmtp_obj),
            attributes: HashMap::new(),
        };
        let marshaled = mi.marshal();
        assert!(marshaled.contains("m=video 0 RTP/AVP 96"));
        assert!(marshaled.contains("a=fmtp:"));
        assert!(marshaled.contains("packetization-mode=1"));
    }

    #[test]
    fn test_sdp_bandwidth_roundtrip() {
        use super::Bandwidth;
        use crate::rtsp::global_trait::{Marshal, Unmarshal};

        let bw = Bandwidth::unmarshal("AS:284").unwrap();
        let marshaled = bw.marshal();
        assert_eq!(marshaled, "AS:284\r\n");
        let bw2 = Bandwidth::unmarshal(marshaled.trim_end()).unwrap();
        assert_eq!(bw.b_type, bw2.b_type);
        assert_eq!(bw.bandwidth, bw2.bandwidth);
    }

    #[test]
    fn test_sdp_full_roundtrip() {
        let data = "v=0\r\n\
            o=- 0 0 IN IP4 127.0.0.1\r\n\
            s=Test Session\r\n\
            c=IN IP4 0.0.0.0\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            b=AS:284\r\n\
            a=rtpmap:96 H264/90000\r\n\
            a=control:trackID=0\r\n\
            m=audio 0 RTP/AVP 97\r\n\
            a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
            a=control:trackID=1\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        let marshaled = sdp.marshal();
        let sdp2 = Sdp::unmarshal(&marshaled).unwrap();

        assert_eq!(sdp.version, sdp2.version);
        assert_eq!(sdp.session, sdp2.session);
        assert_eq!(sdp.medias.len(), sdp2.medias.len());
        assert_eq!(sdp.medias[0].media_type, sdp2.medias[0].media_type);
        assert_eq!(sdp.medias[1].media_type, sdp2.medias[1].media_type);
    }

    #[test]
    fn test_sdp_unmarshal_media_attribute_no_colon() {
        // a=sendonly inside a media block (no colon = empty value)
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=sendonly\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert_eq!(
            sdp.medias[0].attributes.get("sendonly"),
            Some(&"".to_string())
        );
    }

    #[test]
    fn test_sdp_unmarshal_fmtp_with_unsupported_codec() {
        // rtpmap sets encoding_name to "VP8", fmtp tries to parse
        // but Fmtp::new("VP8", ...) returns Err, so fmtp stays None
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 VP8/90000\r\n\
            a=fmtp:96 some-params=value\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.medias[0].fmtp.is_none());
    }

    #[test]
    fn test_sdp_unmarshal_empty_string() {
        let sdp = Sdp::unmarshal("").unwrap();
        assert!(sdp.medias.is_empty());
        assert_eq!(sdp.version, 0);
    }

    #[test]
    fn test_sdp_media_info_unmarshal_large_fmt_values_ignored() {
        use super::SdpMediaInfo;
        // fmt values > 255 can't fit in u8 and should be silently skipped
        let mi = SdpMediaInfo::unmarshal("video 0 RTP/AVP 96 300 97").unwrap();
        assert_eq!(mi.fmts, vec![96, 97]); // 300 skipped
    }

    #[test]
    fn test_sdp_unmarshal_h265_fmtp() {
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=video 0 RTP/AVP 96\r\n\
            a=rtpmap:96 H265/90000\r\n\
            a=fmtp:96 sprop-vps=QAEMAf; sprop-sps=QgEBAW; sprop-pps=RAHA\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.medias[0].fmtp.is_some());
    }

    #[test]
    fn test_sdp_unmarshal_mpeg4_fmtp() {
        let data = "v=0\r\n\
            s=Test\r\n\
            t=0 0\r\n\
            m=audio 0 RTP/AVP 97\r\n\
            a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
            a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n";

        let sdp = Sdp::unmarshal(data).unwrap();
        assert!(sdp.medias[0].fmtp.is_some());
    }
}
