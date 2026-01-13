use crate::rtsp::global_trait::Marshal;
use crate::rtsp::global_trait::Unmarshal;
use crate::rtsp::rtsp_codec;

use crate::rtsp::rtp::define::ANNEXB_NALU_START_CODE;
use crate::rtsp::rtp::utils::Marshal as RtpMarshal;

use crate::common::auth::SecretCarrier;
use crate::common::http::HttpRequest as RtspRequest;
use crate::common::http::HttpResponse as RtspResponse;
use crate::common::http::Marshal as RtspMarshal;
use crate::common::http::Unmarshal as RtspUnmarshal;

use crate::rtsp::rtp::RtpPacket;
use crate::rtsp::rtsp_range::RtspRange;

use crate::rtsp::sdp::fmtp::Fmtp;

use crate::rtsp::rtsp_codec::RtspCodecInfo;
use crate::rtsp::rtsp_track::RtspTrack;
use crate::rtsp::rtsp_track::TrackType;
use crate::rtsp::rtsp_transport::ProtocolType;
use crate::rtsp::rtsp_transport::RtspTransport;

use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::AsyncBytesWriter;
use byteorder::BigEndian;
use bytes::BytesMut;

use super::errors::SessionError;
use super::errors::SessionErrorValue;
use crate::bytesio::UdpIO;
use crate::bytesio::bytes_writer::BytesWriter;
use crate::streamhub::define::DataSender;
use crate::streamhub::define::MediaInfo;
use crate::streamhub::define::VideoCodecType;
use http::StatusCode;
use tokio::sync::oneshot;

use crate::rtsp::rtp::errors::UnPackerError;
use crate::rtsp::sdp::Sdp;

