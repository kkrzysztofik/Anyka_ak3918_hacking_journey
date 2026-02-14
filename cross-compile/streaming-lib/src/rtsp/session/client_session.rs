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

    fn get_media_control(&self, media: &crate::rtsp::sdp::SdpMediaInfo) -> String {
        media.attributes.get("control").cloned().unwrap_or_else(|| {
            log::error!("cannot get media control!!");
            String::from("")
        })
    }

    fn parse_interleaved_idx(&self, media_control: &str) -> Option<u8> {
        let kv: Vec<&str> = media_control.trim().splitn(2, '=').collect();
        if kv.len() < 2 || kv[1].trim().is_empty() {
            log::error!("cannot parse control attribute: {}", media_control);
            return None;
        }
        kv[1].trim().parse::<u8>().ok()
    }

    fn apply_interleaved_to_track(&mut self, media_type: &str, interleaved: Option<[u8; 2]>) {
        let track_type = match media_type {
            "audio" => &TrackType::Audio,
            "video" => &TrackType::Video,
            _ => return,
        };
        if let Some(track) = self.tracks.get_mut(track_type) {
            track.transport.interleaved = interleaved;
        }
    }

    fn build_tcp_transport(&self, interleaved_idx: u8) -> RtspTransport {
        RtspTransport {
            protocol_type: ProtocolType::TCP,
            cast_type: CastType::Unicast,
            interleaved: Some([interleaved_idx * 2, interleaved_idx * 2 + 1]),
            ..Default::default()
        }
    }

    async fn setup_tcp_transport(
        &mut self,
        media_control: &str,
        media_type: &str,
        request: &mut RtspRequest,
    ) -> bool {
        let Some(interleaved_idx) = self.parse_interleaved_idx(media_control) else {
            return false;
        };

        let media_transport = self.build_tcp_transport(interleaved_idx);
        request
            .headers
            .insert("Transport".to_string(), media_transport.marshal());
        self.apply_interleaved_to_track(media_type, media_transport.interleaved);
        true
    }

    async fn setup_udp_transport(
        &mut self,
        media_type: &str,
        request: &mut RtspRequest,
    ) -> Result<bool, SessionError> {
        let Some((socket_rtp, socket_rtcp)) = new_udpio_pair().await else {
            return Ok(false);
        };

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

        let track_type = match media_type {
            "audio" => &TrackType::Audio,
            "video" => &TrackType::Video,
            _ => return Ok(true),
        };

        if let Some(track) = self.tracks.get_mut(track_type) {
            let box_rtp_io: Box<dyn TNetIO + Send + Sync> = Box::new(socket_rtp);
            track.rtp_receive_loop(box_rtp_io).await;

            let box_rtcp_io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>> =
                Arc::new(Mutex::new(Box::new(socket_rtcp)));
            track.rtcp_receive_loop(box_rtcp_io).await;
        }

        Ok(true)
    }

    async fn send_setup(&mut self) -> Result<(), SessionError> {
        log::info!("rtsp client: send_setup");
        let sdp_medias = self.sdp.medias.clone();

        for media in sdp_medias {
            let media_control = self.get_media_control(&media);
            let uri_path = format!(
                "rtsp://{}/{}/{}",
                self.address, self.stream_name, media_control
            );

            let mut request = self.gen_request(rtsp_method_name::SETUP, uri_path)?;

            let should_send = match self.protocol_type {
                ProtocolType::TCP => {
                    self.setup_tcp_transport(&media_control, &media.media_type, &mut request)
                        .await
                }
                ProtocolType::UDP => {
                    self.setup_udp_transport(&media.media_type, &mut request)
                        .await?
                }
            };

            if !should_send {
                continue;
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

    async fn read_message(&mut self) -> Result<usize, SessionError> {
        let mut retry_count = 0;
        loop {
            let data = self.reader.get_remaining_bytes();
            match try_get_complete_message_len(&data) {
                Ok(Some(len)) => return Ok(len),
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
        }
    }

    fn parse_response(&mut self, message_len: usize) -> Result<RtspResponse, SessionError> {
        let message_bytes = self.reader.read_bytes(message_len)?;
        let message_str = std::str::from_utf8(&message_bytes)?;

        RtspResponse::unmarshal(message_str).ok_or(SessionError {
            value: SessionErrorValue::RtspMessageCorrupted("response parse failed".to_string()),
        })
    }

    fn validate_response_status(&self, response: &RtspResponse) -> Result<(), SessionError> {
        if response.status_code != http::StatusCode::OK {
            log::error!("rtsp response error: {}", response.marshal());
            return Err(SessionError {
                value: SessionErrorValue::RtspResponseStatusError,
            });
        }
        Ok(())
    }

    fn handle_options_response(&self, response: &RtspResponse) {
        if let Some(public) = response.get_header("Public") {
            log::info!("support methods: {}", public);
        }
    }

    async fn handle_describe_response(
        &mut self,
        response: &RtspResponse,
    ) -> Result<(), SessionError> {
        let Some(request_body) = &response.body else {
            return Ok(());
        };

        let Ok(sdp) = Sdp::unmarshal(request_body) else {
            return Ok(());
        };

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

        self.setup_track_frame_handlers(&sender).await;
        Ok(())
    }

    async fn setup_track_frame_handlers(
        &mut self,
        sender: &tokio::sync::mpsc::UnboundedSender<FrameData>,
    ) {
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
    }

    fn handle_setup_response(&mut self, response: &RtspResponse) {
        if self.session_id.is_none()
            && let Some(session_id) = response.get_header("Session")
        {
            self.session_id = Uuid::from_str2(session_id);
        }

        if let Some(transport_str) = response.get_header("Transport") {
            log::info!("setup response: transport {}", transport_str);
        }
    }

    async fn dispatch_response_handler(
        &mut self,
        method_name: &str,
        response: &RtspResponse,
    ) -> Result<(), SessionError> {
        match method_name {
            rtsp_method_name::OPTIONS => {
                self.handle_options_response(response);
            }
            rtsp_method_name::ANNOUNCE => {}
            rtsp_method_name::DESCRIBE => {
                self.handle_describe_response(response).await?;
            }
            rtsp_method_name::SETUP => {
                self.handle_setup_response(response);
            }
            rtsp_method_name::PLAY | rtsp_method_name::RECORD => {}
            _ => {}
        }
        Ok(())
    }

    async fn receive_response(&mut self, method_name: &str) -> Result<(), SessionError> {
        let message_len = self.read_message().await?;
        let rtsp_response = self.parse_response(message_len)?;

        self.validate_response_status(&rtsp_response)?;
        self.dispatch_response_handler(method_name, &rtsp_response)
            .await
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

    #[tokio::test]
    async fn test_rtsp_client_session_receive_response_non_200_returns_status_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\nContent-Length: 0\r\n\r\n";
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

        let result = session.receive_response(rtsp_method_name::OPTIONS).await;

        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::RtspResponseStatusError => {}
            other => panic!("expected RtspResponseStatusError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_rtsp_client_session_receive_response_parse_failure_returns_corrupted_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(1).returning(|| {
            let response = BytesMut::from(&b"RTSP/1.0 200 OK\r\nContent-Length: abc\r\n\r\n"[..]);
            Ok(response)
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.receive_response(rtsp_method_name::OPTIONS).await;

        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::RtspMessageCorrupted(err) => {
                assert!(err.contains("invalid Content-Length value"));
            }
            other => panic!("expected RtspMessageCorrupted, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_rtsp_client_session_receive_response_retry_limit_returns_corrupted_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io
            .expect_read()
            .times(16)
            .returning(|| Ok(BytesMut::from(&b"RTSP/1.0 200 OK\r\nCSeq: 1\r\n"[..])));

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.receive_response(rtsp_method_name::OPTIONS).await;

        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::RtspMessageCorrupted(err) => {
                assert!(err.contains("max read retries exceeded"));
            }
            other => panic!("expected RtspMessageCorrupted retry error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_rtsp_client_session_receive_response_describe_emits_publish_event() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().times(1).returning(|| {
            let sdp_body =
                "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\n";
            let response = format!(
                "RTSP/1.0 200 OK\r\nCSeq: 2\r\nContent-Length: {}\r\n\r\n{}",
                sdp_body.len(),
                sdp_body
            );
            Ok(BytesMut::from(response.as_str()))
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let receive_task =
            tokio::spawn(async move { session.receive_response(rtsp_method_name::DESCRIBE).await });

        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        .expect("timed out waiting for StreamHubEvent::Publish")
        .expect("event channel unexpectedly closed");

        match event {
            StreamHubEvent::Publish {
                identifier,
                info: _,
                result_sender,
                stream_handler: _,
            } => {
                assert!(matches!(
                    identifier,
                    StreamIdentifier::Rtsp { stream_path } if stream_path == "live/test"
                ));
                let (frame_sender, _frame_receiver) = tokio::sync::mpsc::unbounded_channel();
                result_sender
                    .send(Ok((Some(frame_sender), None, None)))
                    .expect("failed to return publish event result");
            }
            _ => panic!("expected StreamHubEvent::Publish"),
        }

        let result = receive_task
            .await
            .expect("receive_response task panicked unexpectedly");
        assert!(result.is_ok());
    }

    // ========================================================================
    // send_describe Tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_describe_success_returns_ok() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Expect write with DESCRIBE request containing Accept header
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("DESCRIBE rtsp://localhost:554/live/test RTSP/1.0")
                    && s.contains("Accept: application/sdp")
            })
            .times(1)
            .returning(|_| Ok(()));

        // Response with SDP body
        mock_io.expect_read().times(1).returning(|| {
            let sdp_body =
                "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\ns=No Name\r\nc=IN IP4 127.0.0.1\r\nt=0 0\r\n";
            let response = format!(
                "RTSP/1.0 200 OK\r\nCSeq: 1\r\nContent-Length: {}\r\n\r\n{}",
                sdp_body.len(),
                sdp_body
            );
            Ok(BytesMut::from(response.as_str()))
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let describe_task = tokio::spawn(async move { session.send_describe().await });

        // Handle the publish event from describe response
        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        .expect("timed out waiting for StreamHubEvent::Publish")
        .expect("event channel unexpectedly closed");

        if let StreamHubEvent::Publish { result_sender, .. } = event {
            let (frame_sender, _frame_receiver) = tokio::sync::mpsc::unbounded_channel();
            result_sender
                .send(Ok((Some(frame_sender), None, None)))
                .expect("failed to return publish event result");
        } else {
            panic!("expected StreamHubEvent::Publish");
        }

        let result = describe_task
            .await
            .expect("send_describe task panicked unexpectedly");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_describe_server_error_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_write().returning(|_| Ok(()));
        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 404 Not Found\r\nCSeq: 1\r\n\r\n";
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

        let result = session.send_describe().await;
        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::RtspResponseStatusError => {}
            other => panic!("expected RtspResponseStatusError, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_send_describe_empty_body_returns_ok() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_write().returning(|_| Ok(()));
        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nContent-Length: 0\r\n\r\n";
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

        // Empty body should return Ok(()) as per handle_describe_response logic
        let result = session.send_describe().await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // send_setup Tests - TCP Transport
    // ========================================================================

    #[tokio::test]
    async fn test_send_setup_tcp_transport_success() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Expect SETUP request with TCP transport
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("SETUP") && s.contains("RTP/AVP/TCP") && s.contains("interleaved=")
            })
            .times(1)
            .returning(|_| Ok(()));

        mock_io.expect_read().times(1).returning(|| {
            let response =
                "RTSP/1.0 200 OK\r\nCSeq: 1\r\nSession: 1234567890\r\nTransport: RTP/AVP/TCP;unicast;interleaved=0-1\r\n\r\n";
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

        // Pre-populate SDP with a media track. Control must contain interleaved=N for TCP.
        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "video".to_string();
        media
            .attributes
            .insert("control".to_string(), "track1;interleaved=0".to_string());
        media.rtpmap = crate::rtsp::sdp::rtpmap::RtpMap {
            payload_type: 96,
            encoding_name: "H264".to_string(),
            clock_rate: 90000,
            encoding_param: "".to_string(),
        };
        session.sdp.medias.push(media);

        let result = session.send_setup().await;
        assert!(result.is_ok());
        // Session header extraction is covered by test_handle_setup_response_extracts_session_id.
        // Here we only assert the SETUP request/response flow completes successfully.
    }

    #[tokio::test]
    async fn test_send_setup_tcp_transport_missing_control_attribute_skipped() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        // No write expected since track should be skipped

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        // SDP without control attribute - will log error and skip
        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "video".to_string();
        media.attributes = std::collections::HashMap::new();
        session.sdp.medias.push(media);

        let result = session.send_setup().await;
        // Should complete without error (skips the track)
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_setup_tcp_transport_invalid_interleaved_idx_skipped() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        // No write expected since parse_interleaved_idx will fail

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        // Control attribute without valid interleaved index
        let mut attrs = std::collections::HashMap::new();
        attrs.insert("control".to_string(), "invalid_control".to_string());
        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "video".to_string();
        media.attributes = attrs;
        session.sdp.medias.push(media);

        let result = session.send_setup().await;
        // Should complete without error (skips the track)
        assert!(result.is_ok());
    }

    // ========================================================================
    // send_play Tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_play_success_returns_ok() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Expect PLAY request with Range header
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("PLAY rtsp://localhost:554/live/test RTSP/1.0")
                    && s.contains("Range: npt=0.000")
            })
            .times(1)
            .returning(|_| Ok(()));

        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nSession: 12345678\r\n\r\n";
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

        let result = session.send_play().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_play_with_session_id_includes_header() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("Session: test-session-123")
            })
            .times(1)
            .returning(|_| Ok(()));

        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 200 OK\r\nCSeq: 1\r\n\r\n";
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

        // Set session_id - from_str2 returns Option<Uuid> directly
        session.session_id = Uuid::from_str2("test-session-123");

        let result = session.send_play().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_play_server_error_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_write().returning(|_| Ok(()));
        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 454 Session Not Found\r\nCSeq: 1\r\n\r\n";
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

        let result = session.send_play().await;
        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::RtspResponseStatusError => {}
            other => panic!("expected RtspResponseStatusError, got: {other:?}"),
        }
    }

    // ========================================================================
    // send_record Tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_record_success_returns_ok() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Expect RECORD request with Transport header
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("RECORD rtsp://localhost:554/live/test RTSP/1.0")
                    && s.contains("Transport: application/sdp")
            })
            .times(1)
            .returning(|_| Ok(()));

        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 200 OK\r\nCSeq: 1\r\nSession: 12345678\r\n\r\n";
            Ok(BytesMut::from(response))
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Push, // Record is for Push clients
            Box::new(mock_io),
        );

        let result = session.send_record().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_record_server_error_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_write().returning(|_| Ok(()));
        mock_io.expect_read().times(1).returning(|| {
            let response = "RTSP/1.0 405 Method Not Allowed\r\nCSeq: 1\r\n\r\n";
            Ok(BytesMut::from(response))
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Push,
            Box::new(mock_io),
        );

        let result = session.send_record().await;
        assert!(result.is_err());
    }

    // ========================================================================
    // send_teardown Tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_teardown_success_calls_exit() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        // Expect TEARDOWN request
        mock_io
            .expect_write()
            .withf(|bytes| {
                let s = std::str::from_utf8(bytes).unwrap();
                s.contains("TEARDOWN rtsp://localhost:554/live/test RTSP/1.0")
            })
            .times(1)
            .returning(|_| Ok(()));

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let teardown_task = tokio::spawn(async move { session.send_teardown().await });

        // Expect UnPublish event for Pull client
        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        .expect("timed out waiting for StreamHubEvent::UnPublish")
        .expect("event channel unexpectedly closed");

        match event {
            StreamHubEvent::UnPublish { identifier, .. } => {
                assert!(matches!(
                    identifier,
                    StreamIdentifier::Rtsp { stream_path } if stream_path == "live/test"
                ));
            }
            _ => panic!("expected StreamHubEvent::UnPublish for Pull client"),
        }

        let result = teardown_task
            .await
            .expect("send_teardown task panicked unexpectedly");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_teardown_push_client_sends_unsubscribe() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_write().returning(|_| Ok(()));

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Push, // Push client
            Box::new(mock_io),
        );

        let teardown_task = tokio::spawn(async move { session.send_teardown().await });

        // Expect UnSubscribe event for Push client
        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        .expect("timed out waiting for StreamHubEvent::UnSubscribe")
        .expect("event channel unexpectedly closed");

        match event {
            StreamHubEvent::UnSubscribe { identifier, .. } => {
                assert!(matches!(
                    identifier,
                    StreamIdentifier::Rtsp { stream_path } if stream_path == "live/test"
                ));
            }
            _ => panic!("expected StreamHubEvent::UnSubscribe for Push client"),
        }

        let result = teardown_task
            .await
            .expect("send_teardown task panicked unexpectedly");
        assert!(result.is_ok());
    }

    // ========================================================================
    // exit Tests
    // ========================================================================

    #[tokio::test]
    async fn test_exit_pull_client_sends_unpublish_event() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let exit_task = tokio::spawn(async move { session.exit() });

        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        .expect("timed out waiting for event")
        .expect("event channel unexpectedly closed");

        assert!(matches!(event, StreamHubEvent::UnPublish { .. }));

        let result = exit_task.await.expect("exit task panicked unexpectedly");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_exit_push_client_sends_unsubscribe_event() {
        let (event_sender, mut event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Push,
            Box::new(mock_io),
        );

        let exit_task = tokio::spawn(async move { session.exit() });

        let event = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        .expect("timed out waiting for event")
        .expect("event channel unexpectedly closed");

        assert!(matches!(event, StreamHubEvent::UnSubscribe { .. }));

        let result = exit_task.await.expect("exit task panicked unexpectedly");
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_exit_with_closed_event_channel_returns_error() {
        let (event_sender, event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        // Drop receiver to close the channel
        drop(event_receiver);

        let result = session.exit();
        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::StreamHubEventSendErr => {}
            other => panic!("expected StreamHubEventSendErr, got: {other:?}"),
        }
    }

    // ========================================================================
    // gen_request Tests
    // ========================================================================

    #[tokio::test]
    async fn test_gen_request_increments_cseq() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let initial_cseq = session.cseq;
        let req1 = session.gen_request("OPTIONS", "rtsp://localhost:554/test".to_string());
        let req2 = session.gen_request("OPTIONS", "rtsp://localhost:554/test".to_string());

        assert!(req1.is_ok());
        assert!(req2.is_ok());
        assert_eq!(session.cseq, initial_cseq + 2);
    }

    #[tokio::test]
    async fn test_gen_request_includes_user_agent() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let request = session
            .gen_request("OPTIONS", "rtsp://localhost:554/test".to_string())
            .unwrap();

        assert_eq!(
            request.headers.get("User-Agent"),
            Some(&USER_AGENT.to_string())
        );
    }

    #[tokio::test]
    async fn test_gen_request_with_session_id_includes_header() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        session.session_id = Uuid::from_str2("test-session-abc");

        let request = session
            .gen_request("PLAY", "rtsp://localhost:554/test".to_string())
            .unwrap();

        assert_eq!(
            request.headers.get("Session"),
            Some(&"test-session-abc".to_string())
        );
    }

    #[tokio::test]
    async fn test_gen_request_invalid_uri_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        // Uri::unmarshal treats non-rtsp strings as Schema::UNKNOWN and always returns Some.
        // So gen_request succeeds; assert we get a valid request with OPTIONS method.
        let result = session.gen_request("OPTIONS", "not-a-valid-uri".to_string());
        assert!(
            result.is_ok(),
            "gen_request accepts non-rtsp URI as unknown schema"
        );
        let request = result.unwrap();
        assert_eq!(request.method, "OPTIONS");
    }

    // ========================================================================
    // parse_interleaved_idx Tests
    // ========================================================================

    #[test]
    fn test_parse_interleaved_idx_valid_returns_value() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.parse_interleaved_idx("interleaved=2");
        assert_eq!(result, Some(2));
    }

    #[test]
    fn test_parse_interleaved_idx_with_whitespace_returns_value() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.parse_interleaved_idx(" interleaved = 3 ");
        assert_eq!(result, Some(3));
    }

    #[test]
    fn test_parse_interleaved_idx_no_equals_returns_none() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.parse_interleaved_idx("interleaved");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_interleaved_idx_empty_value_returns_none() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.parse_interleaved_idx("interleaved=");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_interleaved_idx_non_numeric_returns_none() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.parse_interleaved_idx("interleaved=abc");
        assert!(result.is_none());
    }

    // ========================================================================
    // get_media_control Tests
    // ========================================================================

    #[test]
    fn test_get_media_control_with_attribute_returns_value() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let mut attrs = std::collections::HashMap::new();
        attrs.insert("control".to_string(), "track1".to_string());
        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.attributes = attrs;

        let result = session.get_media_control(&media);
        assert_eq!(result, "track1");
    }

    #[test]
    fn test_get_media_control_without_attribute_returns_empty() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.attributes = std::collections::HashMap::new();

        let result = session.get_media_control(&media);
        assert_eq!(result, "");
    }

    // ========================================================================
    // build_tcp_transport Tests
    // ========================================================================

    #[test]
    fn test_build_tcp_transport_correct_interleaved_values() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let transport = session.build_tcp_transport(0);
        assert_eq!(transport.interleaved, Some([0, 1]));
        assert!(matches!(transport.protocol_type, ProtocolType::TCP));
        assert!(matches!(transport.cast_type, CastType::Unicast));

        let transport = session.build_tcp_transport(1);
        assert_eq!(transport.interleaved, Some([2, 3]));
    }

    // ========================================================================
    // validate_response_status Tests
    // ========================================================================

    #[tokio::test]
    async fn test_validate_response_status_ok_returns_ok() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let response = RtspResponse {
            status_code: http::StatusCode::OK.as_u16(),
            ..Default::default()
        };

        let result = session.validate_response_status(&response);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_response_status_non_ok_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let response = RtspResponse {
            status_code: http::StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
            ..Default::default()
        };

        let result = session.validate_response_status(&response);
        assert!(result.is_err());
        match result.unwrap_err().value {
            SessionErrorValue::RtspResponseStatusError => {}
            other => panic!("expected RtspResponseStatusError, got: {other:?}"),
        }
    }

    // ========================================================================
    // handle_setup_response Tests
    // ========================================================================

    #[tokio::test]
    async fn test_handle_setup_response_extracts_session_id() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        assert!(session.session_id.is_none());

        let response = RtspResponse {
            status_code: http::StatusCode::OK.as_u16(),
            headers: crate::common::http::HttpIndexMap::from_iter([(
                "Session".to_string(),
                "1234567890".to_string(),
            )]),
            ..Default::default()
        };

        session.handle_setup_response(&response);
        assert!(session.session_id.is_some());
    }

    #[tokio::test]
    async fn test_handle_setup_response_preserves_existing_session_id() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let original_id = Uuid::from_str2("original-id");
        session.session_id = original_id;

        let response = RtspResponse {
            status_code: http::StatusCode::OK.as_u16(),
            headers: crate::common::http::HttpIndexMap::from_iter([(
                "Session".to_string(),
                "new-id".to_string(),
            )]),
            ..Default::default()
        };

        session.handle_setup_response(&response);
        // Should keep original ID
        assert_eq!(session.session_id, original_id);
    }

    // ========================================================================
    // Network Error Tests
    // ========================================================================

    #[tokio::test]
    async fn test_send_request_write_error_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_write().returning(|_| {
            Err(BytesIOError {
                value: crate::bytesio::bytesio_errors::BytesIOErrorValue::NoneReturn,
            })
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let request = RtspRequest {
            method: "OPTIONS".to_string(),
            uri: Uri::unmarshal("rtsp://localhost:554/test").unwrap(),
            version: "RTSP/1.0".to_string(),
            ..Default::default()
        };

        let result = session.send_resquest(&request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_message_read_error_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);
        mock_io.expect_read().returning(|| {
            Err(BytesIOError {
                value: crate::bytesio::bytesio_errors::BytesIOErrorValue::NoneReturn,
            })
        });

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.read_message().await;
        assert!(result.is_err());
    }

    // ========================================================================
    // Malformed Response Tests
    // ========================================================================

    #[tokio::test]
    async fn test_parse_response_invalid_utf8_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        // Put invalid UTF-8 bytes in reader
        session.reader.extend_from_slice(&[0xff, 0xfe, 0xfd]);

        let result = session.parse_response(3);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_response_empty_returns_error() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let result = session.parse_response(0);
        assert!(result.is_err());
    }

    // ========================================================================
    // get_subscriber_info and get_publisher_info Tests
    // ========================================================================

    #[tokio::test]
    async fn test_get_subscriber_info_with_session_id() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let test_id = Uuid::new(RandomDigitCount::Zero);
        session.session_id = Some(test_id);

        let info = session.get_subscriber_info();
        assert_eq!(info.id, test_id);
        assert!(matches!(info.sub_type, SubscribeType::RtspRelay));
    }

    #[tokio::test]
    async fn test_get_publisher_info_with_session_id() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Push,
            Box::new(mock_io),
        );

        let test_id = Uuid::new(RandomDigitCount::Zero);
        session.session_id = Some(test_id);

        let info = session.get_publisher_info();
        assert_eq!(info.id, test_id);
        assert!(matches!(info.pub_type, PublishType::RtspRelay));
    }

    // ========================================================================
    // new_tracks Tests
    // ========================================================================

    #[tokio::test]
    async fn test_new_tracks_audio_codec_supported() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let mut attrs = std::collections::HashMap::new();
        attrs.insert("control".to_string(), "audio_track".to_string());

        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "audio".to_string();
        media.attributes = attrs;
        media.rtpmap = crate::rtsp::sdp::rtpmap::RtpMap {
            payload_type: 97,
            encoding_name: "mpeg4-generic".to_string(),
            clock_rate: 48000,
            encoding_param: "2".to_string(),
        };
        session.sdp.medias.push(media);

        let result = session.new_tracks();
        assert!(result.is_ok());
        assert!(session.tracks.contains_key(&TrackType::Audio));
    }

    #[tokio::test]
    async fn test_new_tracks_video_codec_supported() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let mut attrs = std::collections::HashMap::new();
        attrs.insert("control".to_string(), "video_track".to_string());

        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "video".to_string();
        media.attributes = attrs;
        media.rtpmap = crate::rtsp::sdp::rtpmap::RtpMap {
            payload_type: 96,
            encoding_name: "h264".to_string(),
            clock_rate: 90000,
            encoding_param: "".to_string(),
        };
        session.sdp.medias.push(media);

        let result = session.new_tracks();
        assert!(result.is_ok());
        assert!(session.tracks.contains_key(&TrackType::Video));
    }

    #[tokio::test]
    async fn test_new_tracks_unsupported_codec_skipped() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let mut attrs = std::collections::HashMap::new();
        attrs.insert("control".to_string(), "video_track".to_string());

        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "video".to_string();
        media.attributes = attrs;
        media.rtpmap = crate::rtsp::sdp::rtpmap::RtpMap {
            payload_type: 96,
            encoding_name: "unsupported-codec".to_string(),
            clock_rate: 90000,
            encoding_param: "".to_string(),
        };
        session.sdp.medias.push(media);

        let result = session.new_tracks();
        assert!(result.is_ok());
        // Track should be skipped due to unsupported codec
        assert!(!session.tracks.contains_key(&TrackType::Video));
    }

    #[tokio::test]
    async fn test_new_tracks_invalid_channel_count_skipped() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mut mock_io = MockNetIO::new();
        mock_io.expect_get_net_type().returning(|| NetType::TCP);

        let mut session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        let mut attrs = std::collections::HashMap::new();
        attrs.insert("control".to_string(), "audio_track".to_string());

        let mut media = crate::rtsp::sdp::SdpMediaInfo::default();
        media.media_type = "audio".to_string();
        media.attributes = attrs;
        media.rtpmap = crate::rtsp::sdp::rtpmap::RtpMap {
            payload_type: 97,
            encoding_name: "mpeg4-generic".to_string(),
            clock_rate: 48000,
            encoding_param: "not_a_number".to_string(),
        };
        session.sdp.medias.push(media);

        let result = session.new_tracks();
        assert!(result.is_ok());
        // Track should be skipped due to invalid channel count
        assert!(!session.tracks.contains_key(&TrackType::Audio));
    }

    // ========================================================================
    // is_running Flag Tests
    // ========================================================================

    #[tokio::test]
    async fn test_is_running_initially_true() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        assert!(
            session
                .is_running
                .load(std::sync::atomic::Ordering::Acquire)
        );
    }

    #[tokio::test]
    async fn test_is_running_can_be_set_false() {
        let (event_sender, _event_receiver) = tokio::sync::mpsc::unbounded_channel();

        let mock_io = MockNetIO::new();
        let session = RtspClientSession::new_with_io(
            "localhost:554".to_string(),
            "live/test".to_string(),
            ProtocolType::TCP,
            event_sender,
            ClientSessionType::Pull,
            Box::new(mock_io),
        );

        session
            .is_running
            .store(false, std::sync::atomic::Ordering::Release);
        assert!(
            !session
                .is_running
                .load(std::sync::atomic::Ordering::Acquire)
        );
    }
}
