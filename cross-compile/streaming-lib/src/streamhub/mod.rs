use crate::container::define::aac_packet_type;
use define::{
    FrameDataReceiver, PacketDataReceiver, PacketDataSender, RelayType, StatisticData,
    StatisticDataReceiver, StatisticDataSender,
};
use serde_json::{Value, json};
use statistics::{StatisticSubscriber, StatisticsStream};
use tokio::sync::oneshot;

use define::PacketData;

pub mod define;
pub mod errors;
pub mod mock_publisher;
pub mod notify;
pub mod statistics;
pub mod stream;
pub mod utils;

use {
    define::{
        BroadcastEvent, BroadcastEventReceiver, BroadcastEventSender, DataReceiver, DataSender,
        FrameData, FrameDataSender, Information, StreamHubEvent, StreamHubEventReceiver,
        StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler, TransceiverEvent,
        TransceiverEventReceiver, TransceiverEventSender,
    },
    errors::{StreamHubError, StreamHubErrorValue},
    notify::Notifier,
    std::collections::HashMap,
    std::sync::Arc,
    stream::StreamIdentifier,
    tokio::sync::{Mutex, broadcast, mpsc, mpsc::UnboundedReceiver},
    utils::Uuid,
};

//Receive audio data/video data/meta data/media info from a publisher and send to players/subscribers
//Receive statistic information from a publisher and send to api callers.
pub struct StreamDataTransceiver {
    //used for receiving Audio/Video data from publishers
    data_receiver: DataReceiver,
    //used for receiving event
    event_receiver: TransceiverEventReceiver,
    //used for sending audio/video frame data to players/subscribers
    id_to_frame_sender: Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
    //used for sending audio/video packet data to players/subscribers
    id_to_packet_sender: Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
    //publisher and subscribers use this sender to submit statistical data
    statistic_data_sender: StatisticDataSender,
    //used for receiving statistical data from publishers and subscribers
    statistic_data_receiver: StatisticDataReceiver,
    //The publisher and subscribers's statistics data of a stream need to be aggregated and sent to the caller as needed.
    statistic_data: Arc<Mutex<StatisticsStream>>,
    //a hander implement by protocols, such as rtmp, webrtc, http-flv, hls
    stream_handler: Arc<dyn TStreamHandler>,
}

impl StreamDataTransceiver {
    fn new(
        data_receiver: DataReceiver,
        event_receiver: UnboundedReceiver<TransceiverEvent>,
        identifier: StreamIdentifier,
        h: Arc<dyn TStreamHandler>,
    ) -> Self {
        let (statistic_data_sender, statistic_data_receiver) = mpsc::unbounded_channel();
        Self {
            data_receiver,
            event_receiver,
            statistic_data_sender,
            statistic_data_receiver,
            id_to_frame_sender: Arc::new(Mutex::new(HashMap::new())),
            id_to_packet_sender: Arc::new(Mutex::new(HashMap::new())),
            stream_handler: h,
            statistic_data: Arc::new(Mutex::new(StatisticsStream::new(identifier))),
        }
    }

    async fn receive_frame_data(
        data: Option<FrameData>,
        frame_senders: &Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
    ) {
        if let Some(val) = data {
            match val {
                FrameData::MetaData {
                    timestamp: _,
                    data: _,
                } => {}
                FrameData::Audio { timestamp, data } => {
                    let data = FrameData::Audio {
                        timestamp,
                        data: data.clone(),
                    };

                    for (_, v) in frame_senders.lock().await.iter() {
                        if let Err(audio_err) = v.send(data.clone()).map_err(|_| StreamHubError {
                            value: StreamHubErrorValue::SendAudioError,
                        }) {
                            log::error!("Transmiter send error: {}", audio_err);
                        }
                    }
                }
                FrameData::Video { timestamp, data } => {
                    let data = FrameData::Video {
                        timestamp,
                        data: data.clone(),
                    };
                    for (_, v) in frame_senders.lock().await.iter() {
                        if let Err(video_err) = v.send(data.clone()).map_err(|_| StreamHubError {
                            value: StreamHubErrorValue::SendVideoError,
                        }) {
                            log::error!("Transmiter send error: {}", video_err);
                        }
                    }
                }
                FrameData::MediaInfo {
                    media_info: info_value,
                } => {
                    let data = FrameData::MediaInfo {
                        media_info: info_value,
                    };
                    for (_, v) in frame_senders.lock().await.iter() {
                        if let Err(media_err) = v.send(data.clone()).map_err(|_| StreamHubError {
                            value: StreamHubErrorValue::SendVideoError,
                        }) {
                            log::error!("Transmiter send error: {}", media_err);
                        }
                    }
                }
            }
        }
    }

