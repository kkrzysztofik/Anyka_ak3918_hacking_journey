//! ONVIF HTTP/SOAP Server implementation.
//!
//! This module provides the complete ONVIF 2.5 HTTP/SOAP server infrastructure
//! including request parsing, response building, service routing, and error handling.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         OnvifServer                                  │
//! │  ┌─────────────────────────────────────────────────────────────┐   │
//! │  │                    axum::Router                              │   │
//! │  │  /onvif/device_service  → ServiceDispatcher → DeviceService │   │
//! │  │  /onvif/media_service   → ServiceDispatcher → MediaService  │   │
//! │  │  /onvif/ptz_service     → ServiceDispatcher → PTZService    │   │
//! │  │  /onvif/imaging_service → ServiceDispatcher → ImagingService│   │
//! │  └─────────────────────────────────────────────────────────────┘   │
//! │                                                                      │
//! │  Middleware: TimeoutLayer, DefaultBodyLimit, CorsLayer              │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Modules
//!
//! - [`server`] - HTTP server and router setup
//! - [`soap`] - SOAP envelope parsing and building
//! - [`dispatcher`] - Service routing and action dispatching
//! - [`error`] - ONVIF error types and SOAP fault generation
//!
//! # Example
//!
//! ```ignore
//! use onvif_rust::onvif::{OnvifServer, OnvifServerConfig};
//! use onvif_rust::config::ConfigRuntime;
//! use onvif_rust::platform::StubPlatform;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = OnvifServerConfig::default();
//!     let server = OnvifServer::new(config)?;
//!     server.start().await?;
//!     Ok(())
//! }
//! ```

pub mod dispatcher;
pub mod error;
pub mod server;
pub mod soap;
pub mod types;

// Service modules (to be implemented in later phases)
pub mod device;
pub mod imaging;
pub mod media;
pub mod ptz;

// Re-exports for convenience
pub use dispatcher::{ServiceDispatcher, ServiceHandler};
pub use error::{OnvifError, OnvifResult};
pub use server::{OnvifServer, OnvifServerConfig};
pub use soap::{SoapBody, SoapEnvelope, SoapHeader};
