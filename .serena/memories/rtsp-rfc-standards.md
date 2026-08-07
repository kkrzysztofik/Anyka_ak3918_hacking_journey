# RTSP + AAC + H264 RFC Standards Reference

This document captures which RFCs govern our RTSP/RTP/SDP implementation in
`cross-compile/streaming-lib/` and the ONVIF streaming bridge in
`cross-compile/onvif-rust/src/streaming/`.

---

## RFC Coverage Map

| RFC | Subject | Status | Primary Files |
| --- | --- | --- | --- |
| **RFC 2326** | RTSP/1.0 | ✅ Implemented | `src/protocol/rtsp/session/server_session.rs`, `src/protocol/rtsp/rtsp_transport.rs` |
| **RFC 3550** | RTP/RTCP | ✅ Implemented | `src/protocol/rtsp/rtp/rtp_header.rs`, `src/protocol/rtsp/rtp/rtcp/rtcp_context.rs` |
| **RFC 3551** | Static RTP payload types (PCMA/PCMU) | ✅ Defined | `src/protocol/rtsp/rtsp_codec.rs` |
| **RFC 4566** | SDP | ✅ Implemented | `src/protocol/rtsp/sdp/mod.rs`, `src/protocol/rtsp/sdp/fmtp.rs` |
| **RFC 6184** | H.264 over RTP | ✅ Implemented | `src/protocol/rtsp/rtp/rtp_h264.rs`, `src/protocol/rtsp/rtp/define.rs` |
| **RFC 3640** | MPEG-4 Generic (AAC) over RTP | ✅ Implemented | `src/protocol/rtsp/rtp/rtp_aac.rs` |
| **RFC 5905** | NTP timestamps (64-bit format) | ✅ Implemented | `src/protocol/rtsp/rtp/utils.rs` |

> **H.265 (RFC 7798) support was removed.** The repo is H.264 + AAC only; no
> `rtp_h265.rs` exists in `src/protocol/rtsp/rtp/`.

All `src/` paths are relative to `cross-compile/streaming-lib/`.

---

## RFC 6184 — H.264 over RTP

### NAL Type Constants (`src/protocol/rtsp/rtp/define.rs`)

| Constant | Value | RFC 6184 Type |
| --- | --- | --- |
| `STAP_A` | 24 | Single-Time Aggregation Packet A |
| `STAP_B` | 25 | STAP-B (parse/receive only) |
| `MTAP_16` | 26 | MTAP 16-bit DON (parse/receive only) |
| `MTAP_24` | 27 | MTAP 24-bit DON (parse/receive only) |
| `FU_A` | 28 | Fragmentation Unit A — **primary send path** |
| `FU_B` | 29 | FU-B (parse/receive only) |
| `FU_START` | 0x80 | FU header start bit |
| `FU_END` | 0x40 | FU header end bit |

### Packetization Rules We Must Follow

- Input: Annex-B bytestream; detect start codes via `find_start_code`
- Per-NALU routing (RFC 6184 §5):
  - `nalu.len() + 12 <= MTU` → **Single NAL Unit** (§5.6)
  - larger → **FU-A fragmentation** (§5.8)
- **Marker bit**: MUST be set on the last VCL NAL (types 1–5) of each access unit (§5.3)
- `VideoAccessUnitAssembler` in `server_session.rs` coalesces same-timestamp chunks before packetization to avoid duplicate marker bits at the same RTP timestamp
- FU-A indicator byte: `(first_byte & 0xE0) | FU_A`
- FU-A header byte: `(first_byte & 0x1F) | FU_START` or `FU_END`
- Sequence numbers: **must use `wrapping_add(1)`** (RFC 3550 §5.1)

### SDP fmtp for H264 (RFC 6184 §8.1)

```sdp
a=rtpmap:96 H264/90000
a=fmtp:96 packetization-mode=1; sprop-parameter-sets=<SPS_b64>,<PPS_b64>; profile-level-id=<3-byte hex>
```

