//! Discovery and scope-related Device Service handlers.
//!
//! This module contains handlers for:
//! - Scopes (GetScopes, SetScopes, AddScopes, RemoveScopes)
//! - Discovery mode (GetDiscoveryMode, SetDiscoveryMode)

#![cfg_attr(not(test), allow(dead_code))]

use crate::onvif::device::faults::validate_scope;
use crate::onvif::error::OnvifResult;
use crate::onvif::types::common::{DiscoveryMode, Scope, ScopeDefinition};
use crate::onvif::types::device::{
    AddScopes, AddScopesResponse, GetDiscoveryMode, GetDiscoveryModeResponse, GetScopes,
    GetScopesResponse, RemoveScopes, RemoveScopesResponse, SetDiscoveryMode,
    SetDiscoveryModeResponse, SetScopes, SetScopesResponse,
};

/// Get the default scopes.
///
/// The `ptz_enabled` flag controls whether the `type/ptz` fixed scope is
/// advertised; a device with `[ptz] enabled = false` has no pan/tilt motor and
/// must not claim PTZ support during discovery.
pub fn default_scopes(ptz_enabled: bool) -> Vec<Scope> {
    let mut scopes = vec![
        Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/type/video_encoder".to_string(),
        },
        Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/type/audio_encoder".to_string(),
        },
    ];
    if ptz_enabled {
        scopes.push(Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/type/ptz".to_string(),
        });
    }
    scopes.extend([
        Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: "onvif://www.onvif.org/location/country/unknown".to_string(),
        },
        Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: "onvif://www.onvif.org/name/OnvifCamera".to_string(),
        },
    ]);
    scopes
}

/// Handle GetScopes request.
///
/// Returns device scopes.
pub fn handle_get_scopes(
    scopes: &parking_lot::RwLock<Vec<Scope>>,
    _request: GetScopes,
) -> OnvifResult<GetScopesResponse> {
    tracing::debug!("GetScopes request");

    // Note: clone required to release the RwLock before returning
    let scopes = scopes.read().clone();

    Ok(GetScopesResponse { scopes })
}

/// Handle GetScopes request with a Vec<Scope>.
///
/// Returns device scopes.
pub fn handle_get_scopes_from_vec(
    scopes: &[Scope],
    _request: GetScopes,
) -> OnvifResult<GetScopesResponse> {
    tracing::debug!("GetScopes request");

    Ok(GetScopesResponse {
        scopes: scopes.to_vec(),
    })
}

/// Handle SetScopes request.
///
/// Replaces configurable scopes.
pub fn handle_set_scopes(
    scopes: &parking_lot::RwLock<Vec<Scope>>,
    request: SetScopes,
) -> OnvifResult<SetScopesResponse> {
    tracing::debug!("SetScopes request: {} scopes", request.scopes.len());

    // Validate all scopes
    for scope in &request.scopes {
        validate_scope(scope)?;
    }

    let mut scopes_lock = scopes.write();

    // Keep fixed scopes, replace configurable ones
    let fixed_scopes: Vec<Scope> = scopes_lock
        .iter()
        .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
        .cloned()
        .collect();

    let new_configurable: Vec<Scope> = request
        .scopes
        .into_iter()
        .map(|s| Scope {
            scope_def: ScopeDefinition::Configurable,
            scope_item: s,
        })
        .collect();

    *scopes_lock = fixed_scopes;
    scopes_lock.extend(new_configurable);

    tracing::info!("SetScopes: updated to {} scopes", scopes_lock.len());

    Ok(SetScopesResponse {})
}

/// Handle AddScopes request.
///
/// Adds new scopes to the device.
pub fn handle_add_scopes(
    scopes: &parking_lot::RwLock<Vec<Scope>>,
    request: AddScopes,
) -> OnvifResult<AddScopesResponse> {
    tracing::debug!("AddScopes request: {} scopes", request.scope_item.len());

    // Validate all scopes
    for scope in &request.scope_item {
        validate_scope(scope)?;
    }

    let mut scopes_lock = scopes.write();

    for scope_item in request.scope_item {
        // Check if scope already exists
        if !scopes_lock.iter().any(|s| s.scope_item == scope_item) {
            scopes_lock.push(Scope {
                scope_def: ScopeDefinition::Configurable,
                scope_item,
            });
        }
    }

    tracing::info!("AddScopes: now have {} scopes", scopes_lock.len());

    Ok(AddScopesResponse {})
}

