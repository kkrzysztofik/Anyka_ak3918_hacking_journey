//! System-related Device Service handlers.
//!
//! This module contains handlers for:
//! - Device information (GetDeviceInformation)
//! - Capabilities (GetCapabilities, GetServices, GetServiceCapabilities)
//! - System date/time (GetSystemDateAndTime, SetSystemDateAndTime)
//! - System operations (SystemReboot, SetSystemFactoryDefault)
//! - Backup/Restore (GetSystemBackup, RestoreSystem)
//! - Certificates (GetCertificates, GetCertificatesStatus, CreateCertificate, etc.)
//! - Relay outputs (GetRelayOutputs)

use std::sync::Arc;

use chrono::{Datelike, Timelike, Utc};

use crate::config::ConfigRuntime;
use crate::onvif::device::types::{
    DEVICE_SERVICE_NAMESPACE, IMAGING_SERVICE_NAMESPACE, MEDIA_SERVICE_NAMESPACE, ONVIF_VERSION,
    PTZ_SERVICE_NAMESPACE, build_device_capabilities, build_device_service_capabilities,
    build_imaging_capabilities, build_media_capabilities, build_ptz_capabilities, create_service,
};
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::{
    Date, DateTime, SetDateTimeType, SystemDateTime, Time, TimeZone,
};
use crate::onvif::types::device::{
    BackupFile, CreateCertificate, CreateCertificateResponse, DeleteCertificates,
    DeleteCertificatesResponse, GetCapabilities, GetCapabilitiesResponse, GetCertificates,
    GetCertificatesResponse, GetCertificatesStatus, GetCertificatesStatusResponse,
    GetDeviceInformationResponse, GetRelayOutputs, GetRelayOutputsResponse, GetServiceCapabilities,
    GetServiceCapabilitiesResponse, GetServices, GetServicesResponse, GetSystemBackup,
    GetSystemBackupResponse, GetSystemDateAndTime, GetSystemDateAndTimeResponse, LoadCertificates,
    LoadCertificatesResponse, RestoreSystemResponse, SetSystemDateAndTime,
    SetSystemDateAndTimeResponse, SetSystemFactoryDefault, SetSystemFactoryDefaultResponse,
    SystemReboot, SystemRebootResponse,
};
use crate::platform::{DeviceInfo, Platform, external_ip};

/// Handle GetDeviceInformation request.
///
/// Returns device manufacturer, model, firmware version, serial number, and hardware ID.
pub async fn handle_get_device_information(
    platform: &Option<Arc<dyn Platform>>,
    config: &Arc<ConfigRuntime>,
) -> OnvifResult<GetDeviceInformationResponse> {
    tracing::debug!("GetDeviceInformation request");

    let info = if let Some(platform) = platform {
        platform.get_device_info().await.unwrap_or_else(|e| {
            tracing::warn!("Failed to get device info from platform: {}", e);
            device_info_from_config(config)
        })
    } else {
        device_info_from_config(config)
    };

    Ok(GetDeviceInformationResponse {
        manufacturer: info.manufacturer,
        model: info.model,
        // The running build's git describe, not a config string that drifts.
        // What the config carries is the device-family firmware the vendor
        // flashed; what operators and the diagnostics page need is the version
        // of *this* bundle.
        firmware_version: crate::build_version().to_string(),
        serial_number: info.serial_number,
        hardware_id: info.hardware_id,
    })
}

/// Get device info from configuration.
pub fn device_info_from_config(config: &Arc<ConfigRuntime>) -> DeviceInfo {
    let c = config.read();
    DeviceInfo {
        manufacturer: c.device.manufacturer.clone(),
        model: c.device.model.clone(),
        firmware_version: c.device.firmware_version.clone(),
        serial_number: c.device.serial_number.clone(),
        hardware_id: c.device.hardware_id.clone(),
    }
}

/// Handle GetCapabilities request (legacy).
///
/// Returns device capabilities in the legacy format.
// TODO: `request.category` is intentionally ignored — this single-profile embedded camera
// always returns all capabilities regardless of the requested category filter.
pub fn handle_get_capabilities(
    config: &Arc<ConfigRuntime>,
    _request: GetCapabilities,
) -> OnvifResult<GetCapabilitiesResponse> {
    tracing::debug!("GetCapabilities request");

    let base_url = base_url(config);

    Ok(GetCapabilitiesResponse {
        capabilities: crate::onvif::types::device::Capabilities {
            analytics: None,
            device: Some(build_device_capabilities(&base_url)),
            events: None,
            imaging: Some(build_imaging_capabilities(&base_url)),
            media: Some(build_media_capabilities(&base_url)),
            ptz: Some(build_ptz_capabilities(&base_url)),
            extension: None,
        },
    })
}

