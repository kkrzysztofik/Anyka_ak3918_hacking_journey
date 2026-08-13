# HTTP-FLV SPS/PPS recovery (refuse + IDR kick)

Date: 2026-08-13  
Status: approved

## Problem

Vendor H.264 often emits SPS/PPS only around a startup IDR. If the bridge misses them, main stays without cached params: RTSP recovers via `send_information` → coalesced `request_idr`; HTTP-FLV only logs in `send_prior_data` and leaves HTTP 200 with an empty body (remuxer never starts).

## Decision

Approach 1 — refuse incomplete FLV subscribe; fix hub so that failure does not kill the stream loop. Client reconnects (Live View / mpegts.js). No mid-stream seq-header injection, no remuxer-drop heal path, no WebUI changes.

## Design

### onvif-rust (`LiveStreamHandler::send_prior_data`)

When `SubscribeType::HttpFlvPull` and SPS or PPS missing:

1. Coalesced `request_idr(is_main)` via existing `idr_requested` + `bridge.idr_requester` (same as `send_information`).
2. Return `Err` using an existing `StreamHubError` path — no new variant unless nothing fits.
3. Send no MediaInfo / prior frames on that path.

Other subscribe types unchanged. RTSP path unchanged.

### streaming-lib (`StreamsHub::handle_subscribe_event`)

Today `send_prior_data` Err logs and `return true`, breaking the whole stream event loop.

Change to: log, do not insert frame/packet sender, **`return false`**. Fail only that subscribe so the HTTP session ends; later retries and other subscribers keep working.

### Non-goals

- Injecting AVC sequence headers onto an already-open FLV body
- Dropping subscribers when remuxer initializes
- WebUI copy / special warming-up UX
- Proactive IDR at streaming start (can be a follow-up)

## Testing

- onvif-rust: HttpFlvPull + empty SPS → one `request_idr` + Err; with SPS/PPS → prior data Ok
- streaming-lib: prior-data Err → that subscribe fails; event loop still accepts a later subscribe
