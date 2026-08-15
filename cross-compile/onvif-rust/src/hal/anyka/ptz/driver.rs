//! Native Rust PTZ driver for Qiwen/Anycloud kernel motor devices.
//!
//! Talks directly to `/dev/ak-motor0` (horizontal) and `/dev/ak-motor1` (vertical)
//! via ioctl/read. ABI matches [qiwen/anycloud39ev300 SDK kernel ak_motor.h](https://github.com/...).
//! No C PTZ library dependency.
//!
//! # Concurrency model
//!
//! Motor completion is signalled asynchronously by the kernel via `read()`. Waiting
//! for that signal must never block the driver `Mutex`, otherwise a concurrent `Stop`
//! (or a superseding move) cannot preempt an in-flight turn. Two mechanisms make the
//! wait cooperative:
//!
//! 1. `turn`/`wait_turn` clone an `Arc<MotorHandle>` out of the driver `Mutex` and
//!    release the lock **before** entering the blocking `select()`/`read()` loop, so
//!    the stop path can still reach the motor fds.
//! 2. `wait_event_interruptible` polls a shared [`AtomicBool`] stop flag on a ~100ms
//!    cadence; when the flag is set it issues `AK_MOTOR_TURN_STOP` and returns
//!    `interrupted = true` instead of blocking for the full timeout.

use std::fs::File;
use std::io::{Read, Result as IoResult};
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use libc::{c_int, c_uint};

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
    (IOC_WRITE << IOC_DIRSHIFT)
        | ((type_ << IOC_TYPESHIFT) & 0xff00)
        | (nr << IOC_NRSHIFT)
        | (size << IOC_SIZESHIFT)
}

#[allow(clippy::identity_op)]
const fn _ior(type_: c_uint, nr: c_uint, size: c_uint) -> c_uint {
    const IOC_READ: c_uint = 2;
    const IOC_NRSHIFT: c_uint = 0;
    const IOC_TYPESHIFT: c_uint = 8;
    const IOC_SIZESHIFT: c_uint = 16;
    const IOC_DIRSHIFT: c_uint = 30;
    (IOC_READ << IOC_DIRSHIFT)
        | ((type_ << IOC_TYPESHIFT) & 0xff00)
        | (nr << IOC_NRSHIFT)
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

// The "Anycloud V500" command family. Both of these pass a *struct* pointer but are
// declared `_IOW(..., int)` in the vendor driver (ak_drv_ptz.c:39,62) — the size field
// is part of the command word, so encoding them from the struct size produces a
// different ioctl number and the kernel rejects it.
const MOTOR_PARM: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 0x00, SIZE_INT);
const MOTOR_GET_STATUS: c_uint = _iow(AK_MOTOR_IOC_MAGIC, 0x43, SIZE_INT);

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

/// motor_parm for the MOTOR_PARM ioctl - Qiwen ABI (field order matters).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct MotorParm {
    pub pos: c_int,
    pub speed_step: c_int,
    pub steps_one_circle: c_int,
    pub total_steps: c_int,
    pub boundary_steps: c_int,
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

// `TurnOutcome` lives in `hal::common::ptz` alongside the trait that returns it, so the
// signature (and its mock) are available on host builds where this driver is not compiled.
pub use crate::hal::common::ptz::TurnOutcome;

// --- PTZ types matching SDK (for PtzHalTrait) - no C header dependency ---

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
/// Steps from limit to "middle" (matches C driver init_pos = fulldst_step/2).
const RESET_STEP: i32 = 2048 / 2;
const DEFAULT_SPEED: c_int = 100;
/// `motor_parm.speed_step` seed, matching the vendor's `ak_drv_ptz_setup_step_param()`.
const DEFAULT_SPEED_STEP: c_int = 400;
/// Total travel of each axis in degrees. `MOTOR_PARM` wants this as a step count
/// (`2048 * degrees / 360`), exactly as the vendor's `AK_DRV_PTZ_SETUP` macro computes it
/// (`libplat/include/ak_drv_ptz.h:113`). Kept in sync with the platform layer's
/// `PTZ_{MAX,MIN}_{PAN,TILT}_DEGREES` by a unit test below.
pub(crate) const PAN_TRAVEL_DEGREES: i32 = 700;
pub(crate) const TILT_TRAVEL_DEGREES: i32 = 260;
/// Timeout for calibration move to limit (full rotation can be slow).
const CALIBRATION_TIMEOUT_SECS: u64 = 120;
/// Upper bound for a single motor turn to complete before we give up.
const TURN_TIMEOUT_SECS: u64 = 60;
/// Poll cadence for the interruptible wait loop (milliseconds).
const WAIT_POLL_MS: i64 = 100;

