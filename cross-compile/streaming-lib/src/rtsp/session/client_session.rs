use super::define;
use super::define::ClientSessionType;

use crate::rtsp::global_trait::Marshal;
use crate::rtsp::global_trait::Unmarshal;
use crate::rtsp::rtsp_codec;

use crate::rtsp::rtsp_transport::CastType;

use super::server_session::InterleavedBinaryData;
use crate::common::http::HttpRequest as RtspRequest;
use crate::common::http::HttpResponse as RtspResponse;
use crate::common::http::Marshal as RtspMarshal;
use crate::common::http::Unmarshal as RtspUnmarshal;
use crate::common::http::Uri;
use crate::common::http::try_get_complete_message_len;
use crate::streamhub::define::SubscriberInfo;

use crate::rtsp::rtp::RtpPacket;

use crate::rtsp::rtsp_codec::RtspCodecInfo;
use crate::rtsp::rtsp_track::RtspTrack;
use crate::rtsp::rtsp_track::TrackType;
use crate::rtsp::rtsp_transport::ProtocolType;
use crate::rtsp::rtsp_transport::RtspTransport;

use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::AsyncBytesWriter;
use bytes::BytesMut;

use super::errors::SessionError;
use super::errors::SessionErrorValue;

use tokio::sync::oneshot;

use crate::rtsp::rtp::errors::UnPackerError;
use crate::rtsp::sdp::Sdp;

use super::define::rtsp_method_name;

use crate::bytesio::TNetIO;
use crate::bytesio::TcpIO;

use std::collections::HashMap;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::define::USER_AGENT;

use crate::streamhub::{
    define::{
        FrameData, NotifyInfo, PublishType, PublisherInfo, StreamHubEvent, StreamHubEventSender,
        SubscribeType,
    },
    stream::StreamIdentifier,
    utils::{RandomDigitCount, Uuid},
};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use super::server_session::RtspStreamHandler;

use crate::bytesio::new_udpio_pair;

pub struct RtspClientSession {
    address: String,
    stream_name: String,

    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    reader: BytesReader,
    writer: AsyncBytesWriter,

    protocol_type: ProtocolType,
    tracks: HashMap<TrackType, RtspTrack>,
    sdp: Sdp,
    pub session_id: Option<Uuid>,
    pub client_type: super::define::ClientSessionType,
    cseq: u16,
    stream_handler: Arc<RtspStreamHandler>,

    event_producer: StreamHubEventSender,
    pub is_running: Arc<AtomicBool>,
}

impl RtspClientSession {
    pub async fn new(
        address: String,
        stream_name: String,
        protocol_type: ProtocolType,
        event_producer: StreamHubEventSender,
        client_type: ClientSessionType,
    ) -> Result<Self, SessionError> {
        let stream = TcpStream::connect(address.clone()).await?;

        let net_io: Box<dyn TNetIO + Send + Sync> = Box::new(TcpIO::new(stream));
        let io = Arc::new(Mutex::new(net_io));

        Ok(Self {
            address,
            stream_name,
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            protocol_type,
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            client_type,
            event_producer,

            cseq: 1,

            stream_handler: Arc::new(RtspStreamHandler::new()),
            is_running: Arc::new(AtomicBool::new(true)),
        })
    }

    pub fn new_with_io(
        address: String,
        stream_name: String,
        protocol_type: ProtocolType,
        event_producer: StreamHubEventSender,
        client_type: ClientSessionType,
        net_io: Box<dyn TNetIO + Send + Sync>,
    ) -> Self {
        let io = Arc::new(Mutex::new(net_io));

        Self {
            address,
            stream_name,
            io: io.clone(),
            reader: BytesReader::new(BytesMut::default()),
            writer: AsyncBytesWriter::new(io),
            protocol_type,
            tracks: HashMap::new(),
            sdp: Sdp::default(),
            session_id: None,
            client_type,
            event_producer,

            cseq: 1,

            stream_handler: Arc::new(RtspStreamHandler::new()),
            is_running: Arc::new(AtomicBool::new(true)),
        }
    }