    async fn receive_frame_data_loop(
        mut exit: broadcast::Receiver<()>,
        mut receiver: FrameDataReceiver,
        frame_senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data = receiver.recv() => {
                       if data.is_none() {
                           break;
                       }
                       Self::receive_frame_data(data, &frame_senders).await;
                    }
                    _ = exit.recv()=>{
                        break;
                    }
                }
            }
        });
    }

    async fn receive_packet_data(
        data: Option<PacketData>,
        packet_senders: &Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
    ) {
        if let Some(val) = data {
            match val {
                PacketData::Audio { timestamp, data } => {
                    let data = PacketData::Audio {
                        timestamp,
                        data: data.clone(),
                    };

                    for (_, v) in packet_senders.lock().await.iter() {
                        if let Err(audio_err) = v.send(data.clone()).map_err(|_| StreamHubError {
                            value: StreamHubErrorValue::SendAudioError,
                        }) {
                            log::error!("Transmiter send error: {}", audio_err);
                        }
                    }
                }
                PacketData::Video { timestamp, data } => {
                    let data = PacketData::Video {
                        timestamp,
                        data: data.clone(),
                    };
                    for (_, v) in packet_senders.lock().await.iter() {
                        if let Err(video_err) = v.send(data.clone()).map_err(|_| StreamHubError {
                            value: StreamHubErrorValue::SendVideoError,
                        }) {
                            log::error!("Transmiter send error: {}", video_err);
                        }
                    }
                }
            }
        }
    }

    async fn receive_packet_data_loop(
        mut exit: broadcast::Receiver<()>,
        mut receiver: PacketDataReceiver,
        packet_senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
    ) {
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data = receiver.recv() => {
                       if data.is_none() {
                           break;
                       }
                       Self::receive_packet_data(data, &packet_senders).await;
                    }
                    _ = exit.recv()=>{
                        break;
                    }
                }
            }
        });
    }

    async fn receive_statistics_data(
        data: Option<StatisticData>,
        statistics_data: &Arc<Mutex<StatisticsStream>>,
    ) {
        if let Some(val) = data {
            match val {
                StatisticData::Audio {
                    uuid,
                    data_size,
                    aac_packet_type,
                    duration: _,
                } => {
                    if let Some(uid) = uuid {
                        {
                            let subscriber = &mut statistics_data.lock().await.subscribers;
                            if let Some(sub) = subscriber.get_mut(&uid) {
                                sub.send_bytes += data_size;
                            }
                        }

                        statistics_data.lock().await.total_send_bytes += data_size;
                    } else {
                        match aac_packet_type {
                            aac_packet_type::AAC_RAW => {
                                let audio_data = &mut statistics_data.lock().await.publisher.audio;
                                audio_data.recv_bytes += data_size;
                            }
                            aac_packet_type::AAC_SEQHDR => {}
                            _ => {}
                        }
                        statistics_data.lock().await.total_recv_bytes += data_size;
                    }
                }
                StatisticData::Video {
                    uuid,
                    data_size,
                    frame_count,
                    is_key_frame,
                    duration: _,
                } => {
                    //if it is a subscriber, we need to update the send_bytes
                    if let Some(uid) = uuid {
                        {
                            let subscriber = &mut statistics_data.lock().await.subscribers;
                            if let Some(sub) = subscriber.get_mut(&uid) {
                                sub.send_bytes += data_size;
                                sub.total_send_bytes += data_size;
                            }
                        }

                        statistics_data.lock().await.total_send_bytes += data_size;
                    }
                    //if it is a publisher, we need to update the recv_bytes
                    else {
                        let stat_data = &mut statistics_data.lock().await;
                        stat_data.total_recv_bytes += data_size;
                        stat_data.publisher.video.recv_bytes += data_size;
                        stat_data.publisher.video.recv_frame_count += frame_count;
                        stat_data.publisher.recv_bytes += data_size;
                        if let Some(is_key) = is_key_frame {
                            if is_key {
                                stat_data.publisher.video.gop =
                                    stat_data.publisher.video.recv_frame_count_for_gop;
                                stat_data.publisher.video.recv_frame_count_for_gop = 1;
                            } else {
                                stat_data.publisher.video.recv_frame_count_for_gop += frame_count;
                            }
                        }
                    }
                }
                StatisticData::AudioCodec {
                    sound_format,
                    profile,
                    samplerate,
                    channels,
                } => {
                    let audio_codec_data = &mut statistics_data.lock().await.publisher.audio;
                    audio_codec_data.sound_format = sound_format;
                    audio_codec_data.profile = profile;
                    audio_codec_data.samplerate = samplerate;
                    audio_codec_data.channels = channels;
                }
                StatisticData::VideoCodec {
                    codec,
                    profile,
                    level,
                    width,
                    height,
                } => {
                    let video_codec_data = &mut statistics_data.lock().await.publisher.video;
                    video_codec_data.codec = codec;
                    video_codec_data.profile = profile;
                    video_codec_data.level = level;
                    video_codec_data.width = width;
                    video_codec_data.height = height;
                }
                StatisticData::Publisher {
                    id,
                    remote_addr,
                    start_time,
                } => {
                    let publisher = &mut statistics_data.lock().await.publisher;
                    publisher.id = id;
                    publisher.remote_address = remote_addr;

                    publisher.start_time = start_time;
                }
                StatisticData::Subscriber {
                    id,
                    remote_addr,
                    sub_type,
                    start_time,
                } => {
                    let subscriber = &mut statistics_data.lock().await.subscribers;
                    let sub = StatisticSubscriber {
                        id,
                        remote_address: remote_addr,
                        sub_type,
                        start_time,
                        send_bitrate: 0,
                        send_bytes: 0,
                        total_send_bytes: 0,
                    };
                    subscriber.insert(id, sub);
                }
            }
        }
    }

    async fn receive_statistics_data_loop(
        mut exit_receive: broadcast::Receiver<()>,
        exit_calculate: broadcast::Receiver<()>,
        mut receiver: StatisticDataReceiver,
        statistics_data: Arc<Mutex<StatisticsStream>>,
    ) {
        let mut statistic_calculate =
            statistics::StatisticsCalculate::new(statistics_data.clone(), exit_calculate);
        tokio::spawn(async move { statistic_calculate.start().await });

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    data = receiver.recv()  =>
                    {
                        if data.is_none() {
                            break;
                        }
                        Self::receive_statistics_data(data, &statistics_data).await;
                    }
                    _ = exit_receive.recv()=>{
                        break;
                    }
                }
            }
        });
    }

    async fn receive_event_loop(
        stream_handler: Arc<dyn TStreamHandler>,
        exit: broadcast::Sender<()>,
        mut receiver: TransceiverEventReceiver,
        packet_senders: Arc<Mutex<HashMap<Uuid, PacketDataSender>>>,
        frame_senders: Arc<Mutex<HashMap<Uuid, FrameDataSender>>>,
        statistic_sender: StatisticDataSender,
        statistics_data: Arc<Mutex<StatisticsStream>>,
    ) {
        tokio::spawn(async move {
            while let Some(val) = receiver.recv().await {
                match val {
                    TransceiverEvent::Subscribe {
                        sender,
                        info,
                        result_sender,
                    } => {
                        if let Err(err) = stream_handler
                            .send_prior_data(sender.clone(), info.sub_type)
                            .await
                        {
                            log::error!("receive_event_loop send_prior_data err: {}", err);
                            break;
                        }
                        match sender {
                            DataSender::Frame {
                                sender: frame_sender,
                            } => {
                                frame_senders.lock().await.insert(info.id, frame_sender);
                            }
                            DataSender::Packet {
                                sender: packet_sender,
                            } => {
                                packet_senders.lock().await.insert(info.id, packet_sender);
                            }
                        }

                        if let Err(err) = result_sender.send(statistic_sender.clone()) {
                            log::error!("receive_event_loop:send statistic send err :{:?} ", err)
                        }

                        let mut statistics_data = statistics_data.lock().await;
                        statistics_data.subscriber_count += 1;
                    }
                    TransceiverEvent::UnSubscribe { info } => {
                        match info.sub_type {
                            SubscribeType::RtpPull | SubscribeType::WhepPull => {
                                packet_senders.lock().await.remove(&info.id);
                            }
                            _ => {
                                frame_senders.lock().await.remove(&info.id);
                            }
                        }
                        let mut statistics_data = statistics_data.lock().await;
                        let subscribers = &mut statistics_data.subscribers;
                        subscribers.remove(&info.id);

                        statistics_data.subscriber_count -= 1;
                    }
                    TransceiverEvent::UnPublish {} => {
                        if let Err(err) = exit.send(()) {
                            log::error!("TransmitterEvent::UnPublish send error: {}", err);
                        }
                        break;
                    }
                    TransceiverEvent::Api { sender, uuid } => {
                        log::info!("api:  stream identifier: {:?}", uuid);
                        let statistic_data = if let Some(uid) = uuid {
                            statistics_data.lock().await.query_by_uuid(uid)
                        } else {
                            log::info!("api2:  stream identifier: {:?}", statistics_data);
                            statistics_data.lock().await.clone()
                        };

                        if let Err(err) = sender.send(statistic_data) {
                            log::info!("Transmitter send avstatistic data err: {}", err);
                        }
                    }
                    TransceiverEvent::Request { sender } => {
                        stream_handler.send_information(sender).await;
                    }
                }
            }
        });
    }

    pub async fn run(self) -> Result<(), StreamHubError> {
        let (tx, _) = broadcast::channel::<()>(1);

        if let Some(receiver) = self.data_receiver.frame_receiver {
            Self::receive_frame_data_loop(
                tx.subscribe(),
                receiver,
                self.id_to_frame_sender.clone(),
            )
            .await;
        }

        if let Some(receiver) = self.data_receiver.packet_receiver {
            Self::receive_packet_data_loop(
                tx.subscribe(),
                receiver,
                self.id_to_packet_sender.clone(),
            )
            .await;
        }

        Self::receive_statistics_data_loop(
            tx.subscribe(),
            tx.subscribe(),
            self.statistic_data_receiver,
            self.statistic_data.clone(),
        )
        .await;

        Self::receive_event_loop(
            self.stream_handler,
            tx,
            self.event_receiver,
            self.id_to_packet_sender,
            self.id_to_frame_sender,
            self.statistic_data_sender,
            self.statistic_data.clone(),
        )
        .await;

        Ok(())
    }

    pub fn get_statistics_data_sender(&self) -> StatisticDataSender {
        self.statistic_data_sender.clone()
    }
}

