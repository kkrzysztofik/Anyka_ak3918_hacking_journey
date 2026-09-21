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
use crate::hal::common::imaging::{
    AE_ATTR_WIRE_LEN, AE_RUN_INFO_WIRE_LEN, AWB_STAT_WIRE_LEN, AeAttr, AeRunInfo, ImagingHalTrait,
    MWB_ATTR_WIRE_LEN,
};

use super::{
    AnykaIpc, CMD_ISP_AE_GET_RUN_INFO, CMD_ISP_AE_SET_ATTR, CMD_ISP_AE_SET_MODE,
    CMD_ISP_GET_AE_ATTR, CMD_ISP_GET_AE_LUMA, CMD_ISP_GET_AWB_STAT, CMD_ISP_GET_LUM_FACTOR,
    CMD_ISP_GET_MWB_ATTR, CMD_ISP_SET_BLC, CMD_ISP_SET_BRIGHTNESS, CMD_ISP_SET_CONTRAST,
    CMD_ISP_SET_HUE, CMD_ISP_SET_IR_FILTER, CMD_ISP_SET_MWB_ATTR, CMD_ISP_SET_POWER_HZ,
    CMD_ISP_SET_SATURATION, CMD_ISP_SET_SHARPNESS, CMD_ISP_SET_STYLE_ID, CMD_ISP_SET_WB_TYPE,
    CMD_ISP_SET_WDR,
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

    async fn set_wdr(&self, level: i32) -> i32 {
        let req_data = level.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_WDR, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_wdr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_blc(&self, level: i32) -> i32 {
        let req_data = level.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_BLC, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_blc IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_wb_type(&self, wb_type: u16) -> i32 {
        let value: i32 = wb_type as i32;
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_WB_TYPE, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_wb_type IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_mwb_attr(&self, r_gain: u16, b_gain: u16) -> i32 {
        let mut req_data = Vec::with_capacity(4);
        req_data.extend_from_slice(&r_gain.to_le_bytes());
        req_data.extend_from_slice(&b_gain.to_le_bytes());
        match self.request_async(CMD_ISP_SET_MWB_ATTR, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_mwb_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn get_mwb_attr(&self) -> Option<(u16, u16)> {
        match self.request_async(CMD_ISP_GET_MWB_ATTR, &[]).await {
            // The wire contract is exactly the 12-byte AK_ISP_MWB_ATTR
            // (u16 r_gain, u16 g_gain, u16 b_gain, then three s16 offsets).
            // A different length means a daemon/struct mismatch; decoding it
            // would silently return offset-shifted gains.
            Ok((status, data)) if status == AK_SUCCESS_I32 && data.len() == MWB_ATTR_WIRE_LEN => {
                let r_gain = u16::from_le_bytes([data[0], data[1]]);
                let b_gain = u16::from_le_bytes([data[4], data[5]]);
                Some((r_gain, b_gain))
            }
            // A silent `None` would read as "gains unavailable, keep the cache"
            // and mask a real daemon fault, so say so.
            Ok((status, data)) => {
                error!(status, len = data.len(), "get_mwb_attr bad daemon response");
                None
            }
            Err(e) => {
                error!(error = %e, "get_mwb_attr IPC failed");
                None
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

    async fn get_awb_stat(&self) -> Option<[i32; 10]> {
        match self.request_async(CMD_ISP_GET_AWB_STAT, &[]).await {
            // The wire contract is exactly ten little-endian i32 bins. A short
            // payload means a daemon/struct mismatch; decoding it would
            // silently return offset-shifted, plausible-looking bin counts.
            Ok((status, data)) if status == AK_SUCCESS_I32 && data.len() == AWB_STAT_WIRE_LEN => {
                let mut bins = [0i32; 10];
                for (i, bin) in bins.iter_mut().enumerate() {
                    let off = i * 4;
                    *bin = i32::from_le_bytes([
                        data[off],
                        data[off + 1],
                        data[off + 2],
                        data[off + 3],
                    ]);
                }
                Some(bins)
            }
            // Same reasoning as get_ae_luma / get_lum_factor / get_ae_attr: a
            // silent `None` is indistinguishable from a camera correctly
            // holding its mode, and here specifically from a legitimate
            // all-zero AWB reading, so say so.
            Ok((status, data)) => {
                error!(status, len = data.len(), "get_awb_stat bad daemon response");
                None
            }
            Err(e) => {
                error!(error = %e, "get_awb_stat IPC failed");
                None
            }
        }
    }

    async fn set_ae_attr(&self, a_gain_max: Option<i32>, exp_time_max: Option<i32>) -> i32 {
        // [i32 a_gain_max][i32 exp_time_max]; 0 means "leave alone" daemon-side.
        let req_data = [
            a_gain_max.unwrap_or(0).to_le_bytes(),
            exp_time_max.unwrap_or(0).to_le_bytes(),
        ]
        .concat();
        match self.request_async(CMD_ISP_AE_SET_ATTR, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_ae_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn get_ae_run_info(&self) -> Option<AeRunInfo> {
        match self.request_async(CMD_ISP_AE_GET_RUN_INFO, &[]).await {
            // The wire contract is exactly the 36-byte struct; a short payload
            // means a daemon/struct mismatch and would decode offset-shifted
            // plausible-looking numbers.
            Ok((status, data))
                if status == AK_SUCCESS_I32 && data.len() == AE_RUN_INFO_WIRE_LEN =>
            {
                let read_i32 = |offset: usize| {
                    i32::from_le_bytes([
                        data[offset],
                        data[offset + 1],
                        data[offset + 2],
                        data[offset + 3],
                    ])
                };
                Some(AeRunInfo {
                    avg_lumi: data[0],
                    compensation_lumi: data[1],
                    darked_flag: data[2],
                    a_gain: read_i32(4),
                    d_gain: read_i32(8),
                    isp_d_gain: read_i32(12),
                    exp_time: read_i32(16),
                })
            }
            Ok((status, data)) => {
                error!(
                    status,
                    len = data.len(),
                    "get_ae_run_info bad daemon response"
                );
                None
            }
            Err(e) => {
                error!(error = %e, "get_ae_run_info IPC failed");
                None
            }
        }
    }

    async fn set_hue(&self, value: i32) -> i32 {
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_HUE, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_hue IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_power_hz(&self, hz: u16) -> i32 {
        let req_data = (hz as i32).to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_POWER_HZ, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_power_hz IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_style_id(&self, style_id: u8) -> i32 {
        let req_data = (style_id as i32).to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_SET_STYLE_ID, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_style_id IPC failed");
                AK_FAILED_I32
            }
        }
    }

    async fn set_ae_mode(&self, auto: bool) -> i32 {
        // Driver exp_type: 1 = auto (AE loop runs), 0 = manual (mae applied).
        let value: i32 = if auto { 1 } else { 0 };
        let req_data = value.to_le_bytes().to_vec();
        match self.request_async(CMD_ISP_AE_SET_MODE, &req_data).await {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "set_ae_mode IPC failed");
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_awb_stat_roundtrip() {
        let mut payload = Vec::with_capacity(40);
        let bins: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        for b in bins {
            payload.extend_from_slice(&b.to_le_bytes());
        }
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_GET_AWB_STAT);
            assert!(req.is_empty());
            (AK_SUCCESS_I32, payload.clone())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_awb_stat(&ipc).await,
            Some(bins)
        );
    }

    /// All-zero bins are a legitimate AWB reading (AWB going quiet under IR)
    /// and must round-trip as `Some([0; 10])`, never collapse to `None`.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_awb_stat_all_zero_bins_is_some_not_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8; 40]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_awb_stat(&ipc).await,
            Some([0i32; 10])
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_awb_stat_wrong_length_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8; 36]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_awb_stat(&ipc).await,
            None
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_awb_stat_error_status_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_FAILED_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_awb_stat(&ipc).await,
            None
        );
    }

    /// set_wb_type sends the type as a little-endian i32, as the daemon
    /// expects for `[i32]` commands.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_wb_type_sends_i32_wire_format() {
        let daemon = FakeDaemon::start(|cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_SET_WB_TYPE);
            assert_eq!(req, &1i32.to_le_bytes()[..]); // WB_TYPE_AUTO
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::set_wb_type(&ipc, 1).await,
            AK_SUCCESS_I32
        );
    }

    /// set_mwb_attr packs `[u16 r_gain][u16 b_gain]` little-endian; the byte
    /// order is load-bearing, so a swapped pair must fail this assertion.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_mwb_attr_sends_u16_pair_wire_format() {
        let daemon = FakeDaemon::start(|cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_SET_MWB_ATTR);
            let expected = {
                let mut v = Vec::new();
                v.extend_from_slice(&2u16.to_le_bytes());
                v.extend_from_slice(&7u16.to_le_bytes());
                v
            };
            assert_eq!(req, &expected[..]);
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::set_mwb_attr(&ipc, 2, 7).await,
            AK_SUCCESS_I32
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_mwb_attr_decodes_gains_from_raw_struct() {
        // r_gain and b_gain sit at the head of the 12-byte AK_ISP_MWB_ATTR;
        // g_gain and the offsets must be ignored, not mis-decoded.
        let mut payload = vec![0u8; 12];
        payload[0..2].copy_from_slice(&11u16.to_le_bytes()); // r_gain
        payload[2..4].copy_from_slice(&12u16.to_le_bytes()); // g_gain
        payload[4..6].copy_from_slice(&13u16.to_le_bytes()); // b_gain
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_GET_MWB_ATTR);
            assert!(req.is_empty());
            (AK_SUCCESS_I32, payload.clone())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_mwb_attr(&ipc).await,
            Some((11, 13))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_mwb_attr_wrong_length_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8; 8]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_mwb_attr(&ipc).await,
            None
        );
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_ae_attr_sends_both_ceilings_le() {
        let daemon = FakeDaemon::start(|cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_AE_SET_ATTR);
            let want = [24i32.to_le_bytes(), 2250i32.to_le_bytes()].concat();
            assert_eq!(
                &req[..],
                want.as_slice(),
                "AE attr payload is [i32][i32] LE"
            );
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::set_ae_attr(&ipc, Some(24), Some(2250)).await,
            AK_SUCCESS_I32
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_ae_attr_none_sends_zeros() {
        // 0 is the daemon-side "leave alone" sentinel, so a None field must
        // travel as an explicit 0, not be dropped from the payload.
        let daemon = FakeDaemon::start(|cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_AE_SET_ATTR);
            assert_eq!(req.len(), 8);
            assert!(req.iter().all(|&b| b == 0), "both fields left alone");
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::set_ae_attr(&ipc, None, None).await,
            AK_SUCCESS_I32
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_run_info_roundtrip() {
        // Layout: 3 status bytes + pad, then i32 a_gain / d_gain / isp_d_gain /
        // exp_time, then four ignored u32 step fields. Distinct values catch
        // any offset slip.
        let mut payload = vec![42u8, 40, 1, 0];
        for v in [1000i32, 256, 64, 120] {
            payload.extend_from_slice(&v.to_le_bytes());
        }
        for _ in 0..4 {
            payload.extend_from_slice(&0i32.to_le_bytes());
        }
        assert_eq!(payload.len(), AE_RUN_INFO_WIRE_LEN);
        let daemon = FakeDaemon::start(move |cmd_id, req| {
            assert_eq!(cmd_id, CMD_ISP_AE_GET_RUN_INFO);
            assert!(req.is_empty());
            (AK_SUCCESS_I32, payload.clone())
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_ae_run_info(&ipc).await,
            Some(AeRunInfo {
                avg_lumi: 42,
                compensation_lumi: 40,
                darked_flag: 1,
                a_gain: 1000,
                d_gain: 256,
                isp_d_gain: 64,
                exp_time: 120,
            })
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_get_ae_run_info_short_payload_is_none() {
        let daemon = FakeDaemon::start(|_c, _r| (AK_SUCCESS_I32, vec![0u8; 8]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        assert_eq!(
            <AnykaIpc as ImagingHalTrait>::get_ae_run_info(&ipc).await,
            None
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_set_ae_mode_encodes_exp_type() {
        let daemon = FakeDaemon::start(|cmd_id, _req| {
            assert_eq!(cmd_id, CMD_ISP_AE_SET_MODE);
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);
        // AUTO first: the driver's exp_type is 1 = auto, 0 = manual.
        let status = <AnykaIpc as ImagingHalTrait>::set_ae_mode(&ipc, true).await;
        assert_eq!(status, AK_SUCCESS_I32);
        let status = <AnykaIpc as ImagingHalTrait>::set_ae_mode(&ipc, false).await;
        assert_eq!(status, AK_SUCCESS_I32);
    }
}
