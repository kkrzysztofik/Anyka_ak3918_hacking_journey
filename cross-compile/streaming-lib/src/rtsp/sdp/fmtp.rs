use crate::rtsp::global_trait::{Marshal, Unmarshal};
use base64::{Engine as _, engine::general_purpose};
use bytes::{BufMut, BytesMut};

// pub trait Fmtp: TMsgConverter {}

#[derive(Debug, Clone)]
pub struct H264Fmtp {
    pub payload_type: u16,
    pub packetization_mode: u8,
    profile_level_id: BytesMut,
    pub sps: BytesMut,
    pub pps: BytesMut,
}

impl Default for H264Fmtp {
    fn default() -> Self {
        Self {
            payload_type: 0,
            // Our H.264 packetizer supports FU-A fragmentation which requires packetization-mode=1.
            // Defaulting to 1 avoids advertising an incompatible mode when a source SDP omits it.
            packetization_mode: 1,
            profile_level_id: BytesMut::new(),
            sps: BytesMut::new(),
            pps: BytesMut::new(),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct H265Fmtp {
    pub payload_type: u16,
    pub vps: BytesMut,
    pub sps: BytesMut,
    pub pps: BytesMut,
}
#[derive(Debug, Clone, Default)]
pub struct Mpeg4Fmtp {
    pub payload_type: u16,
    pub asc: BytesMut,
    profile_level_id: BytesMut,
    mode: String,
    size_length: u16,
    index_length: u16,
    index_delta_length: u16,
}
#[derive(Debug, Clone)]
pub enum Fmtp {
    H264(H264Fmtp),
    H265(H265Fmtp),
    Mpeg4(Mpeg4Fmtp),
}

impl Fmtp {
    pub fn new(codec: &str, raw_data: &str) -> Result<Fmtp, String> {
        let codec_lower = codec.to_lowercase();
        match codec_lower.as_str() {
            "h264" => Self::parse_h264(raw_data),
            "h265" => Self::parse_h265(raw_data),
            "mpeg4-generic" => Self::parse_mpeg4(raw_data),
            _ => Err("Unsupported codec".to_string()),
        }
    }

    fn parse_h264(raw_data: &str) -> Result<Fmtp, String> {
        H264Fmtp::unmarshal(raw_data).map(Fmtp::H264)
    }

    fn parse_h265(raw_data: &str) -> Result<Fmtp, String> {
        H265Fmtp::unmarshal(raw_data).map(Fmtp::H265)
    }

    fn parse_mpeg4(raw_data: &str) -> Result<Fmtp, String> {
        Mpeg4Fmtp::unmarshal(raw_data).map(Fmtp::Mpeg4)
    }

    pub fn marshal(&self) -> String {
        match self {
            Fmtp::H264(h264fmtp) => h264fmtp.marshal(),
            Fmtp::H265(h265fmtp) => h265fmtp.marshal(),
            Fmtp::Mpeg4(mpeg4fmtp) => mpeg4fmtp.marshal(),
        }
    }
}

// a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016
impl Unmarshal for H264Fmtp {
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut h264_fmtp = H264Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();

        if eles.is_empty() {
            log::warn!("H264FmtpSdp parse err: empty input");
            return Err("Empty input".to_string());
        }

        if let Ok(payload_type) = eles[0].parse::<u16>() {
            h264_fmtp.payload_type = payload_type;
        } else {
            log::warn!(
                "H264FmtpSdp parse err: invalid payload type in {}",
                raw_data
            );
            return Err("Invalid payload type".to_string());
        }

        // If no parameters provided, return with defaults
        if eles.len() < 2 {
            return Ok(h264_fmtp);
        }

        let parameters: Vec<&str> = eles[1].split(';').collect();
        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("H264FmtpSdp parse key=value err: {}", parameter);
                continue;
            }

            match kv[0] {
                "packetization-mode" => {
                    if let Ok(packetization_mode) = kv[1].parse::<u8>() {
                        h264_fmtp.packetization_mode = packetization_mode;
                    }
                }
                "sprop-parameter-sets" => {
                    let spspps: Vec<&str> = kv[1].split(',').collect();
                    if spspps.len() < 2 {
                        log::warn!("H264FmtpSdp parse err: missing sps/pps");
                        continue;
                    }
                    match general_purpose::STANDARD.decode(spspps[0]) {
                        Ok(sps) => h264_fmtp.sps.put(&sps[..]),
                        Err(err) => log::warn!("H264FmtpSdp sps decode err: {err}"),
                    }
                    match general_purpose::STANDARD.decode(spspps[1]) {
                        Ok(pps) => h264_fmtp.pps.put(&pps[..]),
                        Err(err) => log::warn!("H264FmtpSdp pps decode err: {err}"),
                    }
                }
                "profile-level-id" => {
                    h264_fmtp.profile_level_id = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Ok(h264_fmtp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_h264fmtp_invalid_base64() {
        let parser = H264Fmtp::unmarshal(
            "96 packetization-mode=1; sprop-parameter-sets=!!invalid!!,??bad??; profile-level-id=640016",
        )
        .unwrap();

        assert_eq!(parser.payload_type, 96);
        assert_eq!(parser.packetization_mode, 1);
        assert_eq!(parser.profile_level_id, "640016");
        assert!(parser.sps.is_empty());
        assert!(parser.pps.is_empty());
    }

    #[test]
    fn test_parse_mpeg4fmtp_invalid_hex() {
        let parser = Mpeg4Fmtp::unmarshal(
            "97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=zzzz",
        )
        .unwrap();

        assert_eq!(parser.payload_type, 97);
        assert_eq!(parser.profile_level_id, "1");
        assert!(parser.asc.is_empty());
    }
}

impl Marshal for H264Fmtp {
    // a=fmtp:96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016
    fn marshal(&self) -> String {
        let sps_str = general_purpose::STANDARD.encode(&self.sps);
        let pps_str = general_purpose::STANDARD.encode(&self.pps);
        let profile_level_id_str = String::from_utf8_lossy(&self.profile_level_id).to_string();

        let h264_fmtp = format!(
            "{} packetization-mode={}; sprop-parameter-sets={},{}; profile-level-id={}",
            self.payload_type, self.packetization_mode, sps_str, pps_str, profile_level_id_str
        );

        format!("{h264_fmtp}\r\n")
    }
}

impl Unmarshal for H265Fmtp {
    //\"a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ\"
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut h265_fmtp = H265Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("H265FmtpSdp parse err: {}", raw_data);
            return Err("Invalid H265 FMTP format".to_string());
        }

        if let Ok(payload_type) = eles[0].parse::<u16>() {
            h265_fmtp.payload_type = payload_type;
        }

        let parameters: Vec<&str> = eles[1].split(';').collect();
        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("H265FmtpSdp parse key=value err: {}", parameter);
                continue;
            }

            match kv[0] {
                "sprop-vps" => {
                    h265_fmtp.vps = kv[1].into();
                }
                "sprop-sps" => {
                    h265_fmtp.sps = kv[1].into();
                }
                "sprop-pps" => {
                    h265_fmtp.pps = kv[1].into();
                }
                _ => {
                    log::info!("not parsed: {}", kv[0])
                }
            }
        }

        Ok(h265_fmtp)
    }
}

impl Marshal for H265Fmtp {
    //"a=fmtp:96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ"
    fn marshal(&self) -> String {
        let sps_str = String::from_utf8_lossy(&self.sps).to_string();
        let pps_str = String::from_utf8_lossy(&self.pps).to_string();
        let vps_str = String::from_utf8_lossy(&self.vps).to_string();

        let h265_fmtp = format!(
            "{} sprop-vps={}; sprop-sps={}; sprop-pps={}",
            self.payload_type, vps_str, sps_str, pps_str
        );

        format!("{h265_fmtp}\r\n")
    }
}

impl Mpeg4Fmtp {
    /// Parses a key-value parameter and updates the fmtp struct
    fn parse_parameter(&mut self, key: &str, value: &str) {
        match key.to_lowercase().as_str() {
            "mode" => self.mode = value.to_string(),
            "config" => self.parse_config(value),
            "profile-level-id" => self.profile_level_id = value.into(),
            "sizelength" => self.parse_u16_param(value, |fmtp, v| fmtp.size_length = v),
            "indexlength" => self.parse_u16_param(value, |fmtp, v| fmtp.index_length = v),
            "indexdeltalength" => {
                self.parse_u16_param(value, |fmtp, v| fmtp.index_delta_length = v)
            }
            _ => log::info!("not parsed: {}", key),
        }
    }