pub struct StreamsHub {
    //stream identifier to transceiver event sender
    streams: HashMap<StreamIdentifier, TransceiverEventSender>,
    //construct UnSubscribe and UnPublish event from Subscribe and Publish event to kick off client
    un_pub_sub_events: HashMap<Uuid, StreamHubEvent>,
    //event is consumed in Stream hub, produced from other protocol sessions
    hub_event_receiver: StreamHubEventReceiver,
    //event is produced from other protocol sessions
    hub_event_sender: StreamHubEventSender,
    //
    client_event_sender: BroadcastEventSender,
    //The rtmp static push/pull and the hls transfer is triggered actively,
    //add a control switches separately.
    rtmp_push_enabled: bool,
    rtmp_remuxer_enabled: bool,
    //enable rtmp pull
    rtmp_pull_enabled: bool,
    //enable hls
    hls_enabled: bool,
    //http notifier on sub/pub event
    notifier: Option<Arc<dyn Notifier>>,
}

impl StreamsHub {
    pub fn new(notifier: Option<Arc<dyn Notifier>>) -> Self {
        let (event_producer, event_consumer) = mpsc::unbounded_channel();
        let (client_producer, _) = broadcast::channel(100);

        Self {
            streams: HashMap::new(),
            un_pub_sub_events: HashMap::new(),
            hub_event_receiver: event_consumer,
            hub_event_sender: event_producer,
            client_event_sender: client_producer,
            rtmp_push_enabled: false,
            rtmp_pull_enabled: false,
            rtmp_remuxer_enabled: false,
            hls_enabled: false,
            notifier,
        }
    }
    pub async fn run(&mut self) {
        self.event_loop().await;
    }

    pub fn set_rtmp_push_enabled(&mut self, enabled: bool) {
        self.rtmp_push_enabled = enabled;
    }

    pub fn set_rtmp_pull_enabled(&mut self, enabled: bool) {
        self.rtmp_pull_enabled = enabled;
    }

