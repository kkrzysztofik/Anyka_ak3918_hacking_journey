# RTP Send Latency on the AK3918 — Investigation Record

**Date:** 2026-07-28
**Device:** Anyka AK3918 (armv5te, uClibc), 192.168.2.198
**Branch:** `fix/rtp-udp-pacing-scheduler-latency`
**Symptom:** persistent `rtp_send_slow` warnings and visible video stutter on the main RTSP stream.

---

## Executive summary

The stutter was never one bug. It was three independent costs stacked on a single-core SoC, and the
investigation only converged once each was measured separately rather than inferred from `send_ms`
alone.

| # | Cost | Status | Commit |
|---|------|--------|--------|
| 1 | `tokio::time::sleep` pacing charged ~12 ms per batch instead of 300 µs | **fixed** | `dc3cafb` |
| 2 | Per-datagram tokio mutex spending the cooperative-scheduler budget | **fixed**, worth ~15% | `d3a3f04` |
| 3 | Per-packet marshalling + `sendmmsg`, ~1.56 ms/packet of pure CPU | **open** | — |

The residual (3) is not a defect. It is the device's actual throughput ceiling, and at the configured
**2000 kbps** the main stream asks for roughly **twice** what the hardware can packetise inside a frame
interval.

---

## Platform constant: an await that parks costs ~12 ms

Established early and load-bearing for everything after.

`onvif-rust` runs 2 tokio workers (`onvif-w00`, `onvif-w01`) on **one** core, shared with
`vendor-daemon` (~25% CPU), the `ssv6xxx_hci_tx_` WiFi TX thread, and `ksoftirqd/0`. Any task that
yields waits a full scheduler quantum before it is rescheduled.

Measured by regressing `send_ms` over 1301 `rtp_send_slow` lines:

```text
send_ms ~= 31.6 + 12.1 * (batches - 1)
```

`tokio::time::sleep(300us)` cost **12.1 ms** — a ~40× overshoot. Fitted against uncensored buckets:

| batches | predicted | observed |
|---|---|---|
| 1 | 31.6 | 32.4 |
| 5 | 80.0 | 80.3 |
| 7 | 104.2 | 104.2 |
| 8 | 116.3 | 116.3 |

**Rule:** never add sub-millisecond sleeps, per-item yields, or fine-grained pacing to a hot path on
this device. They cost ~40× nominal and test fine on a multi-core dev box.

---

## Refuted hypotheses

Recording these deliberately: each was plausible, supported by real structural evidence in the source,
and wrong. The pattern worth remembering is that every one of them was killed by a direct measurement
that took minutes, after costing far longer in reasoning.

### R1 — A ~450 KB/s wire ceiling (claimed in commit `e7b627e`)

Disproved: I-frames clear at 9.2 Mbps; `/proc/net/snmp` showed `SndbufErrors = 0` across 287,365
datagrams; `/proc/net/udp` `tx_queue = 0` while `rtp_send_slow` fired continuously.

### R2 — IP fragmentation from a 1500-byte MTU

Disproved: `rtsp_channel.rs:37` sets `DEFAULT_MAX_RTP_PAYLOAD_SIZE = 1400`. No fragmentation.

### R3 — Per-packet `packet.clone()` in `rtp_h264.rs` feeding the RTCP handler

Disproved: `on_packet_for_rtcp_handler` is installed **only** in `handle_record` (the publish path).
On the PLAY path it is `None`, so the deep copy never runs.

### R4 — WiFi burst backpressure parking the sender

Predicted `park_us` in the thousands and `send_attempts` of 2-5. Measured, three separate times:

```text
lock_us=10  park_us=20  send_attempts=1   (78 packets)
lock_us=10  park_us=19  send_attempts=1   (17 packets)
lock_us=18  park_us=21  send_attempts=1   (70 packets)
```

Microseconds, and always a single `sendmmsg`. The kernel never refused a datagram. This also closes
the risk introduced when the pacing fix raised the per-syscall batch from 10 to ~80 packets.

### R5 — Lock contention between the main and sub streams

Same evidence: `lock_us` is 10-18 µs.

### R6 — vendor-daemon ignoring SIGTERM because `pthread_join` hangs

Structurally real (see below) but **did not reproduce**. Live test: SIGTERM with 6 threads and both
push streams active produced a clean exit in **15 ms**, both threads joined.