- `packetization-mode=1` declares FU-A support
- `profile-level-id` derived from SPS bytes 1–3
- Generated in `cross-compile/onvif-rust/src/streaming/helpers.rs`

---

## RFC 3640 — AAC (MPEG-4 Generic) over RTP

### AAC-hbr Wire Layout (RFC 3640 §3.3)

```text
[AU-headers-length: 16 bits = 0x0010]
[AU header: 13-bit AU-size | 3-bit AU-index = 0]
[raw AAC frame bytes]
```

- Marker bit = 1 always (one complete frame per packet)
- AU-headers-length field = 16 (big-endian; counts bits of following AU-header data)
- Written with `BitsWriter` for bit-exact encoding (`src/protocol/rtsp/rtp/rtp_aac.rs`)

### Unpacker Rules

- `au_size = (byte[0] << 5) | (byte[1] >> 3)` — 13-bit size + 3-bit index
- Multiple AUs per packet MUST be supported on receive
- Timestamp incremented by 1024 samples per AU (RFC 3640 §3.3)

### SDP fmtp for AAC (RFC 3640 §4.4)

```sdp
a=rtpmap:97 MPEG4-GENERIC/<sample_rate>/<channels>
a=fmtp:97 profile-level-id=1;mode=AAC-hbr;sizelength=13;indexlength=3;indexdeltalength=3;config=<hex>
```

- `config=` = AudioSpecificConfig (ASC) bytes in uppercase hex
- Timestamps MUST be in native sample-rate clock units (not scaled)

---

## RFC 3550 — RTP / RTCP

### RTP Header Rules (RFC 3550 §5.1)

- V=2 always (enforced in `Default` impl)
- Sequence numbers MUST wrap with `wrapping_add(1)`
- Initial RTP timestamp SHOULD be random (currently may start at 0)
- Video clock rate = 90000 Hz; audio clock rate = sample rate

### Timestamp Handling

- Video source timestamps (ms) → scaled to 90kHz via `RtpTimestampNormalizer`
- Audio timestamps passed through unscaled at native sample-rate clock
- `RtpTimestampNormalizer` keeps timestamps monotonic across source resets
  while preserving u32 wrap-around (RFC 3550 §5.1)

### RTCP Rules

| Constant | Value | Required Behaviour |
| --- | --- | --- |
| `RTCP_SR` | 200 | Sent every 5 seconds; NTP/RTP pair must be contemporaneous (§6.4.1) |
| `RTCP_RR` | 201 | Full jitter tracking per Appendix A |
| `RTCP_SDES` | 202 | Constant defined; **no send/parse yet** |
| `RTCP_BYE` | 203 | Marshals/unmarshals SSRC list + optional reason |
| `RTCP_APP` | 204 | Supported |

### RTCP SR Layout (RFC 3550 §6.4.1)

- NTP timestamp: 64-bit (32b seconds since 1900 + 32b fraction)
- SR length field = 6 (= 28 bytes / 4 − 1)

### RTCP RR Jitter Constants (RFC 3550 Appendix A)

`MAX_DROPOUT=3000`, `MAX_MISORDER=100`, `MIN_SEQUENTIAL=2`

### NTP Epoch (RFC 5905)

```rust
const NTP_UNIX_EPOCH_DIFF: u64 = 2_208_988_800; // 70 years
ntp_secs = unix_secs + NTP_UNIX_EPOCH_DIFF
ntp_frac = (unix_nanos << 32) / 1_000_000_000
```

---

## RFC 2326 — RTSP/1.0

### Methods We Implement

`OPTIONS`, `DESCRIBE`, `ANNOUNCE`, `SETUP`, `PLAY`, `PAUSE`, `TEARDOWN`,
`GET_PARAMETER`, `SET_PARAMETER`, `REDIRECT`, `RECORD`

### Key Section Requirements

