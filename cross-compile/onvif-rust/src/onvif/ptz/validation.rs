//! PTZ value validation.
//!
//! This module provides validation for PTZ position and velocity values
//! to ensure they are within hardware bounds before calling FFI functions.

use crate::onvif::common::validate_range_f32;
use crate::onvif::error::{OnvifError, OnvifResult};
use crate::onvif::types::common::PTZVector;

/// PTZ-specific range validators.
///
/// These wrappers provide PTZ-specific subcodes for validation errors.
fn validate_pan(value: f32) -> OnvifResult<()> {
    validate_range_f32(value, -1.0, 1.0, "Pan").map_err(|e| match e {
        OnvifError::InvalidArgVal { subcode, reason } => OnvifError::InvalidArgVal {
            subcode: format!("InvalidPanValue:{}", subcode),
            reason,
        },
        other => other,
    })
}

/// Validate tilt position range (-1.0 to 1.0).
fn validate_tilt(value: f32) -> OnvifResult<()> {
    validate_range_f32(value, -1.0, 1.0, "Tilt").map_err(|e| match e {
        OnvifError::InvalidArgVal { subcode, reason } => OnvifError::InvalidArgVal {
            subcode: format!("InvalidTiltValue:{}", subcode),
            reason,
        },
        other => other,
    })
}

/// Validate zoom position range (0.0 to 1.0).
fn validate_zoom_position(value: f32) -> OnvifResult<()> {
    validate_range_f32(value, 0.0, 1.0, "Zoom").map_err(|e| match e {
        OnvifError::InvalidArgVal { subcode, reason } => OnvifError::InvalidArgVal {
            subcode: format!("InvalidZoomValue:{}", subcode),
            reason,
        },
        other => other,
    })
}

/// Validate zoom velocity range (-1.0 to 1.0, allows negative for direction).
fn validate_zoom_velocity(value: f32) -> OnvifResult<()> {
    validate_range_f32(value, -1.0, 1.0, "Zoom velocity").map_err(|e| match e {
        OnvifError::InvalidArgVal { subcode, reason } => OnvifError::InvalidArgVal {
            subcode: format!("InvalidZoomVelocity:{}", subcode),
            reason,
        },
        other => other,
    })
}

/// Validate pan velocity range (-1.0 to 1.0).
fn validate_pan_velocity(value: f32) -> OnvifResult<()> {
    validate_range_f32(value, -1.0, 1.0, "Pan velocity").map_err(|e| match e {
        OnvifError::InvalidArgVal { subcode, reason } => OnvifError::InvalidArgVal {
            subcode: format!("InvalidPanVelocity:{}", subcode),
            reason,
        },
        other => other,
    })
}

/// Validate tilt velocity range (-1.0 to 1.0).
fn validate_tilt_velocity(value: f32) -> OnvifResult<()> {
    validate_range_f32(value, -1.0, 1.0, "Tilt velocity").map_err(|e| match e {
        OnvifError::InvalidArgVal { subcode, reason } => OnvifError::InvalidArgVal {
            subcode: format!("InvalidTiltVelocity:{}", subcode),
            reason,
        },
        other => other,
    })
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
    validate_pan(pan)?;
    validate_tilt(tilt)?;
    validate_zoom_position(zoom)?;
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
        validate_pan(pan_tilt.x)?;
        validate_tilt(pan_tilt.y)?;
    }

    if let Some(zoom) = &vector.zoom {
        validate_zoom_position(zoom.x)?;
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
        validate_pan_velocity(pan_tilt.x)?;
        validate_tilt_velocity(pan_tilt.y)?;
    }

    if let Some(zoom) = &vector.zoom {
        // For velocity, zoom can be negative (unlike position)
        validate_zoom_velocity(zoom.x)?;
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
