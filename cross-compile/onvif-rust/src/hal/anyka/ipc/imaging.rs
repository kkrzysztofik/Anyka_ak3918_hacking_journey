//! ImagingHalTrait implementation for AnykaIpc.

use tracing::error;

use crate::hal::common::AK_FAILED_I32;
use crate::hal::common::imaging::ImagingHalTrait;

use super::{
    AnykaIpc, CMD_ISP_SET_BRIGHTNESS, CMD_ISP_SET_CONTRAST, CMD_ISP_SET_IR_FILTER,
    CMD_ISP_SET_SATURATION, CMD_ISP_SET_SHARPNESS, CMD_ISP_SET_WDR,
};

impl ImagingHalTrait for AnykaIpc {
    fn set_brightness(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_BRIGHTNESS, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_brightness IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_contrast(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_CONTRAST, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_contrast IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_saturation(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_SATURATION, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_saturation IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_sharpness(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_SHARPNESS, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_sharpness IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_ir_filter(&self, enabled: bool) -> i32 {
        let value: i32 = if enabled { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_IR_FILTER, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_ir_filter IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn set_wdr(&self, enabled: bool) -> i32 {
        let value: i32 = if enabled { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.send_request(CMD_ISP_SET_WDR, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_wdr IPC failed");
                AK_FAILED_I32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use crate::hal::common::AK_SUCCESS_I32;

    /// set_brightness round-trips correctly through the fake daemon.
    #[test]
    fn test_set_brightness_roundtrip() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        let result = <AnykaIpc as ImagingHalTrait>::set_brightness(&ipc, 50);
        assert_eq!(result, AK_SUCCESS_I32, "expected AK_SUCCESS from daemon");
    }

    /// Concurrent set_brightness calls all succeed.
    #[test]
    fn test_concurrent_set_brightness() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();

        for i in 0..3 {
            let result = <AnykaIpc as ImagingHalTrait>::set_brightness(&ipc, 50 + i);
            assert_eq!(result, AK_SUCCESS_I32, "request {} should succeed", i);
        }
    }
}