/// Outcome of a single `select()` poll tick on a motor fd.
enum PollOutcome {
    /// The fd is readable; `notify_data` is pending.
    Readable,
    /// The poll tick expired with no event.
    TimedOut,
    /// `select()` was interrupted by a signal.
    Interrupted,
}

/// Single motor handle (one fd).
///
/// All methods take `&self`: the kernel serialises ioctls, and `read()` is performed
/// through `&File` (via `impl Read for &File`). This lets a single `Arc<MotorHandle>`
/// be shared between the turning path and the stop path without a per-handle mutex.
struct MotorHandle {
    file: File,
    /// Device path, carried so every ioctl failure says *which* motor.
    name: &'static str,
    /// Steps per full revolution. Seeded from [`CYCLE_STEP`], then replaced by whatever
    /// the kernel reports after `MOTOR_PARM` — the hardware's own gearing wins.
    cycle_step: i32,
    /// Usable travel in steps, likewise adopted from the kernel.
    total_steps: i32,
}

impl MotorHandle {
    fn open(path: &'static str) -> IoResult<Self> {
        let file = File::options()
            .read(true)
            .write(true)
            .open(Path::new(path))?;
        Ok(Self {
            file,
            name: path,
            cycle_step: CYCLE_STEP,
            total_steps: 0,
        })
    }

