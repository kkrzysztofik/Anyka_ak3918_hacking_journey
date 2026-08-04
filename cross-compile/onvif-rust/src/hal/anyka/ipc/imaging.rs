//! ImagingHalTrait implementation for AnykaIpc.
//!
//! These methods run inside async ONVIF handlers (`SetImagingSettings` and friends),
//! so each one `.await`s [`AnykaIpc::request_async`]. The awaiting task yields its
//! tokio worker to other work while the dedicated owner thread performs the blocking
//! control-socket I/O — it never parks a worker via `block_in_place`.

use async_trait::async_trait;
use tracing::error;

use crate::hal::common::AK_FAILED_I32;
use crate::hal::common::imaging::ImagingHalTrait;

use super::{
    AnykaIpc, CMD_ISP_GET_AE_LUMA, CMD_ISP_SET_BRIGHTNESS, CMD_ISP_SET_CONTRAST,
    CMD_ISP_SET_IR_FILTER, CMD_ISP_SET_SATURATION, CMD_ISP_SET_SHARPNESS, CMD_ISP_SET_WDR,
};

#[async_trait]
impl ImagingHalTrait for AnykaIpc {
    async fn set_brightness(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_BRIGHTNESS, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_brightness IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_contrast(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_CONTRAST, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_contrast IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_saturation(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_SATURATION, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_saturation IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_sharpness(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_SHARPNESS, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_sharpness IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_ir_filter(&self, enabled: bool) -> i32 {
        let value: i32 = if enabled { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_IR_FILTER, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_ir_filter IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_wdr(&self, enabled: bool) -> i32 {
        let value: i32 = if enabled { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_WDR, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_wdr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn get_ae_luma(&self) -> Option<u8> {
        match self.request_async(CMD_ISP_GET_AE_LUMA, &[]).await {
            Ok((status, data)) if status == 0 && !data.is_empty() => Some(data[0]),
            Ok(_) => None,
            Err(e) => {
                error!(error = %e, "get_ae_luma IPC failed");
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use crate::hal::common::AK_SUCCESS_I32;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_luma_roundtrip() {
        let daemon = FakeDaemon::start(|cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_GET_AE_LUMA);
            assert!(req.is_empty());
            (AK_SUCCESS_I32, vec![42u8])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await,
            Some(42)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_luma_error_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (crate::hal::common::AK_FAILED_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await, None);
    }

    /// set_brightness round-trips correctly through the fake daemon.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_brightness_roundtrip() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        let result = <AnykaIpc as ImagingHalTrait>::set_brightness(&ipc, 50).await;
        assert_eq!(result, AK_SUCCESS_I32, "expected AK_SUCCESS from daemon");
    }

    /// Concurrent set_brightness calls all succeed.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_concurrent_set_brightness() {
        let daemon = FakeDaemon::start(|_cmd_id, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        // Stand in for a completed attach: the epoch gate refuses every
        // request while detached.
        ipc.set_epochs_for_test(1, 1);

        for i in 0..3 {
            let result = <AnykaIpc as ImagingHalTrait>::set_brightness(&ipc, 50 + i).await;
            assert_eq!(result, AK_SUCCESS_I32, "request {} should succeed", i);
        }
    }

    /// The real async imaging HAL path (`ImagingHalTrait::set_brightness`, which now
    /// `.await`s `request_async`) must not park a tokio worker: while one spawned task
    /// is blocked on a hung control RPC, unrelated timers on the runtime keep firing
    /// promptly. This is the Phase 2 review's required "real imaging path" concurrency
    /// proof (previously only `request_async` was exercised directly).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_imaging_hal_async_path_does_not_stall_executor() {
        let daemon = FakeDaemon::start_with_delay(Duration::from_secs(15), |_c, _r| {
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = Arc::new(AnykaIpc::new_with_path(&daemon.socket_path).unwrap());
        // Stand in for a completed attach so the delayed fake-daemon path is reached.
        ipc.set_epochs_for_test(1, 1);

        // Drive the imaging HAL method (not `request_async` directly) from a task.
        let ipc_task = Arc::clone(&ipc);
        let hung = tokio::spawn(async move {
            <AnykaIpc as ImagingHalTrait>::set_brightness(ipc_task.as_ref(), 50).await
        });

        // Unrelated timers must keep firing while the imaging RPC is stuck.
        let start = Instant::now();
        for _ in 0..5 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            start.elapsed() < Duration::from_secs(1),
            "executor stalled: unrelated timers took {:?} while imaging RPC was hung",
            start.elapsed()
        );

        // Drain the hung task; it returns AK_FAILED once the owner's socket timeout fires.
        let result = hung.await.expect("imaging task should not panic");
        assert_eq!(
            result, AK_FAILED_I32,
            "hung imaging RPC should surface AK_FAILED, not hang"
        );
    }
}
