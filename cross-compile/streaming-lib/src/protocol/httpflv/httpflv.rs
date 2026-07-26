use crate::hub::define::{StatisticData, StatisticDataSender};
use tokio::sync::oneshot;
use {
    super::{
        define::{HttpResponseDataProducer, tag_type},
        errors::{HttpFLvError, HttpFLvErrorValue},
    },
    crate::container::muxer::{FlvMuxer, HEADER_LENGTH},
    crate::hub::define::{
        FrameData, FrameDataReceiver, NotifyInfo, StreamHubEvent, StreamHubEventSender,
        SubDataType, SubscribeType, SubscriberInfo,
    },
    crate::hub::{
        stream::StreamIdentifier,
        utils::{RandomDigitCount, Uuid},
    },
    bytes::BytesMut,
    std::net::SocketAddr,
};

/// Upper bound on frames to wait before assuming missing audio/video.
const MAX_AV_FRAME_NUM_TO_GUESS_AV: usize = 10;

struct HeaderState {
    frame_count: usize,
    cached_frames: Vec<FrameData>,
}

impl HeaderState {
    fn new() -> Self {
        Self {
            frame_count: 0,
            cached_frames: Vec::new(),
        }
    }
}

pub struct HttpFlv {
    app_name: String,
    stream_name: String,

    muxer: FlvMuxer,

    has_audio: bool,
    has_video: bool,
    has_send_header: bool,

    event_producer: StreamHubEventSender,
    data_receiver: FrameDataReceiver,
    /* now used for subscriber session */
    statistic_data_sender: Option<StatisticDataSender>,
    http_response_data_producer: HttpResponseDataProducer,
    subscriber_id: Uuid,
    request_url: String,
    remote_addr: SocketAddr,
}

impl HttpFlv {
    pub fn new(
        app_name: String,
        stream_name: String,
        event_producer: StreamHubEventSender,
        http_response_data_producer: HttpResponseDataProducer,
        request_url: String,
        remote_addr: SocketAddr,
    ) -> Self {
        let (_, data_receiver) = crate::hub::define::frame_data_channel();
        let subscriber_id = Uuid::new(RandomDigitCount::Four);

        Self {
            app_name,
            stream_name,
            muxer: FlvMuxer::new(),
            has_audio: false,
            has_video: false,
            has_send_header: false,
            data_receiver,
            statistic_data_sender: None,
            event_producer,
            http_response_data_producer,
            subscriber_id,
            request_url,
            remote_addr,
        }
    }

    pub async fn run(&mut self) -> Result<(), HttpFLvError> {
        self.subscribe_from_stream_hub().await?;
        self.send_media_stream().await?;

        Ok(())
    }

    pub async fn send_media_stream(&mut self) -> Result<(), HttpFLvError> {
        let mut retry_count = 0;
        let mut header_state = HeaderState::new();

        loop {
            let Some(data) = self.data_receiver.recv().await else {
                retry_count += 1;
                if retry_count > 10 {
                    break;
                }
                continue;
            };

            if !self.has_send_header {
                self.process_header_phase(&mut header_state, data).await?;
                continue;
            }

            retry_count = self.process_frame_with_retry(data, retry_count);
            if retry_count > 10 {
                break;
            }
        }
        self.unsubscribe_from_stream_hub().await
    }

    async fn process_header_phase(
        &mut self,
        header_state: &mut HeaderState,
        data: FrameData,
    ) -> Result<(), HttpFLvError> {
        header_state.frame_count += 1;

        match &data {
            FrameData::Audio { .. } => {
                self.has_audio = true;
                header_state.cached_frames.push(data);
            }
            FrameData::Video { .. } => {
                self.has_video = true;
                header_state.cached_frames.push(data);
            }
            _ => {}
        }

        let header_ready = (self.has_audio && self.has_video)
            || header_state.frame_count > MAX_AV_FRAME_NUM_TO_GUESS_AV;

        if header_ready {
            self.finalize_header(header_state)?;
        }

        Ok(())
    }

    fn finalize_header(&mut self, header_state: &mut HeaderState) -> Result<(), HttpFLvError> {
        self.has_send_header = true;
        self.muxer
            .write_flv_header(self.has_audio, self.has_video)?;
        self.muxer.write_previous_tag_size(0)?;
        self.flush_response_data()?;

        for frame in &header_state.cached_frames {
            self.write_flv_tag(frame.clone())?;
        }
        header_state.cached_frames.clear();
        Ok(())
    }

    fn process_frame_with_retry(&mut self, data: FrameData, mut retry_count: u32) -> u32 {
        if let Err(err) = self.write_flv_tag(data) {
            if let HttpFLvErrorValue::MpscSendError(err_in) = &err.value
                && err_in.is_disconnected()
            {
                tracing::debug!(error = %err_in, "write_flv_tag_disconnected");
                return 11;
            }
            tracing::error!(error = %err, "write_flv_tag_error");
            retry_count += 1;
        } else {
            retry_count = 0;
        }
        retry_count
    }

    //used for the http-flv protocol

