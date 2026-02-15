---
name: rtsp-rtp-streaming
description: |
  RTSP/RTP H.264 codec specialist for streaming-lib project. Handles NAL unit packing, fragmentation, SPS/PPS extraction, and RTP packet management.
  Triggers on: "RTP packet", "H.264", "NAL unit", "SPS/PPS", "RTSP streaming", "fragmentation", "STAP", "FU-A", "H264Packer".
version: 1.0.0
---

# RTSP/RTP H.264 Streaming

Implement H.264 video codec support for RTP streaming in the streaming-lib project. Handle NAL unit packing, fragmentation strategies, and RTP packet construction following RFC 3984.

## H.264 NAL Unit Structure

Every H.264 stream consists of NAL (Network Abstraction Layer) units with a 1-byte header:

```
+----+-----+
| F  |NRI | Type (5 bits) |
+----+-----+
 1b   2b        5 bits (0-31)
```

**Common NAL Types:**
- Type 1: Coded slice (video data)
- Type 5: IDR slice (keyframe, always starts stream)
- Type 6: SEI (supplemental enhancement information)
- Type 7: SPS (sequence parameter set - dimensions, framerate)
- Type 8: PPS (picture parameter set - encoding parameters)

```rust
pub enum NalType {
    Unspecified = 0,
    CodedSlice = 1,
    DataPartitionA = 2,
    DataPartitionB = 3,
    DataPartitionC = 4,
    IdrSlice = 5,
    SEI = 6,
    SPS = 7,
    PPS = 8,
    AccessUnitDelimiter = 9,
    EndOfSequence = 10,
    EndOfStream = 11,
    FillerData = 12,
    SpsExt = 13,
    AuxiliaryCodedSlice = 19,
}
```

## Packing Strategy

H.264 over RTP uses three packing modes based on NAL unit size:

### 1. Single NAL Unit Mode (Small NAL)

For NAL units ≤ MTU (maximum transmission unit, typically 1200-1500 bytes):

```rust
impl RtpH264Packer {
    fn pack_single(&mut self, nalu: &[u8]) -> Vec<RtpPacket> {
        // NAL unit fits in one RTP packet
        let mut packet = RtpPacket::new();
        packet.set_header(self.sequence_number, self.timestamp);
        packet.payload = nalu.to_vec();

        self.sequence_number += 1;
        vec![packet]
    }
}
```

### 2. STAP (Single-Time Aggregation Packet)

Multiple small NAL units with the same timestamp in one RTP packet:

```
+------+
|STAP Header (1 byte)|
+------+
|Size (2 bytes) | NAL Unit 1 |
+------+
|Size (2 bytes) | NAL Unit 2 |
+------+
```

```rust
fn pack_stap_a(&mut self, nalus: &[&[u8]]) -> Vec<RtpPacket> {
    let mut payload = vec![0x60 | 0x08];  // STAP-A header

    for nalu in nalus {
        let size = (nalu.len() as u16).to_be_bytes();
        payload.extend_from_slice(&size);
        payload.extend_from_slice(nalu);
    }

    let mut packet = RtpPacket::new();
    packet.set_header(self.sequence_number, self.timestamp);
    packet.payload = payload;

    self.sequence_number += 1;
    vec![packet]
}
```

### 3. FU-A (Fragmentation Unit Type A)

Large NAL units fragmented across multiple RTP packets:

```
FU Header:
+----+-----+-----+
| F  |NRI  | FUT |  (FUT = 28 for FU-A)
+----+-----+-----+

FU Indicator:
+----+-----+
| S  | E  | R |NAL Type| (S=start, E=end, R=reserved)
+----+-----+-----+

[Payload ...]
```

```rust
fn pack_fu_a(&mut self, nalu: &[u8]) -> Vec<RtpPacket> {
    const MTU: usize = 1200;
    const HEADER_SIZE: usize = 2;  // FU header + indicator
    const MAX_PAYLOAD: usize = MTU - HEADER_SIZE - 12;  // 12 = RTP header

    let mut packets = Vec::new();
    let original_nal_type = nalu[0] & 0x1F;
    let payload = &nalu[1..];  // Skip original NAL header

    let mut offset = 0;
    let mut is_start = true;

    while offset < payload.len() {
        let chunk_size = std::cmp::min(MAX_PAYLOAD, payload.len() - offset);
        let is_end = offset + chunk_size >= payload.len();

        // FU Header: 28 (FU-A type)
        let fu_header = 0x1C;  // 00011100

        // FU Indicator: S|E|R|NAL_Type
        let fu_indicator = if is_start { 0x80 } else { 0x00 }  // S bit
                         | if is_end { 0x40 } else { 0x00 }    // E bit
                         | original_nal_type;

        let mut packet_payload = vec![fu_header, fu_indicator];
        packet_payload.extend_from_slice(&payload[offset..offset + chunk_size]);

        let mut packet = RtpPacket::new();
        packet.marker = is_end;  // Mark final fragment
        packet.set_header(self.sequence_number, self.timestamp);
        packet.payload = packet_payload;

        packets.push(packet);
        self.sequence_number += 1;
        offset += chunk_size;
        is_start = false;
    }

    packets
}
```

