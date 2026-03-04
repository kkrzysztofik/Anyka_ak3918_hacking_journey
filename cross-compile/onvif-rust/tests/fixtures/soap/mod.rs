//! SOAP envelope fixtures for testing ONVIF SOAP message parsing.
//!
//! This module provides test fixtures for validating SOAP envelope parsing,
//! including valid envelopes (happy path) and invalid envelopes (rejection cases).
//! These fixtures are used to test the SOAP dispatcher and security handlers.
//!
//! ## Bug References
//!
//! - `anyka-dev-2sx`: Tests for xmlns ordering issues (tds before s)
//! - `anyka-dev-2hh`: Tests for prefix handling (tds: prefix on body elements)

pub mod envelopes;