/// Handle GetServices request.
///
/// Returns list of available services with namespace URIs and XAddrs.
// TODO: `request.include_capability` is logged but service-level capabilities are not
// embedded in the response. All services are always returned — this single-profile
// embedded camera has no per-service selectors or conditional availability.
pub fn handle_get_services(
    config: &Arc<ConfigRuntime>,
    request: GetServices,
) -> OnvifResult<GetServicesResponse> {
    tracing::debug!(
        "GetServices request, include_capability={}",
        request.include_capability
    );

    let base_url = base_url(config);

    let services = vec![
        create_service(
            DEVICE_SERVICE_NAMESPACE,
            &format!("{}/onvif/device_service", base_url),
            ONVIF_VERSION,
        ),
        create_service(
            MEDIA_SERVICE_NAMESPACE,
            &format!("{}/onvif/media_service", base_url),
            ONVIF_VERSION,
        ),
        create_service(
            PTZ_SERVICE_NAMESPACE,
            &format!("{}/onvif/ptz_service", base_url),
            ONVIF_VERSION,
        ),
        create_service(
            IMAGING_SERVICE_NAMESPACE,
            &format!("{}/onvif/imaging_service", base_url),
            ONVIF_VERSION,
        ),
    ];

    Ok(GetServicesResponse { services })
}

/// Handle GetServiceCapabilities request.
///
/// Returns device service capabilities.
pub fn handle_get_service_capabilities(
    _request: GetServiceCapabilities,
) -> OnvifResult<GetServiceCapabilitiesResponse> {
    tracing::debug!("GetServiceCapabilities request");

    Ok(GetServiceCapabilitiesResponse {
        capabilities: build_device_service_capabilities(),
    })
}

/// Handle GetSystemDateAndTime request.
///
/// Returns current system date and time in UTC and local timezone.
pub fn handle_get_system_date_and_time(
    _request: GetSystemDateAndTime,
) -> OnvifResult<GetSystemDateAndTimeResponse> {
    tracing::debug!("GetSystemDateAndTime request");

    let now = Utc::now();

    let utc_date_time = DateTime {
        time: Time {
            hour: now.hour() as i32,
            minute: now.minute() as i32,
            second: now.second() as i32,
        },
        date: Date {
            year: now.year(),
            month: now.month() as i32,
            day: now.day() as i32,
        },
    };

    // For simplicity, local time is same as UTC (no timezone offset)
    let local_date_time = utc_date_time.clone();

    Ok(GetSystemDateAndTimeResponse {
        system_date_and_time: SystemDateTime {
            date_time_type: SetDateTimeType::Manual,
            daylight_savings: false,
            time_zone: Some(TimeZone {
                tz: "UTC".to_string(),
            }),
            utc_date_time: Some(utc_date_time),
            local_date_time: Some(local_date_time),
            extension: None,
        },
    })
}

/// Handle SetSystemDateAndTime request.
///
/// Not yet implemented - returns ActionNotSupported fault.
// TODO: Implement platform call for SetSystemDateAndTime
pub fn handle_set_system_date_and_time(
    request: SetSystemDateAndTime,
) -> OnvifResult<SetSystemDateAndTimeResponse> {
    tracing::debug!(
        "SetSystemDateAndTime request: type={:?} (not implemented)",
        request.date_time_type
    );

    Err(OnvifError::ActionNotSupported(
        "SetSystemDateAndTime".to_string(),
    ))
}

/// Handle SystemReboot request.
///
/// Not yet implemented - returns ActionNotSupported fault.
// TODO: Implement platform call for SystemReboot
pub fn handle_system_reboot(_request: SystemReboot) -> OnvifResult<SystemRebootResponse> {
    tracing::info!("SystemReboot request (not implemented)");

    Err(OnvifError::ActionNotSupported("SystemReboot".to_string()))
}

/// Handle SetSystemFactoryDefault request.
///
/// Not supported - returns ActionNotSupported error.
pub fn handle_set_system_factory_default(
    request: SetSystemFactoryDefault,
) -> OnvifResult<SetSystemFactoryDefaultResponse> {
    tracing::debug!(
        "SetSystemFactoryDefault request - factory_default={:?} (not supported)",
        request.factory_default
    );

    Err(OnvifError::ActionNotSupported(
        "SetSystemFactoryDefault".to_string(),
    ))
}

