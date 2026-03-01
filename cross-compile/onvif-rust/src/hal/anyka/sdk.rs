//! Hardware type definitions for Anyka AK3918 platform.
//!
//! Core types used by the HAL layer: `VideoDevice`, `Resolution`,
//! `PtzMotor`, and `PtzDirection`.

/// Video device identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VideoDevice(pub u32);

impl VideoDevice {
    /// Primary video device.
    pub const DEV0: VideoDevice = VideoDevice(0);
}

/// Video resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Resolution {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Resolution {
    /// Create a new resolution.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Common resolution: 1920x1080 (1080p).
    pub const HD1080: Resolution = Resolution {
        width: 1920,
        height: 1080,
    };
    /// Common resolution: 1280x720 (720p).
    pub const HD720: Resolution = Resolution {
        width: 1280,
        height: 720,
    };
    /// Common resolution: 640x480 (VGA).
    pub const VGA: Resolution = Resolution {
        width: 640,
        height: 480,
    };
    /// Common resolution: 320x240 (QVGA).
    pub const QVGA: Resolution = Resolution {
        width: 320,
        height: 240,
    };
}

/// PTZ device (pan/tilt motor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtzMotor {
    /// Horizontal (pan) motor.
    Horizontal,
    /// Vertical (tilt) motor.
    Vertical,
}

impl PtzMotor {
    /// Convert to FFI device ID.
    pub fn to_device_id(self) -> i32 {
        match self {
            PtzMotor::Horizontal => 0,
            PtzMotor::Vertical => 1,
        }
    }
}

/// PTZ movement direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtzDirection {
    /// Move left.
    Left,
    /// Move right.
    Right,
    /// Move up.
    Up,
    /// Move down.
    Down,
}

impl PtzDirection {
    /// Convert to FFI direction code.
    pub fn to_direction_code(self) -> i32 {
        match self {
            PtzDirection::Left => 1,
            PtzDirection::Right => 2,
            PtzDirection::Up => 3,
            PtzDirection::Down => 4,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolution_new() {
        let res = Resolution::new(1920, 1080);
        assert_eq!(res.width, 1920);
        assert_eq!(res.height, 1080);
    }

    #[test]
    fn test_resolution_constants() {
        assert_eq!(Resolution::HD1080.width, 1920);
        assert_eq!(Resolution::HD1080.height, 1080);
        assert_eq!(Resolution::HD720.width, 1280);
        assert_eq!(Resolution::HD720.height, 720);
        assert_eq!(Resolution::VGA.width, 640);
        assert_eq!(Resolution::VGA.height, 480);
        assert_eq!(Resolution::QVGA.width, 320);
        assert_eq!(Resolution::QVGA.height, 240);
    }

    #[test]
    fn test_ptz_direction_codes() {
        assert_eq!(PtzDirection::Left.to_direction_code(), 1);
        assert_eq!(PtzDirection::Right.to_direction_code(), 2);
        assert_eq!(PtzDirection::Up.to_direction_code(), 3);
        assert_eq!(PtzDirection::Down.to_direction_code(), 4);
    }

    #[test]
    fn test_ptz_motor_to_device_id() {
        assert_eq!(PtzMotor::Horizontal.to_device_id(), 0);
        assert_eq!(PtzMotor::Vertical.to_device_id(), 1);
    }

    #[test]
    fn test_video_device_constants() {
        assert_eq!(VideoDevice::DEV0.0, 0);
    }
}
