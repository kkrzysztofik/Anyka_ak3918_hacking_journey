use bytes::{BufMut, BytesMut};
use thiserror::Error;

use streaming_lib::container::define::{AvcCodecId, aac_packet_type, avc_packet_type, frame_type};
use streaming_lib::streamhub::define::FrameData;

#[derive(Debug, Error)]
pub enum ValidationHttpFlvRemuxError {
    #[error("missing SPS/PPS for AVC sequence header")]
    MissingSpsPps,
    #[error("invalid H264 frame: no Annex-B NAL units found")]
    InvalidH264Frame,
}

#[derive(Debug, Clone)]
pub struct ValidationHttpFlvRemuxer {
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
    audio_config: Option<Vec<u8>>,
    audio_sample_rate: u32,
    video_framerate: u32,
}

impl ValidationHttpFlvRemuxer {
    pub fn new(
        sps: Vec<u8>,
        pps: Vec<u8>,
        audio_config: Option<Vec<u8>>,
        audio_sample_rate: u32,
        video_framerate: u32,
    ) -> Self {
        Self {
            sps: (!sps.is_empty()).then_some(sps),
            pps: (!pps.is_empty()).then_some(pps),
            audio_config,
            audio_sample_rate,
            video_framerate,
        }
    }

    /// Build the FLV onMetaData script-tag payload (AMF0 ECMA array).
    pub fn on_metadata_tag(&self, timestamp: u32) -> FrameData {
        let mut data = BytesMut::new();
        amf0_string(&mut data, "onMetaData");
        amf0_ecma_array(
            &mut data,
            &[
                ("videocodecid", amf0_number(7.0)),
                ("hasVideo", amf0_bool(true)),
                ("hasAudio", amf0_bool(self.audio_config.is_some())),
                ("framerate", amf0_number(self.video_framerate as f64)),
            ],
        );
        FrameData::MetaData { timestamp, data }
    }

    pub fn remux_frame(
        &mut self,
        frame: FrameData,
    ) -> Result<Option<FrameData>, ValidationHttpFlvRemuxError> {
        match frame {
            FrameData::Video { timestamp, data } => self.remux_video_frame(timestamp, data),
            FrameData::Audio { timestamp, data } => Ok(self.remux_audio_frame(timestamp, data)),
            FrameData::MetaData { .. } => Ok(Some(frame)),
            FrameData::MediaInfo { .. } => Ok(None),
        }
    }

    pub fn video_sequence_header(
        &self,
        timestamp: u32,
    ) -> Result<FrameData, ValidationHttpFlvRemuxError> {
        let sps = self
            .sps
            .as_ref()
            .ok_or(ValidationHttpFlvRemuxError::MissingSpsPps)?;
        let pps = self
            .pps
            .as_ref()
            .ok_or(ValidationHttpFlvRemuxError::MissingSpsPps)?;
        let avc_config = build_avc_decoder_configuration_record(sps, pps);

        let mut payload = BytesMut::with_capacity(5 + avc_config.len());
        payload.put_u8((frame_type::KEY_FRAME << 4) | AvcCodecId::H264 as u8);
        payload.put_u8(avc_packet_type::AVC_SEQHDR);
        payload.extend_from_slice(&[0x00, 0x00, 0x00]); // composition time
        payload.extend_from_slice(&avc_config);

        Ok(FrameData::Video {
            timestamp,
            data: payload,
        })
    }

    pub fn audio_sequence_header(&self, timestamp: u32) -> Option<FrameData> {
        let config = self.audio_config.as_ref()?;
        let mut payload = BytesMut::with_capacity(2 + config.len());
        payload.put_u8(0xAF);
        payload.put_u8(aac_packet_type::AAC_SEQHDR);
        payload.extend_from_slice(config);

        Some(FrameData::Audio {
            timestamp,
            data: payload,
        })
    }

