use crate::container::define::{AacProfile, AvcCodecId, AvcLevel, AvcProfile, SoundFormat};
use chrono::{DateTime, Local};
use serde::Deserialize;
use serde_json::Value;

use super::utils;

use {
    super::errors::StreamHubError,
    super::statistics::StatisticsStream,
    super::stream::StreamIdentifier,
    async_trait::async_trait,
    bytes::BytesMut,
    serde::Serialize,
    serde::Serializer,
    serde::ser::SerializeStruct,
    std::fmt,
    std::sync::Arc,
    tokio::sync::{broadcast, mpsc, oneshot},
    utils::Uuid,
};

/* Subscribe streams from stream hub */
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum SubscribeType {
    /* Remote client request pulling(play) a rtsp stream.*/
    RtspPull,
    /* Remote client request pulling(play) http-flv stream.*/
    HttpFlvPull,
}

/* Publish streams to stream hub */
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum PublishType {
    /* Receive rtsp stream from remote push client */
    RtspPush,
}

#[derive(Debug, Serialize, Clone)]
pub struct NotifyInfo {
    pub request_url: String,
    pub remote_addr: String,
}

#[derive(Debug, Clone)]
pub struct SubscriberInfo {
    pub id: Uuid,
    pub sub_type: SubscribeType,
    pub notify_info: NotifyInfo,
    pub sub_data_type: SubDataType,
}

impl Serialize for SubscriberInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("SubscriberInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("sub_type", &self.sub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Debug, Clone)]
pub struct PublisherInfo {
    pub id: Uuid,
    pub pub_type: PublishType,
    pub pub_data_type: PubDataType,
    pub notify_info: NotifyInfo,
}

impl Serialize for PublisherInfo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // 3 is the number of fields in the struct.
        let mut state = serializer.serialize_struct("PublisherInfo", 3)?;

        state.serialize_field("id", &self.id.to_string())?;
        state.serialize_field("pub_type", &self.pub_type)?;
        state.serialize_field("notify_info", &self.notify_info)?;
        state.end()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum VideoCodecType {
    H264,
    H265,
}

#[derive(Clone, Debug)]
pub struct MediaInfo {
    pub audio_clock_rate: u32,
    pub video_clock_rate: u32,
    pub vcodec: VideoCodecType,
}

#[derive(Clone, Debug)]
pub enum FrameData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
    MetaData { timestamp: u32, data: BytesMut },
    MediaInfo { media_info: MediaInfo },
}

//Used to pass rtp raw data.
#[derive(Clone)]
pub enum PacketData {
    Video { timestamp: u32, data: BytesMut },
    Audio { timestamp: u32, data: BytesMut },
}

//used to save data which needs to be transferred between client/server sessions
#[derive(Clone)]
pub enum Information {
    Sdp { data: String },
}

//used to transfer a/v frame between different protocols(rtmp/rtsp/webrtc/http-flv/hls)
//or send a/v frame data from publisher to subscribers.
pub type FrameDataSender = mpsc::UnboundedSender<FrameData>;
pub type FrameDataReceiver = mpsc::UnboundedReceiver<FrameData>;

//used to transfer rtp packet data,it includles the following directions:
// rtsp(publisher)->stream hub->rtsp(subscriber)
// webrtc(publisher whip)->stream hub->webrtc(subscriber whep)
pub type PacketDataSender = mpsc::UnboundedSender<PacketData>;
pub type PacketDataReceiver = mpsc::UnboundedReceiver<PacketData>;

pub type InformationSender = mpsc::UnboundedSender<Information>;
pub type InformationReceiver = mpsc::UnboundedReceiver<Information>;

pub type StreamHubEventSender = mpsc::UnboundedSender<StreamHubEvent>;
pub type StreamHubEventReceiver = mpsc::UnboundedReceiver<StreamHubEvent>;

pub type BroadcastEventSender = broadcast::Sender<BroadcastEvent>;
pub type BroadcastEventReceiver = broadcast::Receiver<BroadcastEvent>;

pub type TransceiverEventSender = mpsc::UnboundedSender<TransceiverEvent>;
pub type TransceiverEventReceiver = mpsc::UnboundedReceiver<TransceiverEvent>;

pub type StatisticDataSender = mpsc::UnboundedSender<StatisticData>;
pub type StatisticDataReceiver = mpsc::UnboundedReceiver<StatisticData>;

pub type StatisticStreamSender = mpsc::UnboundedSender<StatisticsStream>;
pub type StatisticStreamReceiver = mpsc::UnboundedReceiver<StatisticsStream>;