    /// Parses the config parameter as hex-encoded data
    fn parse_config(&mut self, value: &str) {
        match hex::decode(value) {
            Ok(asc) => self.asc.put(&asc[..]),
            Err(err) => log::warn!("Mpeg4FmtpSdp hex decode err: {err}"),
        }
    }

    /// Parses a u16 parameter and applies it using the provided setter
    fn parse_u16_param<F>(&mut self, value: &str, setter: F)
    where
        F: FnOnce(&mut Self, u16),
    {
        if let Ok(parsed) = value.parse::<u16>() {
            setter(self, parsed);
        }
    }
}

impl Unmarshal for Mpeg4Fmtp {
    //a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=121056e500
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut mpeg4_fmtp = Mpeg4Fmtp::default();
        let eles: Vec<&str> = raw_data.splitn(2, ' ').collect();
        if eles.len() < 2 {
            log::warn!("Mpeg4FmtpSdp parse err: {}", raw_data);
            return Err("Invalid MPEG4 FMTP format".to_string());
        }

        if let Ok(payload_type) = eles[0].parse::<u16>() {
            mpeg4_fmtp.payload_type = payload_type;
        }

        let parameters: Vec<&str> = eles[1].split(';').collect();
        for parameter in parameters {
            let kv: Vec<&str> = parameter.trim().splitn(2, '=').collect();
            if kv.len() < 2 {
                log::warn!("Mpeg4FmtpSdp parse key=value err: {}", parameter);
                continue;
            }
            mpeg4_fmtp.parse_parameter(kv[0], kv[1]);
        }

