use bytes::BytesMut;
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::mpsc::UnboundedSender;
use std::io;
pub mod tag_type {
    pub const AUDIO: u8 = 8;
    pub const VIDEO: u8 = 9;
    pub const SCRIPT_DATA_AMF: u8 = 18;
}
pub type HttpResponseDataProducer = UnboundedSender<io::Result<BytesMut>>;
pub type HttpResponseDataConsumer = UnboundedReceiver<io::Result<BytesMut>>;

#[cfg(test)]
mod tests {
    use super::tag_type;

    #[test]
    fn test_tag_type_audio() {
        assert_eq!(tag_type::AUDIO, 8);
    }

    #[test]
    fn test_tag_type_video() {
        assert_eq!(tag_type::VIDEO, 9);
    }

    #[test]
    fn test_tag_type_script_data_amf() {
        assert_eq!(tag_type::SCRIPT_DATA_AMF, 18);
    }

    #[test]
    fn test_tag_types_are_distinct() {
        assert_ne!(tag_type::AUDIO, tag_type::VIDEO);
        assert_ne!(tag_type::AUDIO, tag_type::SCRIPT_DATA_AMF);
        assert_ne!(tag_type::VIDEO, tag_type::SCRIPT_DATA_AMF);
    }
}
