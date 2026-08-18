//! Discovery and scope-related Device Service helpers.
//!
//! Fixed scopes are derived via [`default_scopes`]; GetScopes responses are
//! built by [`handle_get_scopes_from_vec`]. Mutation handlers live on
//! `DeviceService`.

use crate::onvif::error::OnvifResult;
use crate::onvif::types::common::{Scope, ScopeDefinition};
use crate::onvif::types::device::{GetScopes, GetScopesResponse};

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
        Scope {
            scope_def: ScopeDefinition::Fixed,
            scope_item: "onvif://www.onvif.org/Profile/Streaming".to_string(),
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

/// Merge derived fixed scopes with the configured configurable ones.
///
/// Fixed scopes are never stored: they follow device capabilities, so a PTZ
/// config change is reflected with no migration and a client cannot persist a
/// bogus fixed scope.
pub fn merge_scopes(ptz_enabled: bool, configured: &[String]) -> Vec<Scope> {
    let mut scopes: Vec<Scope> = default_scopes(ptz_enabled)
        .into_iter()
        .filter(|s| matches!(s.scope_def, ScopeDefinition::Fixed))
        .collect();

    scopes.extend(configured.iter().map(|item| Scope {
        scope_def: ScopeDefinition::Configurable,
        scope_item: item.clone(),
    }));

    scopes
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_merged_scopes_combine_fixed_and_configured() {
        let configured = vec!["onvif://www.onvif.org/name/Cam".to_string()];
        let merged = merge_scopes(true, &configured);

        // Fixed scopes are derived, never stored.
        assert!(
            merged
                .iter()
                .any(|s| s.scope_item == "onvif://www.onvif.org/type/ptz"
                    && matches!(s.scope_def, ScopeDefinition::Fixed))
        );
        assert!(
            merged
                .iter()
                .any(|s| s.scope_item == "onvif://www.onvif.org/name/Cam"
                    && matches!(s.scope_def, ScopeDefinition::Configurable))
        );
    }

    #[test]
    fn test_merged_scopes_omit_ptz_when_disabled() {
        let merged = merge_scopes(false, &[]);
        assert!(!merged.iter().any(|s| s.scope_item.ends_with("/type/ptz")));
    }

    #[test]
    fn test_merged_scopes_always_include_profile_streaming() {
        for ptz_enabled in [true, false] {
            let merged = merge_scopes(ptz_enabled, &[]);
            assert!(
                merged.iter().any(|s| {
                    s.scope_item == "onvif://www.onvif.org/Profile/Streaming"
                        && matches!(s.scope_def, ScopeDefinition::Fixed)
                }),
                "boot-derived scopes must retain Profile/Streaming (ptz={ptz_enabled})"
            );
        }
    }
}
