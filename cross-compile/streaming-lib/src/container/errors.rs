use {
    crate::bytesio::bits_errors::BitError,
    crate::bytesio::bytes_errors::{BytesReadError, BytesWriteError},
    crate::codec::errors::H264Error,
    thiserror::Error,
};

#[derive(Debug, Error)]
pub enum TagParseErrorValue {
    #[error(transparent)]
    BytesReadError(#[from] BytesReadError),
    #[error("tag data length error")]
    TagDataLength,
    #[error("unknown tag type error")]
    UnknownTagType,
}
#[derive(Debug, Error)]
#[error(transparent)]
pub struct TagParseError(#[from] pub TagParseErrorValue);

impl From<BytesReadError> for TagParseError {
    fn from(error: BytesReadError) -> Self {
        TagParseError(TagParseErrorValue::BytesReadError(error))
    }
}
#[derive(Debug, Error)]
pub enum MuxerErrorValue {
    #[error(transparent)]
    BytesWriteError(#[from] BytesWriteError),
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct FlvMuxerError {
    pub value: MuxerErrorValue,
}

impl From<MuxerErrorValue> for FlvMuxerError {
    fn from(value: MuxerErrorValue) -> Self {
        FlvMuxerError { value }
    }
}

impl From<BytesWriteError> for FlvMuxerError {
    fn from(error: BytesWriteError) -> Self {
        FlvMuxerError {
            value: MuxerErrorValue::BytesWriteError(error),
        }
    }
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct FlvDemuxerError {
    pub value: DemuxerErrorValue,
}

impl From<DemuxerErrorValue> for FlvDemuxerError {
    fn from(value: DemuxerErrorValue) -> Self {
        FlvDemuxerError { value }
    }
}

impl From<BytesWriteError> for FlvDemuxerError {
    fn from(error: BytesWriteError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::BytesWriteError(error),
        }
    }
}

impl From<BytesReadError> for FlvDemuxerError {
    fn from(error: BytesReadError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::BytesReadError(error),
        }
    }
}

impl From<Mpeg4AvcHevcError> for FlvDemuxerError {
    fn from(error: Mpeg4AvcHevcError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::MpegAvcError(error),
        }
    }
}

impl From<MpegAacError> for FlvDemuxerError {
    fn from(error: MpegAacError) -> Self {
        FlvDemuxerError {
            value: DemuxerErrorValue::MpegAacError(error),
        }
    }
}

#[derive(Debug, Error)]
pub enum DemuxerErrorValue {
    #[error(transparent)]
    BytesWriteError(#[from] BytesWriteError),
    #[error(transparent)]
    BytesReadError(#[from] BytesReadError),
    #[error(transparent)]
    MpegAvcError(#[from] Mpeg4AvcHevcError),
    #[error(transparent)]
    MpegAacError(#[from] MpegAacError),
    #[error("invalid flv header: {reason}")]
    InvalidFlvHeader { reason: String },
}

#[derive(Debug, Error)]
pub enum MpegErrorValue {
    #[error(transparent)]
    BytesReadError(#[from] BytesReadError),
    #[error(transparent)]
    BytesWriteError(#[from] BytesWriteError),
    #[error(transparent)]
    BitError(#[from] BitError),
    #[error(transparent)]
    H264Error(#[from] H264Error),
    #[error("there is not enough bits to read")]
    NotEnoughBitsToRead,
    #[error("should not come here")]
    ShouldNotComeHere,
    #[error("the sps nal unit type is not correct")]
    SPSNalunitTypeNotCorrect,
    #[error("not supported sampling frequency")]
    NotSupportedSamplingFrequency,
}
#[derive(Debug, Error)]
#[error("{value}")]
pub struct Mpeg4AvcHevcError {
    pub value: MpegErrorValue,
}

impl From<MpegErrorValue> for Mpeg4AvcHevcError {
    fn from(value: MpegErrorValue) -> Self {
        Mpeg4AvcHevcError { value }
    }
}

impl From<BytesReadError> for Mpeg4AvcHevcError {
    fn from(error: BytesReadError) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for Mpeg4AvcHevcError {
    fn from(error: BytesWriteError) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::BytesWriteError(error),
        }
    }
}

impl From<BitError> for Mpeg4AvcHevcError {
    fn from(error: BitError) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::BitError(error),
        }
    }
}

impl From<H264Error> for Mpeg4AvcHevcError {
    fn from(error: H264Error) -> Self {
        Mpeg4AvcHevcError {
            value: MpegErrorValue::H264Error(error),
        }
    }
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct MpegAacError {
    pub value: MpegErrorValue,
}

impl From<MpegErrorValue> for MpegAacError {
    fn from(value: MpegErrorValue) -> Self {
        MpegAacError { value }
    }
}

impl From<BytesReadError> for MpegAacError {
    fn from(error: BytesReadError) -> Self {
        MpegAacError {
            value: MpegErrorValue::BytesReadError(error),
        }
    }
}

impl From<BytesWriteError> for MpegAacError {
    fn from(error: BytesWriteError) -> Self {
        MpegAacError {
            value: MpegErrorValue::BytesWriteError(error),
        }
    }
}

impl From<BitError> for MpegAacError {
    fn from(error: BitError) -> Self {
        MpegAacError {
            value: MpegErrorValue::BitError(error),
        }
    }
}

#[derive(Debug, Error)]
pub enum BitVecErrorValue {
    #[error("not enough bits left")]
    NotEnoughBits,
}

#[derive(Debug, Error)]
#[error("{value}")]
pub struct BitVecError {
    pub value: BitVecErrorValue,
}

impl From<BitVecErrorValue> for BitVecError {
    fn from(value: BitVecErrorValue) -> Self {
        BitVecError { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_errors::{BytesReadError, BytesReadErrorValue};
    use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};

    // ========== TagParseErrorValue Display Tests ==========

    #[test]
    fn test_tag_parse_error_value_tag_data_length_display() {
        let err = TagParseErrorValue::TagDataLength;
        assert_eq!(format!("{}", err), "tag data length error");
    }

    #[test]
    fn test_tag_parse_error_value_unknown_tag_type_display() {
        let err = TagParseErrorValue::UnknownTagType;
        assert_eq!(format!("{}", err), "unknown tag type error");
    }

    // ========== TagParseError From Trait Tests ==========

    #[test]
    fn test_tag_parse_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err: TagParseError = read_err.into();
        assert!(matches!(err.0, TagParseErrorValue::BytesReadError(_)));
    }

    // ========== TagParseError Display Tests ==========

    #[test]
    fn test_tag_parse_error_display() {
        let err = TagParseError(TagParseErrorValue::TagDataLength);
        assert_eq!(format!("{}", err), "tag data length error");
    }

    // ========== MuxerErrorValue Display Tests ==========

    #[test]
    fn test_muxer_error_value_bytes_write_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = MuxerErrorValue::BytesWriteError(write_err);
        assert_eq!(format!("{}", err), "bytes write error");
    }

    // ========== FlvMuxerError From Trait Tests ==========

    #[test]
    fn test_flv_muxer_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let err: FlvMuxerError = write_err.into();
        assert!(matches!(err.value, MuxerErrorValue::BytesWriteError(_)));
    }

    // ========== FlvMuxerError Display Tests ==========

    #[test]
    fn test_flv_muxer_error_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = FlvMuxerError {
            value: MuxerErrorValue::BytesWriteError(write_err),
        };
        assert_eq!(format!("{}", err), "bytes write error");
    }

    // ========== DemuxerErrorValue Display Tests ==========

    #[test]
    fn test_demuxer_error_value_bytes_read_display() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        let err = DemuxerErrorValue::BytesReadError(read_err);
        assert!(format!("{}", err).contains("bytes read error"));
    }

    #[test]
    fn test_demuxer_error_value_bytes_write_display() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = DemuxerErrorValue::BytesWriteError(write_err);
        assert!(format!("{}", err).contains("bytes write error"));
    }

    // ========== FlvDemuxerError From Trait Tests ==========

    #[test]
    fn test_flv_demuxer_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::NotEnoughBytes,
        };
        let err: FlvDemuxerError = read_err.into();
        assert!(matches!(err.value, DemuxerErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_flv_demuxer_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err: FlvDemuxerError = write_err.into();
        assert!(matches!(err.value, DemuxerErrorValue::BytesWriteError(_)));
    }

    // ========== MpegErrorValue Display Tests ==========

    #[test]
    fn test_mpeg_error_value_not_enough_bits_display() {
        let err = MpegErrorValue::NotEnoughBitsToRead;
        assert_eq!(format!("{}", err), "there is not enough bits to read");
    }

    #[test]
    fn test_mpeg_error_value_should_not_come_here_display() {
        let err = MpegErrorValue::ShouldNotComeHere;
        assert_eq!(format!("{}", err), "should not come here");
    }

    #[test]
    fn test_mpeg_error_value_sps_nalunit_type_display() {
        let err = MpegErrorValue::SPSNalunitTypeNotCorrect;
        assert_eq!(format!("{}", err), "the sps nal unit type is not correct");
    }

    #[test]
    fn test_mpeg_error_value_not_supported_sampling_frequency_display() {
        let err = MpegErrorValue::NotSupportedSamplingFrequency;
        assert_eq!(format!("{}", err), "not supported sampling frequency");
    }

    // ========== Mpeg4AvcHevcError From Trait Tests ==========

    #[test]
    fn test_mpeg4_avc_hevc_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        let err: Mpeg4AvcHevcError = read_err.into();
        assert!(matches!(err.value, MpegErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_mpeg4_avc_hevc_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err: Mpeg4AvcHevcError = write_err.into();
        assert!(matches!(err.value, MpegErrorValue::BytesWriteError(_)));
    }

    // ========== MpegAacError From Trait Tests ==========

    #[test]
    fn test_mpeg_aac_error_from_bytes_read_error() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::IndexOutofRange,
        };
        let err: MpegAacError = read_err.into();
        assert!(matches!(err.value, MpegErrorValue::BytesReadError(_)));
    }

    #[test]
    fn test_mpeg_aac_error_from_bytes_write_error() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::OutofIndex,
        };
        let err: MpegAacError = write_err.into();
        assert!(matches!(err.value, MpegErrorValue::BytesWriteError(_)));
    }