    //publish stream: OPTIONS->ANNOUNCE->SETUP->RECORD->TEARDOWN
    //subscribe stream: OPTIONS->DESCRIBE->SETUP->PLAY->TEARDOWN
    pub async fn run(&mut self) -> Result<(), SessionError> {
        self.send_options().await?;

        match self.client_type {
            ClientSessionType::Pull => {
                self.send_describe().await?;
                self.send_setup().await?;
                self.send_play().await?;
            }
            ClientSessionType::Push => {
                self.send_announce().await?;
                self.send_setup().await?;
                self.send_record().await?;
            }
        }

        while self.is_running.load(std::sync::atomic::Ordering::Acquire) {
            while self.reader.len() < 4 {
                let data = self.io.lock().await.read().await?;
                self.reader.extend_from_slice(&data[..]);
            }

            if let Ok(Some(a)) = InterleavedBinaryData::new(&mut self.reader) {
                while self.reader.len() < a.length as usize {
                    let data = self.io.lock().await.read().await?;
                    self.reader.extend_from_slice(&data[..]);
                }
                self.on_rtp_over_rtsp_message(a.channel_identifier, a.length as usize)
                    .await?;
            }
        }

        self.send_teardown().await?;

        Ok(())
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
    async fn send_options(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_options");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let request = self.gen_request(rtsp_method_name::OPTIONS, uri_path)?;
        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::OPTIONS).await
    }

