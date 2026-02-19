//! Native Rust PTZ driver for Qiwen/Anycloud kernel motor devices.
//!
//! Talks directly to `/dev/ak-motor0` (horizontal) and `/dev/ak-motor1` (vertical)
//! via ioctl/read. ABI matches [qiwen/anycloud39ev300 SDK kernel ak_motor.h](https://github.com/...).
//! No C PTZ library dependency.

use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::Mutex;

use libc::{c_int, c_uint};
use tracing;

use crate::platform::PlatformError;
use crate::platform::PlatformResult;

// --- Kernel ABI (Qiwen anycloud39ev300 SDK kernel include/plat-anyka/ak_motor.h) ---

const AK_MOTOR_IOC_MAGIC: c_uint = b'm' as c_uint;

#[allow(clippy::identity_op)]
const fn _iow(type_: c_uint, nr: c_uint, size: c_uint) -> c_uint {
    const IOC_WRITE: c_uint = 1;
    const IOC_NRSHIFT: c_uint = 0;
    const IOC_TYPESHIFT: c_uint = 8;
    const IOC_SIZESHIFT: c_uint = 16;
    const IOC_DIRSHIFT: c_uint = 30;
    (IOC_WRITE << IOC_DIRSHIFT) | ((type_ << IOC_TYPESHIFT) & 0xff00) | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)
}

#[allow(clippy::identity_op)]
const fn _ior(type_: c_uint, nr: c_uint, size: c_uint) -> c_uint {
    const IOC_READ: c_uint = 2;
    const IOC_NRSHIFT: c_uint = 0;
    const IOC_TYPESHIFT: c_uint = 8;
    const IOC_SIZESHIFT: c_uint = 16;
    const IOC_DIRSHIFT: c_uint = 30;
    (IOC_READ << IOC_DIRSHIFT) | ((type_ << IOC_TYPESHIFT) & 0xff00) | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)
}

const SIZE_INT: c_uint = std::mem::size_of::<c_int>() as c_uint;

#[allow(dead_code)] // Kernel ABI; some ioctls reserved for future use
const AK_MOTOR_SET_ANG_SPEED: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 11, SIZE_INT);
#[allow(dead_code)]
const AK_MOTOR_GET_ANG_SPEED: c_uint = _ior(AK_MOTOR_IOC_MAGIC, 12, SIZE_INT);
const AK_MOTOR_TURN_CLKWISE: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 13, SIZE_INT);
const AK_MOTOR_TURN_ANTICLKWISE: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 14, SIZE_INT);
#[allow(dead_code)]
const AK_MOTOR_GET_HIT_STATUS: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 15, SIZE_INT);
const AK_MOTOR_TURN_STOP: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 16, SIZE_INT);

const MOTOR_GET_STATUS: c_uint = _ior(
    AK_MOTOR_IOC_MAGIC,
    0x43,
    std::mem::size_of::<MotorMessage>() as c_uint,
);

pub const AK_MOTOR_EVENT_HIT: c_int = 1;
pub const AK_MOTOR_EVENT_UNHIT: c_int = 1 << 1;
pub const AK_MOTOR_EVENT_STOP: c_int = 1 << 2;

pub const AK_MOTOR_MIN_SPEED: c_int = 1;
pub const AK_MOTOR_MAX_SPEED: c_int = 200;

/// notify_data from kernel read() - Qiwen ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct NotifyData {
    pub hit_num: c_int,
    pub event: c_int,
    pub remain_steps: c_int,
}

/// motor_message for MOTOR_GET_STATUS ioctl - Qiwen ABI.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MotorMessage {
    pub status: c_int, // enum motor_status
    pub pos: c_int,
    pub speed_step: c_int,
    pub speed_angle: c_int,
    pub steps_one_circle: c_int,
    pub total_steps: c_int,
    pub boundary_steps: c_int,
    pub attach_timer: c_int,
}

// --- PTZ types matching SDK (for PtzFfiTrait) - no C header dependency ---

/// PTZ device (motor index). Matches C enum ptz_device.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum ptz_device {
    #[default]
    PTZ_DEV_H = 0,
    PTZ_DEV_V = 1,
}

/// PTZ feedback pin type. Matches C enum ptz_feedback_pin.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ptz_feedback_pin {
    PTZ_FEEDBACK_PIN_NONE = 0,
    PTZ_FEEDBACK_PIN_EXIST = 1,
}

/// PTZ turn direction. Matches C enum ptz_turn_direction.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum ptz_turn_direction {
    PTZ_TURN_RESERVED = 0,
    PTZ_TURN_LEFT = 1,
    PTZ_TURN_RIGHT = 2,
    PTZ_TURN_UP = 3,
    PTZ_TURN_DOWN = 4,
}

