use super::errors::PackerError;
use super::errors::UnPackerError;
use super::utils::OnFrameFn;
use super::utils::OnRtpPacketFn;
use super::utils::OnRtpPacketFn2;
use super::utils::TPacker;

use super::RtpHeader;
use super::RtpPacket;
use super::utils::TRtpReceiverForRtcp;
use super::utils::TUnPacker;
use super::utils::Unmarshal;
use async_trait::async_trait;
use byteorder::BigEndian;
use bytes::{BufMut, BytesMut};

use crate::bytesio::TNetIO;
use crate::bytesio::bits_writer::BitsWriter;
use crate::bytesio::bytes_errors::{BytesWriteError, BytesWriteErrorValue};
use crate::bytesio::bytes_reader::BytesReader;
use crate::bytesio::bytes_writer::BytesWriter;
use crate::streamhub::define::FrameData;
use std::sync::Arc;
use tokio::sync::Mutex;

// pub type OnPacketFn = fn(BytesMut) -> Result<(), PackerError>;

pub struct RtpAacPacker {
    header: RtpHeader,
    on_packet_handler: Option<OnRtpPacketFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
}

impl RtpAacPacker {
    pub fn new(
        payload_type: u8,
        ssrc: u32,
        init_seq: u16,
        io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,
    ) -> Self {
        RtpAacPacker {
            header: RtpHeader {
                payload_type,
                seq_number: init_seq,
                ssrc,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            io,
            on_packet_handler: None,
            on_packet_for_rtcp_handler: None,
        }
    }
}
#[async_trait]
impl TPacker for RtpAacPacker {
    async fn pack(&mut self, data: &mut BytesMut, timestamp: u32) -> Result<(), PackerError> {
        self.header.timestamp = timestamp;

        let data_len = data.len();
        let mut packet = RtpPacket {
            header: self.header.clone(),
            ..Default::default()
        };

        // Use BitsWriter for proper AU-headers-length encoding with bit-level precision
        let bytes_writer = BytesWriter::new();
        let mut bits_writer = BitsWriter::new(bytes_writer);

        // AU-headers-length: 16 bits containing the number of bits in AU header section
        // Each AU header is 16 bits (13 bits size + 3 bits index)
        let au_headers_length_bits = 16u16; // Single AU header
        bits_writer
            .write_n_bits(au_headers_length_bits as u64, 16)
            .map_err(|_| {
                let err = BytesWriteError {
                    value: BytesWriteErrorValue::Timeout,
                };
                PackerError::from(err)
            })?;

        // AU header: 13 bits size + 3 bits index
        bits_writer
            .write_n_bits((data_len as u64) << 3, 13)
            .map_err(|_| {
                let err = BytesWriteError {
                    value: BytesWriteErrorValue::Timeout,
                };
                PackerError::from(err)
            })?;
        bits_writer
            .write_n_bits(0, 3) // index = 0
            .map_err(|_| {
                let err = BytesWriteError {
                    value: BytesWriteErrorValue::Timeout,
                };
                PackerError::from(err)
            })?;

        // Flush any remaining bits (padding) and get the bytes
        bits_writer.bits_alignment_8().map_err(|_| {
            let err = BytesWriteError {
                value: BytesWriteErrorValue::Timeout,
            };
            PackerError::from(err)
        })?;
        let au_headers_data = bits_writer.get_current_bytes();

        packet.payload.put(au_headers_data);
        packet.payload.put(data);

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(packet.clone()).await;
        }

        if let Some(f) = &self.on_packet_handler {
            f(self.io.clone(), packet).await?;
        }

        self.header.seq_number = self.header.seq_number.wrapping_add(1);

        Ok(())
    }

