//! PTZ preset operations.
//!
//! This module handles preset-related PTZ operations:
//! - GetPresets: Get all presets for a profile
//! - SetPreset: Create or update a preset
//! - GotoPreset: Move to a preset position
//! - RemovePreset: Delete a preset

use std::sync::Arc;

use crate::onvif::error::OnvifResult;
use crate::onvif::types::ptz::{GetPresetsResponse, SetPresetResponse};
use crate::platform::PTZControl;

use crate::onvif::ptz::state::PTZStateManager;

/// Handle GetPresets request.
pub fn get_presets(
    state: &PTZStateManager,
    profile_token: &str,
) -> OnvifResult<GetPresetsResponse> {
    tracing::debug!("GetPresets request for profile {}", profile_token);

    Ok(GetPresetsResponse {
        presets: state.get_presets(),
    })
}

/// Handle SetPreset request.
#[allow(unused_variables)]
pub fn set_preset(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
    preset_name: Option<String>,
    preset_token: Option<String>,
) -> OnvifResult<SetPresetResponse> {
    tracing::debug!("SetPreset request for profile {}", profile_token);

    let name = preset_name.unwrap_or_else(|| "Unnamed".to_string());
    let preset_id = state.set_preset(name, preset_token)?;

    Ok(SetPresetResponse {
        preset_token: preset_id,
    })
}

/// Handle GotoPreset request.
pub async fn goto_preset(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
    preset_token: String,
) -> OnvifResult<()> {
    tracing::debug!(
        "GotoPreset request for profile {}, preset {}",
        profile_token,
        preset_token
    );

    // Set moving state
    state.set_moving(true, true);

    // Go to preset position
    state.goto_preset(&preset_token)?;

    // Call platform if available
    if let Some(ptz) = ptz_control {
        ptz.goto_preset(&preset_token).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!(
                "PTZ goto preset failed: {}",
                e
            ))
        })?;
    }

    state.stop();

    Ok(())
}

/// Handle RemovePreset request.
pub fn remove_preset(
    state: &PTZStateManager,
    profile_token: &str,
    preset_token: String,
) -> OnvifResult<()> {
    tracing::debug!(
        "RemovePreset request for profile {}, preset {}",
        profile_token,
        preset_token
    );

    state.remove_preset(&preset_token)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::ptz::types::{PTZVector, Vector1D, Vector2D};
    use std::sync::Arc;

    fn create_test_state() -> Arc<PTZStateManager> {
        Arc::new(PTZStateManager::new())
    }

    #[test]
    fn test_get_presets_empty() {
        let state = create_test_state();

        let response = get_presets(&state, "Profile1").unwrap();

        assert!(response.presets.is_empty());
    }

    #[test]
    fn test_set_preset() {
        let state = create_test_state();

        let response = set_preset(
            &state,
            &None,
            "Profile1",
            Some("TestPreset".to_string()),
            None,
        )
        .unwrap();

        // Should return a token
        assert!(!response.preset_token.is_empty());
    }

    #[tokio::test]
    async fn test_goto_preset() {
        let state = create_test_state();

        // Move to a position and create preset
        state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.6,
                y: 0.7,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.8,
                space: None,
            }),
        });

        let set_response = set_preset(
            &state,
            &None,
            "Profile1",
            Some("GotoTest".to_string()),
            None,
        )
        .unwrap();

        // Move somewhere else
        state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.0,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        });

        // Go to preset
        goto_preset(&state, &None, "Profile1", set_response.preset_token)
            .await
            .unwrap();

        // Verify position
        let pos = state.get_position();
        let pt = pos.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.6);
        assert_eq!(pt.y, 0.7);
    }

    #[test]
    fn test_remove_preset() {
        let state = create_test_state();

        // Create preset
        let set_response = set_preset(
            &state,
            &None,
            "Profile1",
            Some("ToRemove".to_string()),
            None,
        )
        .unwrap();

        // Remove it
        remove_preset(&state, "Profile1", set_response.preset_token.clone()).unwrap();

        // Verify it's gone
        let response = get_presets(&state, "Profile1").unwrap();
        assert!(response.presets.is_empty());
    }

    #[test]
    fn test_remove_preset_invalid_token() {
        let state = create_test_state();

        let result = remove_preset(&state, "Profile1", "NonExistentPreset".to_string());

        assert!(result.is_err());
    }
}
