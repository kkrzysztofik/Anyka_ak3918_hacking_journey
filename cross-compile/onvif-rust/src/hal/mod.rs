//! Hardware Abstraction Layer (HAL) for Anyka AK3918 platform.
//!
//! This module provides the hardware abstraction layer for the Anyka platform.
//! On ARM targets, the AnykaIpc client communicates with vendor-daemon via Unix socket.
//! The PTZ driver uses native Rust ioctl to /dev/ak-motor* devices.
//! For host builds (testing), stub implementations are provided.

pub mod anyka;
pub mod common;
pub mod stub;
