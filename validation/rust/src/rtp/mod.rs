//! Pure pcap/RTP analysis: tshark row parsing, RFC payload validation,
//! stream grouping and packet loss, frame pacing.
//!
//! Nothing here touches `EffectiveConfig`, `TestResult`, ffmpeg or tokio —
//! it takes rows and numbers and returns statistics. Keep it that way.

// Submodules are private: consumers go through the curated re-export list below,
// not through deep `rtp::rows::…` paths.
mod rows;

pub(crate) use rows::{RtpTsharkRow, tshark_extract_rtp_rows};
