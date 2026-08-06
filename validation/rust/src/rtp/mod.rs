//! Pure pcap/RTP analysis: tshark row parsing, RFC payload validation,
//! stream grouping and packet loss, frame pacing.
//!
//! Nothing here touches `EffectiveConfig`, `TestResult`, ffmpeg or tokio —
//! it takes rows and numbers and returns statistics. Keep it that way.

// Submodules are private: consumers go through the curated re-export list below,
// not through deep `rtp::rows::…` paths.
mod payload;
mod rows;

pub(crate) use payload::{
    RtpPcapRfc3640Stats, RtpPcapRfc6184Stats, analyze_aac_rfc3640_from_rows,
    analyze_h264_rfc6184_from_rows,
};
// Temporary: `group_rtp_rows_by_stream` still lives in `rtsp.rs` and calls these two.
// Once it moves into `rtp::streams`, delete this statement and tighten both to `pub(super)`.
pub(crate) use payload::{validate_aac_rtp_payload_rfc3640, validate_h264_rtp_payload_rfc6184};
pub(crate) use rows::{RtpTsharkRow, tshark_extract_rtp_rows};