    // ========== BitVecErrorValue Display Tests ==========

    #[test]
    fn test_bit_vec_error_value_not_enough_bits_display() {
        let err = BitVecErrorValue::NotEnoughBits;
        assert_eq!(format!("{}", err), "not enough bits left");
    }

    // ========== Debug Trait Tests ==========

    #[test]
    fn test_tag_parse_error_debug() {
        let err = TagParseError(TagParseErrorValue::UnknownTagType);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("TagParseError"));
        assert!(debug_str.contains("UnknownTagType"));
    }

    #[test]
    fn test_flv_muxer_error_debug() {
        let write_err = BytesWriteError {
            value: BytesWriteErrorValue::Timeout,
        };
        let err = FlvMuxerError {
            value: MuxerErrorValue::BytesWriteError(write_err),
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("FlvMuxerError"));
    }

    #[test]
    fn test_flv_demuxer_error_debug() {
        let read_err = BytesReadError {
            value: BytesReadErrorValue::EmptyStream,
        };
        let err = FlvDemuxerError {
            value: DemuxerErrorValue::BytesReadError(read_err),
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("FlvDemuxerError"));
    }

    #[test]
    fn test_mpeg4_avc_hevc_error_debug() {
        let err = Mpeg4AvcHevcError {
            value: MpegErrorValue::SPSNalunitTypeNotCorrect,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("Mpeg4AvcHevcError"));
    }

    #[test]
    fn test_mpeg_aac_error_debug() {
        let err = MpegAacError {
            value: MpegErrorValue::NotSupportedSamplingFrequency,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("MpegAacError"));
    }

    #[test]
    fn test_bit_vec_error_debug() {
        let err = BitVecError {
            value: BitVecErrorValue::NotEnoughBits,
        };
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("BitVecError"));
        assert!(debug_str.contains("NotEnoughBits"));
    }
}