    fn on_packet_handler(&mut self, f: OnRtpPacketFn) {
        self.on_packet_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpAacPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[derive(Default)]
pub struct RtpAacUnPacker {
    on_frame_handler: Option<OnFrameFn>,
    on_packet_for_rtcp_handler: Option<OnRtpPacketFn2>,
}

// +---------+-----------+-----------+---------------+
// | RTP     | AU Header | Auxiliary | Access Unit   |
// | Header  | Section   | Section   | Data Section  |
// +---------+-----------+-----------+---------------+
// 	<----------RTP Packet Payload----------->
//
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+
// |AU-headers-length|AU-header|AU-header|      |AU-header|padding|
// |                 |   (1)   |   (2)   |      |   (n)   | bits  |
// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+- .. -+-+-+-+-+-+-+-+-+-+

// Au-headers-length 2 bytes

impl RtpAacUnPacker {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[async_trait]
impl TUnPacker for RtpAacUnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError> {
        let rtp_packet = RtpPacket::unmarshal(reader)?;

        if let Some(f) = &self.on_packet_for_rtcp_handler {
            f(rtp_packet.clone()).await;
        }

        let mut reader_payload = BytesReader::new(rtp_packet.payload);

        let au_headers_length = (reader_payload.read_u16::<BigEndian>()?).div_ceil(8);
        let au_header_length = 2;
        let aus_number = au_headers_length / au_header_length;

        let mut au_lengths = Vec::new();
        for _ in 0..aus_number {
            let au_size_high = reader_payload.read_u8()? as usize;
            let au_size_low = reader_payload.read_u8()? as usize;
            let au_length = (au_size_high << 5) | (au_size_low >> 3);
            au_lengths.push(au_length);
        }

        log::debug!(
            "send audio : au_headers_length :{}, aus_number: {}, au_lengths: {:?}",
            au_headers_length,
            aus_number,
            au_lengths,
        );

        for (i, item) in au_lengths.iter().enumerate() {
            let au_data = reader_payload.read_bytes(*item)?;
            if let Some(f) = &self.on_frame_handler {
                f(FrameData::Audio {
                    timestamp: rtp_packet.header.timestamp + i as u32 * 1024,
                    data: au_data,
                })?;
            }
        }

        Ok(())
    }
    fn on_frame_handler(&mut self, f: OnFrameFn) {
        self.on_frame_handler = Some(f);
    }
}

impl TRtpReceiverForRtcp for RtpAacUnPacker {
    fn on_packet_for_rtcp_handler(&mut self, f: OnRtpPacketFn2) {
        self.on_packet_for_rtcp_handler = Some(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytesio::bytes_reader::BytesReader;
    
    use crate::rtsp::rtp::utils::Marshal;
    use crate::streamhub::define::FrameData;
    use async_trait::async_trait;
    
    use bytes::BytesMut;
    use mockall::mock;
    use tokio::sync::Mutex;

    use crate::bytesio::bytesio_errors::BytesIOError;
    use crate::bytesio::{NetType, TNetIO};
    use bytes::Bytes;
    use std::time::Duration;

    mock! {
        NetIO {}

        #[async_trait]
        impl TNetIO for NetIO {
            async fn write(&mut self, bytes: Bytes) -> Result<(), BytesIOError>;
            async fn read(&mut self) -> Result<BytesMut, BytesIOError>;
            async fn read_timeout(&mut self, duration: Duration) -> Result<BytesMut, BytesIOError>;
            fn get_net_type(&self) -> NetType;
        }
    }

    fn create_test_aac_data() -> BytesMut {
        // Create simple AAC audio data
        let mut data = BytesMut::new();
        data.extend_from_slice(&[0x12, 0x10, 0x56, 0xe5, 0x00, 0x00, 0x00, 0x00]);
        data
    }

    #[tokio::test]
    async fn test_rtp_aac_packer_new() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let packer = RtpAacPacker::new(97, 12345, 0, mock_io);
        assert_eq!(packer.header.payload_type, 97);
        assert_eq!(packer.header.ssrc, 12345);
        assert_eq!(packer.header.seq_number, 0);
        assert_eq!(packer.header.marker, 1);
    }

    #[tokio::test]
    async fn test_rtp_aac_packer_pack() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpAacPacker::new(97, 12345, 0, mock_io);

        let packet_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let received_timestamp = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let packet_count_clone = packet_count.clone();
        let timestamp_clone = received_timestamp.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            *packet_count_clone.lock().unwrap() += 1;
            *timestamp_clone.lock().unwrap() = packet.header.timestamp;
            assert_eq!(packet.header.payload_type, 97);
            assert_eq!(packet.header.marker, 1);
            // Verify AU header structure
            assert!(packet.payload.len() >= 4); // AU-headers-length (2) + AU header (2)
            Box::pin(async move { Ok(()) })
        }));

        let mut aac_data = create_test_aac_data();
        let timestamp = 1000;
        let result = packer.pack(&mut aac_data, timestamp).await;