    pub fn remux_video_frame(
        &mut self,
        timestamp: u32,
        data: BytesMut,
    ) -> Result<Option<FrameData>, ValidationHttpFlvRemuxError> {
        let mut nalus = split_annexb_nalus(&data);
        if nalus.is_empty() {
            if is_probable_h264_nal(&data) {
                nalus.push(&data);
            } else {
                return Err(ValidationHttpFlvRemuxError::InvalidH264Frame);
            }
        }

        let mut has_vcl = false;
        let mut is_key_frame = false;
        let mut payload_nalus: Vec<&[u8]> = Vec::new();

        for nal in nalus {
            if nal.is_empty() {
                continue;
            }

            let nal_type = nal[0] & 0x1F;
            match nal_type {
                7 => {
                    self.sps = Some(nal.to_vec());
                }
                8 => {
                    self.pps = Some(nal.to_vec());
                }
                9 => {
                    // AUD is optional and not needed in FLV AVC NALU payloads.
                }
                5 => {
                    is_key_frame = true;
                    has_vcl = true;
                    payload_nalus.push(nal);
                }
                1 => {
                    has_vcl = true;
                    payload_nalus.push(nal);
                }
                _ => {
                    payload_nalus.push(nal);
                }
            }
        }

        if !has_vcl {
            // SPS/PPS-only frame: keep as cached state update but do not emit media frame.
            return Ok(None);
        }

        let frame_type = if is_key_frame {
            frame_type::KEY_FRAME
        } else {
            frame_type::INTER_FRAME
        };

        let mut payload = BytesMut::new();
        payload.put_u8((frame_type << 4) | AvcCodecId::H264 as u8);
        payload.put_u8(avc_packet_type::AVC_NALU);
        payload.extend_from_slice(&[0x00, 0x00, 0x00]); // composition time
        for nal in payload_nalus {
            payload.put_u32(nal.len() as u32);
            payload.extend_from_slice(nal);
        }

        Ok(Some(FrameData::Video {
            timestamp,
            data: payload,
        }))
    }

    pub fn remux_audio_frame(&self, timestamp_samples: u32, data: BytesMut) -> Option<FrameData> {
        if self.audio_sample_rate == 0 {
            return None;
        }

        let timestamp_ms = ((timestamp_samples as u64)
            .saturating_mul(1000)
            .saturating_div(self.audio_sample_rate as u64))
        .min(u32::MAX as u64) as u32;

        let mut payload = BytesMut::with_capacity(2 + data.len());
        payload.put_u8(0xAF);
        payload.put_u8(aac_packet_type::AAC_RAW);
        payload.extend_from_slice(&data);

        Some(FrameData::Audio {
            timestamp: timestamp_ms,
            data: payload,
        })
    }
}

fn amf0_string(out: &mut BytesMut, s: &str) {
    out.put_u8(0x02);
    out.put_u16(s.len() as u16);
    out.extend_from_slice(s.as_bytes());
}

fn amf0_number(value: f64) -> BytesMut {
    let mut v = BytesMut::with_capacity(9);
    v.put_u8(0x00);
    v.put_f64(value);
    v
}

fn amf0_bool(value: bool) -> BytesMut {
    BytesMut::from(&[0x01, value as u8][..])
}

/// AMF0 ECMA array: `08 <u32be count> (u16be keylen + key + value)* 00 00 09`.
fn amf0_ecma_array(out: &mut BytesMut, entries: &[(&str, BytesMut)]) {
    out.put_u8(0x08);
    out.put_u32(entries.len() as u32);
    for (key, value) in entries {
        out.put_u16(key.len() as u16);
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(value);
    }
    out.extend_from_slice(&[0x00, 0x00, 0x09]);
}

fn is_probable_h264_nal(data: &[u8]) -> bool {
    let Some(&header) = data.first() else {
        return false;
    };
    let forbidden_zero_bit = header & 0x80;
    let nal_type = header & 0x1F;
    forbidden_zero_bit == 0 && nal_type != 0
}

