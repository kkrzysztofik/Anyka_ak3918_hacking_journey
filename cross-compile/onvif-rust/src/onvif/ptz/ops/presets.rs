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
///
/// Presets are stored globally and shared across all profiles. The
/// `profile_token` is logged for diagnostics but does not filter the
/// returned preset list.
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
///
/// `ptz_control` is accepted but not yet used — hardware preset storage
/// (writing preset positions to the motor controller's non-volatile memory)
/// will be added when the platform PTZ driver supports it.
/// TODO(ptz-hw): Forward preset to `ptz_control` for on-device storage.
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
///
/// Presets live in the ONVIF `PresetStore`. Drive the motors with a normal
/// absolute move to the stored position — the platform layer no longer keeps
/// its own preset map (see the position-tracking design §6c).
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

    let preset = state.get_preset(&preset_token)?;
    let position = preset.ptz_position.ok_or_else(|| {
        crate::onvif::error::OnvifError::Internal(
            "Preset exists but has no stored position".to_string(),
        )
    })?;

    // Mark moving before hardware accepts the command — do NOT write the target
    // yet (truth comes from the platform after accept / completion).
    state.set_moving(true, true);

    if let Some(ptz) = ptz_control {
        let pos = super::movement::vector_to_position(&position);
        ptz.move_to_position(pos).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!(
                "PTZ goto preset failed: {}",
                e
            ))
        })?;
        super::movement::sync_position_from_platform(state, ptz).await;
        // Leave moving=true; Stop / completion path clears it. GetStatus reads live.
    } else {
        // No platform: update the cached position only (unit-test / stub path).
        state.set_position(&position);
        state.stop();
    }

    Ok(())
}

/// Handle RemovePreset request.
///
/// `profile_token` is logged for diagnostics but not used for lookup because
/// presets are stored globally (not per-profile) in the current implementation.
/// TODO(ptz-profile): Scope presets per profile if multi-profile PTZ is needed.
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

    #[tokio::test]
    async fn test_goto_preset_drives_platform_absolute_move() {
        use crate::platform::common::traits::{MockPTZControl, PtzPosition};
        use mockall::predicate::eq;

        let state = create_test_state();
        state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        });
        let token = set_preset(
            &state,
            &None,
            "Profile1",
            Some("HardwareGoto".to_string()),
            None,
        )
        .unwrap()
        .preset_token;

        let mut mock = MockPTZControl::new();
        mock.expect_goto_preset().times(0);
        mock.expect_move_to_position()
            .times(1)
            .with(eq(PtzPosition::new(90.0, 0.0, 1.0)))
            .returning(|_| Ok(()));
        mock.expect_get_position()
            .returning(|| Ok(PtzPosition::new(90.0, 0.0, 1.0)));

        let ptz: Option<Arc<dyn PTZControl>> = Some(Arc::new(mock));
        goto_preset(&state, &ptz, "Profile1", token).await.unwrap();
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
