//! PTZ movement operations.
//!
//! This module handles movement-related PTZ operations:
//! - AbsoluteMove: Move to an absolute position
//! - RelativeMove: Move by a relative offset
//! - ContinuousMove: Start continuous movement
//! - Stop: Stop ongoing movement

use std::sync::Arc;

use crate::onvif::error::OnvifResult;
use crate::onvif::types::common::{PTZSpeed, PTZVector, Vector1D, Vector2D};
use crate::platform::{PTZControl, PtzPosition, PtzVelocity};

use crate::onvif::ptz::state::PTZStateManager;
use crate::onvif::ptz::validation;

/// Convert PTZVector to platform PtzPosition.
pub(super) fn vector_to_position(vector: &PTZVector) -> PtzPosition {
    let mut pos = PtzPosition::default();
    if let Some(pt) = &vector.pan_tilt {
        // ONVIF uses -1 to 1, platform uses degrees
        pos.pan = pt.x * 180.0;
        pos.tilt = pt.y * 90.0;
    }
    if let Some(z) = &vector.zoom {
        // ONVIF uses 0 to 1, platform uses 1 to 10
        pos.zoom = z.x * 9.0 + 1.0;
    }
    pos
}

/// Convert PTZSpeed to platform PtzVelocity.
pub(super) fn speed_to_velocity(speed: &PTZSpeed) -> PtzVelocity {
    let mut vel = PtzVelocity::STOP;
    if let Some(pt) = &speed.pan_tilt {
        vel.pan = pt.x;
        vel.tilt = pt.y;
    }
    if let Some(z) = &speed.zoom {
        vel.zoom = z.x;
    }
    vel
}

/// Handle AbsoluteMove request.
pub async fn absolute_move(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
    position: PTZVector,
) -> OnvifResult<()> {
    tracing::debug!("AbsoluteMove request for profile {}", profile_token);

    // Validate PTZ position values before calling FFI
    validation::validate_ptz_vector(&position)?;

    // Update state
    state.set_position(&position);
    state.set_moving(true, true);

    // Call platform if available
    if let Some(ptz) = ptz_control {
        let pos = vector_to_position(&position);
        ptz.move_to_position(pos).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!("PTZ move failed: {}", e))
        })?;
    }

    // Mark as stopped (instant move for stub)
    state.stop();

    Ok(())
}

/// Handle RelativeMove request.
pub async fn relative_move(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
    translation: PTZVector,
) -> OnvifResult<()> {
    tracing::debug!("RelativeMove request for profile {}", profile_token);

    // Validate PTZ translation values before applying
    validation::validate_ptz_velocity_vector(&translation)?;

    // Get current position and apply translation
    let current = state.get_position();
    let mut new_pos = current.clone();

    if let (Some(cur_pt), Some(trans_pt)) = (&current.pan_tilt, &translation.pan_tilt) {
        new_pos.pan_tilt = Some(Vector2D {
            x: (cur_pt.x + trans_pt.x).clamp(-1.0, 1.0),
            y: (cur_pt.y + trans_pt.y).clamp(-1.0, 1.0),
            space: cur_pt.space.clone(),
        });
    }

    if let (Some(cur_z), Some(trans_z)) = (&current.zoom, &translation.zoom) {
        new_pos.zoom = Some(Vector1D {
            x: (cur_z.x + trans_z.x).clamp(0.0, 1.0),
            space: cur_z.space.clone(),
        });
    }

    // Update state
    state.set_position(&new_pos);
    state.set_moving(true, true);

    // Call platform if available
    if let Some(ptz) = ptz_control {
        let pos = vector_to_position(&new_pos);
        ptz.move_to_position(pos).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!(
                "PTZ relative move failed: {}",
                e
            ))
        })?;
    }

    // Mark as stopped
    state.stop();

    Ok(())
}

/// Handle ContinuousMove request.
pub async fn continuous_move(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
    velocity: PTZSpeed,
) -> OnvifResult<()> {
    tracing::debug!("ContinuousMove request for profile {}", profile_token);

    // Validate PTZ velocity values before calling FFI
    // Convert PTZSpeed to PTZVector for validation
    let velocity_vector = PTZVector {
        pan_tilt: velocity.pan_tilt.clone(),
        zoom: velocity.zoom.clone(),
    };
    validation::validate_ptz_velocity_vector(&velocity_vector)?;

    // Set moving state
    let pan_tilt_moving = velocity
        .pan_tilt
        .as_ref()
        .is_some_and(|pt| pt.x != 0.0 || pt.y != 0.0);
    let zoom_moving = velocity.zoom.as_ref().is_some_and(|z| z.x != 0.0);
    state.set_moving(pan_tilt_moving, zoom_moving);

    // Call platform if available
    if let Some(ptz) = ptz_control {
        let vel = speed_to_velocity(&velocity);
        ptz.continuous_move(vel).await.map_err(|e| {
            state.stop();
            crate::onvif::error::OnvifError::HardwareFailure(format!(
                "PTZ continuous move failed: {}",
                e
            ))
        })?;
    }

    Ok(())
}

/// Handle Stop request.
pub async fn stop(
    state: &PTZStateManager,
    ptz_control: &Option<Arc<dyn PTZControl>>,
    profile_token: &str,
    stop_pan_tilt: bool,
    stop_zoom: bool,
) -> OnvifResult<()> {
    tracing::debug!("Stop request for profile {}", profile_token);

    // Update state
    if stop_pan_tilt && stop_zoom {
        state.stop();
    } else {
        let mut pan_tilt_moving = state.is_moving();
        let mut zoom_moving = state.is_moving();

        if stop_pan_tilt {
            pan_tilt_moving = false;
        }
        if stop_zoom {
            zoom_moving = false;
        }
        state.set_moving(pan_tilt_moving, zoom_moving);
    }

    // Call platform if available
    if let Some(ptz) = ptz_control {
        ptz.stop().await.map_err(|e| {
            crate::onvif::error::OnvifError::HardwareFailure(format!("PTZ stop failed: {}", e))
        })?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_vector_to_position() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: 0.5,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        };

        let pos = vector_to_position(&vector);
        assert_eq!(pos.pan, 90.0);
        assert_eq!(pos.tilt, 45.0);
        assert_eq!(pos.zoom, 5.5);
    }

    #[tokio::test]
    async fn test_speed_to_velocity() {
        let speed = PTZSpeed {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: 0.3,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.7,
                space: None,
            }),
        };

        let vel = speed_to_velocity(&speed);
        assert_eq!(vel.pan, 0.5);
        assert_eq!(vel.tilt, 0.3);
        assert_eq!(vel.zoom, 0.7);
    }
}
