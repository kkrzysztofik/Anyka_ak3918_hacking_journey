//! OSD commands over the vendor-daemon control socket.

use std::ffi::c_void;

use tracing::error;

use crate::hal::common::check_result;
use crate::osd::layout::ChannelDims;
use crate::platform::PlatformResult;

use super::{
    AnykaIpc, CMD_OSD_DRAW_STR, CMD_OSD_INIT, CMD_OSD_SET_ENABLE, CMD_OSD_SET_RECT,
    CMD_OSD_SET_STYLE,
};

impl AnykaIpc {
    /// Initialise OSD against an open VI handle.
    ///
    /// Returns per-channel max-rect dimensions `[main, sub]` from the daemon.
    pub fn osd_init(&self, vi_handle: *mut c_void) -> PlatformResult<[ChannelDims; 2]> {
        let req_data = (vi_handle as u64).to_le_bytes().to_vec();
        let (status, resp) = self.send_request(CMD_OSD_INIT, &req_data).map_err(|e| {
            error!(error = %e, "osd_init IPC failed");
            e
        })?;
        check_result(status, "osd_init")?;
        if resp.len() < 16 {
            return Err(crate::platform::PlatformError::HardwareFailure(format!(
                "osd_init: short response ({} bytes)",
                resp.len()
            )));
        }
        let read_i32 = |off: usize| -> i32 {
            i32::from_le_bytes(resp[off..off + 4].try_into().expect("4 bytes"))
        };
        Ok([
            ChannelDims {
                width: read_i32(0),
                height: read_i32(4),
            },
            ChannelDims {
                width: read_i32(8),
                height: read_i32(12),
            },
        ])
    }

    /// Place an OSD rectangle on a channel.
    pub fn osd_set_rect(
        &self,
        vi_handle: *mut c_void,
        channel: i32,
        rect: i32,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> PlatformResult<()> {
        let mut req_data = (vi_handle as u64).to_le_bytes().to_vec();
        for v in [channel, rect, x, y, w, h] {
            req_data.extend_from_slice(&v.to_le_bytes());
        }
        let (status, _) = self
            .send_request(CMD_OSD_SET_RECT, &req_data)
            .map_err(|e| {
                error!(error = %e, "osd_set_rect IPC failed");
                e
            })?;
        check_result(status, "osd_set_rect")
    }

    /// Draw pre-encoded glyph codes into a rectangle.
    pub fn osd_draw_str(
        &self,
        channel: i32,
        rect: i32,
        x: i32,
        y: i32,
        glyphs: &[u16],
    ) -> PlatformResult<()> {
        if glyphs.is_empty() || glyphs.len() > crate::osd::encode::MAX_GLYPHS {
            return Err(crate::platform::PlatformError::InvalidParameter(format!(
                "osd_draw_str: glyph count {} out of range",
                glyphs.len()
            )));
        }
        let mut req_data = Vec::with_capacity(18 + glyphs.len() * 2);
        for v in [channel, rect, x, y] {
            req_data.extend_from_slice(&v.to_le_bytes());
        }
        req_data.extend_from_slice(&(glyphs.len() as u16).to_le_bytes());
        for g in glyphs {
            req_data.extend_from_slice(&g.to_le_bytes());
        }
        let (status, _) = self
            .send_request(CMD_OSD_DRAW_STR, &req_data)
            .map_err(|e| {
                error!(error = %e, "osd_draw_str IPC failed");
                e
            })?;
        check_result(status, "osd_draw_str")
    }

    /// Enable or disable one OSD rectangle.
    pub fn osd_set_enable(&self, channel: i32, rect: i32, enable: bool) -> PlatformResult<()> {
        let mut req_data = Vec::with_capacity(12);
        for v in [channel, rect, i32::from(enable)] {
            req_data.extend_from_slice(&v.to_le_bytes());
        }
        let (status, _) = self
            .send_request(CMD_OSD_SET_ENABLE, &req_data)
            .map_err(|e| {
                error!(error = %e, "osd_set_enable IPC failed");
                e
            })?;
        check_result(status, "osd_set_enable")
    }

    /// Set device-global colour and alpha.
    ///
    /// Rejects out-of-range values before the IPC round trip.
    pub fn osd_set_style(
        &self,
        front_color: i32,
        bg_color: i32,
        edge_color: i32,
        alpha: i32,
    ) -> PlatformResult<()> {
        if !(0..=15).contains(&front_color)
            || !(0..=15).contains(&bg_color)
            || !(0..=15).contains(&edge_color)
            || !(1..=100).contains(&alpha)
        {
            return Err(crate::platform::PlatformError::InvalidParameter(format!(
                "osd_set_style: out of range front={front_color} bg={bg_color} edge={edge_color} alpha={alpha}"
            )));
        }
        let mut req_data = Vec::with_capacity(16);
        for v in [front_color, bg_color, edge_color, alpha] {
            req_data.extend_from_slice(&v.to_le_bytes());
        }
        let (status, _) = self
            .send_request(CMD_OSD_SET_STYLE, &req_data)
            .map_err(|e| {
                error!(error = %e, "osd_set_style IPC failed");
                e
            })?;
        check_result(status, "osd_set_style")
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;
    use crate::hal::anyka::ipc::AnykaIpc;
    use crate::hal::common::AK_SUCCESS_I32;

    #[test]
    fn test_osd_init_returns_channel_dims_from_daemon() {
        let daemon = FakeDaemon::start(|_cmd, _req| {
            let mut reply = Vec::new();
            for v in [1280i32, 720, 640, 360] {
                reply.extend_from_slice(&v.to_le_bytes());
            }
            (AK_SUCCESS_I32, reply)
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let dims = ipc.osd_init(0x1234 as *mut c_void).unwrap();
        assert_eq!(dims[0].width, 1280);
        assert_eq!(dims[1].height, 360);
    }

    #[test]
    fn test_osd_draw_str_encodes_glyphs_little_endian() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = captured.clone();
        let daemon = FakeDaemon::start(move |_cmd, req| {
            *sink.lock().unwrap() = req.to_vec();
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        ipc.osd_draw_str(0, 1, 16, 0, &[0x41, 0x42]).unwrap();

        let req = captured.lock().unwrap().clone();
        // [i32 chn][i32 rect][i32 x][i32 y][u16 count][u16 glyphs...]
        assert_eq!(&req[16..18], &2u16.to_le_bytes());
        assert_eq!(&req[18..22], &[0x41, 0x00, 0x42, 0x00]);
    }

    #[test]
    fn test_osd_set_style_rejects_out_of_range_alpha_before_ipc() {
        let daemon = FakeDaemon::start(|_cmd, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        assert!(ipc.osd_set_style(1, 0, 0, 0).is_err());
        assert!(ipc.osd_set_style(1, 0, 0, 101).is_err());
    }
}
