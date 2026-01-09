//! PTZ value validation.
//!
//! This module provides validation for PTZ position and velocity values
//! to ensure they are within hardware bounds before calling FFI functions.

use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::PTZVector;

/// Create a validation error with the given subcode and reason
fn create_validation_error(subcode: &str, reason: String) -> OnvifError {
    OnvifError::InvalidArgVal {
        subcode: subcode.to_string(),
        reason,
    }
}

/// Validate that a value is within the specified range
fn validate_range(
    value: f32,
    min: f32,
    max: f32,
    field_name: &str,
    error_type: &str,
) -> OnvifResult<()> {
    if !(min..=max).contains(&value) {
        return Err(create_validation_error(
            error_type,
            format!(
                "{} value {} out of range ({} to {})",
                field_name, value, min, max
            ),
        ));
    }
    Ok(())
}

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
    validate_range(pan, -1.0, 1.0, "Pan", "InvalidPanValue")?;
    validate_range(tilt, -1.0, 1.0, "Tilt", "InvalidTiltValue")?;
    validate_range(zoom, 0.0, 1.0, "Zoom", "InvalidZoomValue")?;
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
    validate_range(pan, -1.0, 1.0, "Pan velocity", "InvalidPanVelocity")?;
    validate_range(tilt, -1.0, 1.0, "Tilt velocity", "InvalidTiltVelocity")?;
    validate_range(zoom, -1.0, 1.0, "Zoom velocity", "InvalidZoomVelocity")?;
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
        validate_range(pan_tilt.x, -1.0, 1.0, "Pan", "InvalidPanValue")?;
        validate_range(pan_tilt.y, -1.0, 1.0, "Tilt", "InvalidTiltValue")?;
    }

    if let Some(zoom) = &vector.zoom {
        validate_range(zoom.x, 0.0, 1.0, "Zoom", "InvalidZoomValue")?;
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
        validate_range(pan_tilt.x, -1.0, 1.0, "Pan velocity", "InvalidPanVelocity")?;
        validate_range(
            pan_tilt.y,
            -1.0,
            1.0,
            "Tilt velocity",
            "InvalidTiltVelocity",
        )?;
    }

    if let Some(zoom) = &vector.zoom {
        // For velocity, zoom can be negative (unlike position)
        validate_range(zoom.x, -1.0, 1.0, "Zoom velocity", "InvalidZoomVelocity")?;
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

    #[test]
    fn test_validate_ptz_vector_missing_pan_tilt() {
        let vector = PTZVector {
            pan_tilt: None,
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        };
        assert!(validate_ptz_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_vector_missing_zoom() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: -0.5,
                space: None,
            }),
            zoom: None,
        };
        assert!(validate_ptz_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_vector_empty() {
        let vector = PTZVector {
            pan_tilt: None,
            zoom: None,
        };
        assert!(validate_ptz_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_velocity_vector_missing_pan_tilt() {
        let vector = PTZVector {
            pan_tilt: None,
            zoom: Some(Vector1D {
                x: -0.5,
                space: None,
            }),
        };
        assert!(validate_ptz_velocity_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_velocity_vector_missing_zoom() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: -0.5,
                space: None,
            }),
            zoom: None,
        };
        assert!(validate_ptz_velocity_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_velocity_vector_empty() {
        let vector = PTZVector {
            pan_tilt: None,
            zoom: None,
        };
        assert!(validate_ptz_velocity_vector(&vector).is_ok());
    }

    #[test]
    fn test_validate_ptz_position_boundary_values() {
        // Exact boundaries
        assert!(validate_ptz_position(-1.0, -1.0, 0.0).is_ok());
        assert!(validate_ptz_position(1.0, 1.0, 1.0).is_ok());
        assert!(validate_ptz_position(0.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn test_validate_ptz_velocity_boundary_values() {
        // Exact boundaries
        assert!(validate_ptz_velocity(-1.0, -1.0, -1.0).is_ok());
        assert!(validate_ptz_velocity(1.0, 1.0, 1.0).is_ok());
        assert!(validate_ptz_velocity(0.0, 0.0, 0.0).is_ok());
    }

    #[test]
    fn test_validate_ptz_vector_zoom_boundary() {
        let vector_min = PTZVector {
            pan_tilt: None,
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        };
        assert!(validate_ptz_vector(&vector_min).is_ok());

        let vector_max = PTZVector {
            pan_tilt: None,
            zoom: Some(Vector1D {
                x: 1.0,
                space: None,
            }),
        };
        assert!(validate_ptz_vector(&vector_max).is_ok());

        let vector_invalid = PTZVector {
            pan_tilt: None,
            zoom: Some(Vector1D {
                x: -0.1,
                space: None,
            }),
        };
        assert!(validate_ptz_vector(&vector_invalid).is_err());
    }
}
