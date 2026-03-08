//! ONVIF Events Service implementation (scaffold).
//!
//! This module implements the ONVIF Events Service (tev namespace) providing:
//! - Service capabilities (GetServiceCapabilities)
//! - Event properties (GetEventProperties)
//! - All other operations return ActionNotSupported
//!
//! # Module Structure
//!
//! - [`service`] - EventsService struct and ServiceHandler implementation
//! - [`types`] - Request/response types for Events operations
//! - [`faults`] - Fault types specific to Events Service
//!
//! # Status
//!
//! This is a scaffold implementation. Most operations return ActionNotSupported.

pub mod faults;
pub mod service;
pub mod types;

pub use service::EventsService;