pub type StatisticApiResultSender = oneshot::Sender<Value>;
pub type StatisticApiResultReceiver = oneshot::Receiver<Value>;

pub type SubEventExecuteResultSender =
    oneshot::Sender<Result<(DataReceiver, Option<StatisticDataSender>), StreamHubError>>;
pub type PubEventExecuteResultSender = oneshot::Sender<
    Result<
        (
            Option<FrameDataSender>,
            Option<PacketDataSender>,
            Option<StatisticDataSender>,
        ),
        StreamHubError,
    >,
>;
// The trait bound `BroadcastEvent: Clone` should be satisfied, so here we cannot use oneshot.
pub type BroadcastEventExecuteResultSender = mpsc::Sender<Result<(), StreamHubError>>;
pub type ApiRelayStreamResultSender = oneshot::Sender<Result<(), StreamHubError>>;
pub type TransceiverEventExecuteResultSender = oneshot::Sender<StatisticDataSender>;

#[async_trait]
pub trait TStreamHandler: Send + Sync {
    async fn send_prior_data(
        &self,
        sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError>;
    async fn get_statistic_data(&self) -> Option<StatisticsStream>;
    async fn send_information(&self, sender: InformationSender);
}

//A publisher can publish one or two kinds of av stream at a time.
pub struct DataReceiver {
    pub frame_receiver: Option<FrameDataReceiver>,
    pub packet_receiver: Option<PacketDataReceiver>,
}

//A subscriber only needs to subscribe to one type of stream at a time
#[derive(Debug, Clone)]
pub enum DataSender {
    Frame { sender: FrameDataSender },
    Packet { sender: PacketDataSender },
}
//we can only sub one kind of stream.
#[derive(Debug, Clone, Serialize)]
pub enum SubDataType {
    Frame,
    Packet,
}
//we can pub frame or packet or both.
#[derive(Debug, Clone, Serialize)]
pub enum PubDataType {
    Frame,
    Packet,
    Both,
}

#[derive(Clone, Serialize, Debug)]
pub enum StreamHubEventMessage {
    Subscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    UnSubscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    Publish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    UnPublish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    NotSupport {},
}

//we can pub frame or packet or both.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelayType {
    Pull,
    Push,
}

#[derive(Serialize)]
pub enum StreamHubEvent {
    Subscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
        #[serde(skip_serializing)]
        result_sender: SubEventExecuteResultSender,
    },
    UnSubscribe {
        identifier: StreamIdentifier,
        info: SubscriberInfo,
    },
    Publish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
        #[serde(skip_serializing)]
        result_sender: PubEventExecuteResultSender,
        #[serde(skip_serializing)]
        stream_handler: Arc<dyn TStreamHandler>,
    },
    UnPublish {
        identifier: StreamIdentifier,
        info: PublisherInfo,
    },
    #[serde(skip_serializing)]
    ApiStatistic {
        top_n: Option<usize>,
        identifier: Option<StreamIdentifier>,
        uuid: Option<Uuid>,
        result_sender: StatisticApiResultSender,
    },
    #[serde(skip_serializing)]
    ApiKickClient { id: Uuid },
    #[serde(skip_serializing)]
    ApiStartRelayStream {
        id: String,
        identifier: StreamIdentifier,
        server_address: String,
        relay_type: RelayType,
        result_sender: ApiRelayStreamResultSender,
    },
    #[serde(skip_serializing)]
    ApiStopRelayStream {
        id: String,
        relay_type: RelayType,
        result_sender: ApiRelayStreamResultSender,
    },
    #[serde(skip_serializing)]
    Request {
        identifier: StreamIdentifier,
        sender: InformationSender,
    },
}

impl StreamHubEvent {
    pub fn to_message(&self) -> StreamHubEventMessage {
        match self {
            StreamHubEvent::Subscribe {
                identifier,
                info,
                result_sender: _result_sender,
            } => StreamHubEventMessage::Subscribe {
                identifier: identifier.clone(),
                info: info.clone(),
            },
            StreamHubEvent::UnSubscribe { identifier, info } => {
                StreamHubEventMessage::UnSubscribe {
                    identifier: identifier.clone(),
                    info: info.clone(),
                }
            }
            StreamHubEvent::Publish {
                identifier,
                info,
                result_sender: _result_sender,
                stream_handler: _stream_handler,
            } => StreamHubEventMessage::Publish {
                identifier: identifier.clone(),
                info: info.clone(),
            },
            StreamHubEvent::UnPublish { identifier, info } => StreamHubEventMessage::UnPublish {
                identifier: identifier.clone(),
                info: info.clone(),
            },
            _ => StreamHubEventMessage::NotSupport {},
        }
    }
}