| RFC § | Rule |
| --- | --- |
| §10.5 | Session ID MUST be validated in PLAY |
| §10.10 | `REDIRECT` MUST be excluded from `Public:` header in OPTIONS |
| §10.12 | Interleaved (`$`) payload length MUST be validated |
| §11.3.7 | Invalid Range MUST return `457 Invalid Range` |
| §12.1 | `Accept:` header MUST be honoured in DESCRIBE |
| §12.36 | `Server:` header required |
| §12.37 | `Session` header MUST appear in all PLAY responses |
| §12.39 | Transport header parsing MUST be case-insensitive |

### Transport Rules

- Supported: `RTP/AVP/TCP` (interleaved), `RTP/AVP/UDP`, `RTP/AVP`
- TCP: `interleaved` field required
- UDP: `client_port` required; single port value → RTCP port = RTP port + 1
- SSRC in hex without `0x` prefix
- Multicast: parsed but **not implemented** (intentional non-goal)

---

## RFC 4566 — SDP

### Session-Level Requirements

```sdp
v=0
o=- <ntp_session_id> 0 IN IP4 0.0.0.0
s=No Name
c=IN IP4 0.0.0.0
t=0 0
a=control:*
```

Session ID MUST use NTP epoch (1900) per RFC 4566 §5.2.

### Codec ↔ SDP Payload Type Mapping

| Codec | SDP Name | Dynamic PT | Clock Rate |
| --- | --- | --- | --- |
| H264 | `h264` | 96 | 90000 |
| AAC | `mpeg4-generic` | 97 | per-stream sample rate |
| G.711A | `pcma` | 8 (static, RFC 3551) | 8000 |

---

## Known Compliance Gaps (must fix before shipping)

| Gap | RFC | Severity |
| --- | --- | --- |
| STAP-A not **sent** (parse only) | RFC 6184 §5.7 | Low — interop ok |
| MTAP-16/24 not sent | RFC 6184 §5.7.2 | Low |
| FU-B DON discarded on receive | RFC 6184 §5.8.2 | Low |
| RTCP SDES not sent | RFC 3550 §6.5 | Low |
| RTP initial timestamp not randomized | RFC 3550 §5.1 | Low |
| AAC packs 1 AU/packet on send (no aggregation) | RFC 3640 §3.3 | Low |
| Multicast parsed but not implemented | RFC 2326 §12.39 | Non-goal |

---

## Key Files Quick Reference

| Component | Path (relative to `cross-compile/`) |
| --- | --- |
| H264 packer/unpacker | `streaming-lib/src/protocol/rtsp/rtp/rtp_h264.rs` |
| AAC packer/unpacker | `streaming-lib/src/protocol/rtsp/rtp/rtp_aac.rs` |
| NAL type constants | `streaming-lib/src/protocol/rtsp/rtp/define.rs` |
| RTP header marshal/unmarshal | `streaming-lib/src/protocol/rtsp/rtp/rtp_header.rs` |
| NTP / timestamp utilities | `streaming-lib/src/protocol/rtsp/rtp/utils.rs` |
| RTCP SR/RR/jitter | `streaming-lib/src/protocol/rtsp/rtp/rtcp/rtcp_context.rs` |
| SDP model | `streaming-lib/src/protocol/rtsp/sdp/mod.rs` |
| SDP fmtp parser | `streaming-lib/src/protocol/rtsp/sdp/fmtp.rs` |
| RTSP server session | `streaming-lib/src/protocol/rtsp/session/server_session.rs` |
| RTSP transport parser | `streaming-lib/src/protocol/rtsp/rtsp_transport.rs` |
| RTSP range header | `streaming-lib/src/protocol/rtsp/rtsp_range.rs` |
| Codec ID ↔ SDP name | `streaming-lib/src/protocol/rtsp/rtsp_codec.rs` |
| SDP generation (ONVIF side) | `onvif-rust/src/streaming/helpers.rs` |
| SPS/PPS codec parsing | `streaming-lib/src/codec/h264/sps.rs`, `pps.rs` |
| RFC compliance audit doc | `wiki/RTSP-Validation-Tool.md` |
| Validation tool | `validation/rust/src/probe.rs` |