        Ok(mpeg4_fmtp)
    }
}

impl Marshal for Mpeg4Fmtp {
    //a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3; config=121056e500
    fn marshal(&self) -> String {
        let profile_level_id_str = String::from_utf8_lossy(&self.profile_level_id).to_string();
        let asc_str = hex::encode(&self.asc); //String::from_utf8(self.asc.to_vec()).unwrap();

        let mpeg4_fmtp = format!(
            "{} profile-level-id={};mode={};sizelength={};indexlength={};indexdeltalength={}; config={}",
            self.payload_type,
            profile_level_id_str,
            self.mode,
            self.size_length,
            self.index_length,
            self.index_delta_length,
            asc_str
        );

        format!("{mpeg4_fmtp}\r\n")
    }
}

#[cfg(test)]
mod marshal_tests {

    use bytes::BytesMut;

    use super::H264Fmtp;
    use super::H265Fmtp;
    use super::Mpeg4Fmtp;
    use crate::rtsp::global_trait::Marshal;
    use crate::rtsp::global_trait::Unmarshal;

    #[test]
    fn test_parse_h264fmtpsdp() {
        let parser = H264Fmtp::unmarshal("96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016").expect("Failed to unmarshal H264");

        println!(" parser: {parser:?}");

        assert_eq!(parser.packetization_mode, 1);
        assert_eq!(parser.profile_level_id, "640016");
        // assert_eq!(parser.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        // assert_eq!(parser.pps, "aOvDyyLA");
        //"96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016"

        print!("264 parser: {}", parser.marshal());

        let parser2 = H264Fmtp::unmarshal("96 packetization-mode=1;\nsprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA;\nprofile-level-id=640016").expect("Failed to unmarshal H264 (variant 2)");

        println!(" parser: {parser2:?}");

        assert_eq!(parser2.packetization_mode, 1);
        assert_eq!(parser2.profile_level_id, "640016");
        // assert_eq!(parser2.sps, "Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=");
        // assert_eq!(parser2.pps, "aOvDyyLA");

        print!("264 parser2: {}", parser2.marshal());
    }
    #[test]
    fn test_parse_h265fmtpsdp() {
        let parser = H265Fmtp::unmarshal("96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ").expect("Failed to unmarshal H265");

        println!(" parser: {parser:?}");

        assert_eq!(parser.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
        assert_eq!(
            parser.sps,
            "QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I"
        );
        assert_eq!(parser.pps, "RAHAc8GJ");

        print!("265 parser: {}", parser.marshal());
    }

    #[test]
    fn test_parse_mpeg4fmtpsdp() {
        let parser = Mpeg4Fmtp::unmarshal("97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500").expect("Failed to unmarshal MPEG4");

        println!(" parser: {parser:?}");
        let en_asc = hex::encode(parser.asc.clone());

        assert_eq!(en_asc, "121056e500");
        assert_eq!(parser.profile_level_id, "1");
        assert_eq!(parser.mode, "AAC-hbr");
        assert_eq!(parser.size_length, 13);
        assert_eq!(parser.index_length, 3);
        assert_eq!(parser.index_delta_length, 23);

        print!("mpeg4 parser: {}", parser.marshal());
    }

    #[test]
    fn test_string() {
        let s = String::from("119056E500");
        let ss = hex::decode(s).unwrap();
        let b = BytesMut::from(&ss[..]);

        // Test passes if hex decoding works - rtsp_utils::print removed as function doesn't exist
        assert!(!b.is_empty());
    }

    // H.264 FMTP comprehensive tests
    #[test]
    fn test_h264_fmtp_packetization_mode() {
        let fmtp = H264Fmtp::unmarshal("96 packetization-mode=0").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.packetization_mode, 0);

        let fmtp = H264Fmtp::unmarshal("96 packetization-mode=1").unwrap();
        assert_eq!(fmtp.packetization_mode, 1);

        let fmtp = H264Fmtp::unmarshal("96 packetization-mode=2").unwrap();
        assert_eq!(fmtp.packetization_mode, 2);
    }