#[derive(Debug)]
pub enum TransceiverEvent {
    Subscribe {
        sender: DataSender,
        info: SubscriberInfo,
        result_sender: TransceiverEventExecuteResultSender,
    },
    UnSubscribe {
        info: SubscriberInfo,
    },
    UnPublish {},

    Api {
        sender: StatisticStreamSender,
        uuid: Option<Uuid>,
    },
    Request {
        sender: InformationSender,
    },
}

impl fmt::Display for TransceiverEvent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", *self)
    }
}

#[derive(Debug, Clone)]
pub enum BroadcastEvent {
    /*Need publish(push) a stream to other rtmp server*/
    Publish {
        identifier: StreamIdentifier,
    },
    UnPublish {
        identifier: StreamIdentifier,
    },
    /*Need subscribe(pull) a stream from other rtmp server*/
    Subscribe {
        id: String,
        identifier: StreamIdentifier,
        server_address: Option<String>,
        result_sender: Option<BroadcastEventExecuteResultSender>,
    },
    UnSubscribe {
        id: String,
        result_sender: Option<BroadcastEventExecuteResultSender>,
        //identifier: StreamIdentifier,
        //server_address: Option<String>,
    },
}

pub enum StatisticData {
    AudioCodec {
        sound_format: SoundFormat,
        profile: AacProfile,
        samplerate: u32,
        channels: u8,
    },
    VideoCodec {
        codec: AvcCodecId,
        profile: AvcProfile,
        level: AvcLevel,
        width: u32,
        height: u32,
    },
    Audio {
        uuid: Option<Uuid>,
        data_size: usize,
        aac_packet_type: u8,
        duration: usize,
    },
    Video {
        uuid: Option<Uuid>,
        data_size: usize,
        frame_count: usize,
        is_key_frame: Option<bool>,
        duration: usize,
    },
    Publisher {
        id: Uuid,
        remote_addr: String,
        start_time: DateTime<Local>,
    },
    Subscriber {
        id: Uuid,
        remote_addr: String,
        sub_type: SubscribeType,
        start_time: DateTime<Local>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Segment {
    /*ts duration*/
    pub duration: i64,
    pub discontinuity: bool,
    /*ts name*/
    pub name: String,
    pub path: String,
    pub is_eof: bool,
}

impl Segment {
    pub fn new(
        duration: i64,
        discontinuity: bool,
        name: String,
        path: String,
        is_eof: bool,
    ) -> Self {
        Self {
            duration,
            discontinuity,
            name,
            path,
            is_eof,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== SubscribeType Tests ==========

    #[test]
    fn test_subscribe_type_variants() {
        let types = [SubscribeType::RtspPull, SubscribeType::HttpFlvPull];
        assert_eq!(types.len(), 2);
    }

    #[test]
    fn test_subscribe_type_clone_eq() {
        let t1 = SubscribeType::RtspPull;
        let t2 = t1.clone();
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_subscribe_type_debug() {
        let t = SubscribeType::RtspPull;
        let debug_str = format!("{:?}", t);
        assert!(debug_str.contains("RtspPull"));
    }

    #[test]
    fn test_subscribe_type_serialize() {
        let t = SubscribeType::RtspPull;
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("RtspPull"));
    }

    // ========== PublishType Tests ==========

    #[test]
    fn test_publish_type_variants() {
        let types = [PublishType::RtspPush];
        assert_eq!(types.len(), 1);
    }

    #[test]
    fn test_publish_type_clone_eq() {
        let t1 = PublishType::RtspPush;
        let t2 = t1.clone();
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_publish_type_debug() {
        let t = PublishType::RtspPush;
        let debug_str = format!("{:?}", t);
        assert!(debug_str.contains("RtspPush"));
    }

    #[test]
    fn test_publish_type_serialize() {
        let t = PublishType::RtspPush;
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("RtspPush"));
    }

    // ========== NotifyInfo Tests ==========

    #[test]
    fn test_notify_info_construction() {
        let info = NotifyInfo {
            request_url: "http://example.com/live/stream".to_string(),
            remote_addr: "192.168.1.100:5000".to_string(),
        };
        assert_eq!(info.request_url, "http://example.com/live/stream");
        assert_eq!(info.remote_addr, "192.168.1.100:5000");
    }

    #[test]
    fn test_notify_info_clone() {
        let info = NotifyInfo {
            request_url: "http://test".to_string(),
            remote_addr: "127.0.0.1:1234".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.request_url, cloned.request_url);
        assert_eq!(info.remote_addr, cloned.remote_addr);
    }

    #[test]
    fn test_notify_info_serialize() {
        let info = NotifyInfo {
            request_url: "http://test".to_string(),
            remote_addr: "127.0.0.1:1234".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("request_url"));
        assert!(json.contains("remote_addr"));
        assert!(json.contains("http://test"));
    }

    #[test]
    fn test_notify_info_debug() {
        let info = NotifyInfo {
            request_url: "http://test".to_string(),
            remote_addr: "127.0.0.1:1234".to_string(),
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("NotifyInfo"));
    }

    // ========== VideoCodecType Tests ==========

    #[test]
    fn test_video_codec_type_variants() {
        let h264 = VideoCodecType::H264;
        let h265 = VideoCodecType::H265;
        assert!(h264 == VideoCodecType::H264);
        assert!(h265 == VideoCodecType::H265);
    }

    #[test]
    fn test_video_codec_type_clone() {
        let codec = VideoCodecType::H264;
        let cloned = codec.clone();
        assert!(codec == cloned);
    }

    #[test]
    fn test_video_codec_type_ne() {
        assert!(VideoCodecType::H264 != VideoCodecType::H265);
    }

    // ========== MediaInfo Tests ==========

    #[test]
    fn test_media_info_construction() {
        let info = MediaInfo {
            audio_clock_rate: 48000,
            video_clock_rate: 90000,
            vcodec: VideoCodecType::H264,
        };
        assert_eq!(info.audio_clock_rate, 48000);
        assert_eq!(info.video_clock_rate, 90000);
        assert!(info.vcodec == VideoCodecType::H264);
    }

    #[test]
    fn test_media_info_clone() {
        let info = MediaInfo {
            audio_clock_rate: 48000,
            video_clock_rate: 90000,
            vcodec: VideoCodecType::H265,
        };
        let cloned = info.clone();
        assert_eq!(cloned.audio_clock_rate, 48000);
        assert_eq!(cloned.video_clock_rate, 90000);
    }

    // ========== FrameData Tests ==========

    #[test]
    fn test_frame_data_video_variant() {
        let frame = FrameData::Video {
            timestamp: 1000,
            data: BytesMut::from(&[0x00, 0x01, 0x02][..]),
        };
        if let FrameData::Video { timestamp, data } = frame {
            assert_eq!(timestamp, 1000);
            assert_eq!(data.len(), 3);
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_frame_data_audio_variant() {
        let frame = FrameData::Audio {
            timestamp: 2000,
            data: BytesMut::from(&[0xAA, 0xBB][..]),
        };
        if let FrameData::Audio { timestamp, data } = frame {
            assert_eq!(timestamp, 2000);
            assert_eq!(data.len(), 2);
        } else {
            panic!("Expected Audio variant");
        }
    }

    #[test]
    fn test_frame_data_metadata_variant() {
        let frame = FrameData::MetaData {
            timestamp: 0,
            data: BytesMut::from(&[0x01][..]),
        };
        if let FrameData::MetaData { timestamp, .. } = frame {
            assert_eq!(timestamp, 0);
        } else {
            panic!("Expected MetaData variant");
        }
    }

    #[test]
    fn test_frame_data_media_info_variant() {
        let info = MediaInfo {
            audio_clock_rate: 48000,
            video_clock_rate: 90000,
            vcodec: VideoCodecType::H264,
        };
        let frame = FrameData::MediaInfo { media_info: info };
        if let FrameData::MediaInfo { media_info } = frame {
            assert_eq!(media_info.audio_clock_rate, 48000);
        } else {
            panic!("Expected MediaInfo variant");
        }
    }

    // ========== PacketData Tests ==========

    #[test]
    fn test_packet_data_video_variant() {
        let packet = PacketData::Video {
            timestamp: 3000,
            data: BytesMut::from(&[0x67, 0x42][..]),
        };
        if let PacketData::Video { timestamp, data } = packet {
            assert_eq!(timestamp, 3000);
            assert_eq!(data.len(), 2);
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_packet_data_audio_variant() {
        let packet = PacketData::Audio {
            timestamp: 4000,
            data: BytesMut::from(&[0xFF, 0xF1][..]),
        };
        if let PacketData::Audio { timestamp, data } = packet {
            assert_eq!(timestamp, 4000);
            assert_eq!(data.len(), 2);
        } else {
            panic!("Expected Audio variant");
        }
    }

    // ========== Information Tests ==========

    #[test]
    fn test_information_sdp_variant() {
        let info = Information::Sdp {
            data: "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n".to_string(),
        };
        let Information::Sdp { data } = info;
        assert!(data.contains("v=0"));
    }

    // ========== RelayType Tests ==========

    #[test]
    fn test_relay_type_variants() {
        let pull = RelayType::Pull;
        let push = RelayType::Push;
        let pull_json = serde_json::to_string(&pull).unwrap();
        let push_json = serde_json::to_string(&push).unwrap();
        assert!(pull_json.contains("Pull"));
        assert!(push_json.contains("Push"));
    }

    // ========== Segment Tests ==========

    #[test]
    fn test_segment_new() {
        let segment = Segment::new(
            10,
            false,
            "segment1.ts".to_string(),
            "/path/to/segment1.ts".to_string(),
            false,
        );
        assert_eq!(segment.duration, 10);
        assert!(!segment.discontinuity);
        assert_eq!(segment.name, "segment1.ts");
        assert_eq!(segment.path, "/path/to/segment1.ts");
        assert!(!segment.is_eof);
    }

    #[test]
    fn test_segment_clone() {
        let segment = Segment::new(5, true, "seg.ts".to_string(), "/seg.ts".to_string(), true);
        let cloned = segment.clone();
        assert_eq!(cloned.duration, 5);
        assert!(cloned.discontinuity);
        assert!(cloned.is_eof);
    }

    #[test]
    fn test_segment_debug() {
        let segment = Segment::new(
            10,
            false,
            "seg.ts".to_string(),
            "/seg.ts".to_string(),
            false,
        );
        let debug_str = format!("{:?}", segment);
        assert!(debug_str.contains("Segment"));
        assert!(debug_str.contains("seg.ts"));
    }

    #[test]
    fn test_segment_serialize() {
        let segment = Segment::new(
            10,
            false,
            "seg.ts".to_string(),
            "/seg.ts".to_string(),
            false,
        );
        let json = serde_json::to_string(&segment).unwrap();
        assert!(json.contains("duration"));
        assert!(json.contains("10"));
        assert!(json.contains("seg.ts"));
    }

    #[test]
    fn test_segment_deserialize() {
        let json = r#"{"duration":15,"discontinuity":true,"name":"test.ts","path":"/test.ts","is_eof":false}"#;
        let segment: Segment = serde_json::from_str(json).unwrap();
        assert_eq!(segment.duration, 15);
        assert!(segment.discontinuity);
        assert_eq!(segment.name, "test.ts");
    }

    // ========== SubscriberInfo Serialize Tests ==========

    #[test]
    fn test_subscriber_info_serialize() {
        let info = SubscriberInfo {
            id: Uuid::default(),
            sub_type: SubscribeType::HttpFlvPull,
            notify_info: NotifyInfo {
                request_url: "http://example.com/live/stream.flv".to_string(),
                remote_addr: "192.168.1.1:5000".to_string(),
            },
            sub_data_type: SubDataType::Frame,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("sub_type"));
        assert!(json.contains("notify_info"));
        assert!(json.contains("HttpFlvPull"));
    }

    #[test]
    fn test_subscriber_info_clone() {
        let info = SubscriberInfo {
            id: Uuid::default(),
            sub_type: SubscribeType::RtspPull,
            notify_info: NotifyInfo {
                request_url: "rtsp://test".to_string(),
                remote_addr: "10.0.0.1:554".to_string(),
            },
            sub_data_type: SubDataType::Packet,
        };
        let cloned = info.clone();
        assert_eq!(
            format!("{:?}", info.sub_type),
            format!("{:?}", cloned.sub_type)
        );
    }

    #[test]
    fn test_subscriber_info_debug() {
        let info = SubscriberInfo {
            id: Uuid::default(),
            sub_type: SubscribeType::HttpFlvPull,
            notify_info: NotifyInfo {
                request_url: "http://test".to_string(),
                remote_addr: "127.0.0.1:0".to_string(),
            },
            sub_data_type: SubDataType::Frame,
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("SubscriberInfo"));
        assert!(debug.contains("HttpFlvPull"));
    }

    // ========== PublisherInfo Serialize Tests ==========

    #[test]
    fn test_publisher_info_serialize() {
        let info = PublisherInfo {
            id: Uuid::default(),
            pub_type: PublishType::RtspPush,
            pub_data_type: PubDataType::Both,
            notify_info: NotifyInfo {
                request_url: "rtsp://example.com/live/stream".to_string(),
                remote_addr: "192.168.1.1:554".to_string(),
            },
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("id"));
        assert!(json.contains("pub_type"));
        assert!(json.contains("notify_info"));
        assert!(json.contains("RtspPush"));
    }

    #[test]
    fn test_publisher_info_clone() {
        let info = PublisherInfo {
            id: Uuid::default(),
            pub_type: PublishType::RtspPush,
            pub_data_type: PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: "rtsp://test".to_string(),
                remote_addr: "10.0.0.1:554".to_string(),
            },
        };
        let cloned = info.clone();
        assert_eq!(
            format!("{:?}", info.pub_type),
            format!("{:?}", cloned.pub_type)
        );
    }

    #[test]
    fn test_publisher_info_debug() {
        let info = PublisherInfo {
            id: Uuid::default(),
            pub_type: PublishType::RtspPush,
            pub_data_type: PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: "http://test".to_string(),
                remote_addr: "127.0.0.1:0".to_string(),
            },
        };
        let debug = format!("{:?}", info);
        assert!(debug.contains("PublisherInfo"));
        assert!(debug.contains("RtspPush"));
    }

    // ========== SubDataType / PubDataType Tests ==========

    #[test]
    fn test_sub_data_type_serialize() {
        let frame = SubDataType::Frame;
        let packet = SubDataType::Packet;
        let frame_json = serde_json::to_string(&frame).unwrap();
        let packet_json = serde_json::to_string(&packet).unwrap();
        assert!(frame_json.contains("Frame"));
        assert!(packet_json.contains("Packet"));
    }

    #[test]
    fn test_sub_data_type_debug() {
        let frame = SubDataType::Frame;
        let packet = SubDataType::Packet;
        assert!(format!("{:?}", frame).contains("Frame"));
        assert!(format!("{:?}", packet).contains("Packet"));
    }

    #[test]
    fn test_sub_data_type_clone() {
        let original = SubDataType::Packet;
        let cloned = original.clone();
        assert!(format!("{:?}", cloned).contains("Packet"));
    }

    #[test]
    fn test_pub_data_type_serialize() {
        let frame = PubDataType::Frame;
        let packet = PubDataType::Packet;
        let both = PubDataType::Both;
        let frame_json = serde_json::to_string(&frame).unwrap();
        let packet_json = serde_json::to_string(&packet).unwrap();
        let both_json = serde_json::to_string(&both).unwrap();
        assert!(frame_json.contains("Frame"));
        assert!(packet_json.contains("Packet"));
        assert!(both_json.contains("Both"));
    }

    #[test]
    fn test_pub_data_type_debug() {
        assert!(format!("{:?}", PubDataType::Frame).contains("Frame"));
        assert!(format!("{:?}", PubDataType::Packet).contains("Packet"));
        assert!(format!("{:?}", PubDataType::Both).contains("Both"));
    }

    // ========== DataSender Tests ==========

    #[test]
    fn test_data_sender_frame_variant() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sender = DataSender::Frame { sender: tx };
        let debug = format!("{:?}", sender);
        assert!(debug.contains("Frame"));
    }

    #[test]
    fn test_data_sender_packet_variant() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sender = DataSender::Packet { sender: tx };
        let debug = format!("{:?}", sender);
        assert!(debug.contains("Packet"));
    }

    #[test]
    fn test_data_sender_clone() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let sender = DataSender::Frame { sender: tx };
        let cloned = sender.clone();
        let debug = format!("{:?}", cloned);
        assert!(debug.contains("Frame"));
    }

    // ========== StreamHubEvent to_message Tests ==========

    #[test]
    fn test_stream_hub_event_to_message_unsubscribe() {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: "live/test".to_string(),
        };
        let info = SubscriberInfo {
            id: Uuid::default(),
            sub_type: SubscribeType::HttpFlvPull,
            notify_info: NotifyInfo {
                request_url: "http://localhost/live/test.flv".to_string(),
                remote_addr: "127.0.0.1:1234".to_string(),
            },
            sub_data_type: SubDataType::Frame,
        };
        let event = StreamHubEvent::UnSubscribe {
            identifier: identifier.clone(),
            info,
        };
        let message = event.to_message();
        assert!(matches!(message, StreamHubEventMessage::UnSubscribe { .. }));
    }

    #[test]
    fn test_stream_hub_event_to_message_unpublish() {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: "/live/stream1".to_string(),
        };
        let info = PublisherInfo {
            id: Uuid::default(),
            pub_type: PublishType::RtspPush,
            pub_data_type: PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: "rtsp://localhost/live/stream1".to_string(),
                remote_addr: "127.0.0.1:554".to_string(),
            },
        };
        let event = StreamHubEvent::UnPublish {
            identifier: identifier.clone(),
            info,
        };
        let message = event.to_message();
        assert!(matches!(message, StreamHubEventMessage::UnPublish { .. }));
    }