// --- Device paths and constants ---

const AK_MOTOR_DEV0: &str = "/dev/ak-motor0";
const AK_MOTOR_DEV1: &str = "/dev/ak-motor1";
const CYCLE_STEP: i32 = 2048;
const DEFAULT_SPEED: c_int = 100;

/// Single motor handle (one fd). Not thread-safe; use NativePtzDriver for concurrent access.
struct MotorHandle {
    file: File,
    cycle_step: i32,
}

impl MotorHandle {
    fn open(path: &Path) -> IoResult<Self> {
        let file = File::options().read(true).write(true).open(path)?;
        Ok(Self {
            file,
            cycle_step: CYCLE_STEP,
        })
    }

    fn set_speed(&self, speed: c_int) -> PlatformResult<()> {
        let fd = self.file.as_raw_fd();
        let mut val = speed;
        let ret = unsafe { libc::ioctl(fd, AK_MOTOR_SET_ANG_SPEED as libc::c_ulong, &mut val) };
        if ret != 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "AK_MOTOR_SET_ANG_SPEED failed: errno {}",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            )));
        }
        Ok(())
    }

    fn turn_steps(&self, steps: i32, clockwise: bool) -> PlatformResult<()> {
        let fd = self.file.as_raw_fd();
        let mut val = steps;
        let cmd = if clockwise {
            AK_MOTOR_TURN_CLKWISE
        } else {
            AK_MOTOR_TURN_ANTICLKWISE
        };
        let ret = unsafe { libc::ioctl(fd, cmd as libc::c_ulong, &mut val) };
        if ret != 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "motor turn failed: errno {}",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            )));
        }
        Ok(())
    }

    fn turn_stop(&self) -> PlatformResult<()> {
        let fd = self.file.as_raw_fd();
        let ret = unsafe { libc::ioctl(fd, AK_MOTOR_TURN_STOP as libc::c_ulong, std::ptr::null_mut::<c_int>()) };
        if ret != 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "AK_MOTOR_TURN_STOP failed: errno {}",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            )));
        }
        Ok(())
    }

    fn get_step_pos(&self) -> PlatformResult<i32> {
        let fd = self.file.as_raw_fd();
        let mut msg = MotorMessage::default();
        let ret = unsafe {
            libc::ioctl(
                fd,
                MOTOR_GET_STATUS as libc::c_ulong,
                &mut msg as *mut MotorMessage as *mut libc::c_void,
            )
        };
        if ret != 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "MOTOR_GET_STATUS failed: errno {}",
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            )));
        }
        Ok(msg.pos)
    }

    /// Block until the motor signals an event (HIT or STOP) via read().
    fn wait_event(&mut self, timeout_secs: u64) -> PlatformResult<NotifyData> {
        let mut buf = NotifyData::default();
        let fd = self.file.as_raw_fd();
        let mut read_fds: libc::fd_set = unsafe { std::mem::zeroed() };
        unsafe {
            libc::FD_ZERO(&mut read_fds);
            libc::FD_SET(fd, &mut read_fds);
        }
        let tv_sec_i32 = timeout_secs.min(i32::MAX as u64) as i32;
        let mut tv = libc::timeval {
            tv_sec: tv_sec_i32.into(), // libc::timeval::tv_sec is i32 on some targets, i64 on others
            tv_usec: 0,
        };
        let ret = unsafe {
            libc::select(
                fd + 1,
                &mut read_fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            )
        };
        if ret <= 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "select timeout or error: {}",
                std::io::Error::last_os_error()
            )));
        }
        let n = self.file.read(unsafe {
            std::slice::from_raw_parts_mut(
                &mut buf as *mut NotifyData as *mut u8,
                std::mem::size_of::<NotifyData>(),
            )
        }).map_err(|e| PlatformError::HardwareFailure(e.to_string()))?;
        if n != std::mem::size_of::<NotifyData>() {
            return Err(PlatformError::HardwareFailure(
                "short read on motor notify".to_string(),
            ));
        }
        Ok(buf)
    }

    fn degree_to_steps(&self, degree: i32) -> i32 {
        (self.cycle_step as i64 * degree as i64 / 360) as i32
    }
}

/// Native PTZ driver: two motors, thread-safe via Mutex.
pub struct NativePtzDriver {
    inner: Mutex<Option<NativePtzDriverInner>>,
}

struct NativePtzDriverInner {
    motor_h: MotorHandle,
    motor_v: MotorHandle,
}

impl NativePtzDriver {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    fn with_open<F, T>(&self, f: F) -> PlatformResult<T>
    where
        F: FnOnce(&mut NativePtzDriverInner) -> PlatformResult<T>,
    {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
        let inner = guard.as_mut().ok_or_else(|| {
            PlatformError::HardwareUnavailable("PTZ device not opened".to_string())
        })?;
        f(inner)
    }

