use {
    super::stream::StreamIdentifier,
    crate::container::define::{AacProfile, AvcCodecId, AvcLevel, AvcProfile, SoundFormat},
    crate::hub::{define::SubscribeType, utils::Uuid},
    chrono::{DateTime, Local},
    serde::Serialize,
    std::{collections::HashMap, sync::Arc, time::Duration},
    tokio::{
        sync::{Mutex, broadcast::Receiver},
        time,
    },
    tracing::info,
};

#[derive(Debug, Clone, Serialize, Default)]
pub struct VideoInfo {
    pub codec: AvcCodecId,
    pub profile: AvcProfile,
    pub level: AvcLevel,
    pub width: u32,
    pub height: u32,
    /*used for calculate the bitrate*/
    #[serde(skip_serializing)]
    pub recv_bytes: usize,
    #[serde(rename = "bitrate(kbits/s)")]
    pub bitrate: usize,
    /*used for calculate the frame rate*/
    #[serde(skip_serializing)]
    pub recv_frame_count: usize,
    pub frame_rate: usize,
    /*used for calculate the GOP*/
    #[serde(skip_serializing)]
    pub recv_frame_count_for_gop: usize,
    pub gop: usize,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct AudioInfo {
    pub sound_format: SoundFormat,
    pub profile: AacProfile,
    pub samplerate: u32,
    pub channels: u8,
    /*used for calculate the bitrate*/
    #[serde(skip_serializing)]
    pub recv_bytes: usize,
    #[serde(rename = "bitrate(kbits/s)")]
    pub bitrate: usize,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct StatisticsStream {
    /*publisher information */
    pub publisher: StatisticPublisher,
    /*subscriber information */
    pub subscribers: HashMap<Uuid, StatisticSubscriber>,
    /*How many clients are subscribing to this stream.*/
    pub subscriber_count: usize,
    /*calculate upstream traffic, now equals audio and video traffic received by this server*/
    pub total_recv_bytes: usize,
    /*calculate downstream traffic, now equals audio and video traffic sent to all subscribers*/
    pub total_send_bytes: usize,
}
#[derive(Debug, Clone, Serialize, Default)]
pub struct StatisticPublisher {
    pub id: Uuid,
    identifier: StreamIdentifier,
    pub start_time: DateTime<Local>,
    pub video: VideoInfo,
    pub audio: AudioInfo,
    pub remote_address: String,
    /*used for calculate the recv_bitrate*/
    #[serde(skip_serializing)]
    pub recv_bytes: usize,
    /*the bitrate at which the server receives streaming data*/
    #[serde(rename = "recv_bitrate(kbits/s)")]
    pub recv_bitrate: usize,
}

impl StatisticPublisher {
    pub fn new(identifier: StreamIdentifier) -> Self {
        Self {
            identifier,
            ..Default::default()
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct StatisticSubscriber {
    pub id: Uuid,
    pub start_time: DateTime<Local>,
    pub remote_address: String,
    pub sub_type: SubscribeType,
    /*used for calculate the send_bitrate*/
    #[serde(skip_serializing)]
    pub send_bytes: usize,
    /*the bitrate at which the server send streaming data to a client*/
    #[serde(rename = "send_bitrate(kbits/s)")]
    pub send_bitrate: usize,
    #[serde(rename = "total_send_bytes(kbits/s)")]
    pub total_send_bytes: usize,
}

impl StatisticsStream {
    pub fn new(identifier: StreamIdentifier) -> Self {
        Self {
            publisher: StatisticPublisher::new(identifier),
            ..Default::default()
        }
    }

    fn get_publisher(&self) -> StatisticsStream {
        let mut statistic_stream = self.clone();
        statistic_stream.subscribers.clear();
        statistic_stream
    }

    fn get_subscriber(&self, uuid: Uuid) -> StatisticsStream {
        let mut statistic_stream = self.clone();
        statistic_stream.subscribers.retain(|&id, _| uuid == id);
        statistic_stream
    }

    pub fn query_by_uuid(&self, uuid: Uuid) -> StatisticsStream {
        if uuid == self.publisher.id {
            self.get_publisher()
        } else {
            self.get_subscriber(uuid)
        }
    }
}

pub struct StatisticsCalculate {
    stream: Arc<Mutex<StatisticsStream>>,
    exit: Receiver<()>,
}

impl StatisticsCalculate {
    pub fn new(stream: Arc<Mutex<StatisticsStream>>, exit: Receiver<()>) -> Self {
        Self { stream, exit }
    }

    async fn calculate(&mut self) {
        let stream_statistics_clone = &mut self.stream.lock().await;

        stream_statistics_clone.publisher.video.bitrate =
            stream_statistics_clone.publisher.video.recv_bytes * 8 / 5000;
        stream_statistics_clone.publisher.video.recv_bytes = 0;

        stream_statistics_clone.publisher.video.frame_rate =
            stream_statistics_clone.publisher.video.recv_frame_count / 5;
        stream_statistics_clone.publisher.video.recv_frame_count = 0;

        stream_statistics_clone.publisher.audio.bitrate =
            stream_statistics_clone.publisher.audio.recv_bytes * 8 / 5000;
        stream_statistics_clone.publisher.audio.recv_bytes = 0;

        stream_statistics_clone.publisher.recv_bitrate =
            stream_statistics_clone.publisher.recv_bytes * 8 / 5000;
        stream_statistics_clone.publisher.recv_bytes = 0;

        for subscriber in stream_statistics_clone.subscribers.values_mut() {
            subscriber.send_bitrate = subscriber.send_bytes * 8 / 5000;
            subscriber.send_bytes = 0;
        }
    }
    pub async fn start(&mut self) {
        let mut interval = time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
               _ = interval.tick() => {
                self.calculate().await;
               },
               _ = self.exit.recv() => {
                    info!("avstatistics_shutting_down");
                    return
               },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;
    use std::sync::Arc;
    use tokio::sync::{Mutex, broadcast};

    // ========== Helper Functions ==========

    fn make_rtsp_identifier() -> StreamIdentifier {
        StreamIdentifier::Rtsp {
            stream_path: "/live/test".to_string(),
        }
    }

    fn make_subscriber(uuid: Uuid) -> StatisticSubscriber {
        StatisticSubscriber {
            id: uuid,
            start_time: Local::now(),
            remote_address: "192.168.1.100:5000".to_string(),
            sub_type: SubscribeType::RtspPull,
            send_bytes: 0,
            send_bitrate: 0,
            total_send_bytes: 0,
        }
    }

    // ========== VideoInfo Tests ==========

    #[test]
    fn test_video_info_default() {
        let video = VideoInfo::default();
        assert_eq!(video.width, 0);
        assert_eq!(video.height, 0);
        assert_eq!(video.recv_bytes, 0);
        assert_eq!(video.bitrate, 0);
        assert_eq!(video.recv_frame_count, 0);
        assert_eq!(video.frame_rate, 0);
        assert_eq!(video.recv_frame_count_for_gop, 0);
        assert_eq!(video.gop, 0);
    }

    #[test]
    fn test_video_info_clone() {
        let video = VideoInfo {
            width: 1920,
            height: 1080,
            bitrate: 4000,
            frame_rate: 30,
            ..Default::default()
        };

        let cloned = video.clone();
        assert_eq!(cloned.width, 1920);
        assert_eq!(cloned.height, 1080);
        assert_eq!(cloned.bitrate, 4000);
        assert_eq!(cloned.frame_rate, 30);
    }

    #[test]
    fn test_video_info_debug() {
        let video = VideoInfo::default();
        let debug_str = format!("{:?}", video);
        assert!(debug_str.contains("VideoInfo"));
    }

    #[test]
    fn test_video_info_serialize_skips_internal_fields() {
        let video = VideoInfo {
            recv_bytes: 999,
            recv_frame_count: 100,
            recv_frame_count_for_gop: 50,
            bitrate: 4000,
            frame_rate: 30,
            ..Default::default()
        };

        let json = serde_json::to_string(&video).unwrap();
        // recv_bytes, recv_frame_count, recv_frame_count_for_gop are skip_serializing
        assert!(!json.contains("recv_bytes"));
        assert!(!json.contains("recv_frame_count"));
        assert!(!json.contains("recv_frame_count_for_gop"));
        // bitrate is renamed in serialization
        assert!(json.contains("bitrate(kbits/s)"));
    }

    // ========== AudioInfo Tests ==========

    #[test]
    fn test_audio_info_default() {
        let audio = AudioInfo::default();
        assert_eq!(audio.samplerate, 0);
        assert_eq!(audio.channels, 0);
        assert_eq!(audio.recv_bytes, 0);
        assert_eq!(audio.bitrate, 0);
    }

    #[test]
    fn test_audio_info_clone() {
        let audio = AudioInfo {
            samplerate: 44100,
            channels: 2,
            bitrate: 128,
            ..Default::default()
        };

        let cloned = audio.clone();
        assert_eq!(cloned.samplerate, 44100);
        assert_eq!(cloned.channels, 2);
        assert_eq!(cloned.bitrate, 128);
    }

    #[test]
    fn test_audio_info_debug() {
        let audio = AudioInfo::default();
        let debug_str = format!("{:?}", audio);
        assert!(debug_str.contains("AudioInfo"));
    }

    #[test]
    fn test_audio_info_serialize_skips_internal_fields() {
        let audio = AudioInfo {
            recv_bytes: 500,
            bitrate: 128,
            ..Default::default()
        };

        let json = serde_json::to_string(&audio).unwrap();
        assert!(!json.contains("recv_bytes"));
        assert!(json.contains("bitrate(kbits/s)"));
    }

    // ========== StatisticPublisher Tests ==========

    #[test]
    fn test_statistic_publisher_new_sets_identifier() {
        let identifier = make_rtsp_identifier();
        let publisher = StatisticPublisher::new(identifier.clone());

        // Verify the identifier is set correctly
        let json = serde_json::to_string(&publisher).unwrap();
        assert!(json.contains("rtsp"));
        assert!(json.contains("/live/test"));
    }

    #[test]
    fn test_statistic_publisher_new_defaults() {
        let publisher = StatisticPublisher::new(StreamIdentifier::Unknown);

        assert_eq!(publisher.id, Uuid::default());
        assert_eq!(publisher.remote_address, "");
        assert_eq!(publisher.recv_bytes, 0);
        assert_eq!(publisher.recv_bitrate, 0);
        assert_eq!(publisher.video.width, 0);
        assert_eq!(publisher.audio.samplerate, 0);
    }

    #[test]
    fn test_statistic_publisher_debug() {
        let publisher = StatisticPublisher::new(StreamIdentifier::Unknown);
        let debug_str = format!("{:?}", publisher);
        assert!(debug_str.contains("StatisticPublisher"));
    }

    #[test]
    fn test_statistic_publisher_clone() {
        let mut publisher = StatisticPublisher::new(make_rtsp_identifier());
        publisher.remote_address = "10.0.0.1:8080".to_string();
        publisher.recv_bytes = 1024;

        let cloned = publisher.clone();
        assert_eq!(cloned.remote_address, "10.0.0.1:8080");
        assert_eq!(cloned.recv_bytes, 1024);
    }

    // ========== StatisticsStream Tests ==========

    #[test]
    fn test_statistics_stream_new_creates_with_identifier() {
        let identifier = make_rtsp_identifier();
        let stream = StatisticsStream::new(identifier);

        assert!(stream.subscribers.is_empty());
        assert_eq!(stream.subscriber_count, 0);
        assert_eq!(stream.total_recv_bytes, 0);
        assert_eq!(stream.total_send_bytes, 0);
    }

    #[test]
    fn test_statistics_stream_default() {
        let stream = StatisticsStream::default();

        assert!(stream.subscribers.is_empty());
        assert_eq!(stream.subscriber_count, 0);
        assert_eq!(stream.total_recv_bytes, 0);
        assert_eq!(stream.total_send_bytes, 0);
    }

    #[test]
    fn test_statistics_stream_get_publisher_clears_subscribers() {
        let mut stream = StatisticsStream::new(make_rtsp_identifier());

        // Add a subscriber
        let sub_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        stream
            .subscribers
            .insert(sub_uuid, make_subscriber(sub_uuid));
        stream.subscriber_count = 1;

        let publisher_view = stream.get_publisher();
        assert!(publisher_view.subscribers.is_empty());
        // The original stream still has the subscriber
        assert_eq!(stream.subscribers.len(), 1);
    }

    #[test]
    fn test_statistics_stream_get_subscriber_retains_matching() {
        let mut stream = StatisticsStream::new(make_rtsp_identifier());

        let sub1_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        let sub2_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        stream
            .subscribers
            .insert(sub1_uuid, make_subscriber(sub1_uuid));
        stream
            .subscribers
            .insert(sub2_uuid, make_subscriber(sub2_uuid));
        stream.subscriber_count = 2;

        let sub_view = stream.get_subscriber(sub1_uuid);
        assert_eq!(sub_view.subscribers.len(), 1);
        assert!(sub_view.subscribers.contains_key(&sub1_uuid));
        assert!(!sub_view.subscribers.contains_key(&sub2_uuid));
    }

    #[test]
    fn test_statistics_stream_get_subscriber_no_match_returns_empty() {
        let mut stream = StatisticsStream::new(make_rtsp_identifier());

        let sub_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        stream
            .subscribers
            .insert(sub_uuid, make_subscriber(sub_uuid));

        let non_existent = Uuid::new(crate::hub::utils::RandomDigitCount::Six);
        let sub_view = stream.get_subscriber(non_existent);
        assert!(sub_view.subscribers.is_empty());
    }

    #[test]
    fn test_statistics_stream_query_by_uuid_matches_publisher() {
        let stream = StatisticsStream::new(make_rtsp_identifier());
        let pub_uuid = stream.publisher.id;

        let result = stream.query_by_uuid(pub_uuid);
        // Should return publisher view (no subscribers)
        assert!(result.subscribers.is_empty());
    }

    #[test]
    fn test_statistics_stream_query_by_uuid_matches_subscriber() {
        let mut stream = StatisticsStream::new(make_rtsp_identifier());

        let sub_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        stream
            .subscribers
            .insert(sub_uuid, make_subscriber(sub_uuid));
        stream.subscriber_count = 1;

        let result = stream.query_by_uuid(sub_uuid);
        assert_eq!(result.subscribers.len(), 1);
        assert!(result.subscribers.contains_key(&sub_uuid));
    }

    #[test]
    fn test_statistics_stream_query_by_uuid_no_match() {
        let stream = StatisticsStream::new(make_rtsp_identifier());

        let random_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Six);
        let result = stream.query_by_uuid(random_uuid);
        // Falls into get_subscriber path with no match
        assert!(result.subscribers.is_empty());
    }

    #[test]
    fn test_statistics_stream_clone() {
        let mut stream = StatisticsStream::new(make_rtsp_identifier());
        stream.total_recv_bytes = 5000;
        stream.total_send_bytes = 3000;

        let cloned = stream.clone();
        assert_eq!(cloned.total_recv_bytes, 5000);
        assert_eq!(cloned.total_send_bytes, 3000);
    }

    #[test]
    fn test_statistics_stream_debug() {
        let stream = StatisticsStream::new(make_rtsp_identifier());
        let debug_str = format!("{:?}", stream);
        assert!(debug_str.contains("StatisticsStream"));
    }

    // ========== StatisticsCalculate Tests ==========

    #[tokio::test]
    async fn test_statistics_calculate_new() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);

        let calc = StatisticsCalculate::new(stream.clone(), rx);
        // Verify it holds the same Arc reference
        let locked = calc.stream.lock().await;
        assert_eq!(locked.total_recv_bytes, 0);
    }

    #[tokio::test]
    async fn test_statistics_calculate_computes_video_bitrate() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        // Set up video recv_bytes: bitrate = recv_bytes * 8 / 5000
        {
            let mut locked = stream.lock().await;
            locked.publisher.video.recv_bytes = 625_000; // expect bitrate = 625000 * 8 / 5000 = 1000
        }

        calc.calculate().await;

        let locked = stream.lock().await;
        assert_eq!(locked.publisher.video.bitrate, 1000);
        assert_eq!(locked.publisher.video.recv_bytes, 0); // reset after calculate
    }

    #[tokio::test]
    async fn test_statistics_calculate_computes_video_frame_rate() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        // Set up frame count: frame_rate = recv_frame_count / 5
        {
            let mut locked = stream.lock().await;
            locked.publisher.video.recv_frame_count = 150; // expect frame_rate = 150 / 5 = 30
        }

        calc.calculate().await;

        let locked = stream.lock().await;
        assert_eq!(locked.publisher.video.frame_rate, 30);
        assert_eq!(locked.publisher.video.recv_frame_count, 0); // reset
    }

    #[tokio::test]
    async fn test_statistics_calculate_computes_audio_bitrate() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        // audio bitrate = recv_bytes * 8 / 5000
        {
            let mut locked = stream.lock().await;
            locked.publisher.audio.recv_bytes = 100_000; // expect bitrate = 100000 * 8 / 5000 = 160
        }

        calc.calculate().await;

        let locked = stream.lock().await;
        assert_eq!(locked.publisher.audio.bitrate, 160);
        assert_eq!(locked.publisher.audio.recv_bytes, 0);
    }

