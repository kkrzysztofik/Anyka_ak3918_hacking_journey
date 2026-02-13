use crate::streamhub::define::{StatisticData, StatisticDataSender};
use tokio::sync::oneshot;
use {
    super::{
        define::{HttpResponseDataProducer, tag_type},
        errors::{HttpFLvError, HttpFLvErrorValue},
    },
    crate::container::amf0::amf0_writer::Amf0Writer,
    crate::container::muxer::{FlvMuxer, HEADER_LENGTH},
    crate::streamhub::define::{
        FrameData, FrameDataReceiver, NotifyInfo, StreamHubEvent, StreamHubEventSender,
        SubDataType, SubscribeType, SubscriberInfo,
    },
    crate::streamhub::{
        stream::StreamIdentifier,
        utils::{RandomDigitCount, Uuid},
    },
    bytes::BytesMut,
    std::net::SocketAddr,
    tokio::sync::mpsc,
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
        let (_, data_receiver) = mpsc::unbounded_channel();
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
            FrameData::MetaData { .. } => {
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
                log::info!("write_flv_tag: {}", err_in);
                return 11;
            }
            log::error!("write_flv_tag err: {}", err);
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
            FrameData::MetaData { timestamp, data } => {
                let processed = self.process_metadata(data)?;
                (processed, timestamp, tag_type::SCRIPT_DATA_AMF)
            }
            _ => {
                return Err(HttpFLvError {
                    value: HttpFLvErrorValue::UnexpectedFrameData(format!("{:?}", channel_data)),
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
            log::error!("send statistic data err: {}", err);
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
            log::error!("send statistic data err: {}", err);
        }
    }

    fn process_metadata(&self, data: bytes::BytesMut) -> Result<BytesMut, HttpFLvError> {
        let mut amf_writer: Amf0Writer = Amf0Writer::new();
        amf_writer.write_string(&String::from("@setDataFrame"))?;
        let (_, right) = data.split_at(amf_writer.len());
        Ok(BytesMut::from(right))
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
            sub_type: SubscribeType::RtmpRemux2HttpFlv,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr: self.remote_addr.to_string(),
            },
        };

        let identifier = StreamIdentifier::Rtmp {
            app_name: self.app_name.clone(),
            stream_name: self.stream_name.clone(),
        };

        let subscribe_event = StreamHubEvent::UnSubscribe {
            identifier,
            info: sub_info,
        };
        if let Err(err) = self.event_producer.send(subscribe_event) {
            log::error!("unsubscribe_from_stream_hub err {}", err);
        }

        Ok(())
    }

    pub async fn subscribe_from_stream_hub(&mut self) -> Result<(), HttpFLvError> {
        let sub_info = SubscriberInfo {
            id: self.subscriber_id,
            sub_type: SubscribeType::RtmpRemux2HttpFlv,
            sub_data_type: SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: self.request_url.clone(),
                remote_addr: self.remote_addr.to_string(),
            },
        };

        let identifier = StreamIdentifier::Rtmp {
            app_name: self.app_name.clone(),
            stream_name: self.stream_name.clone(),
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
                sub_type: SubscribeType::RtmpRemux2HttpFlv,
            };
            if let Err(err) = sender.send(statistic_subscriber) {
                log::error!("send statistic_subscriber err: {}", err);
            }
        }

        Ok(())
    }
}