    #[test]
    fn test_h264_fmtp_profile_level_id() {
        let fmtp = H264Fmtp::unmarshal("96 profile-level-id=42001e").unwrap();
        assert_eq!(fmtp.profile_level_id, "42001e");

        let fmtp = H264Fmtp::unmarshal("96 profile-level-id=640016").unwrap();
        assert_eq!(fmtp.profile_level_id, "640016");
    }

    #[test]
    fn test_h264_fmtp_sprop_parameter_sets() {
        let fmtp = H264Fmtp::unmarshal(
            "96 sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA",
        )
        .unwrap();
        assert_eq!(fmtp.payload_type, 96);
        // Verify SPS and PPS are decoded correctly
        assert!(!fmtp.sps.is_empty());
        assert!(!fmtp.pps.is_empty());
    }

    #[test]
    fn test_h264_fmtp_all_parameters() {
        let fmtp = H264Fmtp::unmarshal("96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.packetization_mode, 1);
        assert_eq!(fmtp.profile_level_id, "640016");
        assert!(!fmtp.sps.is_empty());
        assert!(!fmtp.pps.is_empty());
    }

    #[test]
    fn test_h264_fmtp_round_trip() {
        let original = "96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016";
        let fmtp = H264Fmtp::unmarshal(original).unwrap();
        let marshaled = fmtp.marshal();

        // Parse the marshaled output (remove \r\n)
        let marshaled_trimmed = marshaled.trim();
        let fmtp2 = H264Fmtp::unmarshal(marshaled_trimmed).unwrap();

        assert_eq!(fmtp.payload_type, fmtp2.payload_type);
        assert_eq!(fmtp.packetization_mode, fmtp2.packetization_mode);
        assert_eq!(fmtp.profile_level_id, fmtp2.profile_level_id);
        assert_eq!(fmtp.sps, fmtp2.sps);
        assert_eq!(fmtp.pps, fmtp2.pps);
    }

    #[test]
    fn test_h264_fmtp_invalid_payload_type() {
        // Invalid payload type should return Err
        let fmtp = H264Fmtp::unmarshal("invalid packetization-mode=1");
        assert!(fmtp.is_err());
    }

    #[test]
    fn test_h264_fmtp_missing_parameters() {
        let fmtp = H264Fmtp::unmarshal("96").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.packetization_mode, 1); // default
    }

    // H.265 FMTP comprehensive tests
    #[test]
    fn test_h265_fmtp_sprop_vps() {
        let fmtp = H265Fmtp::unmarshal("96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
    }

    #[test]
    fn test_h265_fmtp_sprop_sps() {
        let fmtp = H265Fmtp::unmarshal(
            "96 sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I",
        )
        .unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(
            fmtp.sps,
            "QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I"
        );
    }

    #[test]
    fn test_h265_fmtp_sprop_pps() {
        let fmtp = H265Fmtp::unmarshal("96 sprop-pps=RAHAc8GJ").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.pps, "RAHAc8GJ");
    }

