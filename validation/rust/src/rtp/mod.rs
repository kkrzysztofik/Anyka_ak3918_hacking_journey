//! Pure pcap/RTP analysis: tshark row parsing, RFC payload validation,
//! stream grouping and packet loss, frame pacing.
//!
//! Nothing here touches `EffectiveConfig`, `TestResult`, ffmpeg or tokio —
//! it takes rows and numbers and returns statistics. Keep it that way.

// Submodules are private: consumers go through the curated re-export list below,
// not through deep `rtp::rows::…` paths.
mod pacing;
mod payload;
mod rows;
mod streams;

/// Minimum packet count before a stream is classifiable as H.264/AAC.
///
/// Shared by stream selection (`streams.rs`) and the RFC payload analyzers
/// (`payload.rs`) so the four copies cannot drift apart.
pub(crate) const MIN_PACKETS: u32 = 10;

/// Minimum valid-payload ratio before a stream is classifiable as H.264/AAC.
///
/// Shared by stream selection (`streams.rs`) and the RFC payload analyzers
/// (`payload.rs`) so the four copies cannot drift apart.
pub(crate) const MIN_VALID_RATIO: f64 = 0.80;

pub(crate) use pacing::{FramePacing, GapStats, compute_pacing};
pub(crate) use payload::{
    RtpPcapRfc3640Stats, RtpPcapRfc6184Stats, analyze_aac_rfc3640_from_rows,
    analyze_h264_rfc6184_from_rows,
};
pub(crate) use rows::tshark_extract_rtp_rows;
pub(crate) use streams::{
    RtpLossMetric, compute_stream_loss_metric, group_rtp_rows_by_stream, pick_primary_audio_stream,
    pick_primary_video_stream,
};
