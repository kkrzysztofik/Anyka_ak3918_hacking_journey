//! On-demand device diagnostics served over JSON.
//!
//! Deliberately has no background sampler: everything is read when a request
//! arrives, so an unwatched page costs this single-core device nothing.

pub mod http;
pub mod logs;
pub mod network;
pub mod proc;
pub mod state;
pub mod storage;
pub mod update;
pub mod wifi;