### R7 — No job control over `telnetd -l /bin/sh`, so Ctrl-C never becomes SIGINT

Disproved from `/proc/<pid>/stat`: `tpgid=1383` and the daemon's `pgrp=1383` — it *is* in the
terminal's foreground process group, so Ctrl-C would reach it.

---

## Fix 1 — pacing (`dc3cafb`)

`write_udp_frame` called `packets.chunks(pace_batch)` with `udp_pace_batch.max(1)`, so the documented
"`<= 1` means no intra-frame pacing" contract was inverted: `0` paced after *every* packet.

At the old default of 10 packets/batch a ~110 KB I-frame was 8 batches and spent ~85 ms of its ~116 ms
**asleep**, against a 66 ms budget.

Default changed to `udp_pace_batch = 0` (one `sendmmsg` per frame). The kernel already paces this path:
the socket buffer is ~304 KB, larger than any single frame, with zero `SndbufErrors`.

---

## Fix 2 — tokio cooperative budget (`d3a3f04`)

`setup_udp_play_packet_handler` accumulated packets under a **tokio** `Mutex`, locked once per datagram.
It is never contended — only that handler touches it — but a tokio mutex lock is a *resource operation*,
and tokio gives each task a budget of 128. Past the budget, the next such await returns `Pending`
regardless of readiness, forcing a reschedule that costs a full quantum here.

Swapped to `std::sync::Mutex`, which is not a tokio resource and spends no budget.

**Implementation note:** the guard must not be held across an await. An explicit `drop(guard)` inside a
branch was *not* enough — the future failed its `Send` bound. Scoping the guard into a block that yields
the frame out, and awaiting outside it, makes the release unconditional. Because
`std::sync::MutexGuard` is `!Send`, **the compiler statically proves** no lock is held across an await.

**Result: real but modest.**

```text
before:  send_ms ~= 32.0 + 0.300 * packets
after:   send_ms ~= 27.2 + 0.266 * packets
```

Intercept −15%, slope −11%. The coop budget was one of ~6 per-packet costs, not the mechanism.

---

## The instrumentation that actually resolved it (`f6151d8`, `d3a3f04`)

`send_ms` alone cannot distinguish three causes with opposite fixes. Two log lines now split it.

`slow_udp_write` (transport):

```text
elapsed_ms=42 packets=70 lock_us=18 park_us=21 send_attempts=1 peak_ms=42
```

- high `lock_us` → another track holds the transport
- high `park_us` + `send_attempts > 1` → kernel took a partial run, link not drained
- both low → the cost is upstream, in marshalling

`slow_rtp_pack` (packetisation):

```text
elapsed_ms=12 extract_us=1242 emit_us=11501 frame_bytes=16038 nalus=1 occurrences=279 peak_ms=108
```

- `extract_us` scales with frame **bytes** (start-code scanning)
- `emit_us` scales with **packets**

And `rtp_send_slow.send_ms − slow_udp_write.elapsed_ms` gives the packetisation cost, which no timer
inside `write_udp_frame` can see.

### Sampling bias — read this before trusting any regression here

Both lines fire only at **≥ 10 ms**, and `rtp_send_slow` only at **≥ 25 ms**. Every regression in this
document is therefore fit against the **slow tail, not the mean**. This directly caused a wrong turn:
"cost is flat against packet count" read as evidence of blocking, when it was an artefact of censoring.
`occurrences=279` per 30 s against ~30 frames/s means ~31% of frames exceed 10 ms in packing alone.

---

## Current model

From the measured split, with `nalus=1` (one slice per frame, fragmented FU-A):

| Phase | Sample | Rate |
|---|---|---|
| pack emit | 11.5 ms / ~12 pkts | **0.96 ms/packet** |
| `sendmmsg` | 42 ms / 70 pkts | **0.60 ms/packet** |
| **total** | | **~1.56 ms/packet** |

For the 94 KB I-frame at `send_ms=85`: ~42 ms syscall, ~43 ms marshalling. **Roughly 50/50, both pure
CPU.** `extract_us` loses to `emit_us` 9:1, so NAL extraction is not the problem.

### Per-packet waste in the emit path

