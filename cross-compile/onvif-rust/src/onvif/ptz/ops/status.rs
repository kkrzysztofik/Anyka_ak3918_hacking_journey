//! PTZ status and home position operations.
//!
//! This module handles status and home position-related PTZ operations:
//! - GetStatus: Get current PTZ status
//! - GotoHomePosition: Move to home position
//! - SetHomePosition: Set current position as home

use std::sync::Arc;

use crate::onvif::error::OnvifResult;
use crate::onvif::types::ptz::GetStatusResponse;
use crate::platform::PTZControl;

use crate::onvif::ptz::ops::movement::vector_to_position;
use crate::onvif::ptz::state::PTZStateManager;

/// Handle the ONVIF PTZ GetStatus request.
///
/// Returns the current PTZ status including pan/tilt/zoom position,
/// movement status, and a UTC timestamp.
///
/// # Arguments
///
/// * `state` - The PTZ state manager holding current position and movement state
/// * `profile_token` - The media profile token identifying the PTZ configuration
///
/// # Returns
///
/// `GetStatusResponse` containing the current `PTZStatus` with position,
/// move status, and UTC time.
///
/// # Errors
///
/// This handler is infallible under normal operation.
pub async fn get_status(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
) -> OnvifResult<GetStatusResponse> {
    tracing::debug!("GetStatus request for profile {}", profile_token);

    // Prefer live hardware position over stale dead-reckoning state.
    if let Some(ptz) = ptz_control {
        crate::onvif::ptz::ops::movement::sync_position_from_platform(state, ptz).await;
    }

    Ok(GetStatusResponse {
        ptz_status: state.get_status(),
    })
}

/// Handle the ONVIF PTZ GotoHomePosition request.
///
/// Moves the PTZ unit to its stored home position. If a platform PTZ
/// control is available, the movement is also sent to the hardware.
///
/// # Arguments
///
/// * `state` - The PTZ state manager holding the home position and movement state
/// * `ptz_control` - Optional platform PTZ control for issuing hardware commands
/// * `profile_token` - The media profile token identifying the PTZ configuration
///
/// # Returns
///
/// `()` on success after the PTZ has reached the home position.
///
/// # Errors
///
/// Returns `HardwareFailure` if the platform PTZ control reports a movement error.
pub async fn goto_home_position(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
) -> OnvifResult<()> {
    tracing::debug!("GotoHomePosition request for profile {}", profile_token);

    state.set_moving(true, true);
    state.goto_home();

    if let Some(ptz) = ptz_control {
        // Re-establish the physical origin first: this is the drift reset, and it makes
        // the dead-reckoned leg below the most accurate move the system can make.
        ptz.home().await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!("PTZ re-home failed: {}", e))
        })?;
        let pos = vector_to_position(&state.get_position());
        ptz.move_to_position(pos).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!("PTZ goto home failed: {}", e))
        })?;
        crate::onvif::ptz::ops::movement::sync_position_from_platform(state, ptz).await;
    } else {
        state.stop();
        return Err(crate::onvif::error::OnvifError::ActionNotSupported(
            "PTZ is not available on this device".to_string(),
        ));
    }

    Ok(())
}

/// Handle the ONVIF PTZ SetHomePosition request.
///
/// Saves the current PTZ position as the home position so that future
/// `GotoHomePosition` calls return to this location.
///
/// # Arguments
///
/// * `state` - The PTZ state manager where the home position is stored
/// * `profile_token` - The media profile token identifying the PTZ configuration
///
/// # Returns
///
/// `()` on success after the home position has been saved.
///
/// # Errors
///
/// This handler is infallible under normal operation.
pub fn set_home_position(state: &PTZStateManager, profile_token: &str) -> OnvifResult<()> {
    tracing::debug!("SetHomePosition request for profile {}", profile_token);

    state.set_home_position();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{PTZVector, Vector1D, Vector2D};
    use crate::platform::PtzPosition;
    use crate::platform::common::traits::MockPTZControl;
    use std::sync::Arc;

    fn create_test_state() -> Arc<PTZStateManager> {
        Arc::new(PTZStateManager::new())
    }

    #[tokio::test]
    async fn test_goto_home_position_rehomes_before_moving() {
        let state = create_test_state();
        let mut mock = MockPTZControl::new();
        let mut seq = mockall::Sequence::new();
        mock.expect_home()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|| Ok(()));
        mock.expect_move_to_position()
            .times(1)
            .in_sequence(&mut seq)
            .returning(|_| Ok(()));
        mock.expect_get_position()
            .returning(|| Ok(PtzPosition::HOME));

        let ptz: Option<Arc<dyn crate::platform::PTZControl>> = Some(Arc::new(mock));
        goto_home_position(&state, &ptz, "Profile1").await.unwrap();
    }

    #[tokio::test]
    async fn test_status_get_status_returns_position_and_move_status() {
        let state = create_test_state();

        let response = get_status(&state, &None, "Profile1").await.unwrap();

        // Should have position
        assert!(response.ptz_status.position.is_some());
        // Should have move status
        assert!(response.ptz_status.move_status.is_some());
        // Should have UTC time
        assert!(!response.ptz_status.utc_time.is_empty());
    }

    #[tokio::test]
    async fn test_status_goto_home_without_hardware_faults() {
        let state = create_test_state();

        let result = goto_home_position(&state, &None, "Profile1").await;
        assert!(matches!(
            result,
            Err(crate::onvif::error::OnvifError::ActionNotSupported(_))
        ));
        assert!(!state.is_moving());
    }

    #[test]
    fn test_status_set_home_position_saves_current_position() {
        let state = create_test_state();

        // Move to a position
        state.set_position(&PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.8,
                y: 0.4,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        });

        // Set as home
        set_home_position(&state, "Profile1").unwrap();

        // Verify home was set
        let home = state.get_home_position();
        let pt = home.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.8);
        assert_eq!(pt.y, 0.4);
    }
}