/// Handle RemoveScopes request.
///
/// Removes specified scopes from the device. Only configurable scopes can be removed.
/// Fixed scopes cannot be removed.
pub fn handle_remove_scopes(
    scopes: &parking_lot::RwLock<Vec<Scope>>,
    request: RemoveScopes,
) -> OnvifResult<RemoveScopesResponse> {
    tracing::debug!("RemoveScopes request: {} scopes", request.scope_item.len());

    let mut scopes_lock = scopes.write();
    let mut removed = Vec::new();

    for scope_item in &request.scope_item {
        // Find and remove the scope (only if configurable)
        if let Some(pos) = scopes_lock.iter().position(|s| {
            s.scope_item == *scope_item && matches!(s.scope_def, ScopeDefinition::Configurable)
        }) {
            scopes_lock.remove(pos);
            removed.push(scope_item.clone());
        }
    }

    tracing::info!(
        "RemoveScopes: removed {} scopes, {} remaining",
        removed.len(),
        scopes_lock.len()
    );

    // Return the removed scope items per WSDL
    Ok(RemoveScopesResponse {
        scope_item: removed,
    })
}

/// Handle GetDiscoveryMode request.
///
/// Returns current discovery mode.
pub fn handle_get_discovery_mode(
    discovery_mode: &parking_lot::RwLock<DiscoveryMode>,
    _request: GetDiscoveryMode,
) -> OnvifResult<GetDiscoveryModeResponse> {
    tracing::debug!("GetDiscoveryMode request");

    // Note: clone required to release the RwLock before returning
    let mode = discovery_mode.read().clone();

    Ok(GetDiscoveryModeResponse {
        discovery_mode: mode,
    })
}