fn build_avc_decoder_configuration_record(sps: &[u8], pps: &[u8]) -> BytesMut {
    // AVCDecoderConfigurationRecord (ISO/IEC 14496-15)
    // configurationVersion, AVCProfileIndication, profile_compatibility, AVCLevelIndication
    let profile = sps.get(1).copied().unwrap_or(0x42);
    let compatibility = sps.get(2).copied().unwrap_or(0x00);
    let level = sps.get(3).copied().unwrap_or(0x1E);

    let mut out = BytesMut::with_capacity(11 + sps.len() + pps.len());
    out.put_u8(0x01);
    out.put_u8(profile);
    out.put_u8(compatibility);
    out.put_u8(level);
    out.put_u8(0xFF); // reserved(6) + lengthSizeMinusOne(2) = 3 (4-byte NALU lengths)
    out.put_u8(0xE1); // reserved(3) + numOfSequenceParameterSets(5) = 1
    out.put_u16(sps.len() as u16);
    out.extend_from_slice(sps);
    out.put_u8(0x01); // numOfPictureParameterSets = 1
    out.put_u16(pps.len() as u16);
    out.extend_from_slice(pps);
    out
}

fn split_annexb_nalus(data: &[u8]) -> Vec<&[u8]> {
    let mut nal_ranges: Vec<(usize, usize)> = Vec::new();
    let mut cursor = 0usize;
    let mut current_start: Option<usize> = None;

    while let Some((idx, prefix_len)) = find_start_code(data, cursor) {
        if let Some(start) = current_start
            && idx > start
        {
            nal_ranges.push((start, idx));
        }
        current_start = Some(idx + prefix_len);
        cursor = idx + prefix_len;
    }

    if let Some(start) = current_start
        && start < data.len()
    {
        nal_ranges.push((start, data.len()));
    }

    nal_ranges
        .into_iter()
        .filter_map(|(start, end)| (end > start).then_some(&data[start..end]))
        .collect()
}