    async fn send_announce(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_announce");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let request = self.gen_request(rtsp_method_name::ANNOUNCE, uri_path)?;
        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::ANNOUNCE).await
    }

    async fn send_describe(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_describe");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let mut request = self.gen_request(rtsp_method_name::DESCRIBE, uri_path)?;
        request
            .headers
            .insert("Accept".to_string(), "application/sdp".to_string());
        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::DESCRIBE).await
    }

    async fn send_setup(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_setup");
        let sdp_medias = self.sdp.medias.clone();

        for media in sdp_medias {
            let media_control = if let Some(media_control_val) = media.attributes.get("control") {
                media_control_val.clone()
            } else {
                log::error!("cannot get media control!!");
                String::from("")
            };

            let uri_path = format!(
                "rtsp://{}/{}/{}",
                self.address, self.stream_name, media_control
            );

            let mut request = self.gen_request(rtsp_method_name::SETUP, uri_path)?;

            match self.protocol_type {
                ProtocolType::TCP => {
                    let kv: Vec<&str> = media_control.trim().splitn(2, '=').collect();
                    if kv.len() < 2 || kv[1].trim().is_empty() {
                        log::error!("cannot parse control attribute: {}", media_control);
                        continue;
                    }
                    let mut media_transport = RtspTransport::default();
                    if let Ok(interleaved_idx) = kv[1].parse::<u8>() {
                        media_transport.interleaved =
                            Some([interleaved_idx * 2, interleaved_idx * 2 + 1]);
                    } else {
                        log::error!("cannot get interleaved_idx: {}", kv[1]);
                    }

                    media_transport.protocol_type = ProtocolType::TCP;
                    media_transport.cast_type = CastType::Unicast;
                    request
                        .headers
                        .insert("Transport".to_string(), media_transport.marshal());

                    if media.media_type == "audio" {
                        if let Some(track) = self.tracks.get_mut(&TrackType::Audio) {
                            track.transport.interleaved = media_transport.interleaved;
                        }
                    } else if media.media_type == "video"
                        && let Some(track) = self.tracks.get_mut(&TrackType::Video)
                    {
                        track.transport.interleaved = media_transport.interleaved;
                    }
                }
                ProtocolType::UDP => {
                    if let Some((socket_rtp, socket_rtcp)) = new_udpio_pair().await {
                        let rtp_port = socket_rtp.get_local_port().ok_or(SessionError {
                            value: SessionErrorValue::MissingClientPort,
                        })?;
                        let rtcp_port = socket_rtcp.get_local_port().ok_or(SessionError {
                            value: SessionErrorValue::MissingClientPort,
                        })?;
                        let media_transport = RtspTransport {
                            protocol_type: ProtocolType::UDP,
                            cast_type: CastType::Unicast,
                            client_port: Some([rtp_port, rtcp_port]),
                            ..Default::default()
                        };

                        request
                            .headers
                            .insert("Transport".to_string(), media_transport.marshal());

                        if media.media_type == "audio" {
                            if let Some(track) = self.tracks.get_mut(&TrackType::Audio) {
                                let box_rtp_io: Box<dyn TNetIO + Send + Sync> =
                                    Box::new(socket_rtp);
                                track.rtp_receive_loop(box_rtp_io).await;

                                let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                    Arc::new(Mutex::new(Box::new(socket_rtcp)));
                                track.rtcp_receive_loop(box_rtcp_io).await;
                            }
                        } else if media.media_type == "video"
                            && let Some(track) = self.tracks.get_mut(&TrackType::Video)
                        {
                            let box_rtp_io: Box<dyn TNetIO + Send + Sync> = Box::new(socket_rtp);
                            track.rtp_receive_loop(box_rtp_io).await;

                            let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                                Arc::new(Mutex::new(Box::new(socket_rtcp)));
                            track.rtcp_receive_loop(box_rtcp_io).await;
                        }
                    }
                }
            }

            self.send_resquest(&request).await?;
            self.receive_response(rtsp_method_name::SETUP).await?;
        }
        Ok(())
    }

    async fn send_play(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_play");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let mut request = self.gen_request(rtsp_method_name::PLAY, uri_path)?;
        request
            .headers
            .insert("Range".to_string(), "npt=0.000".to_string());

        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::PLAY).await?;

        Ok(())
    }

    async fn send_record(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_record");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let mut request = self.gen_request(rtsp_method_name::RECORD, uri_path)?;
        request
            .headers
            .insert("Transport".to_string(), "application/sdp".to_string());
        self.send_resquest(&request).await?;
        self.receive_response(rtsp_method_name::RECORD).await
    }

    async fn send_teardown(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_teardown");
        let uri_path = format!("rtsp://{}/{}", self.address, self.stream_name);
        let request = self.gen_request(rtsp_method_name::TEARDOWN, uri_path)?;
        self.send_resquest(&request).await?;
        self.exit()
    }

    fn gen_request(
        &mut self,
        method_name: &str,
        uri_path: String,
    ) -> Result<RtspRequest, SessionError> {
        let uri = Uri::unmarshal(&uri_path).ok_or(SessionError {
            value: SessionErrorValue::RtspMessageCorrupted("invalid rtsp uri".to_string()),
        })?;

        let mut request = RtspRequest {
            method: method_name.to_string(),
            uri,
            version: "RTSP/1.0".to_string(),
            ..Default::default()
        };

        request
            .headers
            .insert("CSeq".to_string(), self.cseq.to_string());
        self.cseq += 1;
        request
            .headers
            .insert("User-Agent".to_string(), USER_AGENT.to_string());

        if let Some(session_id) = self.session_id {
            request
                .headers
                .insert("Session".to_string(), session_id.to_string());
        }

        Ok(request)
    }

    fn get_subscriber_info(&mut self) -> SubscriberInfo {
        let id = if let Some(session_id) = &self.session_id {
            *session_id
        } else {
            Uuid::new(RandomDigitCount::Zero)
        };

        SubscriberInfo {
            id,
            sub_type: SubscribeType::RtspRelay,
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
            pub_type: PublishType::RtspRelay,
            pub_data_type: crate::streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
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
            match media_name.as_str() {
                "audio" => {
                    let codec_id = match rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                    {
                        Some(codec_id) => codec_id.clone(),
                        None => {
                            log::error!("unsupported audio codec: {}", media.rtpmap.encoding_name);
                            continue;
                        }
                    };
                    let channel_count = match media.rtpmap.encoding_param.parse::<u8>() {
                        Ok(val) => val,
                        Err(err) => {
                            log::error!(
                                "invalid audio channel count '{}': {err}",
                                media.rtpmap.encoding_param
                            );
                            continue;
                        }
                    };
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        channel_count,
                    };

                    log::info!("audio codec info: {:?}", codec_info);

                    let track = RtspTrack::new(TrackType::Audio, codec_info, media_control);
                    self.tracks.insert(TrackType::Audio, track);
                }
                "video" => {
                    let codec_id = match rtsp_codec::RTSP_CODEC_NAME_2_ID
                        .get(&media.rtpmap.encoding_name.to_lowercase().as_str())
                    {
                        Some(codec_id) => codec_id.clone(),
                        None => {
                            log::error!("unsupported video codec: {}", media.rtpmap.encoding_name);
                            continue;
                        }
                    };
                    let codec_info = RtspCodecInfo {
                        codec_id,
                        payload_type: media.rtpmap.payload_type as u8,
                        sample_rate: media.rtpmap.clock_rate,
                        ..Default::default()
                    };
                    log::info!("video codec info: {:?}", codec_info);
                    let track = RtspTrack::new(TrackType::Video, codec_info, media_control);
                    self.tracks.insert(TrackType::Video, track);
                }
                _ => {}
            }
        }
        Ok(())
    }

    async fn send_resquest(&mut self, request: &RtspRequest) -> Result<(), SessionError> {
        self.writer.write(request.marshal().as_bytes())?;
        self.writer.flush().await?;

        Ok(())
    }

    async fn receive_response(&mut self, method_name: &str) -> Result<(), SessionError> {
        let mut retry_count = 0;
        let message_len = loop {
            let data = self.reader.get_remaining_bytes();
            match try_get_complete_message_len(&data) {
                Ok(Some(len)) => break len,
                Ok(None) => {
                    if retry_count >= 16 {
                        return Err(SessionError {
                            value: SessionErrorValue::RtspMessageCorrupted(
                                "max read retries exceeded".to_string(),
                            ),
                        });
                    }
                    retry_count += 1;
                    let data_recv = self.io.lock().await.read().await?;
                    self.reader.extend_from_slice(&data_recv[..]);
                }
                Err(err) => {
                    return Err(SessionError {
                        value: SessionErrorValue::RtspMessageCorrupted(err),
                    });
                }
            }
        };

        let message_bytes = self.reader.read_bytes(message_len)?;
        let message_str = std::str::from_utf8(&message_bytes)?;

        let Some(rtsp_response) = RtspResponse::unmarshal(message_str) else {
            return Err(SessionError {
                value: SessionErrorValue::RtspMessageCorrupted("response parse failed".to_string()),
            });
        };

        if rtsp_response.status_code != http::StatusCode::OK {
            log::error!("rtsp response error: {}", rtsp_response.marshal());
            return Err(SessionError {
                value: SessionErrorValue::RtspResponseStatusError,
            });
        }

        match method_name {
            rtsp_method_name::OPTIONS => {
                if let Some(public) = rtsp_response.get_header("Public") {
                    log::info!("support methods: {}", public);
                }
            }
            rtsp_method_name::ANNOUNCE => {}
            rtsp_method_name::DESCRIBE => {
                if let Some(request_body) = &rtsp_response.body
                    && let Ok(sdp) = Sdp::unmarshal(request_body)
                {
                    self.sdp = sdp.clone();
                    self.stream_handler.set_sdp(sdp).await;

                    self.new_tracks()?;

                    let (event_result_sender, event_result_receiver) = oneshot::channel();
                    let identifier = StreamIdentifier::Rtsp {
                        stream_path: self.stream_name.clone(),
                    };

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

                    let sender = event_result_receiver.await??.0.ok_or(SessionError {
                        value: SessionErrorValue::MissingFrameSender,
                    })?;

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
                        rtp_channel_guard.on_packet_for_rtcp_handler(Box::new(
                            move |packet: RtpPacket| {
                                let rtcp_channel_in = Arc::clone(&rtcp_channel);
                                Box::pin(async move {
                                    rtcp_channel_in.lock().await.on_packet(packet);
                                })
                            },
                        ));
                    }
                }
            }
            rtsp_method_name::SETUP => {
                if self.session_id.is_none()
                    && let Some(session_id) = rtsp_response.get_header("Session")
                {
                    self.session_id = Uuid::from_str2(session_id);
                }

                if let Some(transport_str) = rtsp_response.get_header("Transport") {
                    log::info!("setup response: transport {}", transport_str);
                }
            }
            rtsp_method_name::PLAY => {}
            rtsp_method_name::RECORD => {}
            _ => {}
        }
        Ok(())
    }

    pub fn exit(&mut self) -> Result<(), SessionError> {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: self.stream_name.clone(),
        };
        let event = match self.client_type {
            define::ClientSessionType::Push => StreamHubEvent::UnSubscribe {
                identifier,
                info: self.get_subscriber_info(),
            },
            define::ClientSessionType::Pull => StreamHubEvent::UnPublish {
                identifier,
                info: self.get_publisher_info(),
            },
        };

        let event_json_str =
            serde_json::to_string(&event).unwrap_or_else(|_| "<serialize failed>".to_string());

        let rv = self.event_producer.send(event);
        match rv {
            Err(err) => {
                log::error!("session exit: send event error: {err} for event: {event_json_str}");
                Err(SessionError {
                    value: SessionErrorValue::StreamHubEventSendErr,
                })
            }
            Ok(()) => {
                log::info!("session exit: send event success: {event_json_str}");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // ClientSessionType Tests
    // ========================================================================

    #[test]
    fn test_client_session_type_push() {
        let session_type = ClientSessionType::Push;
        assert!(matches!(session_type, ClientSessionType::Push));
    }

    #[test]
    fn test_client_session_type_pull() {
        let session_type = ClientSessionType::Pull;
        assert!(matches!(session_type, ClientSessionType::Pull));
    }

    // ========================================================================
    // Constants Tests
    // ========================================================================

    #[test]
    fn test_user_agent_not_empty() {
        assert!(!USER_AGENT.is_empty());
        assert!(USER_AGENT.len() > 0);
    }

    #[test]
    fn test_rtsp_method_names() {
        assert_eq!(rtsp_method_name::OPTIONS, "OPTIONS");
        assert_eq!(rtsp_method_name::DESCRIBE, "DESCRIBE");
        assert_eq!(rtsp_method_name::ANNOUNCE, "ANNOUNCE");
        assert_eq!(rtsp_method_name::SETUP, "SETUP");
        assert_eq!(rtsp_method_name::PLAY, "PLAY");
        assert_eq!(rtsp_method_name::RECORD, "RECORD");
        assert_eq!(rtsp_method_name::TEARDOWN, "TEARDOWN");
    }

    // ========================================================================
    // SubscriberInfo and PublisherInfo Tests
    // ========================================================================

    #[test]
    fn test_subscriber_info_default_values() {
        let info = SubscriberInfo {
            id: Uuid::new(RandomDigitCount::Zero),
            sub_type: SubscribeType::RtspRelay,
            sub_data_type: crate::streamhub::define::SubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };
        assert!(matches!(info.sub_type, SubscribeType::RtspRelay));
        assert!(info.notify_info.request_url.is_empty());
        assert!(info.notify_info.remote_addr.is_empty());
    }

    #[test]
    fn test_publisher_info_default_values() {
        let info = PublisherInfo {
            id: Uuid::new(RandomDigitCount::Zero),
            pub_type: PublishType::RtspRelay,
            pub_data_type: crate::streamhub::define::PubDataType::Frame,
            notify_info: NotifyInfo {
                request_url: String::from(""),
                remote_addr: String::from(""),
            },
        };
        assert!(matches!(info.pub_type, PublishType::RtspRelay));
        assert!(info.notify_info.request_url.is_empty());
        assert!(info.notify_info.remote_addr.is_empty());
    }

    // ========================================================================
    // Uri Tests
    // ========================================================================

    #[test]
    fn test_uri_unmarshal_valid() {
        let uri_str = "rtsp://192.168.1.1:554/stream";
        let uri = Uri::unmarshal(uri_str);
        assert!(uri.is_some());
        let uri = uri.unwrap();
        assert!(matches!(uri.schema, crate::common::http::Schema::RTSP));
        assert_eq!(uri.host, "192.168.1.1");
    }

    #[test]
    fn test_uri_unmarshal_with_path() {
        let uri_str = "rtsp://example.com/live/stream1";
        let uri = Uri::unmarshal(uri_str);
        assert!(uri.is_some());
        let uri = uri.unwrap();
        assert!(uri.path.contains("live"));
    }

    // ========================================================================
    // RtspTransport Tests
    // ========================================================================

    #[test]
    fn test_rtsp_transport_default() {
        let transport = RtspTransport::default();
        assert!(transport.client_port.is_none());
        assert!(transport.server_port.is_none());
        assert!(transport.interleaved.is_none());
    }

    #[test]
    fn test_rtsp_transport_tcp() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::TCP,
            cast_type: CastType::Unicast,
            interleaved: Some([0, 1]),
            ..Default::default()
        };
        assert!(matches!(transport.protocol_type, ProtocolType::TCP));
        assert!(matches!(transport.cast_type, CastType::Unicast));
        assert_eq!(transport.interleaved, Some([0, 1]));
    }

    #[test]
    fn test_rtsp_transport_udp() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::UDP,
            cast_type: CastType::Unicast,
            client_port: Some([5000, 5001]),
            ..Default::default()
        };
        assert!(matches!(transport.protocol_type, ProtocolType::UDP));
        assert_eq!(transport.client_port, Some([5000, 5001]));
    }

    #[test]
    fn test_rtsp_transport_marshal() {
        let transport = RtspTransport {
            protocol_type: ProtocolType::TCP,
            cast_type: CastType::Unicast,
            interleaved: Some([0, 1]),
            ..Default::default()
        };
        let marshaled = transport.marshal();
        assert!(!marshaled.is_empty());
        assert!(marshaled.contains("RTP/AVP"));
    }

    // ========================================================================
    // Protocol Type Tests
    // ========================================================================

    #[test]
    fn test_protocol_type_tcp() {
        let pt = ProtocolType::TCP;
        assert!(matches!(pt, ProtocolType::TCP));
    }

    #[test]
    fn test_protocol_type_udp() {
        let pt = ProtocolType::UDP;
        assert!(matches!(pt, ProtocolType::UDP));
    }

    // ========================================================================
    // CastType Tests
    // ========================================================================

    #[test]
    fn test_cast_type_unicast() {
        let ct = CastType::Unicast;
        assert!(matches!(ct, CastType::Unicast));
    }

    #[test]
    fn test_cast_type_multicast() {
        let ct = CastType::Multicast;
        assert!(matches!(ct, CastType::Multicast));
    }

    // ========================================================================
    // SessionError Tests
    // ========================================================================

    #[test]
    fn test_session_error_rtsp_response_status_error() {
        let err = SessionError {
            value: SessionErrorValue::RtspResponseStatusError,
        };
        assert!(matches!(
            err.value,
            SessionErrorValue::RtspResponseStatusError
        ));
    }

    #[test]
    fn test_session_error_stream_hub_event_send_err() {
        let err = SessionError {
            value: SessionErrorValue::StreamHubEventSendErr,
        };
        assert!(matches!(
            err.value,
            SessionErrorValue::StreamHubEventSendErr
        ));
    }

    // ========================================================================
    // StreamIdentifier Tests
    // ========================================================================

    #[test]
    fn test_stream_identifier_rtsp() {
        let identifier = StreamIdentifier::Rtsp {
            stream_path: "live/stream1".to_string(),
        };
        if let StreamIdentifier::Rtsp { stream_path } = identifier {
            assert_eq!(stream_path, "live/stream1");
        } else {
            panic!("Expected Rtsp variant");
        }
    }

    // ========================================================================
    // Uuid Tests
    // ========================================================================

    #[test]
    fn test_uuid_new_zero_digits() {
        let uuid = Uuid::new(RandomDigitCount::Zero);
        // Should be created without panic
        let _ = uuid.to_string();
    }

    #[test]
    fn test_uuid_to_string_not_empty() {
        let uuid = Uuid::new(RandomDigitCount::Zero);
        let s = uuid.to_string();
        assert!(!s.is_empty());
    }

    use crate::bytesio::bytesio_errors::BytesIOError;
    use crate::bytesio::{NetType, TNetIO};
    use async_trait::async_trait;
    use bytes::Bytes;
    use mockall::mock;

    mock! {
        pub NetIO {}

        #[async_trait]
        impl TNetIO for NetIO {
            async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
            async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
            async fn read_timeout(&mut self, duration: std::time::Duration) -> Result<BytesMut, BytesIOError>;
            fn get_net_type(&self) -> NetType;
        }
    }

    #[tokio::test]
    async fn test_rtsp_client_session_send_options() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("OPTIONS rtsp://localhost:554/live/test RTSP/1.0")
            })
            .times(1)
            .returning(|_| Ok(()));

        // Mock get_net_type called by AsyncBytesWriter
        mock_io.expect_read()
            .returning(|| {
                let response = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN\r\n\r\n";
                Ok(BytesMut::from(response))
            });
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.send_options().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_client_session_receive_response() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Respond to receive_response
        mock_io.expect_read()
            .times(1)
            .returning(|| {
                let response = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN\r\n\r\n";
                Ok(BytesMut::from(response))
            });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.receive_response("OPTIONS").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rtsp_client_session_receive_response_with_interleaved_binary_buffered() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        mock_io
            .expect_read()
            .times(1)
            .returning(|| {
                let response =
                    "RTSP/1.0 200 OK\r\nCSeq: 1\r\nPublic: OPTIONS, DESCRIBE, SETUP, PLAY, TEARDOWN\r\n\r\n";
                let mut buf = BytesMut::from(response);
                buf.extend_from_slice(&[0x24, 0x00, 0x00, 0x04, 0xff, 0xff, 0xff, 0xff]);
                Ok(buf)
            });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.receive_response("OPTIONS").await;
        assert!(result.is_ok());

        let remaining = session.reader.get_remaining_bytes();
        assert_eq!(remaining.len(), 8);
        assert_eq!(remaining[0], 0x24);
    }
}