    #[tokio::test]
    async fn test_statistics_calculate_computes_publisher_recv_bitrate() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        // recv_bitrate = recv_bytes * 8 / 5000
        {
            let mut locked = stream.lock().await;
            locked.publisher.recv_bytes = 250_000; // expect recv_bitrate = 250000 * 8 / 5000 = 400
        }

        calc.calculate().await;

        let locked = stream.lock().await;
        assert_eq!(locked.publisher.recv_bitrate, 400);
        assert_eq!(locked.publisher.recv_bytes, 0);
    }

    #[tokio::test]
    async fn test_statistics_calculate_computes_subscriber_send_bitrate() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        let sub_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        {
            let mut locked = stream.lock().await;
            let mut sub = make_subscriber(sub_uuid);
            sub.send_bytes = 50_000; // expect send_bitrate = 50000 * 8 / 5000 = 80
            locked.subscribers.insert(sub_uuid, sub);
        }

        calc.calculate().await;

        let locked = stream.lock().await;
        let sub = locked.subscribers.get(&sub_uuid).unwrap();
        assert_eq!(sub.send_bitrate, 80);
        assert_eq!(sub.send_bytes, 0);
    }

    #[tokio::test]
    async fn test_statistics_calculate_multiple_subscribers() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        let sub1_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        let sub2_uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        {
            let mut locked = stream.lock().await;
            let mut sub1 = make_subscriber(sub1_uuid);
            sub1.send_bytes = 25_000; // expect 40
            let mut sub2 = make_subscriber(sub2_uuid);
            sub2.send_bytes = 75_000; // expect 120
            locked.subscribers.insert(sub1_uuid, sub1);
            locked.subscribers.insert(sub2_uuid, sub2);
        }

        calc.calculate().await;

        let locked = stream.lock().await;
        let sub1 = locked.subscribers.get(&sub1_uuid).unwrap();
        assert_eq!(sub1.send_bitrate, 40);
        assert_eq!(sub1.send_bytes, 0);

        let sub2 = locked.subscribers.get(&sub2_uuid).unwrap();
        assert_eq!(sub2.send_bitrate, 120);
        assert_eq!(sub2.send_bytes, 0);
    }

    #[tokio::test]
    async fn test_statistics_calculate_zero_values() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (_tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        // All values default to 0, calculate should produce 0 bitrates
        calc.calculate().await;

        let locked = stream.lock().await;
        assert_eq!(locked.publisher.video.bitrate, 0);
        assert_eq!(locked.publisher.video.frame_rate, 0);
        assert_eq!(locked.publisher.audio.bitrate, 0);
        assert_eq!(locked.publisher.recv_bitrate, 0);
    }

    #[tokio::test]
    async fn test_statistics_calculate_start_exits_on_signal() {
        let stream = Arc::new(Mutex::new(StatisticsStream::new(make_rtsp_identifier())));
        let (tx, rx) = broadcast::channel::<()>(1);
        let mut calc = StatisticsCalculate::new(stream.clone(), rx);

        let handle = tokio::spawn(async move {
            calc.start().await;
        });

        // Give a small delay to ensure start() is running
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Send exit signal
        tx.send(()).unwrap();

        // start() should return within a reasonable time
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "start() should exit after receiving signal");
        assert!(
            result.unwrap().is_ok(),
            "start() task should complete without panic"
        );
    }

    // ========== StatisticSubscriber Tests ==========

    #[test]
    fn test_statistic_subscriber_debug() {
        let uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        let sub = make_subscriber(uuid);
        let debug_str = format!("{:?}", sub);
        assert!(debug_str.contains("StatisticSubscriber"));
    }

    #[test]
    fn test_statistic_subscriber_clone() {
        let uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        let mut sub = make_subscriber(uuid);
        sub.send_bytes = 12345;
        sub.send_bitrate = 99;

        let cloned = sub.clone();
        assert_eq!(cloned.id, uuid);
        assert_eq!(cloned.send_bytes, 12345);
        assert_eq!(cloned.send_bitrate, 99);
        assert_eq!(cloned.remote_address, "192.168.1.100:5000");
    }

    #[test]
    fn test_statistic_subscriber_serialize() {
        let uuid = Uuid::new(crate::hub::utils::RandomDigitCount::Four);
        let mut sub = make_subscriber(uuid);
        sub.send_bytes = 999;
        sub.send_bitrate = 50;
        sub.total_send_bytes = 2000;

        let json = serde_json::to_string(&sub).unwrap();
        // send_bytes is skip_serializing
        assert!(!json.contains("\"send_bytes\""));
        // send_bitrate is renamed
        assert!(json.contains("send_bitrate(kbits/s)"));
        // total_send_bytes is renamed
        assert!(json.contains("total_send_bytes(kbits/s)"));
        // Contains the remote address
        assert!(json.contains("192.168.1.100:5000"));
    }
}
