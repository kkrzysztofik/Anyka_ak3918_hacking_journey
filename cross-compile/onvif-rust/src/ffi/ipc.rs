//! Safe Rust wrappers for Anyka IPC command-server functions.
//!
//! The Anyka reference stack starts a command server on port 7000 before
//! VPSS/video initialization. VPSS init internally registers IPC handlers; if no
//! server exists, the SDK prints noisy register/unregister warnings.

use std::ffi::{CString, c_char, c_long, c_uint};

use crate::platform::{PlatformError, PlatformResult};

const AK_SUCCESS_I32: i32 = 0;
const ANYKA_IPC_PORT: u32 = 7000;
const DEFAULT_CMD_TIMEOUT_MS: u64 = 400;
const CMD_RESULT_BUF_BYTES: usize = 4096;

/// Default command-server port used by Anyka reference services.
pub const DEFAULT_CMD_SERVER_PORT: u32 = ANYKA_IPC_PORT;

/// Internal trait for abstracting IPC FFI calls to enable mocking in tests.
#[cfg_attr(test, mockall::automock)]
pub(crate) trait IpcFfiTrait: Send + Sync {
    fn cmd_server_register(&self, port: c_uint, name: *mut c_char) -> i32;
    fn cmd_server_unregister(&self, port: c_uint) -> i32;
    fn cmd_send(
        &self,
        port: c_uint,
        cmd: *const c_char,
        cmd_len: c_uint,
        result: *mut c_char,
        res_len: c_uint,
        timeout_ms: *mut c_long,
    ) -> i32;
}

/// Default implementation that calls the real FFI symbols.
pub(crate) struct RealIpcFfi;

impl IpcFfiTrait for RealIpcFfi {
    #[cfg(not(use_stubs))]
    fn cmd_server_register(&self, port: c_uint, name: *mut c_char) -> i32 {
        unsafe extern "C" {
            fn ak_cmd_server_register(port: c_uint, name: *mut c_char) -> i32;
        }
        unsafe { ak_cmd_server_register(port, name) }
    }

    #[cfg(use_stubs)]
    fn cmd_server_register(&self, _port: c_uint, _name: *mut c_char) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn cmd_server_unregister(&self, port: c_uint) -> i32 {
        unsafe extern "C" {
            fn ak_cmd_server_unregister(port: c_uint) -> i32;
        }
        unsafe { ak_cmd_server_unregister(port) }
    }

    #[cfg(use_stubs)]
    fn cmd_server_unregister(&self, _port: c_uint) -> i32 {
        AK_SUCCESS_I32
    }

    #[cfg(not(use_stubs))]
    fn cmd_send(
        &self,
        port: c_uint,
        cmd: *const c_char,
        cmd_len: c_uint,
        result: *mut c_char,
        res_len: c_uint,
        timeout_ms: *mut c_long,
    ) -> i32 {
        unsafe extern "C" {
            fn ak_cmd_send(
                port: c_uint,
                cmd: *const c_char,
                cmd_len: c_uint,
                result: *mut c_char,
                res_len: c_uint,
                tv_out: *mut c_long,
            ) -> i32;
        }
        unsafe { ak_cmd_send(port, cmd, cmd_len, result, res_len, timeout_ms) }
    }

    #[cfg(use_stubs)]
    fn cmd_send(
        &self,
        _port: c_uint,
        _cmd: *const c_char,
        _cmd_len: c_uint,
        result: *mut c_char,
        res_len: c_uint,
        _timeout_ms: *mut c_long,
    ) -> i32 {
        const STUB_RESULT: &[u8] = b"[stub]\n";
        if result.is_null() || res_len == 0 {
            return 0;
        }
        let max_copy = res_len.saturating_sub(1) as usize;
        let copy_len = STUB_RESULT.len().min(max_copy);
        unsafe {
            std::ptr::copy_nonoverlapping(STUB_RESULT.as_ptr().cast::<c_char>(), result, copy_len);
            *result.add(copy_len) = 0;
        }
        copy_len as i32
    }
}

fn check_result(ret: i32, operation: &str) -> PlatformResult<()> {
    if ret == AK_SUCCESS_I32 {
        Ok(())
    } else {
        Err(PlatformError::HardwareFailure(format!(
            "{} failed with error code {}",
            operation, ret
        )))
    }
}

pub(crate) fn command_server_register_internal(
    port: u32,
    name: &CString,
    ffi: &dyn IpcFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.cmd_server_register(port, name.as_ptr().cast_mut());
    check_result(ret, "ak_cmd_server_register")
}

