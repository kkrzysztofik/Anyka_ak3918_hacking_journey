//! AudioHalTrait implementation for AnykaIpc.

use std::ffi::c_void;
use tracing::error;

use crate::hal::common::audio::AudioHalTrait;
use crate::hal::common::{AK_FAILED_I32, aenc_attr, audio_param, pcm_param};
use crate::platform::{PlatformError, PlatformResult};

use super::{
    AnykaIpc, CMD_AENC_CLOSE, CMD_AENC_OPEN, CMD_AENC_SET_ATTR, CMD_AI_CLOSE, CMD_AI_OPEN,
    CMD_AI_SET_ADC_VOLUME, CMD_AI_SET_ASLC_VOLUME, CMD_AUDIO_PLAY, VD_STATUS_BUSY,
    VD_STATUS_STALE_EPOCH,
};

impl AudioHalTrait for AnykaIpc {
    fn ai_open(&self, param: *const pcm_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to a
        // `pcm_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_pcm_param_buf(&*param) };
        match self.send_handle_request(CMD_AI_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "ai_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn ai_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_AI_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn ai_set_adc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&vol.to_le_bytes());
        match self.send_request(CMD_AI_SET_ADC_VOLUME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_set_adc_volume IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn ai_set_aslc_volume(&self, handle: *mut c_void, vol: i32) -> i32 {
        let handle_val = handle as u64;
        let mut req_data = handle_val.to_le_bytes().to_vec();
        req_data.extend_from_slice(&vol.to_le_bytes());
        match self.send_request(CMD_AI_SET_ASLC_VOLUME, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "ai_set_aslc_volume IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn aenc_open(&self, param: *const audio_param) -> *mut c_void {
        // SAFETY: caller guarantees `param` is a valid, non-null pointer to an
        // `audio_param` that remains valid for the duration of this call.
        let (req_buf, req_len) = unsafe { Self::encode_audio_param_buf(&*param) };
        match self.send_handle_request(CMD_AENC_OPEN, &req_buf[..req_len]) {
            Ok(handle) => handle,
            Err(e) => {
                error!(error = %e, "aenc_open IPC failed");
                std::ptr::null_mut()
            }
        }
    }

    fn aenc_close(&self, handle: *mut c_void) -> i32 {
        let handle_val = handle as u64;
        let req_data = handle_val.to_le_bytes().to_vec();
        match self.send_request(CMD_AENC_CLOSE, &req_data) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "aenc_close IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn aenc_set_attr(&self, handle: *mut c_void, attr: *const aenc_attr) -> i32 {
        let handle_val = handle as u64;
        let mut req_buf = [0u8; 16]; // 8 bytes handle + 8 bytes padding for alignment
        req_buf[0..8].copy_from_slice(&handle_val.to_le_bytes());
        // SAFETY: caller guarantees `attr` is a valid, non-null pointer to an
        // `aenc_attr` that remains valid for the duration of this call.
        let (attr_buf, attr_len) = unsafe { Self::encode_aenc_attr_buf(&*attr) };
        req_buf[8..8 + attr_len].copy_from_slice(&attr_buf[..attr_len]);
        match self.send_request(CMD_AENC_SET_ATTR, &req_buf[..8 + attr_len]) {
            Ok((status, _)) => status,
            Err(e) => {
                error!(error = %e, "aenc_set_attr IPC failed");
                AK_FAILED_I32
            }
        }
    }

    fn start_audio_push(&self, sample_rate: u32, channels: u32) -> PlatformResult<()> {
        // Fully-qualified so a future refactor cannot turn this into infinite
        // recursion (the inherent method has the same name).
        AnykaIpc::start_audio_push(self, sample_rate, channels)
    }

    fn stop_audio_push(&self) -> PlatformResult<()> {
        AnykaIpc::stop_audio_push(self)
    }
}

/// Result of `CMD_AUDIO_PLAY`: accepted for async playback, or busy (dropped).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioPlayStatus {
    Accepted,
    Busy,
}

impl AnykaIpc {
    /// Ask the daemon to play a raw PCM file on the speaker.
    ///
    /// Wire: `[u32 rate][u32 ch][i32 volume][u32 path_len][path\0]`.
    /// `VD_STATUS_BUSY` is a normal outcome, not an error.
    pub fn audio_play(
        &self,
        path: &str,
        sample_rate: u32,
        channels: u32,
        volume: i32,
    ) -> PlatformResult<AudioPlayStatus> {
        let path_bytes = path.as_bytes();
        let path_len = (path_bytes.len() + 1) as u32; // include NUL
        let mut req = Vec::with_capacity(16 + path_len as usize);
        req.extend_from_slice(&sample_rate.to_le_bytes());
        req.extend_from_slice(&channels.to_le_bytes());
        req.extend_from_slice(&volume.to_le_bytes());
        req.extend_from_slice(&path_len.to_le_bytes());
        req.extend_from_slice(path_bytes);
        req.push(0);

        let (status, _) = self.send_request(CMD_AUDIO_PLAY, &req)?;
        if status == VD_STATUS_STALE_EPOCH {
            return Err(Self::stale_epoch_error(CMD_AUDIO_PLAY));
        }
        if status == VD_STATUS_BUSY {
            return Ok(AudioPlayStatus::Busy);
        }
        if status != crate::hal::common::AK_SUCCESS_I32 {
            return Err(PlatformError::HardwareFailure(format!(
                "CMD_AUDIO_PLAY failed with status {status}"
            )));
        }
        Ok(AudioPlayStatus::Accepted)
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_helpers::FakeDaemon;
    use super::super::{AnykaIpc, CMD_AUDIO_PLAY, VD_STATUS_BUSY};
    use super::*;
    use crate::hal::common::AK_SUCCESS_I32;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_audio_play_encodes_wire_and_accepts() {
        let captured = Arc::new(Mutex::new((0i32, Vec::new())));
        let sink = Arc::clone(&captured);
        let daemon = FakeDaemon::start(move |cmd, req| {
            *sink.lock().unwrap() = (cmd, req.to_vec());
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let status = ipc
            .audio_play("/tmp/a.raw", 8000, 1, 3)
            .expect("play should succeed");
        assert_eq!(status, AudioPlayStatus::Accepted);

        let (cmd, req) = captured.lock().unwrap().clone();
        assert_eq!(cmd, CMD_AUDIO_PLAY);
        assert_eq!(&req[0..4], &8000u32.to_le_bytes());
        assert_eq!(&req[4..8], &1u32.to_le_bytes());
        assert_eq!(&req[8..12], &3i32.to_le_bytes());
        let path_len = u32::from_le_bytes(req[12..16].try_into().unwrap());
        assert_eq!(path_len, 11); // "/tmp/a.raw\0"
        assert_eq!(&req[16..27], b"/tmp/a.raw\0");
    }

    #[test]
    fn test_audio_play_busy_is_ok_outcome() {
        let daemon = FakeDaemon::start(|_cmd, _req| (VD_STATUS_BUSY, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let status = ipc
            .audio_play("/tmp/a.raw", 8000, 1, 3)
            .expect("busy is not an error");
        assert_eq!(status, AudioPlayStatus::Busy);
    }
}