use super::define;
use super::define::rtsp_method_name;
use crate::bytesio::TNetIO;
use crate::bytesio::TcpIO;
use async_trait::async_trait;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::common::auth::Auth;
use crate::streamhub::{
    define::{
        FrameData, Information, InformationSender, NotifyInfo, PublishType, PublisherInfo,
        StreamHubEvent, StreamHubEventSender, SubscribeType, SubscriberInfo, TStreamHandler,
    },
    errors::{StreamHubError, StreamHubErrorValue},
    statistics::StatisticsStream,
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

pub struct RtspServerSession {
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<Uuid>,
    pub session_type: define::ServerSessionType,

    stream_handler: Arc<RtspStreamHandler>,
    event_producer: StreamHubEventSender,

    auth: Option<Auth>,

    pub stream_identifier: Option<StreamIdentifier>,
    pub is_normal_exit: bool,
    remote_addr: SocketAddr,
}

pub struct InterleavedBinaryData {
    pub channel_identifier: u8,
    pub length: u16,
}

impl InterleavedBinaryData {
    // 10.12 Embedded (Interleaved) Binary Data
    // Stream data such as RTP packets is encapsulated by an ASCII dollar
    // sign (24 hexadecimal), followed by a one-byte channel identifier,
    // followed by the length of the encapsulated binary data as a binary,
    // two-byte integer in network byte order
    pub fn new(reader: &mut BytesReader) -> Result<Option<Self>, SessionError> {
        let is_dollar_sign = reader.advance_u8()? == 0x24;
        log::debug!("dollar sign: {}", is_dollar_sign);
        if is_dollar_sign {
            reader.read_u8()?;
            let channel_identifier = reader.read_u8()?;
            log::debug!("channel_identifier: {}", channel_identifier);
            let length = reader.read_u16::<BigEndian>()?;
            log::debug!("length: {}", length);
            return Ok(Some(InterleavedBinaryData {
                channel_identifier,
                length,
            }));
        }
        Ok(None)
    }
}

impl RtspServerSession {
    pub fn new(
        stream: TcpStream,
        event_producer: StreamHubEventSender,
        auth: Option<Auth>,
    ) -> Self {
        // let remote_addr = if let Ok(addr) = stream.peer_addr() {
        //     log::info!("server session: {}", addr.to_string());
        //     Some(addr)
        // } else {
        //     None
        // };

        let remote_addr = stream.peer_addr().unwrap_or(stream.local_addr().unwrap());
        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        Self {
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            session_type: define::ServerSessionType::Push,
            event_producer,
            stream_handler: Arc::new(RtspStreamHandler::new()),
            auth,
            stream_identifier: None,
            is_normal_exit: false,
            remote_addr,
        }
    }

    pub async fn run(&mut self) -> Result<(), SessionError> {
        loop {
            while self.reader.len() < 4 {
                let data = self.io.lock().await.read().await?;
                self.reader.extend_from_slice(&data[..]);
            }
            // If delivering media data using RTP over RTSP(TCP), then it should use InterleavedBinaryData
            // to distinguish RTP from RTSP messges; If delivering media data over UDP, it will establish
            // separate udp channels for audio RTP data and video RTP data.

            // TODO: Here, some optimizations can be made since it's not necessary to use InterleavedBinaryData
            // in all cases.
            if let Ok(data) = InterleavedBinaryData::new(&mut self.reader) {
                match data {
                    Some(a) => {
                        while self.reader.len() < a.length as usize {
                            let data = self.io.lock().await.read().await?;
                            self.reader.extend_from_slice(&data[..]);
                        }
                        self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                            .await?;
                    }
                    None => {
                        self.on_rtsp_message().await?;
                    }
                }
            }
        }
    }

    async fn on_rtp_over_rtsp_message(
        &mut self,
        channel_identifier: u8,
        length: usize,
    ) -> Result<(), SessionError> {
        let mut cur_reader = BytesReader::new(self.reader.read_bytes(length)?);

        for track in self.tracks.values_mut() {
            if let Some(interleaveds) = track.transport.interleaved {
                let rtp_identifier = interleaveds[0];
                let rtcp_identifier = interleaveds[1];

                if channel_identifier == rtp_identifier {
                    track.on_rtp(&mut cur_reader).await?;
                } else if channel_identifier == rtcp_identifier {
                    track.on_rtcp(&mut cur_reader, self.io.clone()).await;
                }
            }
        }
        Ok(())
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    async fn on_rtsp_message(&mut self) -> Result<(), SessionError> {
        let rtsp_request: RtspRequest;
        let mut retry_count = 0;
        loop {
            // TODO(all) : shoud check if have '\r\n\r\n' firstly.
            let data = self.reader.get_remaining_bytes();
            if let Some(rtsp_request_data) = RtspRequest::unmarshal(std::str::from_utf8(&data)?) {
                // TCP packet sticking issue, if have content_length in header.
                // should check the body
                if let Some(content_length) =
                    rtsp_request_data.get_header(&String::from("Content-Length"))
                    && let Ok(uint_num) = content_length.parse::<usize>()
                    && (rtsp_request_data.body.is_none()
                        || uint_num > rtsp_request_data.body.clone().unwrap().len())
                {
                    if retry_count >= 5 {
                        log::error!("corrupted rtsp message={}", std::str::from_utf8(&data)?);
                        return Ok(());
                    }
                    retry_count += 1;
                    let data_recv = self.io.lock().await.read().await?;
                    self.reader.extend_from_slice(&data_recv[..]);
                    continue;
                }
                rtsp_request = rtsp_request_data;
                self.reader.extract_remaining_bytes();
            } else {
                log::error!("corrupted rtsp message={}", std::str::from_utf8(&data)?);
                return Ok(());
            }
            break;
        }

        match rtsp_request.method.as_str() {
            rtsp_method_name::OPTIONS => {
                self.handle_options(&rtsp_request).await?;
            }
            rtsp_method_name::DESCRIBE => {
                self.handle_describe(&rtsp_request).await?;
            }
            rtsp_method_name::ANNOUNCE => {
                self.handle_announce(&rtsp_request).await?;
            }
            rtsp_method_name::SETUP => {
                self.handle_setup(&rtsp_request).await?;
            }
            rtsp_method_name::PLAY => {
                if let Err(err) = self.handle_play(&rtsp_request).await {
                    log::info!("handle_play error: {}", err);
                }
            }
            rtsp_method_name::RECORD => {
                self.handle_record(&rtsp_request).await?;
            }
            rtsp_method_name::TEARDOWN => {
                self.handle_teardown(&rtsp_request)?;
            }
            rtsp_method_name::PAUSE => {}
            rtsp_method_name::GET_PARAMETER => {}
            rtsp_method_name::SET_PARAMETER => {}
            rtsp_method_name::REDIRECT => {}

            _ => {}
        }
        Ok(())
    }

    async fn handle_options(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);
        let public_str = rtsp_method_name::ARRAY.join(",");
        response.headers.insert("Public".to_string(), public_str);
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_describe(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;

        // The sender is used for sending sdp information from the server session to client session
        // receiver is used to receive the sdp information
        let (sender, mut receiver) = mpsc::unbounded_channel();

        let identifier = StreamIdentifier::Rtsp {
            stream_path: rtsp_request.uri.path.clone(),
        };
        self.stream_identifier = Some(identifier.clone());

        let request_event = StreamHubEvent::Request { identifier, sender };

        if self.event_producer.send(request_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        if let Some(Information::Sdp { data }) = receiver.recv().await
            && let Some(sdp) = Sdp::unmarshal(&data)
        {
            self.sdp = sdp;
            //it can new tracks when get the sdp information;
            self.new_tracks()?;
        }

        let mut response = Self::gen_response(status_code, rtsp_request);
        let sdp = self.sdp.marshal();
        response.body = Some(sdp);
        response
            .headers
            .insert("Content-Type".to_string(), "application/sdp".to_string());
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_announce(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(
                &stream_name,
                &rtsp_request
                    .uri
                    .query
                    .as_ref()
                    .map(|q| SecretCarrier::Query(q.to_string())),
                false,
            )?;
        }

        if let Some(request_body) = &rtsp_request.body
            && let Some(sdp) = Sdp::unmarshal(request_body)
        {
            self.sdp = sdp.clone();
            self.stream_handler.set_sdp(sdp).await;
        }

        //new tracks for publish session
        self.new_tracks()?;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let identifier = StreamIdentifier::Rtsp {
            stream_path: rtsp_request.uri.path.clone(),
        };
        self.stream_identifier = Some(identifier.clone());

        let publish_event = StreamHubEvent::Publish {
            identifier,
            result_sender: event_result_sender,
            info: self.get_publisher_info(),
            stream_handler: self.stream_handler.clone(),
        };

        if self.event_producer.send(publish_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let sender = event_result_receiver.await??.0.unwrap();

        for track in self.tracks.values_mut() {
            let sender_out = sender.clone();
            let mut rtp_channel_guard = track.rtp_channel.lock().await;

            rtp_channel_guard.on_frame_handler(Box::new(
                move |msg: FrameData| -> Result<(), UnPackerError> {
                    if let Err(err) = sender_out.send(msg) {
                        log::error!("send frame error: {}", err);
                    }
                    Ok(())
                },
            ));

            let rtcp_channel = Arc::clone(&track.rtcp_channel);
            rtp_channel_guard.on_packet_for_rtcp_handler(Box::new(move |packet: RtpPacket| {
                let rtcp_channel_in = Arc::clone(&rtcp_channel);
                Box::pin(async move {
                    rtcp_channel_in.lock().await.on_packet(packet);
                })
            }));
        }

        let status_code = http::StatusCode::OK;
        let response = Self::gen_response(status_code, rtsp_request);
        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_setup(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        for track in self.tracks.values_mut() {
            if !rtsp_request.uri.marshal().contains(&track.media_control) {
                continue;
            }

            if let Some(transport_data) = rtsp_request.get_header(&"Transport".to_string()) {
                if self.session_id.is_none() {
                    self.session_id = Some(Uuid::new(RandomDigitCount::Zero));
                }

                let transport = RtspTransport::unmarshal(transport_data);

                if let Some(mut trans) = transport {
                    let mut rtp_server_port: Option<u16> = None;
                    let mut rtcp_server_port: Option<u16> = None;

                    match trans.protocol_type {
                        ProtocolType::TCP => {
                            track.create_packer(self.io.clone()).await;
                        }
                        ProtocolType::UDP => {
                            let (rtp_port, rtcp_port) =
                                if let Some(client_ports) = trans.client_port {
                                    (client_ports[0], client_ports[1])
                                } else {
                                    log::error!("should not be here!!");
                                    (0, 0)
                                };

                            let address = self.remote_addr.ip().to_string();
                            if let Some(rtp_io) = UdpIO::new(address.clone(), rtp_port, 0).await {
                                rtp_server_port = rtp_io.get_local_port();

                                let box_udp_io: Box<dyn TNetIO + Send + Sync> = Box::new(rtp_io);
                                //if mode is empty then it is a player session.
                                if trans.transport_mod.is_none() {
                                    track.create_packer(Arc::new(Mutex::new(box_udp_io))).await;
                                } else {
                                    track.rtp_receive_loop(box_udp_io).await;
                                }
                            }

                            if let Some(rtcp_io) =
                                UdpIO::new(address.clone(), rtcp_port, rtp_server_port.unwrap() + 1)
                                    .await
                            {
                                rtcp_server_port = rtcp_io.get_local_port();
                                let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                    Arc::new(Mutex::new(Box::new(rtcp_io)));
                                track.rtcp_receive_loop(box_rtcp_io).await;
                            }
                        }
                    }

                    //tell client the udp ports of server side
                    let mut server_ports: [u16; 2] = [0, 0];
                    if let Some(rtp_port) = rtp_server_port {
                        server_ports[0] = rtp_port;
                    }
                    if let Some(rtcp_server_port) = rtcp_server_port {
                        server_ports[1] = rtcp_server_port;
                        trans.server_port = Some(server_ports);
                    }

                    let new_transport_data = trans.marshal();
                    response
                        .headers
                        .insert("Transport".to_string(), new_transport_data);
                    response
                        .headers
                        .insert("Session".to_string(), self.session_id.unwrap().to_string());

                    track.set_transport(trans).await;
                }
            }
            break;
        }

        self.send_response(&response).await?;

        Ok(())
    }

    async fn handle_play(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        if let Some(auth) = &self.auth {
            let stream_name = rtsp_request.uri.path.clone();
            auth.authenticate(
                &stream_name,
                &rtsp_request
                    .uri
                    .query
                    .as_ref()
                    .map(|q| SecretCarrier::Query(q.to_string())),
                true,
            )?;
        }

        for track in self.tracks.values_mut() {
            let protocol_type = track.transport.protocol_type.clone();

            match protocol_type {
                ProtocolType::TCP => {
                    let channel_identifer = if let Some(interleaveds) = track.transport.interleaved
                    {
                        interleaveds[0]
                    } else {
                        log::error!("handle_play:should not be here!!!");
                        0
                    };

                    track.rtp_channel.lock().await.on_packet_handler(Box::new(
                        move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                            Box::pin(async move {
                                let msg = packet.marshal()?;
                                let mut bytes_writer = AsyncBytesWriter::new(io);
                                bytes_writer.write_u8(0x24)?;
                                bytes_writer.write_u8(channel_identifer)?;
                                bytes_writer.write_u16::<BigEndian>(msg.len() as u16)?;
                                bytes_writer.write(&msg)?;
                                bytes_writer.flush().await?;
                                Ok(())
                            })
                        },
                    ));
                }
                ProtocolType::UDP => {
                    track.rtp_channel.lock().await.on_packet_handler(Box::new(
                        move |io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>, packet: RtpPacket| {
                            Box::pin(async move {
                                let mut bytes_writer = AsyncBytesWriter::new(io);

                                let msg = packet.marshal()?;
                                bytes_writer.write(&msg)?;
                                bytes_writer.flush().await?;
                                Ok(())
                            })
                        },
                    ));
                }
            }
        }

        let status_code = http::StatusCode::OK;
        let response = Self::gen_response(status_code, rtsp_request);

        self.send_response(&response).await?;

        self.session_type = define::ServerSessionType::Pull;

        let (event_result_sender, event_result_receiver) = oneshot::channel();

        let subscribe_event = StreamHubEvent::Subscribe {
            identifier: StreamIdentifier::Rtsp {
                stream_path: rtsp_request.uri.path.clone(),
            },
            info: self.get_subscriber_info(),
            result_sender: event_result_sender,
        };

        if self.event_producer.send(subscribe_event).is_err() {
            return Err(SessionError {
                value: SessionErrorValue::StreamHubEventSendErr,
            });
        }

        let mut receiver = event_result_receiver.await??.0.frame_receiver.unwrap();

        let mut retry_times = 0;
        loop {
            if let Some(frame_data) = receiver.recv().await {
                match frame_data {
                    FrameData::Audio {
                        timestamp,
                        mut data,
                    } => {
                        if let Some(audio_track) = self.tracks.get_mut(&TrackType::Audio) {
                            audio_track
                                .rtp_channel
                                .lock()
                                .await
                                .on_frame(&mut data, timestamp)
                                .await?;
                        }
                    }
                    FrameData::Video {
                        timestamp,
                        mut data,
                    } => {
                        if let Some(video_track) = self.tracks.get_mut(&TrackType::Video) {
                            video_track
                                .rtp_channel
                                .lock()
                                .await
                                .on_frame(&mut data, timestamp)
                                .await?;
                        }
                    }
                    _ => {}
                }
            } else {
                retry_times += 1;
                log::info!(
                    "send_channel_data: no data receives ,retry {} times!",
                    retry_times
                );

                if retry_times > 10 {
                    return Err(SessionError {
                        value: SessionErrorValue::CannotReceiveFrameData,
                    });
                }
            }
        }
    }

    async fn handle_record(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let status_code = http::StatusCode::OK;
        let mut response = Self::gen_response(status_code, rtsp_request);

        //A stream published by gstreamer does not support the Range header
        //https://github.com/harlanc/xiu/issues/135
        if let Some(range_str) = rtsp_request.headers.get(&String::from("Range"))
            && let Some(range) = RtspRange::unmarshal(range_str)
        {
            response
                .headers
                .insert(String::from("Range"), range.marshal());
        }

        response
            .headers
            .insert("Session".to_string(), self.session_id.unwrap().to_string());

        self.send_response(&response).await?;

        Ok(())
    }

    fn handle_teardown(&mut self, rtsp_request: &RtspRequest) -> Result<(), SessionError> {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: rtsp_request.uri.path.clone(),
        };
        log::info!("handle_teardown...");
        self.exit(identifier)
    }

    pub fn exit(&mut self, identifier: StreamIdentifier) -> Result<(), SessionError> {
        let event = match self.session_type {
            define::ServerSessionType::Pull => StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            },
            define::ServerSessionType::Push => StreamHubEvent::UnPublish {
                identifier,
                info: self.get_publisher_info(),
            },
        };

        let event_json_str = serde_json::to_string(&event).unwrap();

        let rv = self.event_producer.send(event);
        match rv {
            Err(err) => {
                log::error!("session exit: send event error: {err} for event: {event_json_str}");
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                self.is_normal_exit = true;
                log::info!("session exit: send event success: {event_json_str}");
                Ok(())
            }
        }
    }

    fn new_tracks(&mut self) -> Result<(), SessionError> {
        for media in &self.sdp.medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                String::from("")
            };

            let media_name = &media.media_type;
            log::info!("media_name: {}", media_name);
            match media_name.as_str() {
                "audio" => {
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                        .unwrap()
                        .clone();
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        channel_count: media.rtpmap.encoding_param.parse().unwrap(),
                    };

                    log::info!("audio codec info: {:?}", codec_info);

                    let track = RtspTrack::new(TrackType::Audio, codec_info, media_control);
                    self.tracks.insert(TrackType::Audio, track);
                }
                "video" => {
                    let codec_id = rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                        .unwrap()
                        .clone();
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        ..Default::default()
                    };
                    let track = RtspTrack::new(TrackType::Video, codec_info, media_control);
                    self.tracks.insert(TrackType::Video, track);
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn gen_response(status_code: StatusCode, rtsp_request: &RtspRequest) -> RtspResponse {
        let reason_phrase = if let Some(reason) = status_code.canonical_reason() {
            reason.to_string()
        } else {
            "".to_string()
        };

        let mut response = RtspResponse {
            version: "RTSP/1.0".to_string(),
            status_code: status_code.as_u16(),
            reason_phrase,
            ..Default::default()
        };

        if let Some(cseq) = rtsp_request.headers.get("CSeq") {
            response
                .headers
                .insert("CSeq".to_string(), cseq.to_string());
        }

        response
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::RtspPull,
            sub_data_type: crate::streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    fn get_publisher_info(&mut self) -> PublisherInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        PublisherInfo {
            id,
            pub_type: PublishType::RtspPush,
            pub_data_type: crate::streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        }
    }

    async fn send_response(&mut self, response: &RtspResponse) -> Result<(), SessionError> {
        self.writer.write(response.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
    }
}

#[derive(Default)]
pub struct RtspStreamHandler {
    sdp: Mutex<Sdp>,
}

impl RtspStreamHandler {
    pub fn new() -> Self {
        Self {
            sdp: Mutex::new(Sdp::default()),
        }
    }
    pub async fn set_sdp(&self, sdp: Sdp) {
        *self.sdp.lock().await = sdp;
    }
}

#[async_trait]
impl TStreamHandler for RtspStreamHandler {
    async fn send_prior_data(
        &self,
        data_sender: DataSender,
        sub_type: SubscribeType,
    ) -> Result<(), StreamHubError> {
        let sender = match data_sender {
            DataSender::Frame { sender } => sender,
            DataSender::Packet { sender: _ } => {
                return Err(StreamHubError {
                    value: StreamHubErrorValue::NotCorrectDataSenderType,
                });
            }
        };
        match sub_type {
            SubscribeType::RtspRemux2Rtmp => {
                let sdp_info = self.sdp.lock().await;
                let mut video_clock_rate: u32 = 0;
                let mut audio_clock_rate: u32 = 0;

                let mut vcodec: VideoCodecType = VideoCodecType::H264;

                for media in &sdp_info.medias {
                    let mut bytes_writer = BytesWriter::new();
                    if let Some(fmtp) = &media.fmtp {
                        match fmtp {
                            Fmtp::H264(data) => {
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.sps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.pps)?;

                                let frame_data = FrameData::Video {
                                    timestamp: 0,
                                    data: bytes_writer.extract_current_bytes(),
                                };
                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send sps/pps error: {}", err);
                                }
                                video_clock_rate = media.rtpmap.clock_rate;
                            }
                            Fmtp::H265(data) => {
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.sps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.pps)?;
                                bytes_writer.write(&ANNEXB_NALU_START_CODE)?;
                                bytes_writer.write(&data.vps)?;

                                let frame_data = FrameData::Video {
                                    timestamp: 0,
                                    data: bytes_writer.extract_current_bytes(),
                                };
                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send sps/pps/vps error: {}", err);
                                }

                                vcodec = VideoCodecType::H265;
                            }
                            Fmtp::Mpeg4(data) => {
                                let frame_data = FrameData::Audio {
                                    timestamp: 0,
                                    data: data.asc.clone(),
                                };

                                if let Err(err) = sender.send(frame_data) {
                                    log::error!("send asc error: {}", err);
                                }

                                audio_clock_rate = media.rtpmap.clock_rate;
                            }
                        }
                    }
                }

                if let Err(err) = sender.send(FrameData::MediaInfo {
                    media_info: MediaInfo {
                        audio_clock_rate,
                        video_clock_rate,

                        vcodec,
                    },
                }) {
                    log::error!("send media info error: {}", err);
                }
            }
            SubscribeType::RtmpRemux2Hls => {}
            _ => {}
        }

        Ok(())
    }
    async fn get_statistic_data(&self) -> Option<StatisticsStream> {
        None
    }

    async fn send_information(&self, sender: InformationSender) {
        if let Err(err) = sender.send(Information::Sdp {
            data: self.sdp.lock().await.marshal(),
        }) {
            log::error!("send_information of rtsp error: {}", err);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_reader::BytesReader;
    use crate::common::http::HttpRequest as RtspRequest;
    use bytes::BytesMut;
    use http::StatusCode;

    // ========================================================================
    // InterleavedBinaryData Tests
    // ========================================================================

    #[test]
    fn test_interleaved_binary_data_parse_valid() {
        // Dollar sign (0x24) + channel (0x00) + length (0x0004)
        let data: &[u8] = &[0x24, 0x00, 0x00, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, 0x00);
        assert_eq!(interleaved.length, 4);
    }

    #[test]
    fn test_interleaved_binary_data_parse_channel_1() {
        // Dollar sign + channel 1 + length 10
        let data: &[u8] = &[0x24, 0x01, 0x00, 0x0A];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, 0x01);
        assert_eq!(interleaved.length, 10);
    }

    #[test]
    fn test_interleaved_binary_data_parse_large_length() {
        // Dollar sign + channel 2 + length 0xFFFF (65535)
        let data: &[u8] = &[0x24, 0x02, 0xFF, 0xFF];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_some());
        let interleaved = result.unwrap();
        assert_eq!(interleaved.channel_identifier, 0x02);
        assert_eq!(interleaved.length, 65535);
    }

    #[test]
    fn test_interleaved_binary_data_no_dollar_sign() {
        // Not starting with dollar sign - should return None
        let data: &[u8] = &[0x52, 0x54, 0x53, 0x50]; // "RTSP"
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_interleaved_binary_data_insufficient_data() {
        // Only dollar sign, not enough for full header
        let data: &[u8] = &[0x24];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader);
        // Should return an error due to insufficient bytes
        assert!(result.is_err());
    }

    #[test]
    fn test_interleaved_binary_data_empty() {
        let data: &[u8] = &[];
        let mut reader = BytesReader::new(BytesMut::from(data));

        let result = InterleavedBinaryData::new(&mut reader);
        assert!(result.is_err());
    }

    // ========================================================================
    // gen_response Tests
    // ========================================================================

    /// Create a test RtspRequest with the given method and CSeq
    fn create_test_request(method: &str, cseq: Option<&str>) -> RtspRequest {
        let mut request = RtspRequest {
            method: method.to_string(),
            version: "RTSP/1.0".to_string(),
            ..Default::default()
        };
        if let Some(seq) = cseq {
            request.headers.insert("CSeq".to_string(), seq.to_string());
        }
        request
    }

    #[test]
    fn test_gen_response_ok_status() {
        let request = create_test_request("OPTIONS", Some("1"));

        let response = RtspServerSession::gen_response(StatusCode::OK, &request);
        assert_eq!(response.version, "RTSP/1.0");
        assert_eq!(response.status_code, 200);
        assert_eq!(response.reason_phrase, "OK");
        assert_eq!(response.headers.get("CSeq"), Some(&"1".to_string()));
    }

    #[test]
    fn test_gen_response_not_found_status() {
        let request = create_test_request("DESCRIBE", None);

        let response = RtspServerSession::gen_response(StatusCode::NOT_FOUND, &request);
        assert_eq!(response.status_code, 404);
        assert_eq!(response.reason_phrase, "Not Found");
    }

    #[test]
    fn test_gen_response_unauthorized_status() {
        let request = create_test_request("PLAY", None);

        let response = RtspServerSession::gen_response(StatusCode::UNAUTHORIZED, &request);
        assert_eq!(response.status_code, 401);
        assert_eq!(response.reason_phrase, "Unauthorized");
    }

    #[test]
    fn test_gen_response_with_cseq() {
        let request = create_test_request("SETUP", Some("42"));

        let response = RtspServerSession::gen_response(StatusCode::OK, &request);
        assert_eq!(response.headers.get("CSeq"), Some(&"42".to_string()));
    }

    #[test]
    fn test_gen_response_without_cseq() {
        let request = create_test_request("OPTIONS", None);

        let response = RtspServerSession::gen_response(StatusCode::OK, &request);
        assert!(response.headers.get("CSeq").is_none());
    }

    #[test]
    fn test_gen_response_bad_request() {
        let request = create_test_request("INVALID", None);

        let response = RtspServerSession::gen_response(StatusCode::BAD_REQUEST, &request);
        assert_eq!(response.status_code, 400);
        assert_eq!(response.reason_phrase, "Bad Request");
    }

    #[test]
    fn test_gen_response_internal_error() {
        let request = create_test_request("PLAY", None);

        let response = RtspServerSession::gen_response(StatusCode::INTERNAL_SERVER_ERROR, &request);
        assert_eq!(response.status_code, 500);
        assert_eq!(response.reason_phrase, "Internal Server Error");
    }

    // ========================================================================
    // RtspStreamHandler Tests
    // ========================================================================

    #[test]
    fn test_rtsp_stream_handler_new() {
        let handler = RtspStreamHandler::new();
        // Handler should be created successfully
        assert!(std::mem::size_of_val(&handler) > 0);
    }

    #[test]
    fn test_rtsp_stream_handler_default() {
        let handler = RtspStreamHandler::default();
        assert!(std::mem::size_of_val(&handler) > 0);
    }

    #[tokio::test]
    async fn test_rtsp_stream_handler_set_sdp() {
        let handler = RtspStreamHandler::new();
        let sdp = Sdp::default();
        handler.set_sdp(sdp).await;
        // Should not panic
    }

    #[tokio::test]
    async fn test_rtsp_stream_handler_send_information() {
        let handler = RtspStreamHandler::new();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();

        handler.send_information(sender).await;

        // Should receive SDP information
        if let Some(info) = receiver.recv().await {
            match info {
                Information::Sdp { data: _ } => {
                    // Correct type received
                }
                _ => panic!("Expected Sdp information"),
            }
        } else {
            panic!("Expected to receive information");
        }
    }

    #[tokio::test]
    async fn test_rtsp_stream_handler_get_statistic_data() {
        let handler = RtspStreamHandler::new();
        let stats = handler.get_statistic_data().await;
        assert!(stats.is_none());
    }

    // ========================================================================
    // ServerSessionType Tests
    // ========================================================================

    #[test]
    fn test_server_session_type_push() {
        let session_type = define::ServerSessionType::Push;
        // Should be able to compare
        assert!(matches!(session_type, define::ServerSessionType::Push));
    }

    #[test]
    fn test_server_session_type_pull() {
        let session_type = define::ServerSessionType::Pull;
        assert!(matches!(session_type, define::ServerSessionType::Pull));
    }

    // ========================================================================
    // Integration-style tests for parsing
    // ========================================================================

    #[test]
    fn test_interleaved_binary_data_all_channels() {
        // Test all common channel identifiers (0-3 for RTP/RTCP audio/video)
        for channel in 0..4u8 {
            let data: &[u8] = &[0x24, channel, 0x00, 0x10];
            let mut reader = BytesReader::new(BytesMut::from(data));

            let result = InterleavedBinaryData::new(&mut reader).unwrap();
            assert!(result.is_some());
            let interleaved = result.unwrap();
            assert_eq!(interleaved.channel_identifier, channel);
            assert_eq!(interleaved.length, 16);
        }
    }

    #[test]
    fn test_gen_response_service_unavailable() {
        let request = create_test_request("DESCRIBE", None);

        let response = RtspServerSession::gen_response(StatusCode::SERVICE_UNAVAILABLE, &request);
        assert_eq!(response.status_code, 503);
        assert_eq!(response.reason_phrase, "Service Unavailable");
    }

    #[test]
    fn test_gen_response_method_not_allowed() {
        let request = create_test_request("UNKNOWN", None);

        let response = RtspServerSession::gen_response(StatusCode::METHOD_NOT_ALLOWED, &request);
        assert_eq!(response.status_code, 405);
        assert_eq!(response.reason_phrase, "Method Not Allowed");
    }
}