pub(crate) fn command_server_unregister_internal(
    port: u32,
    ffi: &dyn IpcFfiTrait,
) -> PlatformResult<()> {
    let ret = ffi.cmd_server_unregister(port);
    check_result(ret, "ak_cmd_server_unregister")
}

pub(crate) fn command_send_internal(
    port: u32,
    command: &CString,
    timeout_ms: u64,
    ffi: &dyn IpcFfiTrait,
) -> PlatformResult<String> {
    let cmd_len_u32 = u32::try_from(command.as_bytes_with_nul().len()).map_err(|_| {
        PlatformError::InvalidParameter("IPC command is too long for Anyka SDK".to_string())
    })?;
    let mut result_buf = vec![0_u8; CMD_RESULT_BUF_BYTES];
    let res_len_u32 = u32::try_from(result_buf.len()).map_err(|_| {
        PlatformError::InvalidParameter("IPC response buffer length overflow".to_string())
    })?;
    let mut timeout = timeout_ms as c_long;
    let ret = ffi.cmd_send(
        port,
        command.as_ptr(),
        cmd_len_u32 as c_uint,
        result_buf.as_mut_ptr().cast::<c_char>(),
        res_len_u32 as c_uint,
        &mut timeout as *mut c_long,
    );
    if ret < 0 {
        return Err(PlatformError::HardwareFailure(format!(
            "ak_cmd_send failed with error code {}",
            ret
        )));
    }
    if ret == 0 {
        return Err(PlatformError::Timeout);
    }
    let used_len = (ret as usize).min(result_buf.len());
    let payload = &result_buf[..used_len];
    let nul_pos = payload
        .iter()
        .position(|b| *b == 0)
        .unwrap_or(payload.len());
    Ok(String::from_utf8_lossy(&payload[..nul_pos]).to_string())
}

/// Register Anyka command server on the default port (`7000`).
pub fn command_server_register(name: &str) -> PlatformResult<()> {
    let c_name = CString::new(name).map_err(|_| {
        PlatformError::InvalidParameter("Command server name contains null byte".to_string())
    })?;
    command_server_register_internal(DEFAULT_CMD_SERVER_PORT, &c_name, &RealIpcFfi)
}

/// Unregister Anyka command server from the default port (`7000`).
pub fn command_server_unregister() -> PlatformResult<()> {
    command_server_unregister_internal(DEFAULT_CMD_SERVER_PORT, &RealIpcFfi)
}

/// Send an IPC command to Anyka command server on port `7000` with a bounded timeout.
pub fn command_send_with_timeout(command: &str, timeout_ms: u64) -> PlatformResult<String> {
    let c_cmd = CString::new(command).map_err(|_| {
        PlatformError::InvalidParameter("IPC command contains null byte".to_string())
    })?;
    command_send_internal(DEFAULT_CMD_SERVER_PORT, &c_cmd, timeout_ms, &RealIpcFfi)
}

/// Send an IPC command to Anyka command server on port `7000`.
///
/// Uses a short default timeout to avoid blocking teardown.
pub fn command_send(command: &str) -> PlatformResult<String> {
    command_send_with_timeout(command, DEFAULT_CMD_TIMEOUT_MS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_send_internal_returns_payload_up_to_nul() {
        let mut ffi = MockIpcFfiTrait::new();
        ffi.expect_cmd_send()
            .returning(|_port, _cmd, _cmd_len, result, res_len, _timeout_ms| {
                let response = b"venc ok\0ignored";
                let max = res_len as usize;
                let len = response.len().min(max);
                unsafe {
                    std::ptr::copy_nonoverlapping(response.as_ptr().cast::<c_char>(), result, len);
                }
                len as i32
            });
        let cmd = CString::new("venc_get_status").unwrap();
        let response = command_send_internal(DEFAULT_CMD_SERVER_PORT, &cmd, 200, &ffi).unwrap();
        assert_eq!(response, "venc ok");
    }

    #[test]
    fn command_send_internal_timeout_maps_to_platform_timeout() {
        let mut ffi = MockIpcFfiTrait::new();
        ffi.expect_cmd_send()
            .returning(|_port, _cmd, _cmd_len, _result, _res_len, _timeout_ms| 0);
        let cmd = CString::new("vi_get_status").unwrap();
        let response = command_send_internal(DEFAULT_CMD_SERVER_PORT, &cmd, 100, &ffi);
        assert!(matches!(response, Err(PlatformError::Timeout)));
    }
}