    fn raw_fd(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    /// Issue an ioctl carrying a pointer argument.
    ///
    /// Failures name the command and the motor, so the caller never has to guess which
    /// of the two devices rejected what.
    fn ioctl_ptr<T>(&self, cmd: c_uint, arg: *mut T, what: &str) -> PlatformResult<()> {
        // SAFETY: `arg` points at a live, correctly-typed value owned by the caller for
        // the duration of the call (or is null, which the STOP command accepts), and
        // `cmd` is one of the kernel ABI constants declared above.
        let ret = unsafe {
            libc::ioctl(
                self.raw_fd(),
                cmd as libc::c_ulong,
                arg as *mut libc::c_void,
            )
        };
        if ret != 0 {
            return Err(PlatformError::HardwareFailure(format!(
                "{} failed on {}: errno {}",
                what,
                self.name,
                std::io::Error::last_os_error().raw_os_error().unwrap_or(-1)
            )));
        }
        Ok(())
    }

    /// Seed the kernel's motor geometry, then adopt what it reports back.
    ///
    /// Mirrors the vendor's `ak_drv_ptz_setup_step_param()` (ak_drv_ptz.c:760). Without
    /// `MOTOR_PARM` the kernel motor has no travel limits or step scale configured, so
    /// the movement ioctls have nothing to work against.
    fn configure(&mut self, travel_degrees: i32) -> PlatformResult<()> {
        let total_steps = CYCLE_STEP * travel_degrees / 360;
        let mut parm = MotorParm {
            // Vendor passes init_pos = -1, which its helper resolves to fulldst_step / 2.
            pos: total_steps / 2,
            speed_step: DEFAULT_SPEED_STEP,
            steps_one_circle: CYCLE_STEP,
            total_steps,
            boundary_steps: 0,
        };
        self.ioctl_ptr(MOTOR_PARM, &mut parm, "MOTOR_PARM")?;

        // Trust the kernel's accounting over our seed values: a differently geared body
        // reports its own numbers here, and this is the only place we can learn them.
        let msg = self.get_status()?;
        if msg.steps_one_circle > 0 {
            self.cycle_step = msg.steps_one_circle;
        }
        if msg.total_steps > 0 {
            self.total_steps = msg.total_steps;
        }
        tracing::info!(
            "{}: cycle_step={}, total_steps={}, pos={}",
            self.name,
            self.cycle_step,
            self.total_steps,
            msg.pos
        );
        Ok(())
    }

    fn set_speed(&self, speed: c_int) -> PlatformResult<()> {
        let mut val = speed;
        self.ioctl_ptr(AK_MOTOR_SET_ANG_SPEED, &mut val, "AK_MOTOR_SET_ANG_SPEED")
    }

    fn turn_steps(&self, steps: i32, clockwise: bool) -> PlatformResult<()> {
        let mut val = steps;
        let (cmd, what) = if clockwise {
            (AK_MOTOR_TURN_CLKWISE, "AK_MOTOR_TURN_CLKWISE")
        } else {
            (AK_MOTOR_TURN_ANTICLKWISE, "AK_MOTOR_TURN_ANTICLKWISE")
        };
        self.ioctl_ptr(cmd, &mut val, what)
    }

    fn turn_stop(&self) -> PlatformResult<()> {
        self.ioctl_ptr(
            AK_MOTOR_TURN_STOP,
            std::ptr::null_mut::<c_int>(),
            "AK_MOTOR_TURN_STOP",
        )
    }

    /// Read the kernel's full motor status (position, run state, geometry).
    fn get_status(&self) -> PlatformResult<MotorMessage> {
        let mut msg = MotorMessage::default();
        self.ioctl_ptr(MOTOR_GET_STATUS, &mut msg, "MOTOR_GET_STATUS")?;
        Ok(msg)
    }

    fn get_step_pos(&self) -> PlatformResult<i32> {
        Ok(self.get_status()?.pos)
    }

    /// Read the pending `notify_data` from the motor fd (fd already known readable).
    fn read_notify(&self) -> PlatformResult<NotifyData> {
        let mut buf = NotifyData::default();
        // `impl Read for &File` lets us read without a `&mut self`.
        let n = (&self.file)
            .read(unsafe {
                std::slice::from_raw_parts_mut(
                    &mut buf as *mut NotifyData as *mut u8,
                    std::mem::size_of::<NotifyData>(),
                )
            })
            .map_err(|e| PlatformError::HardwareFailure(e.to_string()))?;
        // Deliberately NOT checking `n` against the struct size. This driver does not
        // return a byte count from `read()`, and the vendor ignores the return value
        // outright (ak_drv_ptz.c:411) — it reads into the struct and uses it regardless.
        // Treating a "short" read as fatal aborted every calibration sweep at ~2s.
        if n != std::mem::size_of::<NotifyData>() {
            tracing::debug!("{}: motor notify read returned {} bytes", self.name, n);
        }
        Ok(buf)
    }

    /// Block until the motor signals an event, the deadline elapses, or `stop_flag`
    /// is set. On stop-flag set, issue `AK_MOTOR_TURN_STOP` and return with
    /// `interrupted = true`.
    fn wait_event_interruptible(
        &self,
        timeout_secs: u64,
        stop_flag: &AtomicBool,
    ) -> PlatformResult<(NotifyData, bool)> {
        let fd = self.raw_fd();
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        loop {
            if stop_flag.load(Ordering::SeqCst) {
                // Preempted: stop the motor and report the interruption.
                if let Err(e) = self.turn_stop() {
                    tracing::warn!("TURN_STOP during interrupt failed: {}", e);
                }
                return Ok((NotifyData::default(), true));
            }

            match Self::poll_readable(fd)? {
                // Interrupted by a signal: re-check the stop flag and poll again.
                PollOutcome::Interrupted => continue,
                PollOutcome::TimedOut => {
                    // Poll tick with no event: re-check stop flag and deadline.
                    if Instant::now() >= deadline {
                        return Err(PlatformError::HardwareFailure(
                            "motor wait timed out".to_string(),
                        ));
                    }
                    continue;
                }
                PollOutcome::Readable => {
                    // fd readable → consume the notify_data.
                    let notify = self.read_notify()?;
                    tracing::debug!(
                        "{}: notify event=0x{:x} hit_num={} remain_steps={}",
                        self.name,
                        notify.event,
                        notify.hit_num,
                        notify.remain_steps
                    );
                    return Ok((notify, false));
                }
            }
        }
    }

    /// Wait up to one `WAIT_POLL_MS` tick for the motor fd to become readable.
    fn poll_readable(fd: RawFd) -> PlatformResult<PollOutcome> {
        // SAFETY: fd_set is a plain-old-data bitset; all-zero bits is a valid
        // representation, so zero-initializing it is sound.
        let mut read_fds: libc::fd_set = unsafe { std::mem::zeroed() };
        // SAFETY: `read_fds` is a valid, live `fd_set` from the line above.
        // `fd` is the caller-owned motor device fd and is within FD_SETSIZE.
        unsafe {
            libc::FD_ZERO(&mut read_fds);
            libc::FD_SET(fd, &mut read_fds);
        }
        let mut tv = libc::timeval {
            tv_sec: 0,
            tv_usec: (WAIT_POLL_MS * 1000) as libc::suseconds_t,
        };
        // SAFETY: `read_fds` and `tv` are valid, live, and properly
        // initialized above; the write/except sets are null, which
        // `select(2)` accepts; `fd + 1` matches the single fd registered.
        let ret = unsafe {
            libc::select(
                fd + 1,
                &mut read_fds,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut tv,
            )
        };
        if ret < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() == Some(libc::EINTR) {
                return Ok(PollOutcome::Interrupted);
            }
            return Err(PlatformError::HardwareFailure(format!(
                "select error: {}",
                err
            )));
        }
        if ret == 0 {
            return Ok(PollOutcome::TimedOut);
        }
        Ok(PollOutcome::Readable)
    }