    /// Open both motor devices and run minimal calibration (no feedback pin).
    pub fn open(&self) -> PlatformResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
        if guard.is_some() {
            return Ok(());
        }
        let motor_h = MotorHandle::open(Path::new(AK_MOTOR_DEV0)).map_err(|e| {
            PlatformError::HardwareFailure(format!("open {}: {}", AK_MOTOR_DEV0, e))
        })?;
        let motor_v = MotorHandle::open(Path::new(AK_MOTOR_DEV1)).map_err(|e| {
            PlatformError::HardwareFailure(format!("open {}: {}", AK_MOTOR_DEV1, e))
        })?;
        motor_h.set_speed(DEFAULT_SPEED)?;
        motor_v.set_speed(DEFAULT_SPEED)?;
        *guard = Some(NativePtzDriverInner { motor_h, motor_v });
        Ok(())
    }

    pub fn close(&self) -> PlatformResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
        *guard = None;
        Ok(())
    }

    /// check_self: minimal path for PTZ_FEEDBACK_PIN_NONE (no-op; motors already opened and speed set).
    pub fn check_self(&self, _pin_type: ptz_feedback_pin) -> PlatformResult<()> {
        self.with_open(|_| Ok(()))
    }

    /// Turn one motor by degree. direction maps to which motor and clockwise/anticlockwise.
    pub fn turn(&self, direction: ptz_turn_direction, degree: i32) -> PlatformResult<()> {
        self.with_open(|inner| {
            let (motor, clockwise) = match direction {
                ptz_turn_direction::PTZ_TURN_LEFT => (&mut inner.motor_h, false),
                ptz_turn_direction::PTZ_TURN_RIGHT => (&mut inner.motor_h, true),
                ptz_turn_direction::PTZ_TURN_UP => (&mut inner.motor_v, true),
                ptz_turn_direction::PTZ_TURN_DOWN => (&mut inner.motor_v, false),
                ptz_turn_direction::PTZ_TURN_RESERVED => {
                    return Err(PlatformError::InvalidParameter(
                        "PTZ_TURN_RESERVED not valid for turn".to_string(),
                    ));
                }
            };
            let steps = motor.degree_to_steps(degree);
            if steps <= 0 {
                return Ok(());
            }
            motor.turn_steps(steps, clockwise)?;
            // Wait for completion (kernel signals via read).
            let timeout = 60u64;
            let notify = motor.wait_event(timeout)?;
            if (notify.event & AK_MOTOR_EVENT_STOP) != 0 {
                tracing::debug!("motor turn stopped by event");
            }
            Ok(())
        })
    }

    pub fn get_step_pos(&self, motor_no: ptz_device) -> PlatformResult<i32> {
        self.with_open(|inner| {
            let motor = match motor_no {
                ptz_device::PTZ_DEV_H => &inner.motor_h,
                ptz_device::PTZ_DEV_V => &inner.motor_v,
            };
            motor.get_step_pos()
        })
    }

    /// Stop the motor(s) for the given direction (or both if needed). Kernel TURN_STOP stops that motor.
    pub fn stop(&self, direction: ptz_turn_direction) -> PlatformResult<()> {
        self.with_open(|inner| {
            match direction {
                ptz_turn_direction::PTZ_TURN_LEFT | ptz_turn_direction::PTZ_TURN_RIGHT => {
                    inner.motor_h.turn_stop()?;
                }
                ptz_turn_direction::PTZ_TURN_UP | ptz_turn_direction::PTZ_TURN_DOWN => {
                    inner.motor_v.turn_stop()?;
                }
                ptz_turn_direction::PTZ_TURN_RESERVED => {
                    inner.motor_h.turn_stop()?;
                    inner.motor_v.turn_stop()?;
                }
            }
            Ok(())
        })
    }
}

impl Default for NativePtzDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_data_size() {
        assert_eq!(std::mem::size_of::<NotifyData>(), 12);
    }

    #[test]
    fn test_ptz_device_values() {
        assert_eq!(ptz_device::PTZ_DEV_H as i32, 0);
        assert_eq!(ptz_device::PTZ_DEV_V as i32, 1);
    }

    #[test]
    fn test_ptz_turn_direction_values() {
        assert_eq!(ptz_turn_direction::PTZ_TURN_LEFT as i32, 1);
        assert_eq!(ptz_turn_direction::PTZ_TURN_RIGHT as i32, 2);
        assert_eq!(ptz_turn_direction::PTZ_TURN_UP as i32, 3);
        assert_eq!(ptz_turn_direction::PTZ_TURN_DOWN as i32, 4);
    }
}
