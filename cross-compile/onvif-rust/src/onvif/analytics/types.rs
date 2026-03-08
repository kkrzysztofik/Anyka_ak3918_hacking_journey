//! Analytics Service types.
//!
//! This module defines the request and response types for Analytics Service operations
//! as defined in the ONVIF Analytics Service WSDL specification.

use serde::{Deserialize, Serialize};

pub use crate::onvif::common::GetServiceCapabilities;
pub type GetServiceCapabilitiesResponse =
    crate::onvif::common::SharedGetServiceCapabilitiesResponse<AnalyticsServiceCapabilities>;

// ============================================================================
// Service Capabilities
// ============================================================================

/// Analytics Service capabilities.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AnalyticsServiceCapabilities", rename_all = "PascalCase")]
pub struct AnalyticsServiceCapabilities {
    /// XAddr for the service.
    #[serde(rename = "XAddr", default, skip_serializing_if = "Option::is_none")]
    pub x_addr: Option<String>,

    /// Rule support.
    #[serde(
        rename = "RuleSupport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rule_support: Option<bool>,

    /// Analytics module support.
    #[serde(
        rename = "AnalyticsModuleSupport",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub analytics_module_support: Option<bool>,

    /// Cell-based analytics support.
    #[serde(
        rename = "CellBasedAnalytics",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cell_based_analytics: Option<bool>,
}

// ============================================================================
// Analytics Modules
// ============================================================================

/// GetSupportedAnalyticsModules request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetSupportedAnalyticsModules", rename_all = "PascalCase")]
pub struct GetSupportedAnalyticsModules {
    /// Configuration token.
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<String>,
}

/// GetSupportedAnalyticsModules response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(
    rename = "GetSupportedAnalyticsModulesResponse",
    rename_all = "PascalCase"
)]
pub struct GetSupportedAnalyticsModulesResponse {
    /// Supported analytics modules.
    #[serde(
        rename = "SupportedAnalyticsModules",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub supported_analytics_modules: Option<SupportedAnalyticsModules>,
}

/// Supported analytics modules container.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "SupportedAnalyticsModules", rename_all = "PascalCase")]
pub struct SupportedAnalyticsModules {
    /// Module item (repeated).
    #[serde(
        rename = "AnalyticsModule",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub analytics_module: Vec<AnalyticsModuleItem>,
}

/// Analytics module item.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AnalyticsModule", rename_all = "PascalCase")]
pub struct AnalyticsModuleItem {
    /// Module name.
    #[serde(rename = "Name", default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Module type.
    #[serde(
        rename = "Type",
        alias = "type",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub module_type: Option<String>,

    /// Extension element.
    #[serde(rename = "Extension", default, skip_serializing_if = "Option::is_none")]
    pub extension: Option<AnalyticsModuleExtension>,
}

/// Analytics module extension (empty placeholder).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AnalyticsModuleExtension", rename_all = "PascalCase")]
pub struct AnalyticsModuleExtension {}

// ============================================================================
// CreateAnalyticsModules
// ============================================================================

/// CreateAnalyticsModules request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreateAnalyticsModules", rename_all = "PascalCase")]
pub struct CreateAnalyticsModules {
    /// Configuration token.
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<String>,

    /// Analytics modules to create.
    #[serde(
        rename = "AnalyticsModules",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub analytics_modules: Option<CreateAnalyticsModulesList>,
}

/// Analytics modules list for creation.
// TODO: CreateAnalyticsModulesList and GetAnalyticsModulesList are structurally
// identical (both wrap Vec<AnalyticsModuleItem>). Consider consolidating into a
// single AnalyticsModulesList type once all callers are surveyed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AnalyticsModules", rename_all = "PascalCase")]
pub struct CreateAnalyticsModulesList {
    /// Module item.
    #[serde(
        rename = "AnalyticsModule",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub analytics_module: Vec<AnalyticsModuleItem>,
}

/// CreateAnalyticsModules response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "CreateAnalyticsModulesResponse", rename_all = "PascalCase")]
pub struct CreateAnalyticsModulesResponse {}

// ============================================================================
// DeleteAnalyticsModules
// ============================================================================

/// DeleteAnalyticsModules request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "DeleteAnalyticsModules", rename_all = "PascalCase")]
pub struct DeleteAnalyticsModules {
    /// Configuration token.
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<String>,

    /// Names of modules to delete.
    #[serde(
        rename = "AnalyticsModuleName",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub analytics_module_name: Vec<String>,
}

/// DeleteAnalyticsModules response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "DeleteAnalyticsModulesResponse", rename_all = "PascalCase")]
pub struct DeleteAnalyticsModulesResponse {}

// ============================================================================
// GetAnalyticsModules
// ============================================================================

