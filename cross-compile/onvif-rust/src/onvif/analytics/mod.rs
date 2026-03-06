//! ONVIF Analytics Service implementation (scaffold).
//!
//! This module implements the ONVIF Analytics Service (tan namespace) providing:
//! - Service capabilities (GetServiceCapabilities)
//! - Analytics module listing (GetSupportedAnalyticsModules)
//! - All other operations return ActionNotSupported
//!
//! # Module Structure
//!
//! - [`service`] - AnalyticsService struct and ServiceHandler implementation
//! - [`types`] - Request/response types for Analytics operations
//! - [`faults`] - Fault types specific to Analytics Service
//!
//! # Status
//!
//! This is a scaffold implementation. Most operations return ActionNotSupported.

pub mod faults;
pub mod service;
pub mod types;

pub use service::AnalyticsService;