    fn degree_to_steps(&self, degree: i32) -> i32 {
        (self.cycle_step as i64 * degree as i64 / 360) as i32
    }

    /// Steps from the physical limit back to the middle of travel.
    ///
    /// The vendor uses `fulldst_step / 2`; fall back to half a revolution when the
    /// kernel reported no travel range.
    fn reset_step(&self) -> i32 {
        if self.total_steps > 0 {
            self.total_steps / 2
        } else {
            RESET_STEP
        }
    }

    /// Run calibration sequence matching C driver get_motor_param():
    /// turn anticlockwise to limit (HIT or STOP), then clockwise to the middle of travel.
    fn calibrate(&self, stop_flag: &AtomicBool) -> PlatformResult<()> {
        self.set_speed(DEFAULT_SPEED)?;
        // Turn anticlockwise to physical limit; wait for HIT or STOP event.
        self.turn_steps(self.cycle_step, false)?;
        let (notify, _) = self.wait_event_interruptible(CALIBRATION_TIMEOUT_SECS, stop_flag)?;
        if (notify.event & AK_MOTOR_EVENT_HIT) != 0 {
            tracing::debug!(
                "calibration: motor hit limit, remain_steps={}",
                notify.remain_steps
            );
        } else if (notify.event & AK_MOTOR_EVENT_STOP) != 0 {
            tracing::debug!(
                "calibration: motor stop, remain_steps={}",
                notify.remain_steps
            );
        }
        // Turn clockwise to middle position (same as C driver reset_step).
        self.turn_steps(self.reset_step(), true)?;
        self.wait_event_interruptible(TURN_TIMEOUT_SECS, stop_flag)?;

        // Does the kernel's own position accounting actually move? This distinguishes a
        // working MOTOR_GET_STATUS from one that returns success while writing nothing.
        match self.get_status() {
            Ok(msg) => tracing::info!(
                "{}: post-calibration status={} pos={} steps_one_circle={} total_steps={}",
                self.name,
                msg.status,
                msg.pos,
                msg.steps_one_circle,
                msg.total_steps
            ),
            Err(e) => tracing::warn!("{}: post-calibration status read failed: {}", self.name, e),
        }
        Ok(())
    }
}

/// Native PTZ driver: two motors, thread-safe via Mutex.
///
/// The `Mutex` only guards open/close and the brief clone of an `Arc<MotorHandle>`.
/// Blocking motor waits happen on the cloned handle **without** holding the lock, so a
/// concurrent stop or superseding move can always make progress.
pub struct NativePtzDriver {
    inner: Mutex<Option<NativePtzDriverInner>>,
    /// Shared interrupt flag polled by in-flight waits. Setting it preempts the
    /// current turn; movement paths clear it before starting.
    stop_flag: Arc<AtomicBool>,
}