    fn extract_flv_tag_data(
        &mut self,
        channel_data: FrameData,
    ) -> Result<(BytesMut, u32, u32), HttpFLvError> {
        let (common_data, common_timestamp, tag_type) = match channel_data {
            FrameData::Audio { timestamp, data } => {
                self.send_audio_statistics(&data);
                (data, timestamp, tag_type::AUDIO)
            }
            FrameData::Video { timestamp, data } => {
                self.send_video_statistics(&data);
                (data, timestamp, tag_type::VIDEO)
            }
            other => {
                return Err(HttpFLvError {
                    value: HttpFLvErrorValue::UnexpectedFrameData(format!("{other:?}")),
                });
            }
        };

        Ok((common_data, common_timestamp, tag_type as u32))
    }

    fn send_audio_statistics(&self, data: &bytes::BytesMut) {
        let Some(sender) = &self.statistic_data_sender else {
            return;
        };
        let statistic_audio_data = StatisticData::Audio {
            uuid: Some(self.subscriber_id),
            aac_packet_type: 1,
            data_size: data.len(),
            duration: 0,
        };
        if let Err(err) = sender.send(statistic_audio_data) {
            tracing::error!(error = %err, "send_audio_statistics_error");
        }
    }

    fn send_video_statistics(&self, data: &bytes::BytesMut) {
        let Some(sender) = &self.statistic_data_sender else {
            return;
        };
        let statistic_video_data = StatisticData::Video {
            uuid: Some(self.subscriber_id),
            frame_count: 1,
            is_key_frame: None,
            data_size: data.len(),
            duration: 0,
        };
        if let Err(err) = sender.send(statistic_video_data) {
            tracing::error!(error = %err, "send_video_statistics_error");
        }
    }

    pub fn write_flv_tag(&mut self, channel_data: FrameData) -> Result<(), HttpFLvError> {
        let (common_data, common_timestamp, tag_type) = self.extract_flv_tag_data(channel_data)?;

        let common_data_len = common_data.len() as u32;

        self.muxer
            .write_flv_tag_header(tag_type as u8, common_data_len, common_timestamp)?;
        self.muxer.write_flv_tag_body(common_data)?;
        self.muxer
            .write_previous_tag_size(common_data_len + HEADER_LENGTH)?;

        self.flush_response_data()?;

        Ok(())
    }

    pub fn flush_response_data(&mut self) -> Result<(), HttpFLvError> {
        let data = self.muxer.writer.extract_current_bytes();
        self.http_response_data_producer.start_send(Ok(data))?;

        Ok(())
    }

    pub async fn unsubscribe_from_stream_hub(&mut self) -> Result<(), HttpFLvError> {
        let sub_info = SubscriberInfo {
            id: self.subscriber_id,
            sub_type: SubscribeType::HttpFlvPull,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr: self.remote_addr.to_string(),
            },
        };

        let identifier = StreamIdentifier::Rtsp {
            stream_path: format!("{}/{}", self.app_name, self.stream_name),
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier,
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            tracing::error!(error = %err, "unsubscribe_from_stream_hub_error");
        }