/// Handle SetDiscoveryMode request.
///
/// Sets the discovery mode (Discoverable or NonDiscoverable).
pub fn handle_set_discovery_mode(
    discovery_mode: &parking_lot::RwLock<DiscoveryMode>,
    request: SetDiscoveryMode,
) -> OnvifResult<SetDiscoveryModeResponse> {
    tracing::debug!("SetDiscoveryMode request: {:?}", request.discovery_mode);

    let mode = request.discovery_mode.clone();
    *discovery_mode.write() = request.discovery_mode;

    tracing::info!("SetDiscoveryMode: mode set to {:?}", mode);

    Ok(SetDiscoveryModeResponse {})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_scopes() -> parking_lot::RwLock<Vec<Scope>> {
        parking_lot::RwLock::new(default_scopes(true))
    }

    fn create_test_discovery_mode() -> parking_lot::RwLock<DiscoveryMode> {
        parking_lot::RwLock::new(DiscoveryMode::Discoverable)
    }

    #[test]
    fn test_default_scopes_ptz_disabled_omits_ptz() {
        let scopes = default_scopes(false);
        assert!(
            !scopes
                .iter()
                .any(|s| s.scope_item == "onvif://www.onvif.org/type/ptz")
        );
    }

    #[test]
    fn test_default_scopes_ptz_enabled_includes_ptz() {
        let scopes = default_scopes(true);
        assert!(
            scopes
                .iter()
                .any(|s| s.scope_item == "onvif://www.onvif.org/type/ptz")
        );
    }

    // ========================================================================
    // GetScopes Tests (T212)
    // ========================================================================

    #[test]
    fn test_get_scopes() {
        let scopes = create_test_scopes();
        let response = handle_get_scopes(&scopes, GetScopes {}).unwrap();

        // Should have default scopes
        assert!(!response.scopes.is_empty());

        // Should have fixed scopes
        let fixed_scopes: Vec<_> = response
            .scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .collect();
        assert!(!fixed_scopes.is_empty());
    }

    // ========================================================================
    // SetScopes Tests (T213)
    // ========================================================================

    #[test]
    fn test_set_scopes_valid() {
        let scopes = create_test_scopes();
        let result = handle_set_scopes(
            &scopes,
            SetScopes {
                scopes: vec![
                    "onvif://www.onvif.org/location/office".to_string(),
                    "onvif://www.onvif.org/name/MyCamera".to_string(),
                ],
            },
        );
        assert!(result.is_ok());

        // Verify scopes were updated - use the SAME scopes object, not a new one
        let response = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        let scope_items: Vec<&str> = response
            .scopes
            .iter()
            .map(|s| s.scope_item.as_str())
            .collect();
        assert!(scope_items.contains(&"onvif://www.onvif.org/location/office"));
        assert!(scope_items.contains(&"onvif://www.onvif.org/name/MyCamera"));
    }

    #[test]
    fn test_set_scopes_preserves_fixed() {
        let scopes = create_test_scopes();

        // Get initial fixed scopes
        let initial = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        let fixed_count = initial
            .scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .count();

        // Set new configurable scopes
        handle_set_scopes(
            &scopes,
            SetScopes {
                scopes: vec!["onvif://www.onvif.org/name/NewName".to_string()],
            },
        )
        .unwrap();

        // Verify fixed scopes are preserved
        let after = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        let fixed_after = after
            .scopes
            .iter()
            .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .count();

        assert_eq!(fixed_count, fixed_after);
    }

    #[test]
    fn test_set_scopes_invalid_prefix() {
        let scopes = create_test_scopes();
        let result = handle_set_scopes(
            &scopes,
            SetScopes {
                scopes: vec!["http://example.com/scope".to_string()],
            },
        );
        assert!(matches!(
            result,
            Err(crate::onvif::error::OnvifError::InvalidArgVal { subcode, .. }) if subcode == "InvalidScope"
        ));
    }

    // ========================================================================
    // AddScopes Tests (T214)
    // ========================================================================

    #[test]
    fn test_add_scopes() {
        let scopes = create_test_scopes();
        let initial = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        let initial_count = initial.scopes.len();

        // Add a new scope
        handle_add_scopes(
            &scopes,
            AddScopes {
                scope_item: vec!["onvif://www.onvif.org/hardware/HD".to_string()],
            },
        )
        .unwrap();

        // Verify scope was added
        let after = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        assert_eq!(after.scopes.len(), initial_count + 1);
    }

    #[test]
    fn test_add_scopes_no_duplicates() {
        let scopes = create_test_scopes();

        // Add the same scope twice
        handle_add_scopes(
            &scopes,
            AddScopes {
                scope_item: vec!["onvif://www.onvif.org/unique/scope".to_string()],
            },
        )
        .unwrap();

        let count1 = handle_get_scopes(&scopes, GetScopes {})
            .unwrap()
            .scopes
            .len();

        handle_add_scopes(
            &scopes,
            AddScopes {
                scope_item: vec!["onvif://www.onvif.org/unique/scope".to_string()],
            },
        )
        .unwrap();

        let count2 = handle_get_scopes(&scopes, GetScopes {})
            .unwrap()
            .scopes
            .len();

        assert_eq!(count1, count2);
    }

    // ========================================================================
    // RemoveScopes Tests
    // ========================================================================

    #[test]
    fn test_remove_scopes_configurable() {
        let scopes = create_test_scopes();

        // First add a scope
        handle_add_scopes(
            &scopes,
            AddScopes {
                scope_item: vec!["onvif://www.onvif.org/test/removable".to_string()],
            },
        )
        .unwrap();

        let before = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        let before_count = before.scopes.len();

        // Remove the scope
        let response = handle_remove_scopes(
            &scopes,
            RemoveScopes {
                scope_item: vec!["onvif://www.onvif.org/test/removable".to_string()],
            },
        )
        .unwrap();

        // Should return the removed scope
        assert_eq!(response.scope_item.len(), 1);
        assert_eq!(
            response.scope_item[0],
            "onvif://www.onvif.org/test/removable"
        );

        // Verify scope count decreased
        let after = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        assert_eq!(after.scopes.len(), before_count - 1);
    }

    #[test]
    fn test_remove_scopes_fixed_not_removed() {
        let scopes = create_test_scopes();

        // Get a fixed scope
        let scopes_data = scopes.read();
        let maybe_fixed = scopes_data
            .iter()
            .find(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
            .cloned();
        drop(scopes_data);
        assert!(
            maybe_fixed.is_some(),
            "Default scopes should contain at least one fixed scope"
        );
        let fixed_scope = maybe_fixed.unwrap();

        let before_count = scopes.read().len();

        // Try to remove a fixed scope
        let response = handle_remove_scopes(
            &scopes,
            RemoveScopes {
                scope_item: vec![fixed_scope.scope_item.clone()],
            },
        )
        .unwrap();

        // Should return empty (nothing removed)
        assert!(response.scope_item.is_empty());

        // Verify count unchanged
        let after = handle_get_scopes(&scopes, GetScopes {}).unwrap();
        assert_eq!(after.scopes.len(), before_count);
    }

    // ========================================================================
    // GetDiscoveryMode Tests (T215)
    // ========================================================================

    #[test]
    fn test_get_discovery_mode() {
        let discovery_mode = create_test_discovery_mode();
        let response = handle_get_discovery_mode(&discovery_mode, GetDiscoveryMode {}).unwrap();

        // Default should be Discoverable
        assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
    }

    // ========================================================================
    // SetDiscoveryMode Tests (T216)
    // ========================================================================

    #[test]
    fn test_set_discovery_mode() {
        let discovery_mode = create_test_discovery_mode();

        // Set to NonDiscoverable
        handle_set_discovery_mode(
            &discovery_mode,
            SetDiscoveryMode {
                discovery_mode: DiscoveryMode::NonDiscoverable,
            },
        )
        .unwrap();

        let response = handle_get_discovery_mode(&discovery_mode, GetDiscoveryMode {}).unwrap();
        assert_eq!(response.discovery_mode, DiscoveryMode::NonDiscoverable);

        // Set back to Discoverable
        handle_set_discovery_mode(
            &discovery_mode,
            SetDiscoveryMode {
                discovery_mode: DiscoveryMode::Discoverable,
            },
        )
        .unwrap();

        let response = handle_get_discovery_mode(&discovery_mode, GetDiscoveryMode {}).unwrap();
        assert_eq!(response.discovery_mode, DiscoveryMode::Discoverable);
    }
}