fn find_start_code(data: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i + 3 < data.len() {
        if data[i] == 0x00 && data[i + 1] == 0x00 {
            if data[i + 2] == 0x01 {
                return Some((i, 3));
            }
            if i + 3 < data.len() && data[i + 2] == 0x00 && data[i + 3] == 0x01 {
                return Some((i, 4));
            }
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn annexb_nal(nal: &[u8]) -> BytesMut {
        let mut out = BytesMut::new();
        out.extend_from_slice(&[0x00, 0x00, 0x00, 0x01]);
        out.extend_from_slice(nal);
        out
    }

    #[test]
    fn test_remux_video_sequence_header_generation_success() {
        let remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0xE0, 0x1E, 0xAA],
            vec![0x68, 0xCE, 0x06, 0xE2],
            None,
            0,
            15,
        );
        let frame = remuxer.video_sequence_header(123).expect("seq header");
        match frame {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 123);
                assert_eq!(data[0], 0x17); // keyframe + AVC
                assert_eq!(data[1], 0x00); // AVC sequence header
                assert_eq!(data[5], 0x01); // AVC config version
            }
            _ => panic!("expected video frame"),
        }
    }

    #[test]
    fn test_remux_annexb_video_to_avcc_nalu_payload() {
        let mut remuxer = ValidationHttpFlvRemuxer::new(Vec::new(), Vec::new(), None, 0, 15);
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&annexb_nal(&[0x67, 0x42, 0xE0, 0x1E]));
        frame.extend_from_slice(&annexb_nal(&[0x68, 0xCE, 0x06, 0xE2]));
        frame.extend_from_slice(&annexb_nal(&[0x65, 0x88, 0x84, 0x21, 0xA0]));

        let out = remuxer
            .remux_video_frame(40, frame)
            .expect("remux")
            .expect("video frame");
        match out {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 40);
                assert_eq!(data[0], 0x17); // keyframe + AVC
                assert_eq!(data[1], 0x01); // AVC NALU
                let nalu_len = u32::from_be_bytes([data[5], data[6], data[7], data[8]]);
                assert_eq!(nalu_len, 5);
            }
            _ => panic!("expected video frame"),
        }
    }

    #[test]
    fn test_remux_single_raw_nal_without_annexb_start_code() {
        let mut remuxer = ValidationHttpFlvRemuxer::new(Vec::new(), Vec::new(), None, 0, 15);
        let frame = BytesMut::from(&[0x65, 0x88, 0x84, 0x21, 0xA0][..]);

        let out = remuxer
            .remux_video_frame(41, frame)
            .expect("remux")
            .expect("video frame");
        match out {
            FrameData::Video { timestamp, data } => {
                assert_eq!(timestamp, 41);
                assert_eq!(data[0], 0x17); // keyframe + AVC
                assert_eq!(data[1], 0x01); // AVC NALU
                let nalu_len = u32::from_be_bytes([data[5], data[6], data[7], data[8]]);
                assert_eq!(nalu_len, 5);
                assert_eq!(&data[9..14], [0x65, 0x88, 0x84, 0x21, 0xA0]);
            }
            _ => panic!("expected video frame"),
        }
    }

    #[test]
    fn test_remux_audio_timestamp_scaling_success() {
        let remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0xE0, 0x1E],
            vec![0x68, 0xCE, 0x06, 0xE2],
            Some(vec![0x11, 0x90]),
            48_000,
            15,
        );
        let out = remuxer
            .remux_audio_frame(48_000, BytesMut::from(&[0x01, 0x02, 0x03][..]))
            .expect("audio frame");
        match out {
            FrameData::Audio { timestamp, data } => {
                assert_eq!(timestamp, 1000);
                assert_eq!(data[0], 0xAF);
                assert_eq!(data[1], aac_packet_type::AAC_RAW);
            }
            _ => panic!("expected audio frame"),
        }
    }

    #[test]
    fn test_remux_runtime_sps_pps_filtering_skips_headers_only_frame() {
        let mut remuxer = ValidationHttpFlvRemuxer::new(Vec::new(), Vec::new(), None, 0, 15);
        let mut frame = BytesMut::new();
        frame.extend_from_slice(&annexb_nal(&[0x67, 0x42, 0xE0, 0x1E]));
        frame.extend_from_slice(&annexb_nal(&[0x68, 0xCE, 0x06, 0xE2]));

        let out = remuxer.remux_video_frame(0, frame).expect("remux");
        assert!(out.is_none());
        assert!(remuxer.video_sequence_header(0).is_ok());
    }

    #[test]
    fn test_on_metadata_tag_builds_framerate_ecma_array() {
        let remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0xE0, 0x1E],
            vec![0x68, 0xCE, 0x06, 0xE2],
            None,
            0,
            15,
        );
        let FrameData::MetaData { timestamp, data } = remuxer.on_metadata_tag(0) else {
            panic!("expected MetaData frame");
        };
        assert_eq!(timestamp, 0);

        // AMF string: 0x02, u16be len=10, "onMetaData"
        assert_eq!(&data[..3], &[0x02, 0x00, 0x0a]);
        assert_eq!(&data[3..13], b"onMetaData");

        // ECMA array marker + framerate key/value (15.0 = 0x402E000000000000)
        let body = &data[13..];
        assert_eq!(body[0], 0x08, "ECMA array marker");
        let framerate = body
            .windows(9)
            .position(|w| w == b"framerate".as_slice())
            .expect("framerate key present");
        assert_eq!(&body[framerate + 9..framerate + 10], &[0x00], "number type");
        assert_eq!(
            &body[framerate + 10..framerate + 18],
            &[0x40, 0x2E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            "15.0 as f64"
        );
        assert!(body.ends_with(&[0x00, 0x00, 0x09]), "ECMA array terminator");
    }

    #[test]
    fn test_on_metadata_tag_reflects_audio_presence() {
        let remuxer = ValidationHttpFlvRemuxer::new(
            vec![0x67, 0x42, 0xE0, 0x1E],
            vec![0x68, 0xCE, 0x06, 0xE2],
            Some(vec![0x11, 0x90]),
            48_000,
            15,
        );
        let FrameData::MetaData { data, .. } = remuxer.on_metadata_tag(0) else {
            panic!("expected MetaData frame");
        };
        // hasAudio bool (0x01) true (0x01) follows the "hasAudio" key
        let body = &data[13..];
        let idx = body
            .windows(8)
            .position(|w| w == b"hasAudio".as_slice())
            .expect("hasAudio key present");
        assert_eq!(&body[idx + 8..idx + 10], &[0x01, 0x01], "hasAudio = true");
    }
}
