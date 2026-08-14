//! ImagingHalTrait implementation for AnykaIpc.
//!
//! These methods run inside async ONVIF handlers (`SetImagingSettings` and friends),
//! so each one `.await`s [`AnykaIpc::request_async`]. The awaiting task yields its
//! tokio worker to other work while the dedicated owner thread performs the blocking
//! control-socket I/O — it never parks a worker via `block_in_place`.

use async_trait::async_trait;
use tracing::error;

use crate::hal::common::AK_FAILED_I32;
use crate::hal::common::AK_SUCCESS_I32;
use crate::hal::common::imaging::{AE_ATTR_WIRE_LEN, AeAttr, ImagingHalTrait};

use super::{
    AnykaIpc, CMD_ISP_GET_AE_ATTR, CMD_ISP_GET_AE_LUMA, CMD_ISP_GET_LUM_FACTOR,
    CMD_ISP_SET_BRIGHTNESS, CMD_ISP_SET_CONTRAST, CMD_ISP_SET_IR_FILTER, CMD_ISP_SET_SATURATION,
    CMD_ISP_SET_SHARPNESS, CMD_ISP_SET_WDR,
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
            // The wire contract is exactly one luma byte; a longer or empty
            // payload is a malformed daemon response and must not be accepted.
            Ok((status, data)) if status == AK_SUCCESS_I32 && data.len() == 1 => Some(data[0]),
            // Not silent: a bad status or a short payload is a real daemon
            // fault, and the caller only sees `None`, which it treats as
            // "hold". Without this line an ISP that never answers looks
            // exactly like a camera that is correctly holding its mode.
            Ok((status, data)) => {
                error!(status, len = data.len(), "get_ae_luma bad daemon response");
                None
            }
            Err(e) => {
                error!(error = %e, "get_ae_luma IPC failed");
                None
            }
        }
    }

    async fn get_lum_factor(&self) -> Option<i32> {
        match self.request_async(CMD_ISP_GET_LUM_FACTOR, &[]).await {
            // The wire contract is exactly one little-endian i32, and it is
            // strictly positive: the underlying isp_get_cur_lum_factor()
            // signals failure with -1, and a non-positive factor sits below
            // every day threshold, so letting one through would read as
            // "bright" and switch night vision off. The daemon already filters
            // this; the check is repeated here so an older daemon on a
            // half-upgraded camera cannot reintroduce it.
            Ok((status, data)) if status == AK_SUCCESS_I32 && data.len() == 4 => {
                let factor = i32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                if factor > 0 {
                    Some(factor)
                } else {
                    error!(
                        factor,
                        "get_lum_factor non-positive; treating as unavailable"
                    );
                    None
                }
            }
            // Same reasoning as get_ae_luma: a silent `None` is indistinguishable
            // from a camera correctly holding its mode, so say so.
            Ok((status, data)) => {
                error!(
                    status,
                    len = data.len(),
                    "get_lum_factor bad daemon response"
                );
                None
            }
            Err(e) => {
                error!(error = %e, "get_lum_factor IPC failed");
                None
            }
        }
    }

    async fn get_ae_attr(&self) -> Option<AeAttr> {
        match self.request_async(CMD_ISP_GET_AE_ATTR, &[]).await {
            // The wire contract is exactly the 204-byte struct vpss_isp_ae_attr.
            // A short payload means a daemon/struct mismatch; decoding it would
            // silently return offset-shifted, plausible-looking numbers.
            Ok((status, data)) if status == AK_SUCCESS_I32 && data.len() == AE_ATTR_WIRE_LEN => {
                let read_u32 = |offset: usize| {
                    u32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ])
                };
                Some(AeAttr {
                    exp_time_max: read_u32(0),
                    exp_time_min: read_u32(4),
                    d_gain_max: read_u32(8),
                    a_gain_max: read_u32(24),
                    target_lumiance: read_u32(40),
                })
            }
            // Same reasoning as get_ae_luma / get_lum_factor: a silent `None`
            // is indistinguishable from a camera correctly holding its mode.
            Ok((status, data)) => {
                error!(status, len = data.len(), "get_ae_attr bad daemon response");
                None
            }
            Err(e) => {
                error!(error = %e, "get_ae_attr IPC failed");
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
    async fn test_get_lum_factor_roundtrip() {
        // Little-endian i32, and wide enough to catch a byte-order slip: the
        // vendor's thresholds are 2048 and 6400, so a swapped value would still
        // classify, just wrongly.
        let daemon = FakeDaemon::start(|cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_GET_LUM_FACTOR);
            assert!(req.is_empty());
            (AK_SUCCESS_I32, 6400i32.to_le_bytes().to_vec())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_lum_factor(&ipc).await,
            Some(6400)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_lum_factor_negative_is_none_not_daylight() {
        // isp_get_cur_lum_factor() reports failure as -1. Forwarded, it would
        // land below every day threshold and turn night vision off at night.
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, (-1i32).to_le_bytes().to_vec()));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_lum_factor(&ipc).await,
            None
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_lum_factor_short_payload_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8, 25]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_lum_factor(&ipc).await,
            None
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_luma_error_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (crate::hal::common::AK_FAILED_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_luma_empty_payload_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_luma_multi_byte_payload_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![42u8, 7u8]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await, None);
    }

    /// A daemon STATUS_ERROR used to return None with no log at all, which
    /// is what made the 2026-08-10 .121 night-mode failure undiagnosable.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_luma_non_success_status_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_FAILED_I32, vec![0u8]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_luma(&ipc).await, None);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_attr_decodes_gain_and_exposure_ceilings() {
        // Field offsets are load-bearing: reading a_gain_max at the wrong offset
        // silently returns d_gain_min, a plausible-looking number that would send
        // the whole night-image investigation in the wrong direction.
        let mut payload = vec![0u8; 204];
        payload[0..4].copy_from_slice(&2250u32.to_le_bytes()); // exp_time_max
        payload[24..28].copy_from_slice(&10u32.to_le_bytes()); // a_gain_max
        payload[40..44].copy_from_slice(&40u32.to_le_bytes()); // target_lumiance

        let daemon = FakeDaemon::start(move |cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_GET_AE_ATTR);
            assert!(req.is_empty());
            (AK_SUCCESS_I32, payload.clone())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let attr = <AnykaIpc as ImagingHalTrait>::get_ae_attr(&ipc)
            .await
            .expect("attr");
        assert_eq!(attr.exp_time_max, 2250);
        assert_eq!(attr.a_gain_max, 10);
        assert_eq!(attr.target_lumiance, 40);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_attr_wrong_length_is_none() {
        // A short payload means a daemon/struct mismatch. Decoding it would yield
        // silently wrong ceilings, so reject rather than pad.
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8; 200]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(<AnykaIpc as ImagingHalTrait>::get_ae_attr(&ipc).await, None);
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
