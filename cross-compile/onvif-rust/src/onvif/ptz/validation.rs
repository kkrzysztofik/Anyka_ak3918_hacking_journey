//! PTZ value validation.
//!
//! This module provides validation for PTZ position and velocity values
//! to ensure they are within hardware bounds before calling FFI functions.

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::PTZVector;

/// Validate PTZ position values.
///
/// Ensures pan, tilt, and zoom are within valid ranges:
/// - Pan: -180.0 to 180.0 degrees (normalized: -1.0 to 1.0)
/// - Tilt: -90.0 to 90.0 degrees (normalized: -1.0 to 1.0)
/// - Zoom: 0.0 to 1.0 (normalized)
///
/// # Arguments
///
/// * `pan` - Pan position (-1.0 to 1.0)
/// * `tilt` - Tilt position (-1.0 to 1.0)
/// * `zoom` - Zoom level (0.0 to 1.0)
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if out of range.
pub fn validate_ptz_position(pan: f32, tilt: f32, zoom: f32) -> OnvifResult<()> {
    if !(-1.0..=1.0).contains(&pan) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidPanValue".to_string(),
            reason: format!("Pan value {} out of range (-1.0 to 1.0)", pan),
        });
    }

    if !(-1.0..=1.0).contains(&tilt) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidTiltValue".to_string(),
            reason: format!("Tilt value {} out of range (-1.0 to 1.0)", tilt),
        });
    }

    if !(0.0..=1.0).contains(&zoom) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidZoomValue".to_string(),
            reason: format!("Zoom value {} out of range (0.0 to 1.0)", zoom),
        });
    }

    Ok(())
}

/// Validate PTZ velocity values.
///
/// Ensures pan, tilt, and zoom velocities are within valid ranges:
/// - Pan velocity: -1.0 to 1.0
/// - Tilt velocity: -1.0 to 1.0
/// - Zoom velocity: -1.0 to 1.0
///
/// # Arguments
///
/// * `pan` - Pan velocity (-1.0 to 1.0)
/// * `tilt` - Tilt velocity (-1.0 to 1.0)
/// * `zoom` - Zoom velocity (-1.0 to 1.0)
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if out of range.
#[allow(dead_code)] // Kept for potential future use, currently tested but not used in production
pub fn validate_ptz_velocity(pan: f32, tilt: f32, zoom: f32) -> OnvifResult<()> {
    if !(-1.0..=1.0).contains(&pan) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidPanVelocity".to_string(),
            reason: format!("Pan velocity {} out of range (-1.0 to 1.0)", pan),
        });
    }

    if !(-1.0..=1.0).contains(&tilt) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidTiltVelocity".to_string(),
            reason: format!("Tilt velocity {} out of range (-1.0 to 1.0)", tilt),
        });
    }

    if !(-1.0..=1.0).contains(&zoom) {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidZoomVelocity".to_string(),
            reason: format!("Zoom velocity {} out of range (-1.0 to 1.0)", zoom),
        });
    }

    Ok(())
}

/// Validate a PTZ vector (position or translation).
///
/// Validates all components of a PTZVector if present.
///
/// # Arguments
///
/// * `vector` - The PTZ vector to validate
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if any component is out of range.
pub fn validate_ptz_vector(vector: &PTZVector) -> OnvifResult<()> {
    if let Some(pan_tilt) = &vector.pan_tilt {
        if !(-1.0..=1.0).contains(&pan_tilt.x) {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidPanValue".to_string(),
                reason: format!("Pan value {} out of range (-1.0 to 1.0)", pan_tilt.x),
            });
        }
        if !(-1.0..=1.0).contains(&pan_tilt.y) {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidTiltValue".to_string(),
                reason: format!("Tilt value {} out of range (-1.0 to 1.0)", pan_tilt.y),
            });
        }
    }

    if let Some(zoom) = &vector.zoom
        && !(0.0..=1.0).contains(&zoom.x)
    {
        return Err(OnvifError::InvalidArgVal {
            subcode: "InvalidZoomValue".to_string(),
            reason: format!("Zoom value {} out of range (0.0 to 1.0)", zoom.x),
        });
    }

    Ok(())
}

