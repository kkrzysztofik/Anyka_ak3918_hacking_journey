//! Validation file readers for testing and development.
//!
//! This module provides file-based media readers used primarily for:
//! - Unit testing of codec and container components
//! - Integration testing of streaming servers
//! - Development and debugging purposes
//!
//! ## Usage
//!
//! These readers parse H.264/AAC files and extract frames for testing purposes:
//! - [`h264_file_reader::H264FileReader`] - Parses H.264 Annex-B streams
//! - [`aac_file_reader::AacFileReader`] - Parses AAC ADTS streams
//!
//! ## Note
//!
//! In production deployments, these readers are typically not needed as media
//! is received from live sources (RTSP push, HTTP-FLV, etc.). However, they
//! remain in the main binary for flexibility and testing convenience.

pub mod aac_file_reader;
pub mod h264_file_reader;
