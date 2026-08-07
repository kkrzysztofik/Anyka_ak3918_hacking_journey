---
name: rtsp-rtp-streaming
description: Use when implementing or debugging H.264 RTP packing/unpacking in streaming-lib (RtpH264Packer/RtpH264UnPacker, NAL units, SPS/PPS, FU-A/STAP, RTSP, frame callbacks, TPacker/TUnPacker traits).
version: 2.0.0
---

# RTSP/RTP H.264 Streaming (streaming-lib)

Implement/debug H.264 RTP packetization in `cross-compile/streaming-lib` following RFC 3984/6184. The real packer is **callback-driven and async** — verify against `src/protocol/rtsp/rtp/rtp_h264.rs`; older drafts showing `pack_single() -> Vec<RtpPacket>` are wrong.

## NAL Unit Header

Every H.264 NAL unit starts with a 1-byte header:

```
+----+-----+---------+
| F  | NRI |  Type   |   (1b | 2b | 5b)
+----+-----+---------+
```

Types (`src/protocol/rtsp/rtp/define.rs`): 1 CodedSlice, 5 IDR, 6 SEI, 7 SPS, 8 PPS, 9 AUD, 28 FU-A, 29 FU-B, 24 STAP-A, 25 STAP-B.

## RtpH264Packer — Real API

Constructor takes 5 args (payload type, ssrc, initial seq, MTU, and an IO handle); output flows through callbacks, it does **not** return packet vecs:

```rust
use crate::protocol::rtsp::rtp::rtp_h264::RtpH264Packer;
use crate::io::TNetIO;
use std::sync::Arc;
use tokio::sync::Mutex;

let packer = RtpH264Packer::new(
    payload_type: u8,                          // e.g. 96
    ssrc: u32,                                 // e.g. 12345
    init_seq: u16,                             // e.g. 0
    mtu: usize,                                // e.g. 1500
    io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>,  // IO handle
);
```

Key methods (all return `Result<(), PackerError>`):

```rust
impl RtpH264Packer {
    pub fn new(payload_type: u8, ssrc: u32, init_seq: u16, mtu: usize,
               io: Arc<Mutex<Box<dyn TNetIO + Send + Sync>>>) -> Self;
    pub async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError>;
}
impl TPacker for RtpH264Packer {
    async fn pack(&mut self, nalus: &mut BytesMut, timestamp: u32) -> Result<(), PackerError>;
    fn on_packet_handler(&mut self, f: OnRtpPacketFn);
}
impl TVideoPacker for RtpH264Packer {
    async fn pack_nalu(&mut self, nalu: BytesMut) -> Result<(), PackerError>;
}
```

`pack` splits an Annex-B byte stream into NAL units and sends each; `pack_nalu` packs one NAL unit. Internally it chooses single-NAL (fits in MTU) vs FU-A (larger), with `marker` set on the last fragment of a VCL access unit. Callbacks receive `(io, RtpPacket)`.

## RtpH264UnPacker — Real API

Unpacks from a `BytesReader` and emits complete Annex-B frames via a callback:

```rust
use crate::protocol::rtsp::rtp::rtp_h264::RtpH264UnPacker;
use crate::io::bytes_reader::BytesReader;

let mut unpacker = RtpH264UnPacker::new();
unpacker.on_frame_handler(Box::new(|frame: FrameData| {
    // FrameData::Video { timestamp, data: BytesMut /* Annex-B start code + NAL */ }
    Ok(())
}));
unpacker.unpack(&mut BytesReader::new(payload_bytes)).await?;
```

```rust
impl TUnPacker for RtpH264UnPacker {
    async fn unpack(&mut self, reader: &mut BytesReader) -> Result<(), UnPackerError>;
    fn on_frame_handler(&mut self, f: OnFrameFn);
}
```

Handles single NAL, STAP-A/B, MTAP, FU-A/FU-B (reassembling fragments into Annex-B with `0x00 0x00 0x01` start codes). **Marker bit does not gate frame emission** — FU reassembly is driven by the FU header S/E bits, and emitted frames use the packet timestamp.

## NAL Type Constants

`NalType` from `rtp_h264.rs` via `NalType::from_header(byte)`: `Single`, `Stap`, `Mtap`, `Fu`, `Unknown`. FU indicators: `FU_START = 0x80`, `FU_END = 0x40`; `is_fu_start`/`is_fu_end` helpers in `rtp/utils.rs`.

## Testing Patterns

Use the real constructor with a mock IO and an `on_packet_handler` that records packets:

```rust
#[tokio::test]
async fn test_packer_fu_a_marker() {
    let mock_io = Arc::new(Mutex::new(Box::new(MockIo::new()) as Box<dyn TNetIO + Send + Sync>));
    let mut packer = RtpH264Packer::new(96, 12345, 0, 1500, mock_io);

    let markers = Arc::new(std::sync::Mutex::new(Vec::<u8>::new()));
    let markers_clone = markers.clone();
    packer.on_packet_handler(Box::new(move |_io, packet| {
        markers_clone.lock().unwrap().push(packet.header.marker);
        Box::pin(async move { Ok(()) })
    }));

    let large_nalu = /* IDR slice > MTU */;
    packer.pack_nalu(large_nalu).await.unwrap();
    // first fragment header is FU-A (0x1C | type), FU header S bit set on first, E bit + marker on last
}
```

Real test data: `src/codec/test_fixtures.rs` (SPS_BASELINE_720P, PPS_BASELINE) and `codec::sps::SpsParser` for SPS/PPS parsing. Run with `$CARGO test --target x86_64-unknown-linux-gnu` after `source ./setenv.sh` (see `anyka-embedded-build` skill).

## SPS/PPS Handling

SPS/PPS are parameter sets (types 7/8) that must precede IDR frames. `streaming-lib` parses them via `codec::sps::SpsParser` / `codec::pps` (used by the FLV/HTTP-FLV container for avcC). For RTP, parameter sets are transmitted as STAP-A aggregation when combined with the IDR.