/// GetAnalyticsModules request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAnalyticsModules", rename_all = "PascalCase")]
pub struct GetAnalyticsModules {
    /// Configuration token.
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<String>,
}

/// GetAnalyticsModules response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "GetAnalyticsModulesResponse", rename_all = "PascalCase")]
pub struct GetAnalyticsModulesResponse {
    /// Analytics modules.
    #[serde(
        rename = "AnalyticsModules",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub analytics_modules: Option<GetAnalyticsModulesList>,
}

/// Analytics modules list for retrieval.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "AnalyticsModules", rename_all = "PascalCase")]
pub struct GetAnalyticsModulesList {
    /// Module item.
    #[serde(
        rename = "AnalyticsModule",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub analytics_module: Vec<AnalyticsModuleItem>,
}

// ============================================================================
// ModifyAnalyticsModules
// ============================================================================

/// ModifyAnalyticsModules request.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "ModifyAnalyticsModules", rename_all = "PascalCase")]
pub struct ModifyAnalyticsModules {
    /// Configuration token.
    #[serde(
        rename = "ConfigurationToken",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub configuration_token: Option<String>,

    /// Analytics modules to modify.
    #[serde(
        rename = "AnalyticsModules",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub analytics_modules: Option<CreateAnalyticsModulesList>,
}

/// ModifyAnalyticsModules response.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "ModifyAnalyticsModulesResponse", rename_all = "PascalCase")]
pub struct ModifyAnalyticsModulesResponse {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_service_capabilities_default() {
        let req = GetServiceCapabilities {};
        // Empty struct - just check it can be created
        let _ = req;
    }

    #[test]
    fn test_get_supported_analytics_modules_default() {
        let req = GetSupportedAnalyticsModules::default();
        assert_eq!(req.configuration_token, None);
    }

    #[test]
    fn test_supported_analytics_modules_empty() {
        let modules = SupportedAnalyticsModules::default();
        assert!(modules.analytics_module.is_empty());
    }

    #[test]
    fn test_analytics_service_capabilities_defaults() {
        let caps = AnalyticsServiceCapabilities::default();
        assert_eq!(caps.x_addr, None);
        assert_eq!(caps.rule_support, None);
        assert_eq!(caps.analytics_module_support, None);
    }

    #[test]
    fn test_create_analytics_modules_request() {
        let req = CreateAnalyticsModules::default();
        assert_eq!(req.configuration_token, None);
        assert!(req.analytics_modules.is_none());
    }

    #[test]
    fn test_delete_analytics_modules_request() {
        let req = DeleteAnalyticsModules::default();
        assert_eq!(req.configuration_token, None);
        assert!(req.analytics_module_name.is_empty());
    }

    #[test]
    fn test_get_analytics_modules_request() {
        let req = GetAnalyticsModules::default();
        assert_eq!(req.configuration_token, None);
    }

    #[test]
    fn test_modify_analytics_modules_request() {
        let req = ModifyAnalyticsModules::default();
        assert_eq!(req.configuration_token, None);
        assert!(req.analytics_modules.is_none());
    }

    #[test]
    fn test_analytics_capabilities_roundtrip_xml() {
        let caps = AnalyticsServiceCapabilities {
            x_addr: Some("http://192.168.1.1/analytics".to_string()),
            rule_support: Some(true),
            analytics_module_support: Some(false),
            cell_based_analytics: Some(true),
        };

        let xml = quick_xml::se::to_string(&caps).expect("serialize");
        let deserialized: AnalyticsServiceCapabilities =
            quick_xml::de::from_str(&xml).expect("deserialize");

        assert_eq!(caps, deserialized);
    }

    #[test]
    fn test_analytics_module_item_roundtrip_xml() {
        let item = AnalyticsModuleItem {
            name: Some("MotionDetector".to_string()),
            module_type: Some("tt:MotionDetection".to_string()),
            extension: None,
        };

        let xml = quick_xml::se::to_string(&item).expect("serialize");
        let deserialized: AnalyticsModuleItem = quick_xml::de::from_str(&xml).expect("deserialize");

        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_get_supported_analytics_modules_response_roundtrip_xml() {
        let response = GetSupportedAnalyticsModulesResponse {
            supported_analytics_modules: Some(SupportedAnalyticsModules {
                analytics_module: vec![AnalyticsModuleItem {
                    name: Some("CellMotion".to_string()),
                    module_type: Some("tt:CellMotionEngine".to_string()),
                    extension: None,
                }],
            }),
        };

        let xml = quick_xml::se::to_string(&response).expect("serialize");
        let deserialized: GetSupportedAnalyticsModulesResponse =
            quick_xml::de::from_str(&xml).expect("deserialize");

        assert_eq!(response, deserialized);
    }
}