    #[test]
    fn test_stream_hub_event_to_message_request_returns_not_support() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let event = StreamHubEvent::Request {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "live/test".to_string(),
            },
            sender: tx,
        };
        let message = event.to_message();
        assert!(matches!(message, StreamHubEventMessage::NotSupport {}));
    }

    #[test]
    fn test_stream_hub_event_to_message_api_kick_returns_not_support() {
        let event = StreamHubEvent::ApiKickClient {
            id: Uuid::default(),
        };
        let message = event.to_message();
        assert!(matches!(message, StreamHubEventMessage::NotSupport {}));
    }

    // ========== StreamHubEventMessage Serialize Tests ==========

    #[test]
    fn test_stream_hub_event_message_not_support_serialize() {
        let msg = StreamHubEventMessage::NotSupport {};
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("NotSupport"));
    }

    #[test]
    fn test_stream_hub_event_message_unsubscribe_serialize() {
        let msg = StreamHubEventMessage::UnSubscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "live/test".to_string(),
            },
            info: SubscriberInfo {
                id: Uuid::default(),
                sub_type: SubscribeType::HttpFlvPull,
                notify_info: NotifyInfo {
                    request_url: "test".to_string(),
                    remote_addr: "127.0.0.1:0".to_string(),
                },
                sub_data_type: SubDataType::Frame,
            },
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("UnSubscribe"));
    }

    // ========== RelayType Tests ==========

    #[test]
    fn test_relay_type_debug() {
        assert!(format!("{:?}", RelayType::Pull).contains("Pull"));
        assert!(format!("{:?}", RelayType::Push).contains("Push"));
    }

    #[test]
    fn test_relay_type_clone() {
        let pull = RelayType::Pull;
        let cloned = pull.clone();
        assert!(format!("{:?}", cloned).contains("Pull"));
    }

    #[test]
    fn test_relay_type_deserialize() {
        let pull: RelayType = serde_json::from_str(r#""Pull""#).unwrap();
        let push: RelayType = serde_json::from_str(r#""Push""#).unwrap();
        assert!(matches!(pull, RelayType::Pull));
        assert!(matches!(push, RelayType::Push));
    }

    // ========== BroadcastEvent Tests ==========

    #[test]
    fn test_broadcast_event_publish_clone() {
        let event = BroadcastEvent::Publish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "live/test".to_string(),
            },
        };
        let cloned = event.clone();
        assert!(matches!(cloned, BroadcastEvent::Publish { .. }));
    }

    #[test]
    fn test_broadcast_event_unpublish_clone() {
        let event = BroadcastEvent::UnPublish {
            identifier: StreamIdentifier::Rtsp {
                stream_path: "/live/stream".to_string(),
            },
        };
        let cloned = event.clone();
        assert!(matches!(cloned, BroadcastEvent::UnPublish { .. }));
    }

    #[test]
    fn test_broadcast_event_subscribe_debug() {
        let event = BroadcastEvent::Subscribe {
            id: "relay-001".to_string(),
            identifier: StreamIdentifier::Rtsp {
                stream_path: "live/test".to_string(),
            },
            server_address: Some("192.168.1.1:554".to_string()),
            result_sender: None,
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("Subscribe"));
        assert!(debug.contains("relay-001"));
    }

    #[test]
    fn test_broadcast_event_unsubscribe_debug() {
        let event = BroadcastEvent::UnSubscribe {
            id: "relay-002".to_string(),
            result_sender: None,
        };
        let debug = format!("{:?}", event);
        assert!(debug.contains("UnSubscribe"));
        assert!(debug.contains("relay-002"));
    }

    // ========== Segment Edge Cases ==========

    #[test]
    fn test_segment_eof() {
        let segment = Segment::new(
            0,
            false,
            "eof.ts".to_string(),
            "/path/eof.ts".to_string(),
            true,
        );
        assert!(segment.is_eof);
        assert_eq!(segment.duration, 0);
    }

    #[test]
    fn test_segment_discontinuity() {
        let segment = Segment::new(
            5,
            true,
            "disc.ts".to_string(),
            "/disc.ts".to_string(),
            false,
        );
        assert!(segment.discontinuity);
    }

    #[test]
    fn test_segment_serialize_deserialize_roundtrip() {
        let original = Segment::new(
            42,
            true,
            "roundtrip.ts".to_string(),
            "/path/roundtrip.ts".to_string(),
            false,
        );
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Segment = serde_json::from_str(&json).unwrap();
        assert_eq!(original.duration, deserialized.duration);
        assert_eq!(original.discontinuity, deserialized.discontinuity);
        assert_eq!(original.name, deserialized.name);
        assert_eq!(original.path, deserialized.path);
        assert_eq!(original.is_eof, deserialized.is_eof);
    }

    // ========== FrameData Clone Tests ==========

    #[test]
    fn test_frame_data_video_clone() {
        let frame = FrameData::Video {
            timestamp: 5000,
            data: BytesMut::from(&[0x67, 0x42, 0x00, 0x1E][..]),
        };
        let cloned = frame.clone();
        if let FrameData::Video { timestamp, data } = cloned {
            assert_eq!(timestamp, 5000);
            assert_eq!(data.len(), 4);
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_frame_data_audio_clone() {
        let frame = FrameData::Audio {
            timestamp: 3000,
            data: BytesMut::from(&[0xFF, 0xF1, 0x50][..]),
        };
        let cloned = frame.clone();
        if let FrameData::Audio { timestamp, data } = cloned {
            assert_eq!(timestamp, 3000);
            assert_eq!(data.len(), 3);
        } else {
            panic!("Expected Audio variant");
        }
    }

    #[test]
    fn test_frame_data_media_info_clone() {
        let frame = FrameData::MediaInfo {
            media_info: MediaInfo {
                audio_clock_rate: 44100,
                video_clock_rate: 90000,
                vcodec: VideoCodecType::H265,
            },
        };
        let cloned = frame.clone();
        if let FrameData::MediaInfo { media_info } = cloned {
            assert_eq!(media_info.audio_clock_rate, 44100);
            assert_eq!(media_info.video_clock_rate, 90000);
        } else {
            panic!("Expected MediaInfo variant");
        }
    }

    // ========== PacketData Clone Tests ==========

    #[test]
    fn test_packet_data_video_clone() {
        let packet = PacketData::Video {
            timestamp: 7000,
            data: BytesMut::from(&[0x80, 0x60][..]),
        };
        let cloned = packet.clone();
        if let PacketData::Video { timestamp, data } = cloned {
            assert_eq!(timestamp, 7000);
            assert_eq!(data.len(), 2);
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_packet_data_audio_clone() {
        let packet = PacketData::Audio {
            timestamp: 8000,
            data: BytesMut::from(&[0x12, 0x10][..]),
        };
        let cloned = packet.clone();
        if let PacketData::Audio { timestamp, data } = cloned {
            assert_eq!(timestamp, 8000);
            assert_eq!(data.len(), 2);
        } else {
            panic!("Expected Audio variant");
        }
    }

    // ========== Information Clone Tests ==========

    #[test]
    fn test_information_sdp_clone() {
        let info = Information::Sdp {
            data: "v=0\r\n".to_string(),
        };
        let cloned = info.clone();
        let Information::Sdp { data } = cloned;
        assert_eq!(data, "v=0\r\n");
    }

    // ========== PublishType Additional Tests ==========

    #[test]
    fn test_publish_type_all_variants_serialize() {
        let variants = [(PublishType::RtspPush, "RtspPush")];
        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert!(
                json.contains(expected),
                "Expected {} in JSON: {}",
                expected,
                json
            );
        }
    }

    // ========== SubscribeType Additional Tests ==========

    #[test]
    fn test_subscribe_type_all_variants_serialize() {
        let variants = [
            (SubscribeType::RtspPull, "RtspPull"),
            (SubscribeType::HttpFlvPull, "HttpFlvPull"),
        ];
        for (variant, expected) in variants {
            let json = serde_json::to_string(&variant).unwrap();
            assert!(
                json.contains(expected),
                "Expected {} in JSON: {}",
                expected,
                json
            );
        }
    }
}
