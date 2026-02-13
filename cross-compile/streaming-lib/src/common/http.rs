use crate::scanf;
use indexmap::IndexMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::BuildHasherDefault;

pub type HttpIndexMap = IndexMap<String, String, BuildHasherDefault<DefaultHasher>>;

pub trait Unmarshal {
    fn unmarshal(request_data: &str) -> Option<Self>
    where
        Self: Sized;
}

pub trait Marshal {
    fn marshal(&self) -> String;
}

#[derive(Debug, Clone, Default)]
pub enum Schema {
    //used for webrtc(WHIP/WHEP)
    WEBRTC,
    RTSP,
    #[default]
    UNKNOWN,
}

impl fmt::Display for Schema {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Schema::RTSP => {
                write!(f, "rtsp")
            }
            //Because webrtc request uri does not contain the schema name, so here write empty string.
            Schema::WEBRTC => {
                write!(f, "")
            }
            Schema::UNKNOWN => {
                write!(f, "unknown")
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Uri {
    pub schema: Schema,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
}

impl Uri {
    fn detect_schema(url: &str) -> Schema {
        if url.starts_with("rtsp://") {
            Schema::RTSP
        } else if url.starts_with("/whip") || url.starts_with("/whep") {
            Schema::WEBRTC
        } else {
            log::warn!("cannot judge the schema: {}", url);
            Schema::UNKNOWN
        }
    }

    fn extract_host_port(authority: &str) -> (String, Option<u16>) {
        let (host_val, port_val) = scanf!(authority, ':', String, u16);
        let host = host_val.unwrap_or_else(|| authority.to_string());
        (host, port_val)
    }

    fn parse_path_and_query(&mut self, path_with_query: &str) {
        let parts: Vec<&str> = path_with_query.splitn(2, '?').collect();
        self.path = parts[0].to_string();
        if parts.len() > 1 {
            self.query = Some(parts[1].to_string());
        }
    }

    fn parse_rtsp_uri(url: &str) -> Option<Self> {
        let without_prefix = url.strip_prefix("rtsp://")?;
        let (authority, path_with_query) = match without_prefix.find('/') {
            Some(idx) => (&without_prefix[..idx], &without_prefix[idx + 1..]),
            None => (without_prefix, ""),
        };
        let (host, port) = Self::extract_host_port(authority);
        let mut uri = Uri {
            schema: Schema::RTSP,
            host,
            port,
            ..Default::default()
        };
        uri.parse_path_and_query(path_with_query);
        Some(uri)
    }

    fn parse_webrtc_or_unknown_uri(url: &str, schema: Schema) -> Self {
        let mut uri = Uri {
            schema,
            ..Default::default()
        };
        uri.parse_path_and_query(url);
        uri
    }
}

impl Unmarshal for Uri {
    fn unmarshal(url: &str) -> Option<Self> {
        let schema = Self::detect_schema(url);
        match schema {
            Schema::RTSP => Self::parse_rtsp_uri(url),
            Schema::WEBRTC | Schema::UNKNOWN => {
                Some(Self::parse_webrtc_or_unknown_uri(url, schema))
            }
        }
    }
}

impl Marshal for Uri {
    fn marshal(&self) -> String {
        /*first pice path and query together*/
        let path_with_query = if let Some(query) = &self.query {
            format!("{}?{}", self.path, query)
        } else {
            self.path.clone()
        };

        match self.schema {
            Schema::RTSP => {
                let host_with_port = if let Some(port) = &self.port {
                    format!("{}:{}", self.host, port)
                } else {
                    self.host.clone()
                };
                format!("{}://{}/{}", self.schema, host_with_port, path_with_query)
            }
            Schema::WEBRTC => path_with_query,
            Schema::UNKNOWN => path_with_query,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpRequest {
    pub method: String,
    pub uri: Uri,
    /*parse the query and save the results*/
    pub query_pairs: HttpIndexMap,
    pub version: String,
    pub headers: HttpIndexMap,
    pub body: Option<String>,
}

impl HttpRequest {
    pub fn get_header(&self, header_name: &str) -> Option<&String> {
        if let Some(val) = self.headers.get(header_name) {
            return Some(val);
        }

        self.headers.iter().find_map(|(name, value)| {
            if name.trim().eq_ignore_ascii_case(header_name) {
                Some(value)
            } else {
                None
            }
        })
    }
}

pub fn parse_content_length(request_data: &str) -> Option<u32> {
    request_data.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("Content-Length") {
            value.trim().parse().ok()
        } else {
            None
        }
    })
}

fn find_header_terminator(data: &[u8]) -> Option<usize> {
    data.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length_from_header_block(header_block: &str) -> Result<usize, String> {
    for line in header_block.lines().skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };

        if name.trim().eq_ignore_ascii_case("Content-Length") {
            let parsed = value
                .trim()
                .parse::<usize>()
                .map_err(|err| format!("invalid Content-Length value: {err}"))?;
            return Ok(parsed);
        }
    }

    Ok(0)
}

/// Attempts to determine the total length of the next HTTP/RTSP message in `data`.
///
/// This is required for RTSP-over-TCP, where RTSP messages can be coalesced and
/// can be immediately followed by interleaved `$` binary frames in the same read.
///
/// Returns:
/// - `Ok(Some(total_len))` when a full message is present.
/// - `Ok(None)` when more bytes are needed.
/// - `Err(..)` when the header block is invalid UTF-8 or `Content-Length` is invalid.
pub fn try_get_complete_message_len(data: &[u8]) -> Result<Option<usize>, String> {
    let Some(header_start) = find_header_terminator(data) else {
        return Ok(None);
    };

    let header_end = header_start + 4;
    let header_block = std::str::from_utf8(&data[..header_end])
        .map_err(|err| format!("invalid header utf-8: {err}"))?;
    let content_len = parse_content_length_from_header_block(header_block)?;
    let total_len = header_end + content_len;

    if data.len() < total_len {
        return Ok(None);
    }

    Ok(Some(total_len))
}

impl HttpRequest {
    fn parse_request_line(line: &str) -> Option<(String, String, String)> {
        let mut fields = line.split_ascii_whitespace();
        let method = fields.next()?.to_string();
        let url = fields.next()?.to_string();
        let version = fields.next()?.to_string();
        Some((method, url, version))
    }

    fn parse_query_pairs(query: &str) -> HttpIndexMap {
        let mut pairs = HttpIndexMap::default();
        for ele in query.split('&') {
            let (k, v) = scanf!(ele, '=', String, String);
            if let (Some(key), Some(val)) = (k, v) {
                pairs.insert(key, val);
            }
        }
        pairs
    }

    fn parse_headers_and_extract_host<'a>(
        lines: impl Iterator<Item = &'a str>,
    ) -> (HttpIndexMap, Option<(String, Option<u16>)>) {
        let mut headers = HttpIndexMap::default();
        let mut host_info: Option<(String, Option<u16>)> = None;

        for line in lines {
            if let Some(index) = line.find(':') {
                let name = line[..index].to_string();
                let value = line[index + 1..].trim_start().to_string();
                if name.trim().eq_ignore_ascii_case("Host") {
                    let (address_val, port_val) = scanf!(value.as_str(), ':', String, u16);
                    host_info = Some((address_val.unwrap_or(value.clone()), port_val));
                }
                headers.insert(name, value);
            }
        }
        (headers, host_info)
    }
}

impl Unmarshal for HttpRequest {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let idx = request_data.find("\r\n\r\n")?;
        let data_except_body = &request_data[..idx];
        let mut lines = data_except_body.lines();