    #[test]
    fn test_h265_fmtp_all_parameters() {
        let fmtp = H265Fmtp::unmarshal("96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
        assert_eq!(
            fmtp.sps,
            "QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I"
        );
        assert_eq!(fmtp.pps, "RAHAc8GJ");
    }

    #[test]
    fn test_h265_fmtp_round_trip() {
        let original = "96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ";
        let fmtp = H265Fmtp::unmarshal(original).unwrap();
        let marshaled = fmtp.marshal();

        // Parse the marshaled output (remove \r\n)
        let marshaled_trimmed = marshaled.trim();
        let fmtp2 = H265Fmtp::unmarshal(marshaled_trimmed).unwrap();

        assert_eq!(fmtp.payload_type, fmtp2.payload_type);
        assert_eq!(fmtp.vps, fmtp2.vps);
        assert_eq!(fmtp.sps, fmtp2.sps);
        assert_eq!(fmtp.pps, fmtp2.pps);
    }

    #[test]
    fn test_h265_fmtp_partial_parameters() {
        let fmtp = H265Fmtp::unmarshal("96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I").unwrap();
        assert_eq!(fmtp.payload_type, 96);
        assert_eq!(fmtp.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
        assert_eq!(
            fmtp.sps,
            "QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I"
        );
        assert!(fmtp.pps.is_empty()); // PPS not provided
    }

    // MPEG4 FMTP comprehensive tests
    #[test]
    fn test_mpeg4_fmtp_profile_level_id() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 profile-level-id=1").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.profile_level_id, "1");

        let fmtp = Mpeg4Fmtp::unmarshal("97 profile-level-id=15").unwrap();
        assert_eq!(fmtp.profile_level_id, "15");
    }

    #[test]
    fn test_mpeg4_fmtp_mode() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 mode=AAC-hbr").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.mode, "AAC-hbr");

        let fmtp = Mpeg4Fmtp::unmarshal("97 mode=AAC-lbr").unwrap();
        assert_eq!(fmtp.mode, "AAC-lbr");
    }

    #[test]
    fn test_mpeg4_fmtp_sizelength() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 sizelength=13").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.size_length, 13);

