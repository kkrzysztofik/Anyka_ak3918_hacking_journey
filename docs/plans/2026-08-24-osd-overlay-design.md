# OSD Overlay (Camera Name + Timestamp) — Design

**Date:** 2026-08-24
**Status:** Design approved; Phase 0 Stages A+B passed on hardware 2026-08-24
**Scope:** Burn camera name and timestamp into the encoded video, configurable over ONVIF and from the WebUI.

## Goal

Composite two text overlays — a camera name and a live timestamp — into the video
*before* the H.264 encoder, so they appear in every consumer: RTSP to a third-party
NVR, HTTP-FLV, and snapshots. Configuration is exposed through ONVIF Media OSD
operations and a new WebUI settings page.

A browser-side CSS overlay was considered and rejected: it is invisible to anything
that records the RTSP stream, which is the main reason to want a timestamp at all.

## Findings that shape the design

### The video path forces the overlay into C

`vendor-daemon` (C) owns VI capture → VPSS → VENC and pushes *already-encoded*
H.264 to `onvif-rust` over `/tmp/vd-frame-{main,sub}.sock`. Rust never sees a raw
YUV frame. Burned-in text is therefore only possible on the C side, before the
encoder. Decoding and re-encoding on a 36 MB ARMv5TE box is not viable.

### The vendor OSD library is usable, but is not on the camera

`ak_osd.h` declares the OSD API. The only binary in this repo exporting those
symbols is `cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/libmpi_osd.so`
(25.6 KB). It is absent from both `orig/usr/lib/` and
`cross-compile/vendor-daemon/lib/`, so it must be shipped in the payload.

Shipping a `.so` from a foreign vendor bundle is the exact failure mode of the
`.121` libakstreamenc version-gate incident, so this was checked rather than
assumed:

- `readelf -d` shows exactly one `DT_NEEDED`: `libc.so.0`. Everything else
  resolves from the process's global scope at runtime.
- Of its 15 `ak_*` undefined symbols, **13 are exported by our shipped lib set**
  (`libplat_vpss`, `libplat_vi`, `libplat_ipcsrv`, `libplat_common`,
  `libplat_thread`, `libakuio`).
- The two that are missing — `ak_cmd_register_module` and
  `ak_cmd_unregister_module` — are, per disassembly of the PLT call graph,
  reachable **only** from `osd_sys_ipc_register` / `osd_sys_ipc_unregister`.
  Those are the vendor's remote-command registration, which we do not need
  because the daemon already has its own IPC.
- `readelf -d` shows no `BIND_NOW` / `DF_1_NOW`, so ARM lazy PLT binding applies:
  the unresolvable stubs are never entered unless those two functions are called.
- The one call the library makes back *into* our libs is
  `ak_vpss_osd_set_param`, exported by our `libplat_vpss.so`, and
  `struct vpss_osd_param` (`enum id` + `unsigned char data[128]`) is
  **byte-identical** between `cross-compile/vendor-daemon/include/ak_vpss.h` and
  `cross-compile/anyka_reference/platform/libplat/include/ak_vpss.h`.

This differs materially from the libakstreamenc failure, which was a version gate
between libraries that had to agree on encoder state. Here the foreign library is
a leaf that writes a 132-byte param blob through a stable entry point.

**Constraint carried into the build (revised after Stage B):** `ak_osd_init`
always calls `osd_sys_ipc_register`, so provide no-op stubs for
`ak_cmd_register_module` / `ak_cmd_unregister_module` (`osd_ipcsrv_stubs.c`).
Do not remove those stubs. Comment at the link site, because it is
non-obvious.

### The font already ships, and it is Chinese

`/usr/local/ak_font_16.bin` (259.6 KB) is present on the camera — roughly
GB2312's 7445 glyphs at 16×16×4bpp. Consequences:

- Only one font size exists: 16px. `GetOSDOptions` advertises `16..16`.
- There are no Latin diacritic glyphs — no `ó`, `ł`, `ü`. **ASCII-only camera
  names is a hardware truth, not a simplification.** Non-ASCII is rejected at
  validation rather than rendered as garbage, and `iconv` is avoided entirely.