## SPS/PPS Extraction and Handling

SPS and PPS units must be sent before each IDR (keyframe):

```rust
impl RtpH264Packer {
    pub async fn pack_nalu(&mut self, nalu: &[u8]) -> Result<(), Error> {
        let nal_type = nalu[0] & 0x1F;

        match nal_type {
            7 => {
                // SPS - store for later inclusion
                self.sps_buffer = Some(nalu.to_vec());
            }
            8 => {
                // PPS - store for later inclusion
                self.pps_buffer = Some(nalu.to_vec());
            }
            5 => {
                // IDR Slice - keyframe, prepend SPS/PPS
                if let (Some(sps), Some(pps)) = (&self.sps_buffer, &self.pps_buffer) {
                    let packets = self.pack_stap_a(&[sps, pps, nalu])?;
                    for packet in packets {
                        self.on_packet_handler(packet).await?;
                    }
                } else {
                    // Send IDR alone if SPS/PPS not available
                    self.pack_and_send(nalu).await?;
                }
            }
            1 | 2 | 3 | 4 => {
                // Regular slice - send as-is
                self.pack_and_send(nalu).await?;
            }
            _ => {
                // Other NAL types - pass through
                self.pack_and_send(nalu).await?;
            }
        }

        Ok(())
    }
}
```

## Unpacking/Reassembly

Receiving side must handle all three packing modes:

```rust
impl RtpH264UnPacker {
    pub async fn unpack(&mut self, packet: &RtpPacket) -> Result<Option<Frame>, Error> {
        let payload = &packet.payload;
        if payload.is_empty() {
            return Ok(None);
        }

        let nal_header = payload[0];
        let nal_type = nal_header & 0x1F;

        match nal_type {
            0..=23 => {
                // Single NAL unit (or in STAP)
                self.unpack_single(payload).await?
            }
            24 => {
                // STAP-A
                self.unpack_stap(payload).await?
            }
            25 => {
                // STAP-B
                self.unpack_stap_b(payload).await?
            }
            26 => {
                // MTAP16
                self.unpack_mtap16(payload).await?
            }
            27 => {
                // MTAP24
                self.unpack_mtap24(payload).await?
            }
            28 => {
                // FU-A
                self.unpack_fu_a(payload, packet.marker).await?
            }
            29 => {
                // FU-B
                self.unpack_fu_b(payload, packet.marker).await?
            }
            _ => {
                tracing::warn!("Unknown NAL type: {}", nal_type);
            }
        }

        // Check if complete frame is available
        if packet.marker {
            if let Some(frame) = self.complete_frame()? {
                return Ok(Some(frame));
            }
        }

        Ok(None)
    }
}
```

## Testing Patterns

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_sps() -> Vec<u8> {
        // Minimal valid SPS NAL unit
        vec![0x67, 0x42, 0x00, 0x0A, 0xFF, 0xE1, 0x00, 0x16, 0x68, 0xCE, 0x3C, 0x80]
    }

    fn create_test_pps() -> Vec<u8> {
        // Minimal valid PPS NAL unit
        vec![0x68, 0xCE, 0x3C, 0x80]
    }

    fn create_large_nalu(size: usize) -> Vec<u8> {
        let mut nalu = vec![0x65];  // IDR slice type
        nalu.resize(size, 0xFF);
        nalu
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_single_small_nalu() {
        let mut packer = RtpH264Packer::new(1200);
        let small_nalu = vec![0x65, 0x01, 0x02, 0x03];  // Small IDR slice

        let packets = packer.pack_single(&small_nalu);

        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].payload, small_nalu);
        assert_eq!(packets[0].marker, true);
    }

    #[tokio::test]
    async fn test_rtp_h264_packer_pack_fu_a_large_nalu() {
        let mut packer = RtpH264Packer::new(1200);
        let large_nalu = create_large_nalu(4000);

        let packets = packer.pack_fu_a(&large_nalu);

        // Should be fragmented
        assert!(packets.len() > 1);
        // First packet should have S=1
        assert_eq!(packets[0].payload[1] & 0x80, 0x80);
        // Last packet should have E=1 and marker=1
        assert_eq!(packets[packets.len()-1].payload[1] & 0x40, 0x40);
        assert!(packets[packets.len()-1].marker);
    }

    #[tokio::test]
    async fn test_rtp_h264_unpacker_unpack_fu_a() {
        let mut unpacker = RtpH264UnPacker::new();
        let mut packer = RtpH264Packer::new(1200);

        let large_nalu = create_large_nalu(4000);
        let packets = packer.pack_fu_a(&large_nalu);

        // Unpack all fragments
        for (i, packet) in packets.iter().enumerate() {
            let frame = unpacker.unpack(packet).await.unwrap();

            if i == packets.len() - 1 {
                // Last packet should complete the frame
                assert!(frame.is_some());
            } else {
                // Intermediate packets shouldn't
                assert!(frame.is_none());
            }
        }
    }
}
```

## Reference

For detailed codec patterns and packetization examples, see `references/codec-patterns.md`.