        let first_line = lines.next()?;
        let (method, url, version) = Self::parse_request_line(first_line)?;

        let mut uri = Uri::unmarshal(&url)?;
        let query_pairs = uri
            .query
            .as_ref()
            .map(|q| Self::parse_query_pairs(q))
            .unwrap_or_default();

        let (headers, host_info) = Self::parse_headers_and_extract_host(lines);
        if let Some((host, port)) = host_info {
            uri.host = host;
            uri.port = port;
        }

        let header_end_idx = idx + 4;
        log::trace!(
            "header_end_idx is: {} {}",
            header_end_idx,
            request_data.len()
        );

        let body = if request_data.len() > header_end_idx {
            Some(request_data[header_end_idx..].to_string())
        } else {
            None
        };

        Some(HttpRequest {
            method,
            uri,
            query_pairs,
            version,
            headers,
            body,
        })
    }
}

impl Marshal for HttpRequest {
    fn marshal(&self) -> String {
        let mut request_str = format!(
            "{} {} {}\r\n",
            self.method,
            self.uri.marshal(),
            self.version
        );
        let mut wrote_content_length = false;
        for (header_name, header_value) in &self.headers {
            if header_name.trim().eq_ignore_ascii_case("Content-Length") {
                wrote_content_length = true;
                if let Some(body) = &self.body {
                    request_str += &format!("{header_name}: {}\r\n", body.len());
                }
            } else {
                request_str += &format!("{header_name}: {header_value}\r\n");
            }
        }

        if !wrote_content_length && let Some(body) = &self.body {
            request_str += &format!("Content-Length: {}\r\n", body.len());
        }

        request_str += "\r\n";
        if let Some(body) = &self.body {
            request_str += body;
        }
        request_str
    }
}