### Text encoding is known, not guessed

`ak_osd_ex.c:asc_to_short()` in the reference tree gives the exact convention:

- byte `< 0x80` → `u16 = byte`, half-width (8px advance)
- GBK pair → `u16 = (hi << 8) | lo`, full-width (16px advance)

For ASCII this reduces to `s.bytes().map(u16::from)`.

### Colour and alpha are device-global

`ak_osd_set_color(front, bg)` and `ak_osd_set_alpha(alpha)` take no channel or
rect argument. Per-OSD colour is not achievable. This is advertised honestly in
`GetOSDOptions` and labelled as device-wide in the UI, rather than pretending to
per-OSD colour and letting the last writer silently win.

### Policy belongs in Rust

`main.c:2` describes the daemon as an "IPC bridge"; control is a single `poll()`
loop and the only `pthread_create` calls are the frame push thread and a cancel
worker. Adding a 1 Hz `strftime` timer thread in C would cut against that. Rust
already holds the config, the timezone and `chrono`, so it owns the tick and the
formatting; C gets a dumb draw primitive.

## Architecture

```
WebUI OsdPage → osdService.ts (SOAP)
  → onvif-rust  onvif/media/ops/osd.rs   [GetOSDs / GetOSDOptions / SetOSD]
      ├─ persists  [osd]  → anyka.toml
      └─ OsdRenderer  (tokio interval, 1 Hz)
           → CMD_OSD_*  over /tmp/vd-ctrl.sock
               → vendor-daemon handlers_osd.c
                   → libmpi_osd.so  ak_osd_draw_str
                       → ak_vpss_osd_set_param  (our libplat_vpss.so)
                           → VPSS composites into YUV, before VENC
                               → RTSP + HTTP-FLV + snapshots
```

## C side — `vendor-daemon`

Ship `libmpi_osd.so` into the payload lib dir; link `-lmpi_osd`.

New `handlers_osd.c`. Command IDs are appended, never renumbered, per the wire
protocol rule documented at `protocol.h:55`:

| Command | Payload |
|---|---|
| `CMD_OSD_INIT = 22` | `[u64 vi_token]` → `ak_osd_set_font_file(16, …)`, `ak_osd_init`, `ak_osd_get_max_rect` |
| `CMD_OSD_SET_RECT = 23` | `[u64 vi_token][i32 chn][i32 rect][i32 x][i32 y][i32 w][i32 h]` |
| `CMD_OSD_DRAW_STR = 24` | `[i32 chn][i32 rect][i32 x][i32 y][u16 len][u16 codes…]` |
| `CMD_OSD_SET_ENABLE = 25` | `[i32 chn][i32 rect][i32 enable]` |
| `CMD_OSD_SET_STYLE = 26` | `[i32 front][i32 bg][i32 edge][i32 alpha]` (global) |

Handle resolution follows the existing
`vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle)` pattern.
`ak_osd_destroy()` on VI close. No timers, no `strftime`, no TZ logic. ~200 lines.

`CMD_OSD_CLEAN_STR` is deliberately omitted: when a string shrinks, Rust pads with
`0x20` to the previous glyph count, which is what the vendor's own
`osd_disp_stat()` does. One fewer command and one fewer state machine.

## Rust side — `onvif-rust`

- **`osd/encode.rs`** — `to_glyph_codes(&str) -> Vec<u16>`. ASCII only; non-ASCII
  rejected during `SetOSD` validation with a descriptive fault.
- **`osd/layout.rs`** — pure `(corner, glyph_count, channel_dims, font) -> (x, y)`.
  Stage B corrected the paper math: main channel uses 32px height / 16px advance
  (font file doubled); sub keeps 16/8. Left inset equals font height; right-aligned
  `x = width - advance*len`; bottom `y = height - font_height`. Fully host-testable.