struct NativePtzDriverInner {
    motor_h: Arc<MotorHandle>,
    motor_v: Arc<MotorHandle>,
}

impl NativePtzDriver {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(None),
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Clone the `Arc<MotorHandle>` (and clockwise flag) for a direction, holding the
    /// lock only for the clone. Returns an error if the device is not open or the
    /// direction is not a real motor movement.
    fn motor_for(&self, direction: ptz_turn_direction) -> PlatformResult<(Arc<MotorHandle>, bool)> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
        let inner = guard.as_ref().ok_or_else(|| {
            PlatformError::HardwareUnavailable("PTZ device not opened".to_string())
        })?;
        Ok(match direction {
            ptz_turn_direction::PTZ_TURN_LEFT => (Arc::clone(&inner.motor_h), false),
            ptz_turn_direction::PTZ_TURN_RIGHT => (Arc::clone(&inner.motor_h), true),
            ptz_turn_direction::PTZ_TURN_UP => (Arc::clone(&inner.motor_v), true),
            ptz_turn_direction::PTZ_TURN_DOWN => (Arc::clone(&inner.motor_v), false),
            ptz_turn_direction::PTZ_TURN_RESERVED => {
                return Err(PlatformError::InvalidParameter(
                    "PTZ_TURN_RESERVED not valid for turn".to_string(),
                ));
            }
        })
    }

    /// Clone both motor handles for the stop path (brief lock only).
    fn both_motors(&self) -> PlatformResult<Option<(Arc<MotorHandle>, Arc<MotorHandle>)>> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
        Ok(guard
            .as_ref()
            .map(|inner| (Arc::clone(&inner.motor_h), Arc::clone(&inner.motor_v))))
    }

    /// Open both motor devices and set default speed.
    pub fn open(&self) -> PlatformResult<()> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
        if guard.is_some() {
            return Ok(());
        }
        let mut motor_h = MotorHandle::open(AK_MOTOR_DEV0).map_err(|e| {
            PlatformError::HardwareFailure(format!("open {}: {}", AK_MOTOR_DEV0, e))
        })?;
        let mut motor_v = MotorHandle::open(AK_MOTOR_DEV1).map_err(|e| {
            PlatformError::HardwareFailure(format!("open {}: {}", AK_MOTOR_DEV1, e))
        })?;
        // Geometry must be seeded before speed or movement: MOTOR_PARM is what gives the
        // kernel motor its step scale and travel limits.
        motor_h.configure(PAN_TRAVEL_DEGREES)?;
        motor_v.configure(TILT_TRAVEL_DEGREES)?;
        motor_h.set_speed(DEFAULT_SPEED)?;
        motor_v.set_speed(DEFAULT_SPEED)?;
        *guard = Some(NativePtzDriverInner {
            motor_h: Arc::new(motor_h),
            motor_v: Arc::new(motor_v),
        });
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

    /// check_self: run calibration (turn to limit, then to middle) for both motors.
    /// Matches C driver get_motor_param() so the physical calibration movement is heard.
    pub fn check_self(&self, _pin_type: ptz_feedback_pin) -> PlatformResult<()> {
        self.stop_flag.store(false, Ordering::SeqCst);
        let (motor_h, motor_v) = self
            .both_motors()?
            .ok_or_else(|| PlatformError::HardwareUnavailable("PTZ device not opened".into()))?;
        tracing::info!("PTZ calibration: horizontal motor (limit then middle)");
        motor_h.calibrate(&self.stop_flag)?;
        tracing::info!("PTZ calibration: vertical motor (limit then middle)");
        motor_v.calibrate(&self.stop_flag)?;
        tracing::info!("PTZ calibration complete");
        Ok(())
    }

    /// Issue a turn command without waiting for completion. Clears the stop flag so a
    /// fresh movement is not immediately preempted by a stale interrupt. Returns
    /// `Ok(false)` when the requested move rounds to zero steps (no-op).
    pub fn start_turn(&self, direction: ptz_turn_direction, degree: i32) -> PlatformResult<bool> {
        let (motor, clockwise) = self.motor_for(direction)?;
        let steps = motor.degree_to_steps(degree);
        if steps <= 0 {
            return Ok(false);
        }
        self.stop_flag.store(false, Ordering::SeqCst);
        motor.turn_steps(steps, clockwise)?;
        Ok(true)
    }

    /// Wait for the in-flight turn on `direction`'s motor to finish or be interrupted,
    /// then reconcile position from `MOTOR_GET_STATUS`. Does not hold the driver lock
    /// while blocking.
    pub fn wait_turn(&self, direction: ptz_turn_direction) -> PlatformResult<TurnOutcome> {
        let (motor, _) = self.motor_for(direction)?;
        let (notify, interrupted) =
            motor.wait_event_interruptible(TURN_TIMEOUT_SECS, &self.stop_flag)?;
        let step_pos = motor.get_step_pos().unwrap_or(-1);
        Ok(TurnOutcome {
            interrupted,
            step_pos,
            event: notify.event,
        })
    }

    pub fn get_step_pos(&self, motor_no: ptz_device) -> PlatformResult<i32> {
        let motor = {
            let guard = self
                .inner
                .lock()
                .map_err(|e| PlatformError::HardwareFailure(format!("lock poisoned: {}", e)))?;
            let inner = guard.as_ref().ok_or_else(|| {
                PlatformError::HardwareUnavailable("PTZ device not opened".to_string())
            })?;
            match motor_no {
                ptz_device::PTZ_DEV_H => Arc::clone(&inner.motor_h),
                ptz_device::PTZ_DEV_V => Arc::clone(&inner.motor_v),
            }
        };
        motor.get_step_pos()
    }

    /// Set the interrupt flag so any in-flight `wait_turn`/`turn` returns promptly.
    /// The waiting path issues `AK_MOTOR_TURN_STOP` when it observes the flag.
    pub fn interrupt(&self) {
        self.stop_flag.store(true, Ordering::SeqCst);
    }

    /// Stop the motor(s) for the given direction. Runs off the main-turn wait since
    /// `turn`/`wait_turn` do not hold the lock while blocking. Also sets the interrupt
    /// flag so a concurrent in-flight wait unwinds.
    pub fn stop(&self, direction: ptz_turn_direction) -> PlatformResult<()> {
        self.stop_flag.store(true, Ordering::SeqCst);
        let Some((motor_h, motor_v)) = self.both_motors()? else {
            // Not open: nothing to stop.
            return Ok(());
        };
        match direction {
            ptz_turn_direction::PTZ_TURN_LEFT | ptz_turn_direction::PTZ_TURN_RIGHT => {
                motor_h.turn_stop()?;
            }
            ptz_turn_direction::PTZ_TURN_UP | ptz_turn_direction::PTZ_TURN_DOWN => {
                motor_v.turn_stop()?;
            }
            ptz_turn_direction::PTZ_TURN_RESERVED => {
                motor_h.turn_stop()?;
                motor_v.turn_stop()?;
            }
        }
        Ok(())
    }
}