/// Handle GetCertificates request.
///
/// Returns installed certificates (empty list - no certificates supported).
pub fn handle_get_certificates(_request: GetCertificates) -> OnvifResult<GetCertificatesResponse> {
    tracing::debug!("GetCertificates request");

    // Return empty list - no certificates installed
    Ok(GetCertificatesResponse {
        nvt_certificate: vec![],
    })
}

/// Handle GetCertificatesStatus request.
///
/// Returns certificate statuses (empty list - no certificates supported).
pub fn handle_get_certificates_status(
    _request: GetCertificatesStatus,
) -> OnvifResult<GetCertificatesStatusResponse> {
    tracing::debug!("GetCertificatesStatus request");

    // Return empty list - no certificates installed
    Ok(GetCertificatesStatusResponse {
        certificate_status: vec![],
    })
}

/// Handle CreateCertificate request (stub - not supported).
pub fn handle_create_certificate(
    _request: CreateCertificate,
) -> OnvifResult<CreateCertificateResponse> {
    tracing::debug!("CreateCertificate request (not supported)");
    Err(OnvifError::ActionNotSupported(
        "CreateCertificate".to_string(),
    ))
}

/// Handle LoadCertificates request (stub - not supported).
pub fn handle_load_certificates(
    _request: LoadCertificates,
) -> OnvifResult<LoadCertificatesResponse> {
    tracing::debug!("LoadCertificates request (not supported)");
    Err(OnvifError::ActionNotSupported(
        "LoadCertificates".to_string(),
    ))
}

/// Handle DeleteCertificates request (stub - not supported).
pub fn handle_delete_certificates(
    _request: DeleteCertificates,
) -> OnvifResult<DeleteCertificatesResponse> {
    tracing::debug!("DeleteCertificates request (not supported)");
    Err(OnvifError::ActionNotSupported(
        "DeleteCertificates".to_string(),
    ))
}

/// Handle GetSystemBackup request.
///
/// Returns a backup of the system configuration as a base64-encoded TOML file.
pub fn handle_get_system_backup(
    config: &Arc<ConfigRuntime>,
    _request: GetSystemBackup,
) -> OnvifResult<GetSystemBackupResponse> {
    tracing::debug!("GetSystemBackup request");

    // Serialize current configuration to TOML
    let config_toml = config
        .to_toml_string()
        .map_err(|e| OnvifError::HardwareFailure(format!("Failed to serialize config: {}", e)))?;

    // Encode as base64
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let encoded = STANDARD.encode(config_toml.as_bytes());

    Ok(GetSystemBackupResponse {
        backup_files: vec![BackupFile {
            name: "config.toml".to_string(),
            data: encoded,
        }],
    })
}

/// Handle RestoreSystem request.
///
/// Restores system configuration from a backup file.
pub fn handle_restore_system(
    config: &Arc<ConfigRuntime>,
    request: crate::onvif::types::device::RestoreSystem,
) -> OnvifResult<RestoreSystemResponse> {
    tracing::debug!(
        "RestoreSystem request with {} files",
        request.backup_files.len()
    );

    // Find config.toml in backup files
    let config_file = request
        .backup_files
        .iter()
        .find(|f| f.name == "config.toml")
        .ok_or_else(|| OnvifError::WellFormed("Backup must contain config.toml".to_string()))?;

    // Decode base64
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let decoded = STANDARD
        .decode(&config_file.data)
        .map_err(|e| OnvifError::WellFormed(format!("Invalid base64 data: {}", e)))?;

    // Parse as TOML string
    let toml_str = String::from_utf8(decoded)
        .map_err(|e| OnvifError::WellFormed(format!("Invalid UTF-8 in config: {}", e)))?;

    // Apply configuration
    config
        .load_from_toml_string(&toml_str)
        .map_err(|e| OnvifError::HardwareFailure(format!("Failed to apply config: {}", e)))?;

    tracing::info!("RestoreSystem: configuration restored successfully");

    Ok(RestoreSystemResponse {})
}

/// Handle GetRelayOutputs request.
///
/// Returns relay outputs (empty list - no relay outputs supported).
pub fn handle_get_relay_outputs(_request: GetRelayOutputs) -> OnvifResult<GetRelayOutputsResponse> {
    tracing::debug!("GetRelayOutputs request");

    // Return empty list - no relay outputs on this device
    Ok(GetRelayOutputsResponse {
        relay_outputs: vec![],
    })
}