- **`osd/renderer.rs`** — one tokio task on a 1 Hz interval, re-rendering only
  changed strings. The name rect redraws approximately never; the time rect once a
  second. 1 Hz on a control socket that already carries a 25 fps push path is not
  a budget concern, and is far from the hot path where the ~12 ms await quantum
  matters.
- **Lifecycle** — initialise after `CMD_VI_OPEN` and VPSS init succeed;
  re-initialise on daemon restart, which the existing epoch/`CMD_HELLO` handshake
  already signals.

### Config (`config/types.rs`)

```toml
[osd]
enabled = true
color    = 1     # index into the vendor's 16-entry palette
alpha    = 80    # 1..100

[osd.name]
enabled  = true
position = "upper-left"
text     = ""    # empty → falls back to the ONVIF device name

[osd.datetime]
enabled     = true
position    = "lower-right"
date_format = "YYYY-MM-DD"   # | DD/MM/YYYY | MM/DD/YYYY
time_format = "24h"          # | 12h
```

### ONVIF ops (`onvif/media/ops/osd.rs`)

Two fixed, non-deletable instances with tokens `osd_name` and `osd_datetime`.

- `GetOSDs`, `GetOSD`, `GetOSDOptions`, `SetOSD` — implemented.
- `CreateOSD`, `DeleteOSD` — `ter:ActionNotSupported`. Truthful: the rects are
  fixed silicon, so dynamic tokens would be bookkeeping over a fixed array.
- `GetOSDOptions` advertises `Type=[Text]`,
  `Position=[UpperLeft, UpperRight, LowerLeft, LowerRight]`,
  `TextString Type=[Plain, DateAndTime]`, `FontSize 16..16`, and the 16 palette
  colours.
- The `@OSD` capability at `onvif/types/media.rs:1255` flips to true.

Position mirrors to both channels, scaled for the sub frame's dimensions. No
separate main/sub configuration.

## WebUI — `cross-compile/www`

`services/osdService.ts` mirroring `imagingService.ts`, plus
`pages/settings/OsdPage.tsx`, a route and a nav entry. The colour picker renders
the 16 palette swatches. Colour and alpha sit in their own section labelled
device-wide, so the global constraint is visible rather than surprising.

**Skipped:** a client-side placement preview. `LiveViewPage` already shows the
stream, so the burned-in result *is* the preview, with a couple of seconds of
latency. Add one only if that lag proves annoying.

## Testing

Host-side: glyph encoding and non-ASCII rejection; layout across 4 corners × 2
channels; shrink-padding; config serde round-trip; the SOAP ops against the
existing fixture pattern; the React page under Vitest.

Hardware is the only place the central assumption can be tested — see Phase 0.

## Phase 0 — spike

Everything above was paper reasoning about a foreign binary, so it was tested on
hardware before committing to implementation.

### Stage A — library load and font parse — **PASSED on `.198`, 2026-08-24**

A standalone probe (`/tmp/osd_probe.c`, linked against our lib set plus
`-lmpi_osd`) was pushed to `/mnt/anyka_hack/osd-spike/` and run with
`LD_LIBRARY_PATH` covering the scratch dir and
`/mnt/anyka_hack/vendor-daemon/lib`. It needs no VI handle, so it ran alongside
the live stack without disturbing it:

```
[probe] binary started -- so libmpi_osd.so LOADED ok
[probe] ak_osd_get_version() = libmpi_osd V1.1.03
[ak_osd_set_font_file:242] channel=1, font size=16
[get_font_data_from_file:89] fd:4 byte:32
[probe] ak_osd_set_font_file OK  (RSS 508 -> 552 kB, +44)
[probe] PASS
```

Confirmed:

- The dynamic linker loads `libmpi_osd.so` alongside our full SDK lib set with
  `ak_cmd_{,un}register_module` unresolvable — the lazy-binding analysis holds in
  practice, not just on paper.
- Code inside the library executes: version reports `libmpi_osd V1.1.03`.
- The on-camera font at `/usr/local/ak_font_16.bin` (265798 bytes) parses.