impl Default for NativePtzDriver {
    fn default() -> Self {
        Self::new()
    }
}

/// The driver *is* the HAL backend — no adapter struct in between.
///
/// Gated to ARM because on host builds `hal::common::ptz` binds its types to the
/// mirrors in `common::sdk_types` rather than the ones declared above.
#[cfg(not(use_stubs))]
impl crate::hal::common::ptz::PtzHalTrait for NativePtzDriver {
    fn ptz_open(&self) -> PlatformResult<()> {
        self.open()
    }

    fn ptz_close(&self) -> PlatformResult<()> {
        self.close()
    }

    fn ptz_check_self(&self, pin_type: ptz_feedback_pin) -> PlatformResult<()> {
        self.check_self(pin_type)
    }

    fn ptz_start_turn(&self, direction: ptz_turn_direction, degree: i32) -> PlatformResult<bool> {
        self.start_turn(direction, degree)
    }

    fn ptz_wait_turn(&self, direction: ptz_turn_direction) -> PlatformResult<TurnOutcome> {
        self.wait_turn(direction)
    }

    fn ptz_get_step_pos(&self, motor_no: ptz_device) -> PlatformResult<i32> {
        self.get_step_pos(motor_no)
    }

    fn ptz_stop(&self, direction: ptz_turn_direction) -> PlatformResult<()> {
        self.stop(direction)
    }