```rust
let mut packet = RtpPacket::new(self.header.clone());  // alloc #1
packet.payload.put(fu_payload);                        // copy #1 (1398 B)
let msg = packet.marshal()?;                           // header.marshal() -> alloc #2
                                                       // BytesWriter::new()  -> alloc #3
                                                       // writes payload again -> copy #2
let stream_path = stream_identifier
    .map(|id| id.to_string())                          // alloc #4 — used only if a log fires
    .unwrap_or_else(|| "unknown".to_string());
```

plus three `String`/`Arc` clones and a `Box::pin` future per packet in the handler closure:
**~6 allocations and two full copies of every datagram**, on uClibc malloc at 500 MHz.

---

## The dominant lever is not in this code: I-frame size

Main stream config: **1280×720 @ 15 fps, bitrate = 2000, gop_length = 50, quality = 80, CBR.**

On the wire: one 113 KB I-frame plus ~29 P-frames at ~14 KB every ~2 s = 519 KB / 2 s = **2.07 Mbps**.
The encoder is hitting its 2000 kbps target almost exactly. Nothing is malfunctioning; the budget is
wrong for the hardware.

At 1.56 ms/packet:

```text
main @ 2000 kbps -> 179 pkt/s -> 279 ms/s = 27.9% of the one core
sub  @  512 kbps ->  46 pkt/s ->  71 ms/s =  7.1%
                                            -----
                              RTP path alone  35%   before encode, vendor-daemon, WiFi
```

Burst cost, which is what presents as stutter:

| I-frame | packets | cost | vs 67 ms budget |
|---|---|---|---|
| **113 KB (current)** | 83 | **129 ms** | **2× over** |
| 80 KB | 59 | 91 ms | over |
| **58 KB** | 43 | 67 ms | **ceiling** |
| 40 KB | 29 | 46 ms | comfortable |

**~58 KB is the largest I-frame this device can push inside one frame interval.**

### Available levers, cheapest first

1. **`bitrate` 2000 → ~1000.** Config only, no rebuild, instantly reversible. Halves packet count;
   I-frames land near 56 KB and RTP CPU falls 28% → 14%. Best first experiment.
2. **`minqp` 20 → 25.** Hardcoded at `video_encoder.rs:1282`. `minqp` is the *floor* on the quantiser —
   exactly what lets I-frames balloon, since rate control spends its lowest QP there. The SDK documents
   the range as `[20,25]`; we are pinned at the aggressive end.
3. **Wire `METHOD_ISIZE_CTRL`.** The SDK's purpose-built I-frame cap:

   ```c
   enum enc_method { METHOD_DEFAULT, METHOD_ISIZE_CTRL /* take I size under some value */, METHOD_SMART_H264 };
   int ak_venc_set_method(void *enc_handle, enum enc_method method);
   ```

   **Never called anywhere.** vendor-daemon only ever calls `ak_venc_set_rc(bps)`. Needs a new IPC
   command plus the onvif-rust call. The only option that caps the spike without degrading P-frames.

### Not levers

- **CBR is already active** (`#[default]` on `BitrateMode`, not overridden in config). Note the SDK's
  "CBR" still permits an 8× I-to-P size ratio.
- **GOP length** changes how *often* the spike is paid, not its size. Helps average CPU, not stutter,
  and costs loss-recovery time.

---

## Dead configuration found along the way

| Key | Status |
|---|---|
| `ptz.enabled` | was write-only — **fixed** in `afd6b27`; also burned ~2.1 s of failing calibration sweep per boot |
| `ptz.home_on_start` | still write-only, untouched |
| `stream_profile_N.quality` | **write-only** — `ak_venc_set_rc_weight` is never called |

`quality` carries a trap: ONVIF quality 80 means *high quality*, while the SDK weight is **inverted**
(`0 = best quality, 100 = lowest bitrate`). Wiring it naively inverts the meaning of the knob.

**Unresolved discrepancy:** config says `gop_length = 50`, but the observed I-frame interval is 1.98 s
at ~15 fps ≈ **30 frames**. Either a different profile is live than the one at config line 320, or the
setting is not reaching the encoder. Worth confirming before tuning.

---

## vendor-daemon signal handling (`e3b1af9`, `4076511`)

Investigated after the daemon reportedly ignored Ctrl-C and `kill`.

**The handlers are installed and correct** — proven from the running process, not the source:

```text
SigCgt: 0000000180004002    bit 1 = SIGINT, bit 14 = SIGTERM
SigIgn: 0000000000001004    bit 12 = SIGPIPE
```