    pub fn set_rtmp_remuxer_enabled(&mut self, enabled: bool) {
        self.rtmp_remuxer_enabled = enabled;
    }

    pub fn set_hls_enabled(&mut self, enabled: bool) {
        self.hls_enabled = enabled;
    }

    pub fn get_hub_event_sender(&mut self) -> StreamHubEventSender {
        self.hub_event_sender.clone()
    }

    pub fn get_client_event_consumer(&mut self) -> BroadcastEventReceiver {
        self.client_event_sender.subscribe()
    }

    pub async fn event_loop(&mut self) {
        while let Some(event) = self.hub_event_receiver.recv().await {
            let message = event.to_message();

            match event {
                StreamHubEvent::Publish {
                    identifier,
                    info,
                    result_sender,
                    stream_handler,
                } => {
                    let (frame_sender, packet_sender, receiver) = match info.pub_data_type {
                        define::PubDataType::Frame => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                Some(sender_chan),
                                None,
                                DataReceiver {
                                    frame_receiver: Some(receiver_chan),
                                    packet_receiver: None,
                                },
                            )
                        }
                        define::PubDataType::Packet => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                None,
                                Some(sender_chan),
                                DataReceiver {
                                    frame_receiver: None,
                                    packet_receiver: Some(receiver_chan),
                                },
                            )
                        }
                        define::PubDataType::Both => {
                            let (sender_frame_chan, receiver_frame_chan) =
                                mpsc::unbounded_channel();
                            let (sender_packet_chan, receiver_packet_chan) =
                                mpsc::unbounded_channel();

                            (
                                Some(sender_frame_chan),
                                Some(sender_packet_chan),
                                DataReceiver {
                                    frame_receiver: Some(receiver_frame_chan),
                                    packet_receiver: Some(receiver_packet_chan),
                                },
                            )
                        }
                    };

                    let result = match self
                        .publish(identifier.clone(), receiver, stream_handler)
                        .await
                    {
                        Ok(statistic_data_sender) => {
                            if let Some(notifier) = &self.notifier {
                                notifier.on_publish_notify(&message).await;
                            }
                            self.un_pub_sub_events
                                .insert(info.id, StreamHubEvent::UnPublish { identifier, info });

                            Ok((frame_sender, packet_sender, Some(statistic_data_sender)))
                        }
                        Err(err) => {
                            log::error!("event_loop Publish err: {}", err);
                            Err(err)
                        }
                    };