#[derive(Debug, Clone, Default)]
pub struct HttpResponse {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: String,
    pub headers: HttpIndexMap,
    pub body: Option<String>,
}

impl HttpResponse {
    pub fn get_header(&self, header_name: &str) -> Option<&String> {
        if let Some(val) = self.headers.get(header_name) {
            return Some(val);
        }

        self.headers.iter().find_map(|(name, value)| {
            if name.trim().eq_ignore_ascii_case(header_name) {
                Some(value)
            } else {
                None
            }
        })
    }
}

impl Unmarshal for HttpResponse {
    fn unmarshal(request_data: &str) -> Option<Self> {
        let mut http_response = HttpResponse::default();
        let idx = request_data.find("\r\n\r\n")?;
        let data_except_body = &request_data[..idx];
        let mut lines = data_except_body.lines();
        //parse the first line
        if let Some(request_first_line) = lines.next() {
            let mut fields = request_first_line.split_ascii_whitespace();

            if let Some(version) = fields.next() {
                http_response.version = version.to_string();
            }
            if let Some(status) = fields.next()
                && let Ok(status_code) = status.parse::<u16>()
            {
                http_response.status_code = status_code;
            }
            let reason_phrase = fields.collect::<Vec<&str>>().join(" ");
            if !reason_phrase.is_empty() {
                http_response.reason_phrase = reason_phrase;
            }
        }
        //parse headers
        for line in lines {
            if let Some(index) = line.find(':') {
                let name = line[..index].to_string();
                let value = line[index + 1..].trim_start().to_string();
                http_response.headers.insert(name, value);
            }
        }
        let header_end_idx = idx + 4;

        if request_data.len() > header_end_idx {
            //parse body
            http_response.body = Some(request_data[header_end_idx..].to_string());
        }

        Some(http_response)
    }
}

impl Marshal for HttpResponse {
    fn marshal(&self) -> String {
        let mut response_str = format!(
            "{} {} {}\r\n",
            self.version, self.status_code, self.reason_phrase
        );

        let mut wrote_content_length = false;
        for (header_name, header_value) in &self.headers {
            if header_name.trim().eq_ignore_ascii_case("Content-Length") {
                wrote_content_length = true;
                continue;
            }
            {
                response_str += &format!("{header_name}: {header_value}\r\n");
            }
        }

        if let Some(body) = &self.body {
            response_str += &format!("Content-Length: {}\r\n", body.len());
        } else if wrote_content_length {
            response_str += "Content-Length: 0\r\n";
        }

        response_str += "\r\n";
        if let Some(body) = &self.body {
            response_str += body;
        }
        response_str
    }
}

#[cfg(test)]
mod tests {

    use super::Marshal;
    use super::Unmarshal;

    use super::HttpRequest;
    use super::HttpResponse;
    use super::Schema;