        let fmtp = Mpeg4Fmtp::unmarshal("97 sizelength=16").unwrap();
        assert_eq!(fmtp.size_length, 16);
    }

    #[test]
    fn test_mpeg4_fmtp_indexlength() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 indexlength=3").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.index_length, 3);

        let fmtp = Mpeg4Fmtp::unmarshal("97 indexlength=5").unwrap();
        assert_eq!(fmtp.index_length, 5);
    }

    #[test]
    fn test_mpeg4_fmtp_indexdeltalength() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 indexdeltalength=3").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.index_delta_length, 3);

        let fmtp = Mpeg4Fmtp::unmarshal("97 indexdeltalength=23").unwrap();
        assert_eq!(fmtp.index_delta_length, 23);
    }

    #[test]
    fn test_mpeg4_fmtp_config() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 config=121056e500").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        let en_asc = hex::encode(fmtp.asc.clone());
        assert_eq!(en_asc, "121056e500");
    }

    #[test]
    fn test_mpeg4_fmtp_all_parameters() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.profile_level_id, "1");
        assert_eq!(fmtp.mode, "AAC-hbr");
        assert_eq!(fmtp.size_length, 13);
        assert_eq!(fmtp.index_length, 3);
        assert_eq!(fmtp.index_delta_length, 23);
        let en_asc = hex::encode(fmtp.asc.clone());
        assert_eq!(en_asc, "121056e500");
    }

    #[test]
    fn test_mpeg4_fmtp_round_trip() {
        let original = "97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500";
        let fmtp = Mpeg4Fmtp::unmarshal(original).unwrap();
        let marshaled = fmtp.marshal();

        // Parse the marshaled output (remove \r\n)
        let marshaled_trimmed = marshaled.trim();
        let fmtp2 = Mpeg4Fmtp::unmarshal(marshaled_trimmed).unwrap();

        assert_eq!(fmtp.payload_type, fmtp2.payload_type);
        assert_eq!(fmtp.profile_level_id, fmtp2.profile_level_id);
        assert_eq!(fmtp.mode, fmtp2.mode);
        assert_eq!(fmtp.size_length, fmtp2.size_length);
        assert_eq!(fmtp.index_length, fmtp2.index_length);
        assert_eq!(fmtp.index_delta_length, fmtp2.index_delta_length);
        assert_eq!(fmtp.asc, fmtp2.asc);
    }

    #[test]
    fn test_mpeg4_fmtp_case_insensitive() {
        // Test that mode parameter is case-insensitive (based on to_lowercase in code)
        let fmtp = Mpeg4Fmtp::unmarshal("97 MODE=AAC-hbr").unwrap();
        assert_eq!(fmtp.mode, "AAC-hbr");

        let fmtp = Mpeg4Fmtp::unmarshal("97 Mode=AAC-lbr").unwrap();
        assert_eq!(fmtp.mode, "AAC-lbr");
    }

    #[test]
    fn test_mpeg4_fmtp_partial_parameters() {
        let fmtp = Mpeg4Fmtp::unmarshal("97 mode=AAC-hbr; config=121056e500").unwrap();
        assert_eq!(fmtp.payload_type, 97);
        assert_eq!(fmtp.mode, "AAC-hbr");
        let en_asc = hex::encode(fmtp.asc.clone());
        assert_eq!(en_asc, "121056e500");
        // Default values for missing parameters
        assert_eq!(fmtp.size_length, 0);
        assert_eq!(fmtp.index_length, 0);
        assert_eq!(fmtp.index_delta_length, 0);
    }

    // Fmtp enum tests
    #[test]
    fn test_fmtp_new_h264() {
        use super::Fmtp;
        let raw_data = "96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016";
        let fmtp = Fmtp::new("h264", raw_data).unwrap();

        match fmtp {
            Fmtp::H264(h264_fmtp) => {
                assert_eq!(h264_fmtp.payload_type, 96);
                assert_eq!(h264_fmtp.packetization_mode, 1);
            }
            _ => panic!("Expected H264 variant"),
        }
    }

    #[test]
    fn test_fmtp_new_h265() {
        use super::Fmtp;
        let raw_data = "96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ";
        let fmtp = Fmtp::new("h265", raw_data).unwrap();

        match fmtp {
            Fmtp::H265(h265_fmtp) => {
                assert_eq!(h265_fmtp.payload_type, 96);
                assert_eq!(h265_fmtp.vps, "QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA");
            }
            _ => panic!("Expected H265 variant"),
        }
    }

    #[test]
    fn test_fmtp_new_mpeg4() {
        use super::Fmtp;
        let raw_data = "97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500";
        let fmtp = Fmtp::new("mpeg4-generic", raw_data).unwrap();

        match fmtp {
            Fmtp::Mpeg4(mpeg4_fmtp) => {
                assert_eq!(mpeg4_fmtp.payload_type, 97);
                assert_eq!(mpeg4_fmtp.mode, "AAC-hbr");
            }
            _ => panic!("Expected Mpeg4 variant"),
        }
    }

    #[test]
    fn test_fmtp_new_case_insensitive() {
        use super::Fmtp;
        let raw_data = "96 packetization-mode=1";
        let fmtp = Fmtp::new("H264", raw_data).unwrap();

        match fmtp {
            Fmtp::H264(_) => {}
            _ => panic!("Expected H264 variant"),
        }
    }

    #[test]
    fn test_fmtp_new_invalid_codec() {
        use super::Fmtp;
        let raw_data = "96 packetization-mode=1";
        let fmtp = Fmtp::new("invalid", raw_data);
        assert!(fmtp.is_err());
    }

    #[test]
    fn test_fmtp_marshal_h264() {
        use super::Fmtp;
        let raw_data = "96 packetization-mode=1; sprop-parameter-sets=Z2QAFqyyAUBf8uAiAAADAAIAAAMAPB4sXJA=,aOvDyyLA; profile-level-id=640016";
        let fmtp = Fmtp::new("h264", raw_data).unwrap();
        let marshaled = fmtp.marshal();
        assert!(marshaled.contains("96"));
        assert!(marshaled.contains("packetization-mode=1"));
    }

    #[test]
    fn test_fmtp_marshal_h265() {
        use super::Fmtp;
        let raw_data = "96 sprop-vps=QAEMAf//AWAAAAMAkAAAAwAAAwA/ugJA; sprop-sps=QgEBAWAAAAMAkAAAAwAAAwA/oAUCAXHy5bpKTC8BAQAAAwABAAADAA8I; sprop-pps=RAHAc8GJ";
        let fmtp = Fmtp::new("h265", raw_data).unwrap();
        let marshaled = fmtp.marshal();
        assert!(marshaled.contains("96"));
        assert!(marshaled.contains("sprop-vps"));
    }

    #[test]
    fn test_fmtp_marshal_mpeg4() {
        use super::Fmtp;
        let raw_data = "97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=23; config=121056e500";
        let fmtp = Fmtp::new("mpeg4-generic", raw_data).unwrap();
        let marshaled = fmtp.marshal();
        assert!(marshaled.contains("97"));
        assert!(marshaled.contains("mode=AAC-hbr"));
    }
}
