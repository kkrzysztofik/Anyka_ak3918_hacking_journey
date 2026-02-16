//! Live streaming infrastructure for RTSP and HTTP-FLV delivery.
//!
//! This module bridges the platform's frame delivery (`FrameCallback`) to
//! `streaming-lib`'s `StreamsHub`, enabling live RTSP and HTTP-FLV streaming
//! for both main and sub video channels.
//!
//! # Architecture
//!
//! ```text
//! Anyka SDK encoder callbacks
//!     ↓
//! Platform::register_frame_callback()
//!     ↓
//! StreamingBridge::on_frame(Frame)
//!     ↓ copies data into BytesMut, routes by StreamId
//!     ├── main_tx → fanout → RTSP channel (main)  → StreamsHub → RTSP server
//!     │                    → HTTP-FLV channel (main) → StreamsHub → HTTP-FLV server
//!     └── sub_tx  → fanout → RTSP channel (sub)   → StreamsHub → RTSP server
//!                          → HTTP-FLV channel (sub) → StreamsHub → HTTP-FLV server
//! ```
//!
//! # Stream URLs
//!
//! - RTSP main: `rtsp://<ip>:<port>/main`
//! - RTSP sub:  `rtsp://<ip>:<port>/sub`
//! - HTTP-FLV main: `http://<ip>:<port>/live/main.flv`
//! - HTTP-FLV sub:  `http://<ip>:<port>/live/sub.flv`

pub mod bridge;
pub mod config;
pub mod helpers;
pub mod service;
