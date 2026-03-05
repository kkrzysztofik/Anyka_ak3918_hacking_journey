//! Stub types for testing without actual hardware.
//!
//! These types mirror the Anyka SDK types but are available
//! on all platforms for testing purposes.

/// Return codes matching Anyka SDK
pub const AK_SUCCESS: i32 = 0;
pub const AK_FAILED: i32 = -1;
pub const AK_FALSE: i32 = 0;
pub const AK_TRUE: i32 = 1;

/// Video device type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDevType {
    Dev0 = 0,
}

/// Video resolution
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VideoResolution {
    pub width: i32,
    pub height: i32,
    pub max_width: i32,
    pub max_height: i32,
}

/// Encode group type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum EncodeGroupType {
    #[default]
    ENCODE_RECORD = 0,
    ENCODE_MAINCHN_NET = 1,
    ENCODE_SUBCHN_NET = 2,
    ENCODE_PICTURE = 3,
    ENCODE_GRP_NUM = 4,
}

/// Encode use channel
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum EncodeUseChn {
    #[default]
    ENCODE_MAIN_CHN = 0,
    ENCODE_SUB_CHN = 1,
}

/// Encode output type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum EncodeOutputType {
    #[default]
    H264_ENC_TYPE = 0,
    MJPEG_ENC_TYPE = 1,
    HEVC_ENC_TYPE = 2,
}

/// Bitrate control mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum BitrateCtrlMode {
    #[default]
    BR_MODE_CBR = 0,
    BR_MODE_VBR = 1,
}

/// Profile mode
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(non_camel_case_types)]
pub enum ProfileMode {
    #[default]
    PROFILE_MAIN = 0,
    PROFILE_HIGH = 1,
    PROFILE_BASE = 2,
    PROFILE_HEVC_MAIN = 3,
    PROFILE_HEVC_MAIN_STILL = 4,
}

/// Encode parameters
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct EncodeParam {
    pub width: u32,
    pub height: u32,
    pub minqp: i32,
    pub maxqp: i32,
    pub fps: i32,
    pub goplen: i32,
    pub bps: i32,
    pub profile: ProfileMode,
    pub use_chn: EncodeUseChn,
    pub enc_grp: EncodeGroupType,
    pub br_mode: BitrateCtrlMode,
    pub enc_out_type: EncodeOutputType,
}

/// Audio parameters
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AudioParam {
    pub sample_rate: u32,
    pub channel_num: u32,
    pub sample_bits: u32,
    pub type_: i32,
}

/// PTZ device
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PtzDevice {
    PTZ_DEV_H = 0,
    PTZ_DEV_V = 1,
}

/// PTZ feedback pin type
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PtzFeedbackPin {
    PTZ_FEEDBACK_PIN_NONE = 0,
    PTZ_FEEDBACK_PIN_EXIST = 1,
}

/// PTZ turn direction
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PtzTurnDirection {
    PTZ_TURN_RESERVED = 0,
    PTZ_TURN_LEFT = 1,
    PTZ_TURN_RIGHT = 2,
    PTZ_TURN_UP = 3,
    PTZ_TURN_DOWN = 4,
}

/// Video channel attributes
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct VideoChannelAttr {
    pub crop: CropInfo,
    pub res: [VideoResolution; 2], // VIDEO_CHN_NUM = 2 (MAIN, SUB)
}

/// Crop information
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct CropInfo {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

/// PCM parameters for audio input
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PcmParam {
    pub sample_rate: u32,
    pub sample_bits: u32,
    pub channel_num: u32,
}

/// Audio encoder attributes
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct AencAttr {
    pub aac_head: i32, // enum aenc_aac_attr
}

/// Video frame type (matches SDK `enum video_frame_type` in ak_global.h).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoFrameType {
    FrameTypeP = 0,
    FrameTypeI = 1,
    FrameTypeB = 2,
    FrameTypePi = 3,
}

/// Encoded video stream data (matches SDK `struct video_stream` in ak_global.h).
#[repr(C)]
pub struct VideoStream {
    pub data: *mut u8,
    pub len: u32,
    pub ts: u64,
    pub seq_no: std::os::raw::c_ulong,
    pub frame_type: VideoFrameType,
}

// Type aliases for consistency with generated bindings (snake_case)
#[allow(non_camel_case_types)]
pub type video_channel_attr = VideoChannelAttr;
#[allow(non_camel_case_types)]
pub type pcm_param = PcmParam;
#[allow(non_camel_case_types)]
pub type aenc_attr = AencAttr;
#[allow(non_camel_case_types)]
pub type encode_param = EncodeParam;
#[allow(non_camel_case_types)]
pub type video_dev_type = VideoDevType;
#[allow(non_camel_case_types)]
pub type video_resolution = VideoResolution;
#[allow(non_camel_case_types)]
pub type audio_param = AudioParam;
#[allow(non_camel_case_types)]
pub type ptz_device = PtzDevice;
#[allow(non_camel_case_types)]
pub type ptz_feedback_pin = PtzFeedbackPin;
#[allow(non_camel_case_types)]
pub type ptz_turn_direction = PtzTurnDirection;
#[allow(non_camel_case_types)]
pub type profile_mode = ProfileMode;
#[allow(non_camel_case_types)]
pub type encode_use_chn = EncodeUseChn;
#[allow(non_camel_case_types)]
pub type encode_group_type = EncodeGroupType;
#[allow(non_camel_case_types)]
pub type bitrate_ctrl_mode = BitrateCtrlMode;
#[allow(non_camel_case_types)]
pub type encode_output_type = EncodeOutputType;
#[allow(non_camel_case_types)]
pub type video_frame_type = VideoFrameType;
#[allow(non_camel_case_types)]
pub type video_stream = VideoStream;
