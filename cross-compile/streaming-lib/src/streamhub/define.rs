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
    /* Remote client request pulling(play) a rtmp stream.*/
    RtmpPull,
    /* Remote request to play httpflv triggers remux from RTMP to httpflv. */
    RtmpRemux2HttpFlv,
    /* The publishing of RTMP stream triggers remuxing from RTMP to HLS protocol.(NOTICE:It is not triggerred by players.)*/
    RtmpRemux2Hls,
    /* Relay(Push) local RTMP stream from stream hub to other RTMP nodes.*/
    RtmpRelay,
    /* Remote client request pulling(play) a rtsp stream.*/
    RtspPull,
    /* The publishing of RTSP stream triggers remuxing from RTSP to RTMP protocol.*/
    RtspRemux2Rtmp,
    /* Relay(Push) local RTSP stream to other RTSP nodes.*/
    RtspRelay,
    /* Remote client request pulling(play) stream through whep.*/
    WhepPull,
    /* Remuxing webrtc stream to RTMP */
    WebRTCRemux2Rtmp,
    /* Relay(Push) the local webRTC stream to other nodes using Whip.*/
    WhipRelay,
    /* Pull rtp stream by subscribing from stream hub.*/
    RtpPull,
}

/* Publish streams to stream hub */
#[derive(Debug, Serialize, Clone, Eq, PartialEq)]
pub enum PublishType {
    /* Receive rtmp stream from remote push client. */
    RtmpPush,
    /* Relay(Pull) remote RTMP stream to local stream hub. */
    RtmpRelay,
    /* Receive rtsp stream from remote push client */
    RtspPush,
    /* Relay(Pull) remote RTSP stream to local stream hub. */
    RtspRelay,
    /* Receive whip stream from remote push client. */
    WhipPush,
    /* Relay(Pull) remote WebRTC stream to local stream hub using Whep. */
    WhepRelay,
    /* It used for publishing raw rtp data of rtsp/whbrtc(whip) */
    RtpPush,
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
    OnHls {
        identifier: StreamIdentifier,
        segment: Segment,
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
        let types = [
            SubscribeType::RtmpPull,
            SubscribeType::RtmpRemux2HttpFlv,
            SubscribeType::RtmpRemux2Hls,
            SubscribeType::RtmpRelay,
            SubscribeType::RtspPull,
            SubscribeType::RtspRemux2Rtmp,
            SubscribeType::RtspRelay,
            SubscribeType::WhepPull,
            SubscribeType::WebRTCRemux2Rtmp,
            SubscribeType::WhipRelay,
            SubscribeType::RtpPull,
        ];
        assert_eq!(types.len(), 11);
    }

    #[test]
    fn test_subscribe_type_clone_eq() {
        let t1 = SubscribeType::RtmpPull;
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
        let t = SubscribeType::RtmpPull;
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("RtmpPull"));
    }

    // ========== PublishType Tests ==========

    #[test]
    fn test_publish_type_variants() {
        let types = [
            PublishType::RtmpPush,
            PublishType::RtmpRelay,
            PublishType::RtspPush,
            PublishType::RtspRelay,
            PublishType::WhipPush,
            PublishType::WhepRelay,
            PublishType::RtpPush,
        ];
        assert_eq!(types.len(), 7);
    }

    #[test]
    fn test_publish_type_clone_eq() {
        let t1 = PublishType::RtmpPush;
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
        let t = PublishType::RtmpPush;
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("RtmpPush"));
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
}