                    if result_sender.send(result).is_err() {
                        log::error!("event_loop Subscribe error: The receiver dropped.")
                    }
                }

                StreamHubEvent::UnPublish { identifier, info } => {
                    if let Err(err) = self.unpublish(&identifier) {
                        log::error!(
                            "event_loop Unpublish err: {} with identifier: {}",
                            err,
                            identifier
                        );
                    }

                    self.un_pub_sub_events.remove(&info.id);

                    if let Some(notifier) = &self.notifier {
                        notifier.on_unpublish_notify(&message).await;
                    }
                }
                StreamHubEvent::Subscribe {
                    identifier,
                    info,
                    result_sender,
                } => {
                    let sub_id = info.id;
                    let info_clone = info.clone();

                    //new chan for Frame/Packet sender and receiver
                    let (sender, receiver) = match info.sub_data_type {
                        define::SubDataType::Frame => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                DataSender::Frame {
                                    sender: sender_chan,
                                },
                                DataReceiver {
                                    frame_receiver: Some(receiver_chan),
                                    packet_receiver: None,
                                },
                            )
                        }
                        define::SubDataType::Packet => {
                            let (sender_chan, receiver_chan) = mpsc::unbounded_channel();
                            (
                                DataSender::Packet {
                                    sender: sender_chan,
                                },
                                DataReceiver {
                                    frame_receiver: None,
                                    packet_receiver: Some(receiver_chan),
                                },
                            )
                        }
                    };

                    let rv = match self.subscribe(&identifier, info_clone, sender).await {
                        Ok(statistic_data_sender) => {
                            if let Some(notifier) = &self.notifier {
                                notifier.on_play_notify(&message).await;
                            }

                            self.un_pub_sub_events
                                .insert(sub_id, StreamHubEvent::UnSubscribe { identifier, info });
                            Ok((receiver, Some(statistic_data_sender)))
                        }
                        Err(err) => {
                            log::error!("event_loop Subscribe error: {}", err);
                            Err(err)
                        }
                    };

                    if result_sender.send(rv).is_err() {
                        log::error!("event_loop Subscribe error: The receiver dropped.")
                    }
                }
                StreamHubEvent::UnSubscribe { identifier, info } => {
                    let info_id = info.id;
                    if self.unsubscribe(&identifier, info).is_ok()
                        && let Some(notifier) = &self.notifier
                    {
                        notifier.on_stop_notify(&message).await;
                    }

                    self.un_pub_sub_events.remove(&info_id);
                }

                StreamHubEvent::ApiStatistic {
                    top_n,
                    identifier,
                    uuid,
                    result_sender,
                } => {
                    let result = match self.api_statistic(top_n, identifier, uuid).await {
                        Ok(rv) => rv,
                        Err(err) => {
                            log::error!("event_loop api error: {}", err);
                            json!(err.to_string())
                        }
                    };

                    if let Err(err) = result_sender.send(result) {
                        log::error!("event_loop api error: {}", err);
                    }
                }
                StreamHubEvent::ApiKickClient { id } => {
                    if let Err(err) = self.api_kick_off_client(id) {
                        log::error!("api_kick_off_client api error: {}", err);
                    }
                }
                StreamHubEvent::ApiStartRelayStream {
                    id,
                    identifier,
                    server_address,
                    relay_type,
                    result_sender,
                } => {
                    let result = self
                        .api_start_relay_stream(id, &relay_type, identifier, server_address)
                        .await;

                    if let Err(err) = result_sender.send(result) {
                        log::error!("event_loop api error: {:?}", err);
                    }
                }
                StreamHubEvent::ApiStopRelayStream {
                    id,
                    relay_type,
                    result_sender,
                } => {
                    let result = self.api_stop_relay_stream(id, &relay_type).await;

                    if let Err(err) = result_sender.send(result) {
                        log::error!("event_loop api error: {:?}", err);
                    }
                }
                StreamHubEvent::Request { identifier, sender } => {
                    if let Err(err) = self.request(&identifier, sender) {
                        log::error!("event_loop request error: {}", err);
                    }
                }
                StreamHubEvent::OnHls {
                    identifier: _,
                    segment: _,
                } => {
                    if let Some(notifier) = &self.notifier {
                        notifier.on_hls_notify(&message).await;
                    }
                }
            }
        }
    }

    fn request(
        &mut self,
        identifier: &StreamIdentifier,
        sender: mpsc::UnboundedSender<Information>,
    ) -> Result<(), StreamHubError> {
        if let Some(producer) = self.streams.get_mut(identifier) {
            let event = TransceiverEvent::Request { sender };
            log::info!("Request:  stream identifier: {}", identifier);
            producer.send(event).map_err(|_| StreamHubError {
                value: StreamHubErrorValue::SendError,
            })?;
        }
        Ok(())
    }

    async fn api_statistic(
        &mut self,
        top_n: Option<usize>,
        identifier: Option<StreamIdentifier>,
        uuid: Option<Uuid>,
    ) -> Result<Value, StreamHubError> {
        if self.streams.is_empty() {
            return Ok(json!({}));
        }
        log::info!("api_statistic:  stream identifier: {:?}", identifier);
        let (stream_sender, mut stream_receiver) = mpsc::unbounded_channel();

        let mut stream_count: usize = 1;

        if let Some(identifier) = identifier {
            if let Some(event_sender) = self.streams.get_mut(&identifier) {
                let event = TransceiverEvent::Api {
                    sender: stream_sender.clone(),
                    uuid,
                };
                log::info!("api_statistic:  stream identifier: {}", identifier);
                event_sender.send(event).map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
            }
        } else {
            stream_count = self.streams.len();
            for v in self.streams.values() {
                if let Err(err) = v.send(TransceiverEvent::Api {
                    sender: stream_sender.clone(),
                    uuid,
                }) {
                    log::error!("TransmitterEvent  api send data err: {}", err);
                    return Err(StreamHubError {
                        value: StreamHubErrorValue::SendError,
                    });
                }
            }
        }

        let mut data = Vec::new();

        loop {
            log::info!("api_statistic:  stream count: {}", stream_count);
            if let Some(stream_statistics) = stream_receiver.recv().await {
                data.push(stream_statistics);
            }
            if data.len() == stream_count {
                break;
            }
        }

        if let Some(topn) = top_n {
            data.sort_by(|a, b| b.subscriber_count.cmp(&a.subscriber_count));
            let top_streams: Vec<StatisticsStream> = data.into_iter().take(topn).collect();
            return Ok(serde_json::to_value(top_streams)?);
        }

        Ok(serde_json::to_value(data)?)
    }

    fn api_kick_off_client(&mut self, uid: Uuid) -> Result<(), StreamHubError> {
        if let Some(event) = self.un_pub_sub_events.get(&uid) {
            match event {
                StreamHubEvent::UnPublish { identifier, info } => {
                    if self
                        .hub_event_sender
                        .send(StreamHubEvent::UnPublish {
                            identifier: identifier.clone(),
                            info: info.clone(),
                        })
                        .is_err()
                    {
                        return Err(StreamHubError {
                            value: StreamHubErrorValue::SendError,
                        });
                    }
                }
                StreamHubEvent::UnSubscribe { identifier, info } => {
                    if self
                        .hub_event_sender
                        .send(StreamHubEvent::UnSubscribe {
                            identifier: identifier.clone(),
                            info: info.clone(),
                        })
                        .is_err()
                    {
                        return Err(StreamHubError {
                            value: StreamHubErrorValue::SendError,
                        });
                    }
                }
                _ => {}
            }
        } else {
            log::warn!("cannot find uid: {}", uid);
        };

        Ok(())
    }

    async fn api_start_relay_stream(
        &mut self,
        id: String,
        relay_type: &RelayType,
        identifier: StreamIdentifier,
        server_address: String,
    ) -> Result<(), StreamHubError> {
        let (result_sender, mut result_receiver) = mpsc::channel(1);

        match relay_type {
            RelayType::Pull => {
                let client_event = BroadcastEvent::Subscribe {
                    id,
                    identifier,
                    server_address: Some(server_address),
                    result_sender: Some(result_sender),
                };

                //send subscribe info to pull clients
                self.client_event_sender
                    .send(client_event)
                    .map_err(|_| StreamHubError {
                        value: StreamHubErrorValue::SendError,
                    })?;
            }
            RelayType::Push => {}
        }

        if let Some(received_message) = result_receiver.recv().await {
            return received_message;
        }
        Ok(())
    }

    async fn api_stop_relay_stream(
        &mut self,
        id: String,
        relay_type: &RelayType,
    ) -> Result<(), StreamHubError> {
        let (result_sender, mut result_receiver) = mpsc::channel(1);
        match relay_type {
            RelayType::Pull => {
                let client_event = BroadcastEvent::UnSubscribe {
                    id,
                    result_sender: Some(result_sender),
                };

                //send subscribe info to pull clients
                self.client_event_sender
                    .send(client_event)
                    .map_err(|_| StreamHubError {
                        value: StreamHubErrorValue::SendError,
                    })?;
            }
            RelayType::Push => {}
        }

        if let Some(received_message) = result_receiver.recv().await {
            return received_message;
        }
        Ok(())
    }

    //player subscribe a stream
    pub async fn subscribe(
        &mut self,
        identifer: &StreamIdentifier,
        sub_info: SubscriberInfo,
        sender: DataSender,
    ) -> Result<StatisticDataSender, StreamHubError> {
        if let Some(event_sender) = self.streams.get_mut(identifer) {
            let (result_sender, result_receiver) = oneshot::channel();
            let event = TransceiverEvent::Subscribe {
                sender,
                info: sub_info,
                result_sender,
            };
            log::info!("subscribe:  stream identifier: {}", identifer);
            event_sender.send(event).map_err(|_| StreamHubError {
                value: StreamHubErrorValue::SendError,
            })?;

            return Ok(result_receiver.await?);
        }

        if self.rtmp_pull_enabled {
            log::info!("subscribe: try to pull stream, identifier: {}", identifer);

            let client_event = BroadcastEvent::Subscribe {
                id: String::from("rtmp_relay"),
                identifier: identifer.clone(),
                server_address: None,
                result_sender: None,
            };

            //send subscribe info to pull clients
            self.client_event_sender
                .send(client_event)
                .map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
        }

        Err(StreamHubError {
            value: StreamHubErrorValue::NoAppOrStreamName,
        })
    }

    pub fn unsubscribe(
        &mut self,
        identifer: &StreamIdentifier,
        sub_info: SubscriberInfo,
    ) -> Result<(), StreamHubError> {
        match self.streams.get_mut(identifer) {
            Some(producer) => {
                log::info!("unsubscribe....:{}", identifer);
                let event = TransceiverEvent::UnSubscribe { info: sub_info };
                producer.send(event).map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
            }
            None => {
                log::info!("unsubscribe None....:{}", identifer);
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NoAppName,
                });
            }
        }

        Ok(())
    }

    //publish a stream
    pub async fn publish(
        &mut self,
        identifier: StreamIdentifier,
        receiver: DataReceiver,
        handler: Arc<dyn TStreamHandler>,
    ) -> Result<StatisticDataSender, StreamHubError> {
        if self.streams.contains_key(&identifier) {
            return Err(StreamHubError {
                value: StreamHubErrorValue::Exists,
            });
        }

        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let transceiver =
            StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), handler);

        let statistic_data_sender = transceiver.get_statistics_data_sender();
        let identifier_clone = identifier.clone();

        if let Err(err) = transceiver.run().await {
            log::error!(
                "transceiver run error, idetifier: {}, error: {}",
                identifier_clone,
                err,
            );
        } else {
            log::info!("transceiver run success, idetifier: {}", identifier_clone);
        }

        self.streams.insert(identifier.clone(), event_sender);

        if self.rtmp_push_enabled || self.hls_enabled || self.rtmp_remuxer_enabled {
            let client_event = BroadcastEvent::Publish { identifier };

            //send publish info to push clients
            self.client_event_sender
                .send(client_event)
                .map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
        }

        Ok(statistic_data_sender)
    }

    fn unpublish(&mut self, identifier: &StreamIdentifier) -> Result<(), StreamHubError> {
        match self.streams.get_mut(identifier) {
            Some(producer) => {
                let event = TransceiverEvent::UnPublish {};
                producer.send(event).map_err(|_| StreamHubError {
                    value: StreamHubErrorValue::SendError,
                })?;
                self.streams.remove(identifier);
                log::info!("unpublish remove stream, stream identifier: {}", identifier);
            }
            None => {
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NoAppName,
                });
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streamhub::define::{
        DataReceiver, DataSender, FrameData, FrameDataSender, Information, NotifyInfo, PacketData,
        PacketDataSender, PubDataType, PublishType, PublisherInfo, StatisticData, SubDataType,
        SubscribeType, SubscriberInfo,
    };
    use async_trait::async_trait;
    use bytes::BytesMut;
    use mockall::mock;
    use std::sync::Arc;
    use tokio::sync::{mpsc, oneshot};

    // Mock TStreamHandler for testing
    mock! {
        StreamHandler {}

        #[async_trait]
        impl TStreamHandler for StreamHandler {
            async fn send_prior_data(
                &self,
                sender: DataSender,
                sub_type: SubscribeType,
            ) -> Result<(), StreamHubError>;
            async fn get_statistic_data(&self) -> Option<StatisticsStream>;
            async fn send_information(&self, sender: super::define::InformationSender);
        }
    }

    fn create_test_stream_identifier() -> StreamIdentifier {
        StreamIdentifier::Rtmp {
            app_name: "live".to_string(),
            stream_name: "test".to_string(),
        }
    }

    fn create_test_publisher_info() -> PublisherInfo {
        PublisherInfo {
            id: Uuid::new(crate::streamhub::utils::RandomDigitCount::Four),
            pub_type: PublishType::RtmpPush,
            pub_data_type: PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: "rtmp://localhost/live/test".to_string(),
                remote_addr: "127.0.0.1:1935".to_string(),
            },
        }
    }

    fn create_test_subscriber_info() -> SubscriberInfo {
        SubscriberInfo {
            id: Uuid::new(crate::streamhub::utils::RandomDigitCount::Four),
            sub_type: SubscribeType::RtmpPull,
            notify_info: NotifyInfo {
                request_url: "rtmp://localhost/live/test".to_string(),
                remote_addr: "127.0.0.1:1935".to_string(),
            },
            sub_data_type: SubDataType::Frame,
        }
    }

    #[tokio::test]
    async fn test_streams_hub_new() {
        let hub = StreamsHub::new(None);
        assert_eq!(hub.streams.len(), 0);
        assert_eq!(hub.un_pub_sub_events.len(), 0);
        assert!(!hub.rtmp_push_enabled);
        assert!(!hub.rtmp_pull_enabled);
        assert!(!hub.rtmp_remuxer_enabled);
        assert!(!hub.hls_enabled);
    }

    #[tokio::test]
    async fn test_streams_hub_set_flags() {
        let mut hub = StreamsHub::new(None);
        hub.set_rtmp_push_enabled(true);
        hub.set_rtmp_pull_enabled(true);
        hub.set_rtmp_remuxer_enabled(true);
        hub.set_hls_enabled(true);

        assert!(hub.rtmp_push_enabled);
        assert!(hub.rtmp_pull_enabled);
        assert!(hub.rtmp_remuxer_enabled);
        assert!(hub.hls_enabled);
    }

    #[tokio::test]
    async fn test_streams_hub_publish_success() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let receiver = DataReceiver {
            frame_receiver: Some(frame_receiver),
            packet_receiver: None,
        };

        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .times(0)
            .returning(|_, _| Ok(()));
        let mock_handler = Arc::new(mock_handler);

        let result = hub
            .publish(identifier.clone(), receiver, mock_handler)
            .await;

        assert!(result.is_ok());
        assert!(hub.streams.contains_key(&identifier));
    }

    #[tokio::test]
    async fn test_streams_hub_publish_duplicate() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (_frame_sender1, frame_receiver1) = mpsc::unbounded_channel();
        let (_frame_sender2, frame_receiver2) = mpsc::unbounded_channel();

        let receiver1 = DataReceiver {
            frame_receiver: Some(frame_receiver1),
            packet_receiver: None,
        };
        let receiver2 = DataReceiver {
            frame_receiver: Some(frame_receiver2),
            packet_receiver: None,
        };

        let mut mock_handler1 = MockStreamHandler::new();
        mock_handler1
            .expect_send_prior_data()
            .times(0)
            .returning(|_, _| Ok(()));
        let mock_handler1 = Arc::new(mock_handler1);

        let mut mock_handler2 = MockStreamHandler::new();
        mock_handler2
            .expect_send_prior_data()
            .times(0)
            .returning(|_, _| Ok(()));
        let mock_handler2 = Arc::new(mock_handler2);

        let result1 = hub
            .publish(identifier.clone(), receiver1, mock_handler1)
            .await;
        assert!(result1.is_ok());

        let result2 = hub
            .publish(identifier.clone(), receiver2, mock_handler2)
            .await;
        assert!(result2.is_err());
        match result2.unwrap_err().value {
            StreamHubErrorValue::Exists => {}
            _ => panic!("Expected Exists error"),
        }
    }

    #[tokio::test]
    async fn test_streams_hub_subscribe_success() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let receiver = DataReceiver {
            frame_receiver: Some(frame_receiver),
            packet_receiver: None,
        };

        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .returning(|_, _| Ok(()));
        let mock_handler = Arc::new(mock_handler);

        // First publish
        let _ = hub
            .publish(identifier.clone(), receiver, mock_handler.clone())
            .await;

        // Then subscribe
        let sub_info = create_test_subscriber_info();
        let (sender, _) = mpsc::unbounded_channel();
        let data_sender = DataSender::Frame { sender };

        let result = hub
            .subscribe(&identifier, sub_info.clone(), data_sender)
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_streams_hub_subscribe_no_stream() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let sub_info = create_test_subscriber_info();
        let (sender, _) = mpsc::unbounded_channel();
        let data_sender = DataSender::Frame { sender };

        let result = hub.subscribe(&identifier, sub_info, data_sender).await;

        assert!(result.is_err());
        match result.unwrap_err().value {
            StreamHubErrorValue::NoAppOrStreamName => {}
            _ => panic!("Expected NoAppOrStreamName error"),
        }
    }

    #[tokio::test]
    async fn test_streams_hub_unsubscribe_success() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let receiver = DataReceiver {
            frame_receiver: Some(frame_receiver),
            packet_receiver: None,
        };

        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .returning(|_, _| Ok(()));
        let mock_handler = Arc::new(mock_handler);

        // Publish
        let _ = hub
            .publish(identifier.clone(), receiver, mock_handler.clone())
            .await;

        // Subscribe
        let sub_info = create_test_subscriber_info();
        let (sender, _) = mpsc::unbounded_channel();
        let data_sender = DataSender::Frame { sender };
        let _ = hub
            .subscribe(&identifier, sub_info.clone(), data_sender)
            .await;

        // Unsubscribe
        let result = hub.unsubscribe(&identifier, sub_info);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_streams_hub_unsubscribe_no_stream() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let sub_info = create_test_subscriber_info();

        let result = hub.unsubscribe(&identifier, sub_info);
        assert!(result.is_err());
        match result.unwrap_err().value {
            StreamHubErrorValue::NoAppName => {}
            _ => panic!("Expected NoAppName error"),
        }
    }

    #[tokio::test]
    async fn test_streams_hub_unpublish_success() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let receiver = DataReceiver {
            frame_receiver: Some(frame_receiver),
            packet_receiver: None,
        };

        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .times(0)
            .returning(|_, _| Ok(()));
        let mock_handler = Arc::new(mock_handler);

        // Publish
        let _ = hub
            .publish(identifier.clone(), receiver, mock_handler.clone())
            .await;
        assert!(hub.streams.contains_key(&identifier));

        // Unpublish
        let result = hub.unpublish(&identifier);
        assert!(result.is_ok());
        assert!(!hub.streams.contains_key(&identifier));
    }

    #[tokio::test]
    async fn test_streams_hub_unpublish_no_stream() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();

        let result = hub.unpublish(&identifier);
        assert!(result.is_err());
        match result.unwrap_err().value {
            StreamHubErrorValue::NoAppName => {}
            _ => panic!("Expected NoAppName error"),
        }
    }

    #[tokio::test]
    async fn test_streams_hub_request_success() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (_frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let receiver = DataReceiver {
            frame_receiver: Some(frame_receiver),
            packet_receiver: None,
        };

        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .times(0)
            .returning(|_, _| Ok(()));
        mock_handler
            .expect_send_information()
            .times(1)
            .returning(|_| {});
        let mock_handler = Arc::new(mock_handler);

        // Publish
        let _ = hub
            .publish(identifier.clone(), receiver, mock_handler.clone())
            .await;

        // Request
        let (info_sender, _) = mpsc::unbounded_channel();
        let result = hub.request(&identifier, info_sender);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_streams_hub_request_no_stream() {
        let mut hub = StreamsHub::new(None);
        let identifier = create_test_stream_identifier();
        let (info_sender, _) = mpsc::unbounded_channel();

        let result = hub.request(&identifier, info_sender);
        // Request doesn't fail if stream doesn't exist, it just does nothing
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_data_transceiver_new() {
        let (_data_sender, data_receiver) = mpsc::unbounded_channel();
        let (_event_sender, event_receiver) = mpsc::unbounded_channel();
        let identifier = create_test_stream_identifier();
        let mock_handler = Arc::new(MockStreamHandler::new());

        let receiver = DataReceiver {
            frame_receiver: Some(data_receiver),
            packet_receiver: None,
        };

        let transceiver =
            StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), mock_handler);

        let statistic_sender = transceiver.get_statistics_data_sender();
        assert!(
            statistic_sender
                .send(StatisticData::Publisher {
                    id: Uuid::new(crate::streamhub::utils::RandomDigitCount::Four),
                    remote_addr: "127.0.0.1:1935".to_string(),
                    start_time: chrono::Local::now(),
                })
                .is_ok()
        );
    }
    #[tokio::test]
    async fn test_stream_data_transceiver_frame_forwarding() {
        let (frame_sender, frame_receiver) = mpsc::unbounded_channel();
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let identifier = create_test_stream_identifier();
        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .returning(|_, _| Ok(()));
        let mock_handler = Arc::new(mock_handler);

        let receiver = DataReceiver {
            frame_receiver: Some(frame_receiver),
            packet_receiver: None,
        };

        let mut transceiver =
            StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), mock_handler);

        // Run transceiver in background
        tokio::spawn(async move {
            let _ = transceiver.run().await;
        });

        // 1. Subscribe
        let (sub_frame_sender, mut sub_frame_receiver) = mpsc::unbounded_channel();
        let sub_info = create_test_subscriber_info();
        let data_sender = DataSender::Frame {
            sender: sub_frame_sender,
        };

        let (sub_result_sender, _) = oneshot::channel();
        event_sender
            .send(TransceiverEvent::Subscribe {
                sender: data_sender,
                info: sub_info,
                result_sender: sub_result_sender,
            })
            .unwrap();

        // Allow some time for subscription processing
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 2. Send Frame Data
        let frame_data = FrameData::Audio {
            timestamp: 100,
            data: BytesMut::from(&[0x01, 0x02, 0x03][..]),
        };

        frame_sender.send(frame_data).unwrap();

        // 3. Verify Subscriber Received Data
        let received = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            sub_frame_receiver.recv(),
        )
        .await
        .expect("Timeout waiting for frame data");

        assert!(received.is_some());
        match received.unwrap() {
            FrameData::Audio { timestamp, data } => {
                assert_eq!(timestamp, 100);
                assert_eq!(data, BytesMut::from(&[0x01, 0x02, 0x03][..]));
            }
            _ => panic!("Expected Audio frame"),
        }
    }

    #[tokio::test]
    async fn test_stream_data_transceiver_packet_forwarding() {
        let (packet_sender, packet_receiver) = mpsc::unbounded_channel();
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        let identifier = create_test_stream_identifier();
        let mut mock_handler = MockStreamHandler::new();
        mock_handler
            .expect_send_prior_data()
            .returning(|_, _| Ok(()));
        let mock_handler = Arc::new(mock_handler);

        let receiver = DataReceiver {
            frame_receiver: None,
            packet_receiver: Some(packet_receiver),
        };

        let mut transceiver =
            StreamDataTransceiver::new(receiver, event_receiver, identifier.clone(), mock_handler);

        tokio::spawn(async move {
            let _ = transceiver.run().await;
        });

        // 1. Subscribe
        let (sub_packet_sender, mut sub_packet_receiver) = mpsc::unbounded_channel();
        let mut sub_info = create_test_subscriber_info();
        sub_info.sub_data_type = SubDataType::Packet;

        let data_sender = DataSender::Packet {
            sender: sub_packet_sender,
        };

        let (sub_result_sender, _) = oneshot::channel();
        event_sender
            .send(TransceiverEvent::Subscribe {
                sender: data_sender,
                info: sub_info,
                result_sender: sub_result_sender,
            })
            .unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // 2. Send Packet Data
        let packet_data = PacketData::Audio {
            timestamp: 200,
            data: BytesMut::from(&[0x0A, 0x0B][..]),
        };

        packet_sender.send(packet_data).unwrap();

        // 3. Verify Subscriber Received Data
        let received = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            sub_packet_receiver.recv(),
        )
        .await
        .expect("Timeout waiting for packet data");

        assert!(received.is_some());
        match received.unwrap() {
            PacketData::Audio { timestamp, data } => {
                assert_eq!(timestamp, 200);
                assert_eq!(data, BytesMut::from(&[0x0A, 0x0B][..]));
            }
            _ => panic!("Expected Audio packet"),
        }
    }
}