    fn ptz_interrupt(&self) {
        self.interrupt();
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
    fn test_motor_parm_size() {
        assert_eq!(std::mem::size_of::<MotorParm>(), 20);
    }

    /// Pin the exact command words against the vendor driver's macros
    /// (`anyka_reference/platform/libplat/src/drv/ak_drv_ptz.c:39,62`).
    ///
    /// Both are declared `_IOW(magic, nr, int)` even though they carry a struct pointer.
    /// Encoding them from the struct size instead yields a different command word and the
    /// kernel rejects the call — which is exactly how PTZ bring-up failed with `-1`.
    #[test]
    fn test_v500_ioctl_encoding_matches_vendor() {
        assert_eq!(
            MOTOR_PARM, 0x4004_6D00,
            "MOTOR_PARM must be _IOW('m', 0x00, int)"
        );
        assert_eq!(
            MOTOR_GET_STATUS, 0x4004_6D43,
            "MOTOR_GET_STATUS must be _IOW('m', 0x43, int)"
        );
    }

    /// The legacy `nr 11-16` family, unchanged — guards against a careless edit to `_iow`.
    #[test]
    fn test_legacy_ioctl_encoding() {
        assert_eq!(AK_MOTOR_SET_ANG_SPEED, 0x4004_6D0B);
        assert_eq!(AK_MOTOR_TURN_CLKWISE, 0x4004_6D0D);
        assert_eq!(AK_MOTOR_TURN_ANTICLKWISE, 0x4004_6D0E);
        assert_eq!(AK_MOTOR_TURN_STOP, 0x4004_6D10);
    }

    /// `MOTOR_PARM.total_steps` is `2048 * travel / 360`, as the vendor's
    /// `AK_DRV_PTZ_SETUP` macro computes it (`libplat/include/ak_drv_ptz.h:113`).
    #[test]
    fn test_travel_degrees_convert_to_vendor_step_counts() {
        assert_eq!(CYCLE_STEP * PAN_TRAVEL_DEGREES / 360, 3982);
        assert_eq!(CYCLE_STEP * TILT_TRAVEL_DEGREES / 360, 1479);
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

    #[test]
    fn test_start_turn_without_open_returns_unavailable() {
        let driver = NativePtzDriver::new();
        let result = driver.start_turn(ptz_turn_direction::PTZ_TURN_LEFT, 30);
        assert!(matches!(result, Err(PlatformError::HardwareUnavailable(_))));
    }

    #[test]
    fn test_turn_reserved_direction_is_invalid() {
        let driver = NativePtzDriver::new();
        // Reserved is rejected at motor selection even before the open check ordering,
        // but with a closed device the unavailable check fires first; open-less drivers
        // still must not panic.
        let result = driver.start_turn(ptz_turn_direction::PTZ_TURN_RESERVED, 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_stop_without_open_is_ok() {
        let driver = NativePtzDriver::new();
        // Stopping a closed device is a no-op success (nothing to stop).
        assert!(driver.stop(ptz_turn_direction::PTZ_TURN_RESERVED).is_ok());
    }

    #[test]
    fn test_interrupt_sets_stop_flag() {
        let driver = NativePtzDriver::new();
        assert!(!driver.stop_flag.load(Ordering::SeqCst));
        driver.interrupt();
        assert!(driver.stop_flag.load(Ordering::SeqCst));
    }

    #[test]
    fn test_poll_readable_returns_readable_when_data_available() {
        use std::io::Write;
        use std::os::unix::io::AsRawFd;
        use std::os::unix::net::UnixStream;

        let (mut writer, reader) = UnixStream::pair().expect("socketpair");
        writer.write_all(b"x").expect("write");

        let outcome = MotorHandle::poll_readable(reader.as_raw_fd());
        assert!(matches!(outcome, Ok(PollOutcome::Readable)));
    }

    #[test]
    fn test_poll_readable_times_out_when_no_data() {
        use std::os::unix::io::AsRawFd;
        use std::os::unix::net::UnixStream;

        let (_writer, reader) = UnixStream::pair().expect("socketpair");

        let outcome = MotorHandle::poll_readable(reader.as_raw_fd());
        assert!(matches!(outcome, Ok(PollOutcome::TimedOut)));
    }
}