Bit 0 is clear, so **SIGHUP was not caught**, and the daemon runs in the foreground of
`run_vendor_daemon.sh` with no `setsid`/`nohup` — a dropped session killed it by default action with no
thread stop and no ring teardown.

The path that *can* genuinely swallow a signal is shutdown itself: `stop_push_slot()` called
`pthread_join()` unbounded, and the push thread only re-tests its loop condition between
`ak_venc_get_stream()` calls. If the SDK parks inside that call the thread never sees the flag and the
daemon accepts the signal but never exits.

`globals.h` claimed `PUSH_NO_DATA_EXIT_THRESHOLD` *"guarantees that `stop_push_slot()`'s
`pthread_join()` completes in bounded time."* It does not — that counter only advances when the SDK
call **returns**, i.e. in the one case where the thread is already healthy.

Fixed: `pthread_timedjoin_np` with a 3 s deadline, detach and report on timeout; skip the ring teardown
when a thread is still live (`vd_ring_destroy` munmaps memory it may still be writing, turning a hang
into SIGSEGV); a second signal calls `_exit(128+sig)`; `SIGHUP` handled; `g_shutdown` became
`volatile sig_atomic_t` (C11 §7.14.1.1p5).

**Not reproduced** — see R6. The fix is defensive and self-diagnosing: a `join_timeout` line proves the
join hypothesis, a missing `shutdown_stop_begin` proves the signal never arrived.

---

## Logging (`4076511`)

`/mnt/logs/vendor_daemon.log` reached **295 MB**, and regrew to 85 MB within an hour of deletion. Every
byte lands on vfat on the SD card, contending for the core that also encodes and sends video.

Pinned in three places, all at maximum verbosity, across **two independent log systems**:

```c
log_set_level(LOG_DEBUG);              /* log.c global  */
log_add_fp(g_log_fp, LOG_DEBUG);       /* file callback — carries its own level */
ak_print_set_level(LOG_LEVEL_DEBUG);   /* Anyka SDK     */
```

Lowering only the global would have changed nothing: a callback registered at a more verbose level
emits regardless. `VENDOR_DAEMON_LOG_LEVEL = trace|debug|info|warn|error` now drives all three,
defaulting to `info`.

onvif-rust side, set on-device: `level = "info"`, `http_verbose = false`, `stream_frame_debug = false`,
`file_path = ""` (backup at `config.toml.pre-logfix`).

**Do not redirect logs to tmpfs on this device.** 36 MB RAM total, ~2 MB free; `/tmp`, `/var` and `/mnt`
are all tmpfs in `mount`, but `/mnt` is really `/dev/mmcblk0p1`, vfat on SD.

---

## Reproduction notes

- Telnet `192.168.2.198:24`, `root`, empty password. Piping a script into `telnet` does not work —
  drive it with a raw socket client. Use `uv run --no-project python3`, never bare `python3`.
- Grepping the multi-hundred-MB log on vfat exceeds a short client timeout and returns **empty output
  rather than an error** — this silently looked like "no matches". Use `tail -c N | grep`.
- ARM builds go through `cross-compile/onvif-rust/scripts/build.sh --release --target
  armv5te-unknown-linux-uclibceabi`. Invoking the vendored cargo directly fails with
  "can't find crate for `std`". LTO link takes several minutes.
- The device `config.toml` (~11.7 KB, "DEBUG variant") **diverges** from the repo copy (~10.8 KB).
  Deploying the repo copy wholesale overwrites device logging and stream profiles — edit in place with
  `sed -i` (busybox supports `-i`) and back up first.
- `ntpd` runs with `-p 192.168.1.1`, a different subnet from the camera, so it never syncs and log
  timestamps read `1970-01-01` until the clock is set by hand.

---

## Open items

1. **Per-packet allocations in the emit path** — fixes 1-3 below; expected ~15-20% off `send_ms`.
   Cannot touch the `sendmmsg` half.
2. **I-frame size** — the dominant lever. Nothing in the Rust code competes with it.
3. `ptz.home_on_start` still dead config.
4. `gop_length` discrepancy (50 configured vs ~30 observed).
5. `SD_card_contents/.../onvif-rust.bin` and `.../vendor-daemon.bin` are stale in git relative to HEAD.
6. Removing the per-packet `Box::pin` future requires changing `OnRtpPacketFn` — deferred.
