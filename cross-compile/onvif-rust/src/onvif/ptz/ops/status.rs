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

/// Handle GetStatus request.
pub fn get_status(state: &PTZStateManager, profile_token: &str) -> OnvifResult<GetStatusResponse> {
    tracing::debug!("GetStatus request for profile {}", profile_token);

    Ok(GetStatusResponse {
        ptz_status: state.get_status(),
    })
}

/// Handle GotoHomePosition request.
pub async fn goto_home_position(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
) -> OnvifResult<()> {
    tracing::debug!("GotoHomePosition request for profile {}", profile_token);

    // Get home position and move there
    state.set_moving(true, true);
    state.goto_home();

    // Call platform if available
    if let Some(ptz) = ptz_control {
        let home_pos = state.get_position();
        let pos = vector_to_position(&home_pos);
        ptz.move_to_position(pos).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!("PTZ goto home failed: {}", e))
        })?;
    }

    state.stop();

    Ok(())
}

/// Handle SetHomePosition request.
pub fn set_home_position(state: &PTZStateManager, profile_token: &str) -> OnvifResult<()> {
    tracing::debug!("SetHomePosition request for profile {}", profile_token);

    state.set_home_position();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{PTZVector, Vector1D, Vector2D};
    use std::sync::Arc;

    fn create_test_state() -> Arc<PTZStateManager> {
        Arc::new(PTZStateManager::new())
    }

    #[test]
    fn test_get_status() {
        let state = create_test_state();

        let response = get_status(&state, "Profile1").unwrap();

        // Should have position
        assert!(response.ptz_status.position.is_some());
        // Should have move status
        assert!(response.ptz_status.move_status.is_some());
        // Should have UTC time
        assert!(!response.ptz_status.utc_time.is_empty());
    }

    #[tokio::test]
    async fn test_goto_home_position() {
        let state = create_test_state();

        goto_home_position(&state, &None, "Profile1").await.unwrap();
    }

    #[test]
    fn test_set_home_position() {
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