Two facts learned that were not visible from the headers:

- **Glyphs are 32 bytes each — 16×16 at 1bpp**, not the 4bpp format that
  `ak_osd_draw_matrix` documents. The library expands them internally, so the
  `draw_str` path needs no bitmap work from us.
- **Font loading costs only ~44 KB RSS**, not the file's 259 KB: the library
  holds the fd open and reads glyphs on demand. On a box reporting 2.7 MB free
  (`free`: 36540 total / 33840 used, 25140 reclaimable) this is the difference
  between viable and not.

Note the linker precedent this rests on already exists in the tree: the
`vendor-daemon` Makefile passes `-Wl,--allow-shlib-undefined` for the *same*
lazy-binding reason with `libplat_thread.so`.

### Stage B — live draw — **PASSED on `.198` (192.168.2.198), 2026-08-24**

`HELLO OSD` was burned into the live main stream (1280×720 RTSP `/main`) after
`CMD_OSD_*` handlers, `ak_cmd_*_module` stubs, and an ISP mem/context-attr
layout wrap were in place.

Findings that Stage A could not see:

1. **`ak_osd_init` always calls `osd_sys_ipc_register`**, which needs
   `ak_cmd_register_module`. Lazy binding only deferred the crash until live
   init — it does not avoid it. Fixed with no-op stubs in
   `osd_ipcsrv_stubs.c` matching the reference signatures
   `(unsigned port, const char *name)`.
2. **Main-channel font height is `font_file_size * 2` (32px)**; ASCII advance
   is half of that (16px). A 16×16 rect fails `ak_osd_set_rect` validation.
3. **ISP `MEM_ATTR` / `CONTEXT_ATTR` wire layout on this camera does not match
   `libmpi_osd`'s `(chn, …)` prefix.** dmesg showed
   `paddr:(null) size:-2134570752` where the size was our paddr `0x80c50900`
   as int32 — the ISP was reading `AK_ISP_USER_PARAM` from byte 0 so `id`
   occupied `chn` and `data[0]` (our chn=0) became paddr. Fixed by
   `osd_vpss_wrap.c` rewriting both payloads before `ak_vpss_osd_set_param`.
4. **Deploy path is the A/B slot**, not `/mnt/anyka_hack/vendor-daemon/`:
   active slot `b` → `/mnt/anyka_hack/slots/b/vendor-daemon/`.
5. **Never kill `anyka-init`** to restart the stack — it owns Wi-Fi; killing it
   drops the camera off the network until reboot.

Memory after a successful single-rect draw on main (one 144×32 overlay):

```
free:     Mem free ≈ 3.3 MB (unchanged vs pre-OSD ~2.9 MB noise)
vendor-daemon VmRSS: 2336 kB  (pre-OSD baseline was ~2340 kB)
DMA request for the rect: 4608 bytes (ping-pong); canvas: 2304 bytes
```

Both channels fitting is still to be confirmed once Rust enables sub as well;
the per-rect pmem cost is small enough that two rects × two channels stays
under a few tens of KB.

### Phase 6 — E2E on `192.168.2.198` (2026-08-24)

Deployed slot **b** (`onvif-rust.bin` + `www/` with `OsdPage-*.js`). Live config:
`/mnt/anyka_hack/onvif/config.toml` with `[osd]` enabled.

**Main stream (`/main`, 1280×720): PASS**

- `HELLO OSD` upper-left + ISO timestamp lower-right, both visible simultaneously
- Timestamp advances once per second without blanking the name
- `GetOSDs` / `SetOSD` SOAP verified (corner move applies within ~2 s)

**Renderer (single-canvas, 2026-08-24):** this ISP path only composites one
OSD DMA plane per video channel — `osd_vpss_wrap` drops the rect index, so
rect 1 `draw_str` replaced rect 0 (and vice versa), which looked like flicker.
Fix: one full-frame canvas on silicon rect 0; name and datetime are painted
into that shared buffer; only dirty strings are redrawn each tick.