        assert!(result.is_ok());
        assert_eq!(*packet_count.lock().unwrap(), 1);
        assert_eq!(*received_timestamp.lock().unwrap(), timestamp);
    }

    #[tokio::test]
    async fn test_rtp_aac_packer_sequence_number_increment() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpAacPacker::new(97, 12345, 100, mock_io);

        let seq_numbers = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let seq_numbers_clone = seq_numbers.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            seq_numbers_clone
                .lock()
                .unwrap()
                .push(packet.header.seq_number);
            Box::pin(async move { Ok(()) })
        }));

        let mut aac_data1 = create_test_aac_data();
        let mut aac_data2 = create_test_aac_data();

        let _ = packer.pack(&mut aac_data1, 1000).await;
        let _ = packer.pack(&mut aac_data2, 2000).await;

        let seqs = seq_numbers.lock().unwrap();
        assert_eq!(seqs.len(), 2);
        assert_eq!(seqs[0], 100);
        assert_eq!(seqs[1], 101);
    }

    #[tokio::test]
    async fn test_rtp_aac_packer_au_header_encoding() {
        let mock_io = Arc::new(Mutex::new(
            Box::new(MockNetIO::new()) as Box<dyn TNetIO + Send + Sync>
        ));
        let mut packer = RtpAacPacker::new(97, 12345, 0, mock_io);

        let received_payload = std::sync::Arc::new(std::sync::Mutex::new(None::<BytesMut>));
        let received_payload_clone = received_payload.clone();
        packer.on_packet_handler(Box::new(move |_io, packet| {
            *received_payload_clone.lock().unwrap() = Some(packet.payload.clone());
            Box::pin(async move { Ok(()) })
        }));

        let mut aac_data = create_test_aac_data();
        let data_len = aac_data.len();
        let _ = packer.pack(&mut aac_data, 1000).await;

        let payload_opt = received_payload.lock().unwrap();
        assert!(payload_opt.is_some());
        let payload = payload_opt.as_ref().unwrap();

        // Verify AU-headers-length (should be 16 bits = 2 bytes for one AU)
        let au_headers_length = ((payload[0] as u16) << 8) | (payload[1] as u16);
        assert_eq!(au_headers_length, 16); // 2 bytes * 8 bits = 16 bits

        // Verify AU header encoding
        let au_size_high = payload[2];
        let au_size_low = payload[3];
        let decoded_size = ((au_size_high as usize) << 5) | ((au_size_low as usize) >> 3);
        assert_eq!(decoded_size as usize, data_len);
    }

    #[tokio::test]
    async fn test_rtp_aac_unpacker_au_header_rfc_example() {
        let mut unpacker = RtpAacUnPacker::new();
        let frame_sizes = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sizes_clone = frame_sizes.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            if let FrameData::Audio { data, .. } = frame {
                sizes_clone.lock().unwrap().push(data.len());
            }
            Ok(())
        }));

        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 97,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // AU-headers-length: 16 bits (one AU header)
        packet.payload.put_u16(16);
        // AU size 0x123 (291 bytes): high=0x09, low=0x18
        packet.payload.put_u8(0x09);
        packet.payload.put_u8(0x18);
        packet.payload.extend_from_slice(&vec![0u8; 0x123]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);
        unpacker.unpack(&mut reader).await.unwrap();

        let sizes = frame_sizes.lock().unwrap();
        assert_eq!(sizes.len(), 1);
        assert_eq!(sizes[0], 0x123);
    }

    #[tokio::test]
    async fn test_rtp_aac_unpacker_new() {
        let unpacker = RtpAacUnPacker::new();
        assert!(unpacker.on_frame_handler.is_none());
        assert!(unpacker.on_packet_for_rtcp_handler.is_none());
    }

    #[tokio::test]
    async fn test_rtp_aac_unpacker_unpack_single_au() {
        let mut unpacker = RtpAacUnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let received_timestamp = std::sync::Arc::new(std::sync::Mutex::new(0u32));
        let frame_count_clone = frame_count.clone();
        let timestamp_clone = received_timestamp.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            *frame_count_clone.lock().unwrap() += 1;
            if let FrameData::Audio { timestamp, .. } = frame {
                *timestamp_clone.lock().unwrap() = timestamp;
            }
            Ok(())
        }));

        // Create RTP packet with single AU
        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 97,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // AU-headers-length: 16 bits (2 bytes for one AU)
        packet.payload.put_u16(16);

        // AU header: size = 8 bytes
        let au_size = 8;
        packet.payload.put_u8((au_size >> 5) as u8);
        packet.payload.put_u8(((au_size & 0x1F) << 3) as u8);

        // AU data
        packet
            .payload
            .extend_from_slice(&[0x12, 0x10, 0x56, 0xe5, 0x00, 0x00, 0x00, 0x00]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 1);
        assert_eq!(*received_timestamp.lock().unwrap(), 1000);
    }

    #[tokio::test]
    async fn test_rtp_aac_unpacker_unpack_multiple_aus() {
        let mut unpacker = RtpAacUnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create RTP packet with multiple AUs
        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 97,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // AU-headers-length: 32 bits (2 bytes * 2 AUs = 4 bytes = 32 bits)
        packet.payload.put_u16(32);

        // First AU header: size = 4 bytes
        let au1_size = 4;
        packet.payload.put_u8((au1_size >> 5) as u8);
        packet.payload.put_u8(((au1_size & 0x1F) << 3) as u8);

        // Second AU header: size = 4 bytes
        let au2_size = 4;
        packet.payload.put_u8((au2_size >> 5) as u8);
        packet.payload.put_u8(((au2_size & 0x1F) << 3) as u8);

        // First AU data
        packet.payload.extend_from_slice(&[0x12, 0x10, 0x56, 0xe5]);

        // Second AU data
        packet.payload.extend_from_slice(&[0x12, 0x10, 0x56, 0xe5]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 2); // Should receive two frames
    }

    #[tokio::test]
    async fn test_rtp_aac_unpacker_timestamp_calculation() {
        let mut unpacker = RtpAacUnPacker::new();
        let timestamps = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let timestamps_clone = timestamps.clone();

        unpacker.on_frame_handler(Box::new(move |frame| {
            if let FrameData::Audio { timestamp, .. } = frame {
                timestamps_clone.lock().unwrap().push(timestamp);
            }
            Ok(())
        }));

        // Create RTP packet with multiple AUs per RFC 3640
        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 97,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // RFC 3640 layout:
        // 1. AU-headers-length (2 bytes): number of BITS in AU header section
        // 2. AU headers (each 2 bytes: 13-bit size + 3-bit index)
        // 3. AU data section (all AU payloads concatenated)

        // AU-headers-length: 32 bits = 4 bytes = 2 AU headers × 16 bits each
        packet.payload.put_u16(32);

        // First AU header: size=3 bytes (0x18 >> 3 = 3), index=0
        // Encoding: (size << 3) | index = (3 << 3) | 0 = 0x18, high byte = 0
        packet.payload.put_u8(0);
        packet.payload.put_u8(0x18);

        // Second AU header: size=3 bytes, index-delta=1
        packet.payload.put_u8(0);
        packet.payload.put_u8(0x18);

        // Now AU data section: 3 bytes for first AU + 3 bytes for second AU
        packet.payload.extend_from_slice(&[0x12, 0x10, 0x56]); // First AU data
        packet.payload.extend_from_slice(&[0x34, 0x56, 0x78]); // Second AU data

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let _ = unpacker.unpack(&mut reader).await;

        // Verify timestamps: base + (index * 1024)
        let ts = timestamps.lock().unwrap();
        assert_eq!(ts.len(), 2);
        assert_eq!(ts[0], 1000); // base timestamp
        assert_eq!(ts[1], 2024); // base + 1 * 1024
    }

    #[tokio::test]
    async fn test_rtp_aac_unpacker_au_headers_length_calculation() {
        let mut unpacker = RtpAacUnPacker::new();
        let frame_count = std::sync::Arc::new(std::sync::Mutex::new(0));
        let frame_count_clone = frame_count.clone();

        unpacker.on_frame_handler(Box::new(move |_frame| {
            *frame_count_clone.lock().unwrap() += 1;
            Ok(())
        }));

        // Create RTP packet with AU-headers-length that requires ceiling division
        let mut packet = RtpPacket {
            header: RtpHeader {
                payload_type: 97,
                seq_number: 1,
                timestamp: 1000,
                ssrc: 12345,
                version: 2,
                marker: 1,
                ..Default::default()
            },
            ..Default::default()
        };

        // AU-headers-length: 17 bits (should round up to 24 bits = 3 bytes)
        // This tests the div_ceil(8) calculation
        packet.payload.put_u16(17);

        // One AU header (2 bytes)
        packet.payload.put_u8(0);
        packet.payload.put_u8(0x18); // 3 bytes
        packet.payload.extend_from_slice(&[0x12, 0x10, 0x56]);

        let packet_bytes = packet.marshal().unwrap();
        let mut reader = BytesReader::new(packet_bytes);

        let result = unpacker.unpack(&mut reader).await;
        assert!(result.is_ok());
        assert_eq!(*frame_count.lock().unwrap(), 1);
    }
}