    use super::HttpIndexMap;
    use std::io::BufRead;
    #[allow(dead_code)]
    fn read_headers(reader: &mut dyn BufRead) -> Option<HttpIndexMap> {
        let mut headers = HttpIndexMap::default();
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    if let Some(index) = line.find(": ") {
                        let name = line[..index].to_string();
                        let value = line[index + 2..].trim().to_string();
                        headers.insert(name, value);
                    }
                }
                Err(_) => return None,
            }
        }
        Some(headers)
    }

    #[test]
    fn test_parse_http_request() {
        let request = "POST /whip/endpoint?app=live&stream=test HTTP/1.1\r\n\
        Host: whip.example.com\r\n\
        Content-Type: application/sdp\r\n\
        Content-Length: 1326\r\n\
        \r\n\
        v=0\r\n\
        o=- 5228595038118931041 2 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        a=group:BUNDLE 0 1\r\n\
        a=extmap-allow-mixed\r\n\
        a=msid-semantic: WMS\r\n\
        m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:EsAw\r\n\
        a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 DA:7B:57:DC:28:CE:04:4F:31:79:85:C4:31:67:EB:27:58:29:ED:77:2A:0D:24:AE:ED:AD:30:BC:BD:F1:9C:02\r\n\
        a=setup:actpass\r\n\
        a=mid:0\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=sendonly\r\n\
        a=msid:- d46fb922-d52a-4e9c-aa87-444eadc1521b\r\n\
        a=rtcp-mux\r\n\
        a=rtpmap:111 opus/48000/2\r\n\
        a=fmtp:111 minptime=10;useinbandfec=1\r\n\
        m=video 9 UDP/TLS/RTP/SAVPF 96 97\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:EsAw\r\n\
        a=ice-pwd:bP+XJMM09aR8AiX1jdukzR6Y\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 DA:7B:57:DC:28:CE:04:4F:31:79:85:C4:31:67:EB:27:58:29:ED:77:2A:0D:24:AE:ED:AD:30:BC:BD:F1:9C:02\r\n\
        a=setup:actpass\r\n\
        a=mid:1\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\n\
        a=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\n\
        a=sendonly\r\n\
        a=msid:- d46fb922-d52a-4e9c-aa87-444eadc1521b\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:96 VP8/90000\r\n\
        a=rtcp-fb:96 ccm fir\r\n\
        a=rtcp-fb:96 nack\r\n\
        a=rtcp-fb:96 nack pli\r\n\
        a=rtpmap:97 rtx/90000\r\n\
        a=fmtp:97 apt=96\r\n";

        let parser = HttpRequest::unmarshal(request).expect("should parse request");
        let marshal_result = parser.marshal();
        let reparsed =
            HttpRequest::unmarshal(&marshal_result).expect("should parse marshaled request");
        assert_eq!(reparsed.method, parser.method);
        assert_eq!(reparsed.version, parser.version);
        assert_eq!(reparsed.uri.marshal(), parser.uri.marshal());
        assert_eq!(reparsed.body, parser.body);
    }

    #[test]
    fn test_whep_request() {
        let request = "POST /whep?app=live&stream=test HTTP/1.1\r\n\
        Host: localhost:3000\r\n\
        Accept: */*\r\n\
        Sec-Fetch-Site: same-origin\r\n\
        Accept-Language: zh-cn\r\n\
        Accept-Encoding: gzip, deflate\r\n\
        Sec-Fetch-Mode: cors\r\n\
        Content-Type: application/sdp\r\n\
        Origin: http://localhost:3000\r\n\
        User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.6 Safari/605.1.15\r\n\
        Referer: http://localhost:3000/\r\n\
        Content-Length: 3895\r\n\
        Connection: keep-alive\r\n\
        Sec-Fetch-Dest: empty\r\n\
        \r\n\
        v=0\r\n\
        o=- 6550659986740559335 2 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        a=group:BUNDLE 0 1\r\n\
        a=extmap-allow-mixed\r\n\
        a=msid-semantic: WMS\r\n\
        m=video 9 UDP/TLS/RTP/SAVPF 96 97 98 99 100 101 102 125 104 124 106 107 108 109 127 35\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:0mum\r\n\
        a=ice-pwd:DD4LnAhZLQNLSzRZWZRh9Jm4\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 6C:61:89:FF:9D:2F:BA:0A:A4:80:0D:98:C3:CA:43:05:82:EB:59:13:BC:C8:DE:33:2F:26:4A:27:D8:D0:D1:3D\r\n\
        a=setup:actpass\r\n\
        a=mid:0\r\n\
        a=extmap:1 urn:ietf:params:rtp-hdrext:toffset\r\n\
        a=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\r\n\
        a=extmap:3 urn:3gpp:video-orientation\r\n\
        a=extmap:4 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r\n\
        a=extmap:5 http://www.webrtc.org/experiments/rtp-hdrext/playout-delay\r\n\
        a=extmap:6 http://www.webrtc.org/experiments/rtp-hdrext/video-content-type\r\n\
        a=extmap:7 http://www.webrtc.org/experiments/rtp-hdrext/video-timing\r\n\
        a=extmap:8 http://www.webrtc.org/experiments/rtp-hdrext/color-space\r\n\
        a=extmap:9 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\n\
        a=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:96 H264/90000\r\n\
        a=rtcp-fb:96 goog-remb\r\n\
        a=rtcp-fb:96 transport-cc\r\n\
        a=rtcp-fb:96 ccm fir\r\n\
        a=rtcp-fb:96 nack\r\n\
        a=rtcp-fb:96 nack pli\r\n\
        a=fmtp:96 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=640c1f\r\n\
        a=rtpmap:97 rtx/90000\r\n\
        a=fmtp:97 apt=96\r\n\
        a=rtpmap:98 H264/90000\r\n\
        a=rtcp-fb:98 goog-remb\r\n\
        a=rtcp-fb:98 transport-cc\r\n\
        a=rtcp-fb:98 ccm fir\r\n\
        a=rtcp-fb:98 nack\r\n\
        a=rtcp-fb:98 nack pli\r\n\
        a=fmtp:98 level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f\r\n\
        a=rtpmap:99 rtx/90000\r\n\
        a=fmtp:99 apt=98\r\n\
        a=rtpmap:100 H264/90000\r\n\
        a=rtcp-fb:100 goog-remb\r\n\
        a=rtcp-fb:100 transport-cc\r\n\
        a=rtcp-fb:100 ccm fir\r\n\
        a=rtcp-fb:100 nack\r\n\
        a=rtcp-fb:100 nack pli\r\n\
        a=fmtp:100 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=640c1f\r\n\
        a=rtpmap:101 rtx/90000\r\n\
        a=fmtp:101 apt=100\r\n\
        a=rtpmap:102 H264/90000\r\n\
        a=rtcp-fb:102 goog-remb\r\n\
        a=rtcp-fb:102 transport-cc\r\n\
        a=rtcp-fb:102 ccm fir\r\n\
        a=rtcp-fb:102 nack\r\n\
        a=rtcp-fb:102 nack pli\r\n\
        a=fmtp:102 level-asymmetry-allowed=1;packetization-mode=0;profile-level-id=42e01f\r\n\
        a=rtpmap:125 rtx/90000\r\n\
        a=fmtp:125 apt=102\r\n\
        a=rtpmap:104 VP8/90000\r\n\
        a=rtcp-fb:104 goog-remb\r\n\
        a=rtcp-fb:104 transport-cc\r\n\
        a=rtcp-fb:104 ccm fir\r\n\
        a=rtcp-fb:104 nack\r\n\
        a=rtcp-fb:104 nack pli\r\n\
        a=rtpmap:124 rtx/90000\r\n\
        a=fmtp:124 apt=104\r\n\
        a=rtpmap:106 VP9/90000\r\n\
        a=rtcp-fb:106 goog-remb\r\n\
        a=rtcp-fb:106 transport-cc\r\n\
        a=rtcp-fb:106 ccm fir\r\n\
        a=rtcp-fb:106 nack\r\n\
        a=rtcp-fb:106 nack pli\r\n\
        a=fmtp:106 profile-id=0\r\n\
        a=rtpmap:107 rtx/90000\r\n\
        a=fmtp:107 apt=106\r\n\
        a=rtpmap:108 red/90000\r\n\
        a=rtpmap:109 rtx/90000\r\n\
        a=fmtp:109 apt=108\r\n\
        a=rtpmap:127 ulpfec/90000\r\n\
        a=rtpmap:35 flexfec-03/90000\r\n\
        a=rtcp-fb:35 goog-remb\r\n\
        a=rtcp-fb:35 transport-cc\r\n\
        a=fmtp:35 repair-window=10000000\r\n\
        m=audio 9 UDP/TLS/RTP/SAVPF 111 63 103 9 0 8 105 13 110 113 126\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:0mum\r\n\
        a=ice-pwd:DD4LnAhZLQNLSzRZWZRh9Jm4\r\n\
        a=ice-options:trickle\r\n\
        a=fingerprint:sha-256 6C:61:89:FF:9D:2F:BA:0A:A4:80:0D:98:C3:CA:43:05:82:EB:59:13:BC:C8:DE:33:2F:26:4A:27:D8:D0:D1:3D\r\n\
        a=setup:actpass\r\n\
        a=mid:1\r\n\
        a=extmap:14 urn:ietf:params:rtp-hdrext:ssrc-audio-level\r\n\
        a=extmap:2 http://www.webrtc.org/experiments/rtp-hdrext/abs-send-time\r\n\
        a=extmap:4 http://www.ietf.org/id/draft-holmer-rmcat-transport-wide-cc-extensions-01\r\n\
        a=extmap:9 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtpmap:111 opus/48000/2\r\n\
        a=rtcp-fb:111 transport-cc\r\n\
        a=fmtp:111 minptime=10;useinbandfec=1\r\n\
        a=rtpmap:63 red/48000/2\r\n\
        a=fmtp:63 111/111\r\n\
        a=rtpmap:103 ISAC/16000\r\n\
        a=rtpmap:9 G722/8000\r\n\
        a=rtpmap:0 PCMU/8000\r\n\
        a=rtpmap:8 PCMA/8000\r\n\
        a=rtpmap:105 CN/16000\r\n\
        a=rtpmap:13 CN/8000\r\n\
        a=rtpmap:110 telephone-event/48000\r\n\
        a=rtpmap:113 telephone-event/16000\r\n\
        a=rtpmap:126 telephone-event/8000\r\n";

        let parser = HttpRequest::unmarshal(request).expect("should parse request");
        assert!(super::parse_content_length(request).is_some());
        let marshal_result = parser.marshal();
        let reparsed =
            HttpRequest::unmarshal(&marshal_result).expect("should parse marshaled request");
        assert_eq!(reparsed.method, parser.method);
        assert_eq!(reparsed.version, parser.version);
        assert_eq!(reparsed.uri.marshal(), parser.uri.marshal());
        assert_eq!(reparsed.body, parser.body);
    }

    #[test]
    fn test_parse_http_response() {
        let response = "HTTP/1.1 201 Created\r\n\
        Content-Type: application/sdp\r\n\
        Location: https://whip.example.com/resource/id\r\n\
        Content-Length: 1392\r\n\
        \r\n\
        v=0\r\n\
        o=- 1657793490019 1 IN IP4 127.0.0.1\r\n\
        s=-\r\n\
        t=0 0\r\n\
        a=group:BUNDLE 0 1\r\n\
        a=extmap-allow-mixed\r\n\
        a=ice-lite\r\n\
        a=msid-semantic: WMS *\r\n\
        m=audio 9 UDP/TLS/RTP/SAVPF 111\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:38sdf4fdsf54\r\n\
        a=ice-pwd:2e13dde17c1cb009202f627fab90cbec358d766d049c9697\r\n\
        a=fingerprint:sha-256 F7:EB:F3:3E:AC:D2:EA:A7:C1:EC:79:D9:B3:8A:35:DA:70:86:4F:46:D9:2D:CC:D0:BC:81:9F:67:EF:34:2E:BD\r\n\
        a=candidate:1 1 UDP 2130706431 198.51.100.1 39132 typ host\r\n\
        a=setup:passive\r\n\
        a=mid:0\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:111 opus/48000/2\r\n\
        a=fmtp:111 minptime=10;useinbandfec=1\r\n\
        m=video 9 UDP/TLS/RTP/SAVPF 96 97\r\n\
        c=IN IP4 0.0.0.0\r\n\
        a=rtcp:9 IN IP4 0.0.0.0\r\n\
        a=ice-ufrag:38sdf4fdsf54\r\n\
        a=ice-pwd:2e13dde17c1cb009202f627fab90cbec358d766d049c9697\r\n\
        a=fingerprint:sha-256 F7:EB:F3:3E:AC:D2:EA:A7:C1:EC:79:D9:B3:8A:35:DA:70:86:4F:46:D9:2D:CC:D0:BC:81:9F:67:EF:34:2E:BD\r\n\
        a=candidate:1 1 UDP 2130706431 198.51.100.1 39132 typ host\r\n\
        a=setup:passive\r\n\
        a=mid:1\r\n\
        a=bundle-only\r\n\
        a=extmap:4 urn:ietf:params:rtp-hdrext:sdes:mid\r\n\
        a=extmap:10 urn:ietf:params:rtp-hdrext:sdes:rtp-stream-id\r\n\
        a=extmap:11 urn:ietf:params:rtp-hdrext:sdes:repaired-rtp-stream-id\r\n\
        a=recvonly\r\n\
        a=rtcp-mux\r\n\
        a=rtcp-rsize\r\n\
        a=rtpmap:96 VP8/90000\r\n\
        a=rtcp-fb:96 ccm fir\r\n\
        a=rtcp-fb:96 nack\r\n\
        a=rtcp-fb:96 nack pli\r\n\
        a=rtpmap:97 rtx/90000\r\n\
        a=fmtp:97 apt=96\r\n";

        let parser = HttpResponse::unmarshal(response).expect("should parse response");
        let marshal_result = parser.marshal();
        let reparsed =
            HttpResponse::unmarshal(&marshal_result).expect("should parse marshaled response");
        assert_eq!(reparsed.status_code, parser.status_code);
        assert_eq!(reparsed.version, parser.version);
        assert_eq!(reparsed.reason_phrase, parser.reason_phrase);
        assert_eq!(reparsed.body, parser.body);
    }

    // #[test]
    // fn test_parse_rtsp_request_chatgpt() {
    //     let data1 = "ANNOUNCE rtsp://127.0.0.1:5544/stream RTSP/1.0\r\n\
    //     Content-Type: application/sdp\r\n\
    //     CSeq: 2\r\n\
    //     User-Agent: Lavf58.76.100\r\n\
    //     Content-Length: 500\r\n\
    //     \r\n\
    //     v=0\r\n\
    //     o=- 0 0 IN IP4 127.0.0.1\r\n\
    //     s=No Name\r\n\
    //     c=IN IP4 127.0.0.1\r\n\
    //     t=0 0\r\n\
    //     a=tool:libavformat 58.76.100\r\n\
    //     m=video 0 RTP/AVP 96\r\n\
    //     b=AS:284\r\n\
    //     a=rtpmap:96 H264/90000
    //     a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E\r\n\
    //     a=control:streamid=0\r\n\
    //     m=audio 0 RTP/AVP 97\r\n\
    //     b=AS:128\r\n\
    //     a=rtpmap:97 MPEG4-GENERIC/48000/2\r\n\
    //     a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500\r\n\
    //     a=control:streamid=1\r\n";

    //     // v=0：SDP版本号，通常为0。
    //     // o=- 0 0 IN IP4 127.0.0.1：会话的所有者和会话ID，以及会话开始时间和会话结束时间的信息。
    //     // s=No Name：会话名称或标题。
    //     // c=IN IP4 127.0.0.1：表示会话数据传输的地址类型(IPv4)和地址(127.0.0.1)。
    //     // t=0 0：会话时间，包括会话开始时间和结束时间，这里的值都是0，表示会话没有预定义的结束时间。
    //     // a=tool:libavformat 58.76.100：会话所使用的工具或软件名称和版本号。

    //     // m=video 0 RTP/AVP 96：媒体类型(video或audio)、媒体格式(RTP/AVP)、媒体格式编号(96)和媒体流的传输地址。
    //     // b=AS:284：视频流所使用的带宽大小。
    //     // a=rtpmap:96 H264/90000：视频流所使用的编码方式(H.264)和时钟频率(90000)。
    //     // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAHqzZQKAv+XARAAADAAEAAAMAMg8WLZY=,aOvjyyLA; profile-level-id=64001E：视频流的格式参数，如分片方式、SPS和PPS等。
    //     // a=control:streamid=0：指定视频流的流ID。

    //     // m=audio 0 RTP/AVP 97：媒体类型(audio)、媒体格式(RTP/AVP)、媒体格式编号(97)和媒体流的传输地址。
    //     // b=AS:128：音频流所使用的带宽大小。
    //     // a=rtpmap:97 MPEG4-GENERIC/48000/2：音频流所使用的编码方式(MPEG4-GENERIC)、采样率(48000Hz)、和通道数(2)。
    //     // a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=119056E500：音频流的格式参数，如编码方式、采样长度、索引长度等。
    //     // a=control:streamid=1：指定音频流的流ID。

    //     if let Some(request) = parse_request_bytes(&data1.as_bytes()) {
    //         println!(" parser: {:?}", request);
    //     }
    // }

    #[test]
    fn test_parse_rtsp_request() {
        let data1 = "SETUP rtsp://127.0.0.1/stream/streamid=0 RTSP/1.0\r\n\
        Transport: RTP/AVP/TCP;unicast;interleaved=0-1;mode=record\r\n\
        CSeq: 3\r\n\
        User-Agent: Lavf58.76.100\r\n\
        \r\n";

        let parser1 = HttpRequest::unmarshal(data1).expect("should parse rtsp request");
        let marshal_result1 = parser1.marshal();
        assert!(HttpRequest::unmarshal(&marshal_result1).is_some());

        let data2 = "ANNOUNCE rtsp://127.0.0.1/stream RTSP/1.0\r\n\
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

        let parser2 = HttpRequest::unmarshal(data2).expect("should parse rtsp announce");
        let marshal_result2 = parser2.marshal();
        let reparsed2 =
            HttpRequest::unmarshal(&marshal_result2).expect("should parse marshaled announce");
        assert_eq!(reparsed2.method, parser2.method);
        assert_eq!(reparsed2.version, parser2.version);
        assert_eq!(reparsed2.uri.marshal(), parser2.uri.marshal());
        assert_eq!(reparsed2.body, parser2.body);
    }

    #[test]
    fn test_parse_rtsp_request_without_path() {
        let data = "OPTIONS rtsp://127.0.0.1:8554 RTSP/1.0\r\n\
        CSeq: 1\r\n\
        \r\n";

        let parser = HttpRequest::unmarshal(data).expect("should parse rtsp uri without path");
        assert!(matches!(parser.uri.schema, Schema::RTSP));
        assert_eq!(parser.uri.host, "127.0.0.1");
        assert_eq!(parser.uri.port, Some(8554));
        assert_eq!(parser.uri.path, "");
        assert!(parser.uri.query.is_none());
    }

    #[test]
    fn test_parse_rtsp_response_reason_phrase_with_spaces() {
        let response = "RTSP/1.0 404 Not Found\r\nCSeq: 7\r\n\r\n";
        let parsed = HttpResponse::unmarshal(response).expect("should parse rtsp response");
        assert_eq!(parsed.status_code, 404);
        assert_eq!(parsed.reason_phrase, "Not Found");
        assert_eq!(parsed.get_header("cseq").map(String::as_str), Some("7"));
    }

    #[test]
    fn test_try_get_complete_message_len_pipelined_rtsp_and_interleaved_binary() {
        let response = b"RTSP/1.0 200 OK\r\nCSeq: 1\r\n\r\n";
        let mut data = Vec::from(response);
        data.extend_from_slice(&[0x24, 0x00, 0x00, 0x04, 0xff, 0xff, 0xff, 0xff]);
        let len = super::try_get_complete_message_len(&data).expect("should compute len");
        assert_eq!(len, Some(response.len()));
    }

    #[test]
    fn test_parse_rtsp_request_header_without_space_after_colon() {
        let request = "OPTIONS rtsp://127.0.0.1:8554 RTSP/1.0\r\nCSeq:1\r\n\r\n";
        let parsed = HttpRequest::unmarshal(request).expect("should parse request");
        assert_eq!(parsed.get_header("cseq").map(String::as_str), Some("1"));
    }
}