**Sub stream (`/sub`, 640×360): PASS (2026-08-24, after `vi_attr_wrap`)**

- Root cause was not "ISP has no sub OSD". `ak_vi_set_channel_attr` on this
  `libplat_vi` stores the sub frame size in **main.max_*** (libre quirk) and
  leaves `chn_sub` at 0; `ak_vi_get_channel_attr` does not fill `res[SUB]`, so
  libmpi_osd's `get_resolution(1)` read stack garbage → `get_max_rect` returned
  absurd dims → `set_rect` never reached DMA (`sub osd addr: (null)` was a
  consequence).
- Fix: `vi_attr_wrap.c` synthesizes `res[SUB]` from `main.max_*` on every
  `ak_vi_get_channel_attr`. Init now reports `chn=1 max_rect=640x360`; DMA
  size 230400; timestamp (+ name) visible on `/sub` RTSP.

**Memory (dual overlay on main, both rects):**

```
onvif-rust VmRSS:  ~6168 kB  (+~3.8 MB vs pre-OSD daemon stack — includes full ONVIF stack, not OSD alone)
vendor-daemon VmRSS: ~2408 kB  (≈ Stage B baseline)
```

**WebUI:** `index-iHUnpIzG.js` served from slot `www/`; `OsdPage-DjOJGvsB.js` HTTP 200.

### Scratch state left on the camera

`/mnt/anyka_hack/osd-spike/` on `.198` holds `libmpi_osd.so` and `osd_probe`
(both md5-verified after transfer). Harmless, outside the A/B slots, and reusable
for Stage B; delete when the feature lands.

## Acceptance verification on `.198` — 2026-08-24

Run against the final build (`onvif-rust` md5 `7c3c364d…`, `vendor-daemon.bin`
unchanged at `9e3f55b0…`), captured over RTSP with `ffmpeg`. Evidence:
`validation/osd_e2e/verify-0*.jpg`.

| Check | Result |
|---|---|
| Both overlays on main (1280×720) | ✅ `HELLO OSD` upper-left, timestamp lower-right |
| Both overlays on sub (640×360) | ✅ same, at half font size |
| Timestamp advances once per second | ✅ `10:58:02` → `10:59:34` → `11:01:38` |
| `GetOSDs` lists both fixed tokens | ✅ `osd_name`, `osd_datetime` |
| Palette on the wire | ✅ `X="255" Y="127" Z="127" Colorspace=…/YCbCr` — vendor index 1, white |
| `DeleteOSD osd_datetime` | ✅ drops from `GetOSDs` **and** vanishes from video, no residue |
| `CreateOSD osd_datetime` | ✅ returns the token, overlay comes back ticking |
| `[osd] enabled = false` + restart | ✅ completely clean frame — no frozen clock |

The last three are the disable path, which had never run on hardware: before
the fix, `osd_set_enable` was only ever called with `true`, so turning an
overlay off froze its last text on the video permanently.

Memory with both channels drawing: `vendor-daemon` VmRSS **2560 kB** against a
~2340 kB pre-OSD baseline — about **220 kB** for two channels, on a box
reporting 2.8 MB free. Both channels fit.

## Risks

| Risk | Mitigation |
|---|---|
| ~~`libmpi_osd.so` fails to load on this silicon~~ | **Retired** — Stage A passed on `.198`, 2026-08-24 |
| ~~`ak_osd_init` / `draw_str` misbehave against a live VI handle~~ | **Retired** — Stage B drew `HELLO OSD` on live main, 2026-08-24 |
| OSD buffers exhaust physical memory | Font +44 KB; one 144×32 rect ≈ 7 KB pmem; VmRSS unchanged within noise |
| Someone later calls `osd_sys_ipc_register` | Stubs absorb it; comment at the link site and in `handlers_osd.c` |
| ISP mem/context attr layout mismatch | `osd_vpss_wrap.c` rewrites payloads for this camera's ISP |
| VPSS OSD interacts badly with our profile resolutions | Exercised in the spike at both main and sub dimensions |