        Ok(())
    }

    pub async fn subscribe_from_stream_hub(&mut self) -> Result<(), HttpFLvError> {
        let sub_info = SubscriberInfo {
            id: self.subscriber_id,
            sub_type: SubscribeType::HttpFlvPull,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr: self.remote_addr.to_string(),
            },
        };

        let identifier = StreamIdentifier::Rtsp {
            stream_path: format!("{}/{}", self.app_name, self.stream_name),
        };

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier,
            info: sub_info,
            result_sender: event_result_sender,
        };

        let rv = self.event_producer.send(subscribe_event);
        if rv.is_err() {
            return Err(HttpFLvError {
                value: HttpFLvErrorValue::SendFrameDataErr,
            });
        }

        let result_receiver = event_result_receiver.await??;
        let receiver = result_receiver.0.frame_receiver.ok_or(HttpFLvError {
            value: HttpFLvErrorValue::MissingFrameReceiver,
        })?;
        self.data_receiver = receiver;
        self.statistic_data_sender = result_receiver.1;

        if let Some(sender) = &self.statistic_data_sender {
            let statistic_subscriber = StatisticData::Subscriber {
                id: self.subscriber_id,
                remote_addr: self.remote_addr.to_string(),
                start_time: chrono::Local::now(),
                sub_type: SubscribeType::HttpFlvPull,
            };
            if let Err(err) = sender.send(statistic_subscriber) {
                tracing::error!(error = %err, "send_statistic_subscriber_error");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hub::define::{
        DataReceiver, FrameData, MediaInfo, NotifyInfo, StatisticData, StreamHubEvent, SubDataType,
        SubscribeType, SubscriberInfo, VideoCodecType,
    };
    use crate::hub::errors::{StreamHubError, StreamHubErrorValue};
    use crate::hub::stream::StreamIdentifier;
    use crate::hub::utils::{RandomDigitCount, Uuid};
    use crate::protocol::httpflv::errors::HttpFLvErrorValue;
    use bytes::BytesMut;
    use futures::channel::mpsc as futures_mpsc;
    use futures::{SinkExt, StreamExt};
    use std::net::SocketAddr;
    use tokio::sync::mpsc as tokio_mpsc;

    /// Both halves of the stream-hub event channel and the HTTP response body channel.
    type TestChannels = (
        tokio_mpsc::UnboundedSender<StreamHubEvent>,
        tokio_mpsc::UnboundedReceiver<StreamHubEvent>,
        futures_mpsc::UnboundedSender<std::io::Result<BytesMut>>,
        futures_mpsc::UnboundedReceiver<std::io::Result<BytesMut>>,
    );

    /// Helper function to create test channels
    fn create_test_channels() -> TestChannels {
        let (event_sender, event_receiver) = tokio_mpsc::unbounded_channel();
        let (response_sender, response_receiver) = futures_mpsc::unbounded();
        (
            event_sender,
            event_receiver,
            response_sender,
            response_receiver,
        )
    }

    /// Helper to create a sample SocketAddr
    fn create_test_socket_addr() -> SocketAddr {
        "127.0.0.1:8080".parse().unwrap()
    }

    // ========== HeaderState Tests ==========

    #[test]
    fn test_header_state_new_creates_empty_state() {
        let state = HeaderState::new();
        assert_eq!(
            state.frame_count, 0,
            "New HeaderState should have frame_count of 0"
        );
        assert!(
            state.cached_frames.is_empty(),
            "New HeaderState should have empty cached_frames"
        );
    }

    #[test]
    fn test_header_state_frame_count_starts_at_zero() {
        let state = HeaderState::new();
        assert_eq!(state.frame_count, 0);
    }

    #[test]
    fn test_header_state_cached_frames_is_empty_on_creation() {
        let state = HeaderState::new();
        assert_eq!(state.cached_frames.len(), 0);
    }

    // ========== HttpFlv Creation Tests ==========

    #[tokio::test]
    async fn test_httpflv_new_creates_instance_with_correct_app_name() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let _ = httpflv;
    }

    #[tokio::test]
    async fn test_httpflv_new_creates_instance_with_empty_app_name() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let httpflv = HttpFlv::new(
            "".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/stream1.flv".to_string(),
            remote_addr,
        );

        let _ = httpflv;
    }

    #[tokio::test]
    async fn test_httpflv_new_creates_instance_with_special_characters_in_names() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let httpflv = HttpFlv::new(
            "app-name_123".to_string(),
            "stream.name-test".to_string(),
            event_sender,
            response_sender,
            "http://localhost/app-name_123/stream.name-test.flv".to_string(),
            remote_addr,
        );

        let _ = httpflv;
    }

    #[tokio::test]
    async fn test_httpflv_new_with_ipv6_address() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr: SocketAddr = "[::1]:8080".parse().unwrap();

        let httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://[::1]/live/stream1.flv".to_string(),
            remote_addr,
        );

        let _ = httpflv;
    }

    #[tokio::test]
    async fn test_httpflv_new_with_different_port() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr: SocketAddr = "192.168.1.100:9999".parse().unwrap();

        let httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://192.168.1.100:9999/live/stream1.flv".to_string(),
            remote_addr,
        );

        let _ = httpflv;
    }

    // ========== Tag Type Constants Tests ==========

    #[test]
    fn test_tag_type_audio_value() {
        assert_eq!(tag_type::AUDIO, 8, "AUDIO tag type should be 8");
    }

    #[test]
    fn test_tag_type_video_value() {
        assert_eq!(tag_type::VIDEO, 9, "VIDEO tag type should be 9");
    }

    #[test]
    fn test_tag_type_script_data_amf_value() {
        assert_eq!(
            tag_type::SCRIPT_DATA_AMF,
            18,
            "SCRIPT_DATA_AMF tag type should be 18"
        );
    }

    #[test]
    fn test_tag_type_values_are_distinct() {
        let audio = tag_type::AUDIO;
        let video = tag_type::VIDEO;
        let script = tag_type::SCRIPT_DATA_AMF;

        assert_ne!(audio, video);
        assert_ne!(video, script);
        assert_ne!(audio, script);
    }

    // ========== FrameData Variant Tests for Tag Extraction ==========

    #[test]
    fn test_frame_data_video_creation() {
        let data = BytesMut::from(&[0x00, 0x01, 0x02, 0x03][..]);
        let frame = FrameData::Video {
            timestamp: 1000,
            data: data.clone(),
        };

        if let FrameData::Video {
            timestamp,
            data: frame_data,
        } = frame
        {
            assert_eq!(timestamp, 1000, "Video timestamp should be 1000");
            assert_eq!(frame_data.len(), 4, "Video data length should be 4");
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_frame_data_audio_creation() {
        let data = BytesMut::from(&[0xAA, 0xBB][..]);
        let frame = FrameData::Audio {
            timestamp: 2000,
            data: data.clone(),
        };

        if let FrameData::Audio {
            timestamp,
            data: frame_data,
        } = frame
        {
            assert_eq!(timestamp, 2000, "Audio timestamp should be 2000");
            assert_eq!(frame_data.len(), 2, "Audio data length should be 2");
        } else {
            panic!("Expected Audio variant");
        }
    }

    #[test]
    fn test_frame_data_metadata_creation() {
        let data = BytesMut::from(&[0x01, 0x02, 0x03][..]);
        let frame = FrameData::MetaData {
            timestamp: 0,
            data: data.clone(),
        };

        if let FrameData::MetaData {
            timestamp,
            data: frame_data,
        } = frame
        {
            assert_eq!(timestamp, 0, "MetaData timestamp should be 0");
            assert_eq!(frame_data.len(), 3, "MetaData data length should be 3");
        } else {
            panic!("Expected MetaData variant");
        }
    }

    #[test]
    fn test_frame_data_media_info_creation() {
        let media_info = MediaInfo {
            audio_clock_rate: 48000,
            video_clock_rate: 90000,
            vcodec: VideoCodecType::H264,
        };
        let frame = FrameData::MediaInfo { media_info };

        if let FrameData::MediaInfo { media_info: info } = frame {
            assert_eq!(info.audio_clock_rate, 48000);
            assert_eq!(info.video_clock_rate, 90000);
            assert!(info.vcodec == VideoCodecType::H264);
        } else {
            panic!("Expected MediaInfo variant");
        }
    }

    #[test]
    fn test_frame_data_empty_video_data() {
        let data = BytesMut::new();
        let frame = FrameData::Video { timestamp: 0, data };

        if let FrameData::Video { data, .. } = frame {
            assert!(data.is_empty(), "Empty video data should be allowed");
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_frame_data_empty_audio_data() {
        let data = BytesMut::new();
        let frame = FrameData::Audio { timestamp: 0, data };

        if let FrameData::Audio { data, .. } = frame {
            assert!(data.is_empty(), "Empty audio data should be allowed");
        } else {
            panic!("Expected Audio variant");
        }
    }

    #[test]
    fn test_frame_data_large_timestamp() {
        let large_timestamp = u32::MAX;
        let frame = FrameData::Video {
            timestamp: large_timestamp,
            data: BytesMut::new(),
        };

        if let FrameData::Video { timestamp, .. } = frame {
            assert_eq!(timestamp, u32::MAX, "Should handle max u32 timestamp");
        } else {
            panic!("Expected Video variant");
        }
    }

    // ========== Subscriber Info Tests ==========

    #[test]
    fn test_subscriber_info_creation_for_httpflv() {
        let notify_info = NotifyInfo {
            request_url: "http://localhost/live/stream.flv".to_string(),
            remote_addr: "127.0.0.1:12345".to_string(),
        };

        let subscriber_info = SubscriberInfo {
            id: Uuid::new(RandomDigitCount::Four),
            sub_type: SubscribeType::HttpFlvPull,
            sub_data_type: SubDataType::Frame,
            notify_info,
        };

        assert_eq!(subscriber_info.sub_type, SubscribeType::HttpFlvPull);
        assert!(matches!(subscriber_info.sub_data_type, SubDataType::Frame));
        assert_eq!(
            subscriber_info.notify_info.request_url,
            "http://localhost/live/stream.flv"
        );
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

    // ========== Stream Identifier Tests ==========

    #[test]
    fn test_stream_identifier_rtsp_creation() {
        let app_name = "live";
        let stream_name = "test";
        let identifier = StreamIdentifier::Rtsp {
            stream_path: format!("{}/{}", app_name, stream_name),
        };

        if let StreamIdentifier::Rtsp { stream_path } = identifier {
            assert!(stream_path.contains("live"));
            assert!(stream_path.contains("test"));
        } else {
            panic!("Expected Rtsp variant");
        }
    }

    #[test]
    fn test_stream_identifier_clone() {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: "app/stream".to_string(),
        };
        let cloned = identifier.clone();

        if let StreamIdentifier::Rtsp { stream_path } = cloned {
            assert_eq!(stream_path, "app/stream");
        } else {
            panic!("Expected Rtsp variant");
        }
    }

    // ========== StatisticData Tests ==========

    #[test]
    fn test_statistic_data_audio_creation() {
        let stat = StatisticData::Audio {
            uuid: None,
            data_size: 1024,
            aac_packet_type: 1,
            duration: 0,
        };

        if let StatisticData::Audio {
            data_size,
            aac_packet_type,
            ..
        } = stat
        {
            assert_eq!(data_size, 1024);
            assert_eq!(aac_packet_type, 1);
        } else {
            panic!("Expected Audio variant");
        }
    }

    #[test]
    fn test_statistic_data_video_creation() {
        let stat = StatisticData::Video {
            uuid: None,
            data_size: 2048,
            frame_count: 1,
            is_key_frame: Some(true),
            duration: 0,
        };

        if let StatisticData::Video {
            data_size,
            frame_count,
            is_key_frame,
            ..
        } = stat
        {
            assert_eq!(data_size, 2048);
            assert_eq!(frame_count, 1);
            assert_eq!(is_key_frame, Some(true));
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_statistic_data_subscriber_creation() {
        let stat = StatisticData::Subscriber {
            id: Uuid::new(RandomDigitCount::Four),
            remote_addr: "192.168.1.1:12345".to_string(),
            sub_type: SubscribeType::HttpFlvPull,
            start_time: chrono::Local::now(),
        };

        if let StatisticData::Subscriber {
            sub_type,
            remote_addr,
            ..
        } = stat
        {
            assert_eq!(sub_type, SubscribeType::HttpFlvPull);
            assert_eq!(remote_addr, "192.168.1.1:12345");
        } else {
            panic!("Expected Subscriber variant");
        }
    }

    // ========== Subscribe/Unsubscribe Event Tests ==========

    #[tokio::test]
    async fn test_subscribe_event_creation() {
        let (event_sender, mut event_receiver) = tokio_mpsc::unbounded_channel();
        let (result_sender, _result_receiver) = tokio::sync::oneshot::channel();

        let identifier = StreamIdentifier::Rtsp {
            stream_path: "live/stream1".to_string(),
        };

        let subscriber_info = SubscriberInfo {
            id: Uuid::new(RandomDigitCount::Four),
            sub_type: SubscribeType::HttpFlvPull,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: "http://localhost/live/stream1.flv".to_string(),
                remote_addr: "127.0.0.1:12345".to_string(),
            },
        };

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier,
            info: subscriber_info,
            result_sender,
        };

        assert!(
            event_sender.send(subscribe_event).is_ok(),
            "Should be able to send subscribe event"
        );

        if let Some(StreamHubEvent::Subscribe { info, .. }) = event_receiver.recv().await {
            assert_eq!(info.sub_type, SubscribeType::HttpFlvPull);
        } else {
            panic!("Expected Subscribe event");
        }
    }

    #[tokio::test]
    async fn test_unsubscribe_event_creation() {
        let (event_sender, mut event_receiver) = tokio_mpsc::unbounded_channel();

        let identifier = StreamIdentifier::Rtsp {
            stream_path: "live/stream1".to_string(),
        };

        let subscriber_info = SubscriberInfo {
            id: Uuid::new(RandomDigitCount::Four),
            sub_type: SubscribeType::HttpFlvPull,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: "http://localhost/live/stream1.flv".to_string(),
                remote_addr: "127.0.0.1:12345".to_string(),
            },
        };

        let unsubscribe_event = StreamHubEvent::UnSubscribe {
            identifier,
            info: subscriber_info,
        };

        assert!(
            event_sender.send(unsubscribe_event).is_ok(),
            "Should be able to send unsubscribe event"
        );

        if let Some(StreamHubEvent::UnSubscribe { info, .. }) = event_receiver.recv().await {
            assert_eq!(info.sub_type, SubscribeType::HttpFlvPull);
        } else {
            panic!("Expected UnSubscribe event");
        }
    }

    // ========== Channel Communication Tests ==========

    #[tokio::test]
    async fn test_response_channel_communication() {
        let (mut response_sender, mut response_receiver) =
            futures_mpsc::unbounded::<std::io::Result<BytesMut>>();

        let test_data = BytesMut::from(&[0x46, 0x4C, 0x56][..]); // "FLV" signature
        assert!(response_sender.send(Ok(test_data.clone())).await.is_ok());

        if let Some(Ok(received_data)) = response_receiver.next().await {
            assert_eq!(received_data.len(), 3);
            assert_eq!(&received_data[..], &[0x46, 0x4C, 0x56]);
        } else {
            panic!("Expected to receive data");
        }
    }

    #[tokio::test]
    async fn test_response_channel_error_propagation() {
        let (mut response_sender, mut response_receiver) =
            futures_mpsc::unbounded::<std::io::Result<BytesMut>>();

        let error = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Connection lost");
        assert!(response_sender.send(Err(error)).await.is_ok());

        if let Some(Err(e)) = response_receiver.next().await {
            assert_eq!(e.kind(), std::io::ErrorKind::BrokenPipe);
        } else {
            panic!("Expected to receive error");
        }
    }

    #[tokio::test]
    async fn test_response_channel_multiple_messages() {
        let (mut response_sender, mut response_receiver) =
            futures_mpsc::unbounded::<std::io::Result<BytesMut>>();

        for i in 0..5 {
            let data = BytesMut::from(&[i as u8][..]);
            assert!(response_sender.send(Ok(data)).await.is_ok());
        }

        let mut count = 0;
        while let Some(Ok(data)) = response_receiver.next().await {
            assert_eq!(data.len(), 1);
            count += 1;
            if count == 5 {
                break;
            }
        }
        assert_eq!(count, 5, "Should receive all 5 messages");
    }

    // ========== Edge Cases and Error Scenarios ==========

    #[test]
    fn test_frame_data_clone() {
        let frame = FrameData::Video {
            timestamp: 1000,
            data: BytesMut::from(&[0x01, 0x02][..]),
        };
        let cloned = frame.clone();

        if let FrameData::Video { timestamp, data } = cloned {
            assert_eq!(timestamp, 1000);
            assert_eq!(data.len(), 2);
        } else {
            panic!("Expected Video variant");
        }
    }

    #[test]
    fn test_frame_data_debug_format() {
        let frame = FrameData::Video {
            timestamp: 1000,
            data: BytesMut::from(&[0x01][..]),
        };
        let debug_str = format!("{:?}", frame);
        assert!(debug_str.contains("Video"));
        assert!(debug_str.contains("1000"));
    }

    #[test]
    fn test_media_info_clone() {
        let info = MediaInfo {
            audio_clock_rate: 48000,
            video_clock_rate: 90000,
            vcodec: VideoCodecType::H264,
        };
        let cloned = info.clone();
        assert_eq!(cloned.audio_clock_rate, 48000);
        assert_eq!(cloned.video_clock_rate, 90000);
    }

    #[test]
    fn test_video_codec_type_equality() {
        assert!(VideoCodecType::H264 == VideoCodecType::H264);
        assert!(VideoCodecType::H265 == VideoCodecType::H265);
        assert!(VideoCodecType::H264 != VideoCodecType::H265);
    }

    #[test]
    fn test_video_codec_type_clone() {
        let codec = VideoCodecType::H264;
        let cloned = codec.clone();
        assert!(codec == cloned);
    }

    // ========== SubscribeType Tests for HttpFlv ==========

    #[test]
    fn test_subscribe_type_rtmp_remux_2_httpflv() {
        let sub_type = SubscribeType::HttpFlvPull;
        let debug_str = format!("{:?}", sub_type);
        assert!(debug_str.contains("HttpFlvPull"));
    }

    #[test]
    fn test_subscribe_type_clone() {
        let sub_type = SubscribeType::HttpFlvPull;
        let cloned = sub_type.clone();
        assert_eq!(sub_type, cloned);
    }

    #[test]
    fn test_subscribe_type_eq() {
        let t1 = SubscribeType::HttpFlvPull;
        let t2 = SubscribeType::HttpFlvPull;
        assert_eq!(t1, t2);
    }

    // ========== SubDataType Tests ==========

    #[test]
    fn test_sub_data_type_frame() {
        let sub_data_type = SubDataType::Frame;
        let debug_str = format!("{:?}", sub_data_type);
        assert!(debug_str.contains("Frame"));
    }

    #[test]
    fn test_sub_data_type_clone() {
        let sub_data_type = SubDataType::Frame;
        let cloned = sub_data_type.clone();
        assert!(matches!(cloned, SubDataType::Frame));
    }

    // ========== FLV Header Constants Verification ==========

    #[test]
    fn test_flv_tag_sizes() {
        assert_eq!(tag_type::AUDIO, 8, "FLV audio tag type is 8");
        assert_eq!(tag_type::VIDEO, 9, "FLV video tag type is 9");
        assert_eq!(
            tag_type::SCRIPT_DATA_AMF,
            18,
            "FLV script data tag type is 18"
        );
    }

    // ========== BytesMut Edge Cases ==========

    #[test]
    fn test_bytes_mut_empty_data() {
        let data = BytesMut::new();
        assert!(data.is_empty());
        assert_eq!(data.len(), 0);
    }

    #[test]
    fn test_bytes_mut_from_slice() {
        let slice: &[u8] = &[0x01, 0x02, 0x03, 0x04];
        let data = BytesMut::from(slice);
        assert_eq!(data.len(), 4);
        assert_eq!(&data[..], slice);
    }

    #[test]
    fn test_bytes_mut_split_at() {
        let data = BytesMut::from(&[0x01, 0x02, 0x03, 0x04][..]);
        let (left, right) = data.split_at(2);
        assert_eq!(left, &[0x01, 0x02]);
        assert_eq!(right, &[0x03, 0x04]);
    }

    // ========== SocketAddr Edge Cases ==========

    #[test]
    fn test_socket_addr_ipv4() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        assert!(addr.is_ipv4());
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_socket_addr_ipv6() {
        let addr: SocketAddr = "[::1]:8080".parse().unwrap();
        assert!(addr.is_ipv6());
        assert_eq!(addr.port(), 8080);
    }

    #[test]
    fn test_socket_addr_to_string() {
        let addr: SocketAddr = "192.168.1.1:9999".parse().unwrap();
        let addr_str = addr.to_string();
        assert!(addr_str.contains("192.168.1.1"));
        assert!(addr_str.contains("9999"));
    }

    // ========== MAX_AV_FRAME_NUM_TO_GUESS_AV Constant Test ==========

    #[test]
    fn test_max_av_frame_num_constant() {
        const MAX_AV_FRAME_NUM_TO_GUESS_AV: usize = 10;
        assert_eq!(MAX_AV_FRAME_NUM_TO_GUESS_AV, 10);
    }

    // ========== write_flv_tag / extract_flv_tag_data Tests ==========

    #[tokio::test]
    async fn test_write_flv_tag_media_info_returns_unexpected_frame_data_error() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let media_info = MediaInfo {
            audio_clock_rate: 48000,
            video_clock_rate: 90000,
            vcodec: VideoCodecType::H264,
        };
        let frame = FrameData::MediaInfo { media_info };

        let result = httpflv.write_flv_tag(frame);
        assert!(
            result.is_err(),
            "MediaInfo frame should not be writable as FLV tag"
        );
        if let Err(e) = result {
            assert!(matches!(e.value, HttpFLvErrorValue::UnexpectedFrameData(_)));
        }
    }

    #[tokio::test]
    async fn test_write_flv_tag_audio_success() {
        let (event_sender, _event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let frame = FrameData::Audio {
            timestamp: 100,
            data: BytesMut::from(&[0x01, 0x02, 0x03][..]),
        };
        let result = httpflv.write_flv_tag(frame);
        assert!(result.is_ok(), "Audio frame should be writable as FLV tag");
        let _ = response_receiver.next().await;
    }

    #[tokio::test]
    async fn test_write_flv_tag_video_success() {
        let (event_sender, _event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let frame = FrameData::Video {
            timestamp: 200,
            data: BytesMut::from(&[0x00, 0x01, 0x02][..]),
        };
        let result = httpflv.write_flv_tag(frame);
        assert!(result.is_ok(), "Video frame should be writable as FLV tag");
        let _ = response_receiver.next().await;
    }

    #[tokio::test]
    async fn test_write_flv_tag_metadata_rejected() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let mut meta = BytesMut::new();
        meta.extend_from_slice(b"@setDataFrame");
        meta.extend_from_slice(b"onMetaData");
        let frame = FrameData::MetaData {
            timestamp: 0,
            data: meta,
        };
        let result = httpflv.write_flv_tag(frame);
        assert!(result.is_err(), "MetaData frames should be rejected");
        match result.unwrap_err().value {
            HttpFLvErrorValue::UnexpectedFrameData(_) => {}
            other => panic!("Expected UnexpectedFrameData, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_subscribe_from_stream_hub_success() {
        let (event_sender, mut event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_tx, frame_rx) = crate::hub::define::frame_data_channel();
                drop(frame_tx);
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_rx),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(
            result.is_ok(),
            "Subscribe should succeed when hub responds with receiver"
        );
        let _ = hub_task.await;
    }

    #[tokio::test]
    async fn test_subscribe_from_stream_hub_send_fails_returns_error() {
        let (event_sender, event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        drop(event_receiver);

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(
            result.is_err(),
            "Subscribe should fail when event send fails"
        );
        if let Err(e) = result {
            assert!(matches!(e.value, HttpFLvErrorValue::SendFrameDataErr));
        }
    }

    #[tokio::test]
    async fn test_subscribe_from_stream_hub_missing_frame_receiver_returns_error() {
        let (event_sender, mut event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let data_receiver = DataReceiver {
                    frame_receiver: None,
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(
            result.is_err(),
            "Subscribe should fail when frame_receiver is None"
        );
        if let Err(e) = result {
            assert!(matches!(e.value, HttpFLvErrorValue::MissingFrameReceiver));
        }
        let _ = hub_task.await;
    }

    #[tokio::test]
    async fn test_subscribe_from_stream_hub_hub_returns_error() {
        let (event_sender, mut event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let _ = result_sender.send(Err(StreamHubError {
                    value: StreamHubErrorValue::NoStreamName,
                }));
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(
            result.is_err(),
            "Subscribe should fail when hub returns error"
        );
        let _ = hub_task.await;
    }

    // ========== Integration-like Tests ==========

    #[tokio::test]
    async fn test_httpflv_unsubscribe_from_stream_hub_sends_event() {
        let (event_sender, mut event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let result = httpflv.unsubscribe_from_stream_hub().await;
        assert!(result.is_ok(), "Unsubscribe should succeed");

        if let Some(event) = event_receiver.recv().await {
            if let StreamHubEvent::UnSubscribe { info, .. } = event {
                assert_eq!(info.sub_type, SubscribeType::HttpFlvPull);
            } else {
                panic!("Expected UnSubscribe event");
            }
        } else {
            panic!("Should receive unsubscribe event");
        }
    }

    #[tokio::test]
    async fn test_httpflv_multiple_unsubscribe_calls() {
        let (event_sender, mut event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let result1 = httpflv.unsubscribe_from_stream_hub().await;
        assert!(result1.is_ok());

        let result2 = httpflv.unsubscribe_from_stream_hub().await;
        assert!(result2.is_ok());

        let _ = event_receiver.recv().await;
        let _ = event_receiver.recv().await;
    }

    #[tokio::test]
    async fn test_httpflv_with_closed_event_sender_unsubscribe_still_returns_ok() {
        let (event_sender, event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        drop(event_receiver);

        let result = httpflv.unsubscribe_from_stream_hub().await;
        assert!(
            result.is_ok(),
            "Unsubscribe should return Ok even with closed channel"
        );
    }

    // ========== Uuid Tests ==========

    #[test]
    fn test_uuid_new_creates_unique_ids() {
        let id1 = Uuid::new(RandomDigitCount::Four);
        let id2 = Uuid::new(RandomDigitCount::Four);

        assert_ne!(id1.to_string(), id2.to_string(), "UUIDs should be unique");
    }

    #[test]
    fn test_uuid_to_string() {
        let id = Uuid::new(RandomDigitCount::Four);
        let id_str = id.to_string();
        assert!(!id_str.is_empty(), "UUID string should not be empty");
    }

    // ========== send_media_stream / process_header_phase / process_frame_with_retry Tests ==========

    #[tokio::test]
    async fn test_send_media_stream_channel_closed_exits_after_retries() {
        let (event_sender, _event_receiver, response_sender, _response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let result = httpflv.send_media_stream().await;
        assert!(
            result.is_ok(),
            "send_media_stream should complete when channel closed after retries"
        );
    }

    #[tokio::test]
    async fn test_send_media_stream_header_phase_with_audio_video_finalizes() {
        let (event_sender, mut event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_tx, frame_rx) = crate::hub::define::frame_data_channel();
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_rx),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));

                let _ = frame_tx.try_send(FrameData::Audio {
                    timestamp: 0,
                    data: BytesMut::from(&[0x01, 0x02][..]),
                });
                let _ = frame_tx.try_send(FrameData::Video {
                    timestamp: 0,
                    data: BytesMut::from(&[0x00, 0x01][..]),
                });
                drop(frame_tx);
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(result.is_ok());

        let send_task = tokio::spawn(async move { httpflv.send_media_stream().await });

        let result = send_task.await.expect("send_media_stream task panicked");
        assert!(result.is_ok());

        let _ = response_receiver.next().await;
        let _ = hub_task.await;
    }

    #[tokio::test]
    async fn test_send_media_stream_header_phase_metadata_then_audio_video() {
        let (event_sender, mut event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_tx, frame_rx) = crate::hub::define::frame_data_channel();
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_rx),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));

                let mut meta = BytesMut::new();
                meta.extend_from_slice(b"@setDataFrame");
                meta.extend_from_slice(b"onMetaData");
                let _ = frame_tx.try_send(FrameData::MetaData {
                    timestamp: 0,
                    data: meta,
                });
                let _ = frame_tx.try_send(FrameData::Audio {
                    timestamp: 100,
                    data: BytesMut::from(&[0x01][..]),
                });
                let _ = frame_tx.try_send(FrameData::Video {
                    timestamp: 200,
                    data: BytesMut::from(&[0x00, 0x01][..]),
                });
                drop(frame_tx);
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(result.is_ok());

        let send_task = tokio::spawn(async move { httpflv.send_media_stream().await });

        let result = send_task.await.expect("send_media_stream task panicked");
        assert!(result.is_ok());

        let _ = response_receiver.next().await;
        let _ = hub_task.await;
    }

    #[tokio::test]
    async fn test_send_media_stream_header_phase_max_frames_without_av_guesses() {
        let (event_sender, mut event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_tx, frame_rx) = crate::hub::define::frame_data_channel();
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_rx),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));

                for _ in 0..12 {
                    let mut meta = BytesMut::new();
                    meta.extend_from_slice(b"@setDataFrame");
                    meta.extend_from_slice(b"onMetaData");
                    let _ = frame_tx.try_send(FrameData::MetaData {
                        timestamp: 0,
                        data: meta,
                    });
                }
                drop(frame_tx);
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(result.is_ok());

        let send_task = tokio::spawn(async move { httpflv.send_media_stream().await });

        let result = send_task.await.expect("send_media_stream task panicked");
        assert!(result.is_ok());

        let _ = response_receiver.next().await;
        let _ = hub_task.await;
    }

    #[tokio::test]
    async fn test_send_media_stream_process_frame_with_retry_disconnected_breaks_loop() {
        let (event_sender, mut event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_tx, frame_rx) = crate::hub::define::frame_data_channel();
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_rx),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));

                let _ = frame_tx.try_send(FrameData::Audio {
                    timestamp: 0,
                    data: BytesMut::from(&[0x01][..]),
                });
                let _ = frame_tx.try_send(FrameData::Video {
                    timestamp: 0,
                    data: BytesMut::from(&[0x00][..]),
                });
                let _ = frame_tx.try_send(FrameData::Audio {
                    timestamp: 100,
                    data: BytesMut::from(&[0x02][..]),
                });
                drop(frame_tx);
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(result.is_ok());

        let drain_task = tokio::spawn(async move {
            for _ in 0..3 {
                let _ = response_receiver.next().await;
            }
            drop(response_receiver);
        });

        let result = httpflv.send_media_stream().await;
        assert!(
            result.is_ok(),
            "send_media_stream should complete when response channel disconnected after header"
        );

        let _ = hub_task.await;
        let _ = drain_task.await;
    }

    #[tokio::test]
    async fn test_send_media_stream_frame_phase_success() {
        let (event_sender, mut event_receiver, response_sender, mut response_receiver) =
            create_test_channels();
        let remote_addr = create_test_socket_addr();

        let mut httpflv = HttpFlv::new(
            "live".to_string(),
            "stream1".to_string(),
            event_sender,
            response_sender,
            "http://localhost/live/stream1.flv".to_string(),
            remote_addr,
        );

        let hub_task = tokio::spawn(async move {
            if let Some(StreamHubEvent::Subscribe { result_sender, .. }) =
                event_receiver.recv().await
            {
                let (frame_tx, frame_rx) = crate::hub::define::frame_data_channel();
                let data_receiver = DataReceiver {
                    frame_receiver: Some(frame_rx),
                    packet_receiver: None,
                };
                let _ = result_sender.send(Ok((data_receiver, None)));

                let _ = frame_tx.try_send(FrameData::Audio {
                    timestamp: 0,
                    data: BytesMut::from(&[0x01][..]),
                });
                let _ = frame_tx.try_send(FrameData::Video {
                    timestamp: 0,
                    data: BytesMut::from(&[0x00, 0x01][..]),
                });
                for i in 0..5 {
                    let _ = frame_tx.try_send(FrameData::Video {
                        timestamp: (i + 1) * 33,
                        data: BytesMut::from(&[0x00, 0x01, 0x02][..]),
                    });
                }
                drop(frame_tx);
            }
        });

        let result = httpflv.subscribe_from_stream_hub().await;
        assert!(result.is_ok());

        let send_task = tokio::spawn(async move { httpflv.send_media_stream().await });

        let result = send_task.await.expect("send_media_stream task panicked");
        assert!(result.is_ok());

        let mut count = 0;
        while (response_receiver.next().await).is_some() {
            count += 1;
            if count >= 3 {
                break;
            }
        }
        let _ = hub_task.await;
    }
}
