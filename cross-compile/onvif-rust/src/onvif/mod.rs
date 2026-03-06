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

pub mod auth_requirements;
pub mod dispatcher;
pub mod error;
pub mod server;
pub mod soap;
pub mod types;
pub mod ws_security;

// Facade module providing unified access to common infrastructure
pub mod common;

// Service modules (to be implemented in later phases)
pub mod analytics;
pub mod device;
pub mod discovery;
pub mod events;
pub mod imaging;
pub mod media;
pub mod ptz;

// Re-exports for convenience
pub use auth_requirements::{AuthLevel, get_required_level, requires_auth};
pub use dispatcher::{ServiceDispatcher, ServiceHandler};
pub use error::{OnvifError, OnvifResult};
pub use server::{OnvifServer, OnvifServerConfig};
pub use soap::{SoapBody, SoapEnvelope, SoapHeader};
pub use ws_security::{WsSecurityConfig, WsSecurityError, WsSecurityValidator};

#[cfg(test)]
mod common_facade_tests {
    use super::*;

    /// Test that types accessed via common:: are identical to direct paths
    /// This verifies path equivalence at compile time
    #[test]
    fn test_common_facade_path_equivalence() {
        // Test SoapEnvelope equivalence
        type DirectSoap = soap::SoapEnvelope<String>;
        type FacadeSoap = common::soap::SoapEnvelope<String>;
        assert_eq!(
            std::mem::size_of::<DirectSoap>(),
            std::mem::size_of::<FacadeSoap>()
        );

        // Test OnvifError equivalence
        type DirectError = error::OnvifError;
        type FacadeError = common::error::OnvifError;
        assert_eq!(
            std::mem::size_of::<DirectError>(),
            std::mem::size_of::<FacadeError>()
        );

        // Test WsSecurityConfig equivalence
        type DirectWsSec = ws_security::WsSecurityConfig;
        type FacadeWsSec = common::ws_security::WsSecurityConfig;
        assert_eq!(
            std::mem::size_of::<DirectWsSec>(),
            std::mem::size_of::<FacadeWsSec>()
        );

        // Test AuthLevel equivalence
        type DirectAuth = auth_requirements::AuthLevel;
        type FacadeAuth = common::auth_requirements::AuthLevel;
        assert_eq!(
            std::mem::size_of::<DirectAuth>(),
            std::mem::size_of::<FacadeAuth>()
        );
    }

    /// Test that both old and new import paths work for SoapEnvelope
    #[test]
    fn test_soap_envelope_both_paths_work() {
        // Original path
        let direct: soap::SoapEnvelope<String> = soap::SoapEnvelope {
            header: None,
            body: soap::SoapBody {
                content: "test".to_string(),
            },
        };

        // New facade path
        let facade: common::soap::SoapEnvelope<String> = common::soap::SoapEnvelope {
            header: None,
            body: common::soap::SoapBody {
                content: "test".to_string(),
            },
        };

        // Both should be constructable and equal in size
        assert_eq!(
            std::mem::size_of_val(&direct),
            std::mem::size_of_val(&facade)
        );
    }

    /// Test that both old and new import paths work for OnvifError
    #[test]
    fn test_onvif_error_both_paths_work() {
        // Original path
        let direct = error::OnvifError::ActionNotSupported("test".to_string());

        // New facade path
        let facade = common::error::OnvifError::ActionNotSupported("test".to_string());

        // Both should have the same variant
        match (&direct, &facade) {
            (
                error::OnvifError::ActionNotSupported(_),
                common::error::OnvifError::ActionNotSupported(_),
            ) => {}
            _ => panic!("Expected ActionNotSupported variant"),
        }
    }

    /// Test that both old and new import paths work for WsSecurityConfig
    #[test]
    fn test_ws_security_config_both_paths_work() {
        // Original path
        let direct = ws_security::WsSecurityConfig::default();

        // New facade path
        let facade = common::ws_security::WsSecurityConfig::default();

        // Both should have default values
        assert_eq!(direct.require_digest, facade.require_digest);
        assert_eq!(direct.clock_skew_seconds, facade.clock_skew_seconds);
        assert_eq!(direct.nonce_ttl_seconds, facade.nonce_ttl_seconds);
        assert_eq!(direct.max_nonce_cache_size, facade.max_nonce_cache_size);
    }

    /// Test that both old and new import paths work for AuthLevel
    #[test]
    fn test_auth_level_both_paths_work() {
        // Original path
        let direct = auth_requirements::AuthLevel::Administrator;

        // New facade path
        let facade = common::auth_requirements::AuthLevel::Administrator;

        // Both should be equal
        assert_eq!(direct, facade);
        assert_eq!(direct, common::auth_requirements::AuthLevel::Administrator);
    }
}