/// Validate a PTZ velocity vector.
///
/// Validates all components of a PTZVector used for velocity/translation.
/// For velocity, zoom can be negative (unlike position).
///
/// # Arguments
///
/// * `vector` - The PTZ velocity vector to validate
///
/// # Returns
///
/// `Ok(())` if valid, or `OnvifError::InvalidArgVal` if any component is out of range.
pub fn validate_ptz_velocity_vector(vector: &PTZVector) -> OnvifResult<()> {
    if let Some(pan_tilt) = &vector.pan_tilt {
        if !(-1.0..=1.0).contains(&pan_tilt.x) {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidPanVelocity".to_string(),
                reason: format!("Pan velocity {} out of range (-1.0 to 1.0)", pan_tilt.x),
            });
        }
        if !(-1.0..=1.0).contains(&pan_tilt.y) {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidTiltVelocity".to_string(),
                reason: format!("Tilt velocity {} out of range (-1.0 to 1.0)", pan_tilt.y),
            });
        }
    }

    if let Some(zoom) = &vector.zoom {
        // For velocity, zoom can be negative (unlike position)
        if !(-1.0..=1.0).contains(&zoom.x) {
            return Err(OnvifError::InvalidArgVal {
                subcode: "InvalidZoomVelocity".to_string(),
                reason: format!("Zoom velocity {} out of range (-1.0 to 1.0)", zoom.x),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{PTZVector, Vector1D, Vector2D};

    #[test]
    fn test_validate_ptz_position_valid() {
        assert!(validate_ptz_position(0.0, 0.0, 0.5).is_ok());
        assert!(validate_ptz_position(-1.0, -1.0, 0.0).is_ok());
        assert!(validate_ptz_position(1.0, 1.0, 1.0).is_ok());
        assert!(validate_ptz_position(0.5, -0.5, 0.75).is_ok());
    }

    #[test]
    fn test_validate_ptz_position_invalid_pan() {
        assert!(validate_ptz_position(-1.1, 0.0, 0.5).is_err());
        assert!(validate_ptz_position(1.1, 0.0, 0.5).is_err());
    }

    #[test]
    fn test_validate_ptz_position_invalid_tilt() {
        assert!(validate_ptz_position(0.0, -1.1, 0.5).is_err());
        assert!(validate_ptz_position(0.0, 1.1, 0.5).is_err());
    }

    #[test]
    fn test_validate_ptz_position_invalid_zoom() {
        assert!(validate_ptz_position(0.0, 0.0, -0.1).is_err());
        assert!(validate_ptz_position(0.0, 0.0, 1.1).is_err());
    }

    #[test]
    fn test_validate_ptz_velocity_valid() {
        assert!(validate_ptz_velocity(0.0, 0.0, 0.0).is_ok());
        assert!(validate_ptz_velocity(-1.0, -1.0, -1.0).is_ok());
        assert!(validate_ptz_velocity(1.0, 1.0, 1.0).is_ok());
        assert!(validate_ptz_velocity(0.5, -0.5, 0.75).is_ok());
    }

    #[test]
    fn test_validate_ptz_velocity_invalid() {
        assert!(validate_ptz_velocity(-1.1, 0.0, 0.0).is_err());
        assert!(validate_ptz_velocity(1.1, 0.0, 0.0).is_err());
        assert!(validate_ptz_velocity(0.0, -1.1, 0.0).is_err());
        assert!(validate_ptz_velocity(0.0, 1.1, 0.0).is_err());
        assert!(validate_ptz_velocity(0.0, 0.0, -1.1).is_err());
        assert!(validate_ptz_velocity(0.0, 0.0, 1.1).is_err());
    }

    #[test]
    fn test_validate_ptz_vector_valid() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: -0.5,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.75,
                space: None,
            }),
        };
        assert!(validate_ptz_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_vector_invalid() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 1.5,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        };
        assert!(validate_ptz_vector(&vector).is_err());
    }

    #[test]
    fn test_validate_ptz_velocity_vector_valid() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: -0.5,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: -0.5, // Negative zoom allowed for velocity
                space: None,
            }),
        };
        assert!(validate_ptz_velocity_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_velocity_vector_invalid() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 1.5,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        };
        assert!(validate_ptz_velocity_vector(&vector).is_err());
    }
}
