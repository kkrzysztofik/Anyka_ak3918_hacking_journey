# RTSP/RTP/SDP RFC Compliance Notes (streaming-lib)

This document maps the repo’s RTSP server + publisher implementations to the most relevant RFCs and records the current compliance posture (including known gaps).

**Primary implementation**: `cross-compile/streaming-lib/src/rtsp/`

## Relevant RFCs (by feature area)

- **RTSP 1.0 (control plane)**: RFC 2326 (RTSP/1.0 request/response format, CSeq/Session/Transport, and RTP-over-RTSP interleaving).
- **RTP/RTCP (media plane)**: RFC 3550 (RTP header, sequencing, timestamps; RTCP SR/RR/BYE basics).
- **SDP (session description)**: RFC 4566 (SDP syntax and media/attribute lines used in DESCRIBE/ANNOUNCE).
- **RTP payload formats**:
  - H.264: RFC 6184 (`a=rtpmap`, `a=fmtp` params like `packetization-mode` / `sprop-parameter-sets`; FU-A packetization).
  - H.265: RFC 7798 (payload format and SDP `fmtp` conventions for HEVC).
  - AAC (MPEG-4 generic): RFC 3640 (SDP `fmtp` required parameters such as `streamtype`, `mode`, `sizelength`, `indexlength`, etc.).
  - Static audio payload types (e.g. PCMA/PCMU): RFC 3551 (payload type registry and clock rates).

## What we validate in code/tests

### RTSP framing over TCP (RFC 2326)

RTSP over TCP is a **byte stream** where RTSP messages can be coalesced and can be followed immediately by interleaved `$` frames (RTP/RTCP) in the same read.

**Implementation**:
- Interleaved binary header parsing: `cross-compile/streaming-lib/src/rtsp/session/server_session.rs` (`InterleavedBinaryData`).
- RTSP message extraction by `\r\n\r\n` terminator + `Content-Length`: `cross-compile/streaming-lib/src/common/http.rs` (`try_get_complete_message_len`).
- Server and publisher/client use this extraction to avoid UTF-8 parsing across binary boundaries:
  - `cross-compile/streaming-lib/src/rtsp/session/server_session.rs` (`on_rtsp_message`)
  - `cross-compile/streaming-lib/src/rtsp/session/client_session.rs` (`receive_response`)

**Regression tests added**:
- `cross-compile/streaming-lib/src/common/http.rs`: `test_try_get_complete_message_len_pipelined_rtsp_and_interleaved_binary`
- `cross-compile/streaming-lib/src/rtsp/session/server_session.rs`: `test_rtsp_server_session_on_rtsp_message_leaves_interleaved_binary_buffered`
- `cross-compile/streaming-lib/src/rtsp/session/client_session.rs`: `test_rtsp_client_session_receive_response_with_interleaved_binary_buffered`

### Header parsing and lookup robustness (RTSP/HTTP-style)

RTSP headers are case-insensitive; peers vary in formatting (e.g. `CSeq:1` vs `CSeq: 1`).

**Implementation**:
- Header parsing now accepts `:` with optional whitespace (instead of requiring `": "`).
- `get_header(..)` performs a case-insensitive lookup fallback.

**Regression tests added**:
- `cross-compile/streaming-lib/src/common/http.rs`: `test_parse_rtsp_request_header_without_space_after_colon`
- `cross-compile/streaming-lib/src/common/http.rs`: `test_parse_rtsp_response_reason_phrase_with_spaces`

## Implemented interoperability fixes

- **RTSP keep-alives**: `GET_PARAMETER` and `SET_PARAMETER` now respond `200 OK` and include `Session` when available (`cross-compile/streaming-lib/src/rtsp/session/server_session.rs`).
- **Method coverage**: `PAUSE` is implemented to stop playback and unsubscribe; `REDIRECT` returns `405 Method Not Allowed` with `Allow:`; both reply instead of falling through.
- **Transport parameter tolerance**: `Transport:` parsing accepts case-insensitive tokens/keys, extra whitespace, hex SSRC without `0x`, and single-value `client_port`/`interleaved` forms (`cross-compile/streaming-lib/src/rtsp/rtsp_transport.rs`).
- **H.264 access-unit coalescing**: server-side `PLAY` now coalesces consecutive same-timestamp `FrameData::Video` chunks into a single Annex-B access unit before RTP packetization, preventing repeated marker-terminated access units at the same RTP timestamp (`cross-compile/streaming-lib/src/rtsp/session/server_session.rs`).

## Known gaps / non-goals (current state)

These are not necessarily bugs, but they are protocol-surface items that may be required for strict interoperability with some clients:

- **Authentication**: RTSP auth behaviors vary (Basic/Digest over RTSP); current behavior is driven by the `Auth` hooks rather than a full RTSP auth spec implementation.

## Practical conformance testing (recommended)

Use the host-side validator described in `validation/RTSP_VALIDATION_README.md` to exercise RTSP sequences, SDP correctness, RTP loss/ordering, and RTCP emission against `onvif-rust` (which embeds `streaming-lib`).

### PCAP-level RTP payload assertions (RFC 6184 / RFC 3640)

The validation harness now also performs **pcap-level RTP payload structural checks** on the captured RTP stream(s):

- **H.264 over RTP (RFC 6184)**: validates that captured payloads conform to the packetization modes we emit (Single NAL / STAP-A / FU-A), with counters for invalid payloads and FU/STAP parsing failures.
- **AAC MPEG4-GENERIC over RTP (RFC 3640)**: validates AU-headers-length framing and AU-size bounds (the common 16-bit AU header mode: 13-bit size + 3-bit index), and flags payloads that overrun the packet.

**Implementation**: `validation/rust/src/rtsp.rs` (`harness_packet_loss` via `tshark` field extraction + validators).