/// Get the base URL for service addresses.
/// Uses detected IP address for proper XAddr values in capabilities.
fn base_url(config: &Arc<ConfigRuntime>) -> String {
    // Use canonical external_ip helper for consistency across all ONVIF services
    let address = external_ip(config);
    let port = config.read().server.port;
    format!("http://{}:{}", address, port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn create_test_config() -> Arc<ConfigRuntime> {
        Arc::new(ConfigRuntime::new(Default::default()))
    }

    // ========================================================================
    // GetDeviceInformation Tests (T204)
    // ========================================================================

    #[tokio::test]
    async fn test_get_device_information() {
        let config = create_test_config();
        let response = handle_get_device_information(&None, &config).await.unwrap();

        // Default values from config - these should always be present
        assert_eq!(response.manufacturer, "Anyka");
        assert_eq!(response.model, "AK3918 Camera");
        assert_eq!(
            response.firmware_version,
            crate::build_version(),
            "FirmwareVersion must report the build's git describe"
        );
        assert!(!response.firmware_version.is_empty());
    }

    // ========================================================================
    // GetCapabilities Tests (T205)
    // ========================================================================

    #[test]
    fn test_get_capabilities() {
        let config = create_test_config();
        let response =
            handle_get_capabilities(&config, GetCapabilities { category: vec![] }).unwrap();

        // Should have device capabilities
        assert!(response.capabilities.device.is_some());
        let device_caps = response.capabilities.device.unwrap();
        assert!(device_caps.x_addr.contains("device_service"));

        // Should have media capabilities
        assert!(response.capabilities.media.is_some());

        // Should have PTZ capabilities
        assert!(response.capabilities.ptz.is_some());

        // Should have imaging capabilities
        assert!(response.capabilities.imaging.is_some());
    }

    // ========================================================================
    // GetServices Tests (T206)
    // ========================================================================

    #[test]
    fn test_get_services() {
        let config = create_test_config();
        let response = handle_get_services(
            &config,
            GetServices {
                include_capability: false,
            },
        )
        .unwrap();

        // Should have 4 services: Device, Media, PTZ, Imaging
        assert_eq!(response.services.len(), 4);
    }

    // ========================================================================
    // GetSystemDateAndTime Tests (T207)
    // ========================================================================

    #[test]
    fn test_get_system_date_and_time() {
        let response = handle_get_system_date_and_time(GetSystemDateAndTime {}).unwrap();

        let sdt = &response.system_date_and_time;

        // Should have UTC time
        assert!(sdt.utc_date_time.is_some());
        let utc = sdt.utc_date_time.as_ref().unwrap();

        // Basic sanity checks
        assert!(utc.date.year >= 2024);
        assert!(utc.date.month >= 1 && utc.date.month <= 12);
        assert!(utc.date.day >= 1 && utc.date.day <= 31);
        assert!(utc.time.hour >= 0 && utc.time.hour <= 23);
        assert!(utc.time.minute >= 0 && utc.time.minute <= 59);
        assert!(utc.time.second >= 0 && utc.time.second <= 59);

        // Should have local time
        assert!(sdt.local_date_time.is_some());

        // Should have timezone
        assert!(sdt.time_zone.is_some());
    }

    // ========================================================================
    // SystemReboot Tests (T208)
    // ========================================================================

    #[test]
    fn test_system_reboot_not_supported() {
        let result = handle_system_reboot(SystemReboot {});
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // GetServiceCapabilities Test
    // ========================================================================

    #[test]
    fn test_get_service_capabilities() {
        let response = handle_get_service_capabilities(GetServiceCapabilities {}).unwrap();

        let caps = &response.capabilities;

        // Check security capabilities
        assert_eq!(caps.security.max_users, Some(8));
        assert_eq!(caps.security.username_token, Some(true));
        assert_eq!(caps.security.http_digest, Some(true));

        // Check system capabilities
        assert_eq!(caps.system.discovery_bye, Some(true));
        assert_eq!(caps.system.discovery_resolve, Some(true));
    }

    // ========================================================================
    // SetSystemDateAndTime Test
    // ========================================================================

    #[test]
    fn test_set_system_date_and_time_not_supported() {
        let result = handle_set_system_date_and_time(SetSystemDateAndTime {
            date_time_type: SetDateTimeType::Manual,
            daylight_savings: false,
            time_zone: Some(TimeZone {
                tz: "UTC0".to_string(),
            }),
            utc_date_time: Some(DateTime {
                date: Date {
                    year: 2025,
                    month: 1,
                    day: 15,
                },
                time: Time {
                    hour: 12,
                    minute: 30,
                    second: 0,
                },
            }),
        });
        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // GetSystemBackup Tests
    // ========================================================================

    #[test]
    fn test_get_system_backup() {
        let config = create_test_config();
        let response = handle_get_system_backup(&config, GetSystemBackup {}).unwrap();

        // Should have backup files
        assert!(!response.backup_files.is_empty());
        assert_eq!(response.backup_files[0].name, "config.toml");
    }

    // ========================================================================
    // RestoreSystem Tests
    // ========================================================================

    #[test]
    fn test_restore_system_success() {
        let config = create_test_config();

        // Create a backup first
        let backup = handle_get_system_backup(&config, GetSystemBackup {}).unwrap();

        // Restore from backup
        let result = handle_restore_system(
            &config,
            crate::onvif::types::device::RestoreSystem {
                backup_files: backup.backup_files,
            },
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_restore_system_missing_config_toml() {
        let config = create_test_config();

        let result = handle_restore_system(
            &config,
            crate::onvif::types::device::RestoreSystem {
                backup_files: vec![BackupFile {
                    name: "other.toml".to_string(),
                    data: "dGVzdA==".to_string(),
                }],
            },
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::WellFormed(_))));
    }

    #[test]
    fn test_restore_system_invalid_base64() {
        let config = create_test_config();

        let result = handle_restore_system(
            &config,
            crate::onvif::types::device::RestoreSystem {
                backup_files: vec![BackupFile {
                    name: "config.toml".to_string(),
                    data: "!!!invalid base64!!!".to_string(),
                }],
            },
        );

        assert!(result.is_err());
    }

    // ========================================================================
    // GetRelayOutputs Tests
    // ========================================================================

    #[test]
    fn test_get_relay_outputs() {
        let response = handle_get_relay_outputs(GetRelayOutputs {}).unwrap();

        // Should return empty list (no relay outputs supported)
        assert!(response.relay_outputs.is_empty());
    }

    // ========================================================================
    // Certificate Tests
    // ========================================================================

    #[test]
    fn test_get_certificates() {
        let response = handle_get_certificates(GetCertificates {}).unwrap();

        // Should return empty list (no certificates supported)
        assert!(response.nvt_certificate.is_empty());
    }

    #[test]
    fn test_get_certificates_status() {
        let response = handle_get_certificates_status(GetCertificatesStatus {}).unwrap();

        // Should return empty list
        assert!(response.certificate_status.is_empty());
    }

    #[test]
    fn test_create_certificate_not_supported() {
        let result = handle_create_certificate(CreateCertificate {
            certificate_id: Some("test".to_string()),
            subject: Some("CN=test".to_string()),
            valid_not_before: Some("2024-01-01T00:00:00Z".to_string()),
            valid_not_after: Some("2025-01-01T00:00:00Z".to_string()),
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[test]
    fn test_load_certificates_not_supported() {
        let result = handle_load_certificates(LoadCertificates {
            nvt_certificate: vec![],
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    #[test]
    fn test_delete_certificates_not_supported() {
        let result = handle_delete_certificates(DeleteCertificates {
            certificate_id: vec![],
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // SetSystemFactoryDefault Tests
    // ========================================================================

    #[test]
    fn test_set_system_factory_default_not_supported() {
        let result = handle_set_system_factory_default(SetSystemFactoryDefault {
            factory_default: crate::onvif::types::common::FactoryDefaultType::Soft,
        });

        assert!(result.is_err());
        assert!(matches!(result, Err(OnvifError::ActionNotSupported(_))));
    }

    // ========================================================================
    // Base URL Tests
    // ========================================================================

    #[test]
    fn test_base_url_uses_detected_ip() {
        let config = create_test_config();
        {
            let mut c = config.write();
            c.network.detected_ip = "192.168.1.100".to_string();
            c.server.port = 8080;
        }

        let url = base_url(&config);
        assert!(url.contains("192.168.1.100"));
        assert!(url.contains("8080"));
    }

    #[test]
    fn test_base_url_fallback_to_127_0_0_1() {
        let config = create_test_config();
        // No IP config, should fallback to 127.0.0.1 or detected IP

        let url = base_url(&config);
        assert!(url.contains("http://"));
    }
}
