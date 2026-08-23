# OSD Overlay Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Burn a camera name and a live timestamp into the encoded video, configurable over ONVIF Media OSD operations and from a new WebUI settings page.

**Architecture:** VPSS composites text into the YUV frame *before* the H.264 encoder, so the overlay reaches RTSP, HTTP-FLV and snapshots alike. The C `vendor-daemon` gains five dumb draw primitives backed by `libmpi_osd.so`; all policy — timezone, formatting, layout, glyph encoding, the 1 Hz tick — lives in Rust where it is host-testable. The WebUI reaches it through ONVIF SOAP like every other settings page.

**Tech Stack:** C (armv5te uClibc, vendored crosstool-ng), Rust (tokio, chrono, quick-xml, mockall), React 19 + TypeScript + Vitest.

**Design doc:** `docs/plans/2026-08-24-osd-overlay-design.md` — read it first. Phase 0 Stage A already passed on hardware; `libmpi_osd.so` and `ak_osd.h` are already staged in the repo.

---

## Critical constraints

Violating any of these breaks the camera, not just the feature.

1. **Never call `osd_sys_ipc_register` or `osd_sys_ipc_unregister`.** They are the only reachers of `ak_cmd_register_module` / `ak_cmd_unregister_module`, which do not exist in our `libplat_ipcsrv.so`. There is no `BIND_NOW`, so lazy binding keeps the library healthy right up until someone calls one of those two, at which point it crashes at the PLT.
2. **Never renumber existing command IDs** in `protocol.h`. It is a wire protocol; a client/daemon pair can be mid-upgrade. Append only. This is documented at `protocol.h:55`.
3. **ASCII only for OSD text.** `/usr/local/ak_font_16.bin` is a GB2312 font — it has no `ó`, `ł` or `ü`. Reject non-ASCII at validation rather than rendering garbage.
4. **Colour and alpha are device-global.** `ak_osd_set_color` and `ak_osd_set_alpha` take no channel or rect argument.
5. **ARM builds must run from the crate dir** (`cross-compile/onvif-rust/`), not the workspace root, or cargo silently links with the host toolchain.
6. Use the vendored toolchain. `source setenv.sh` sets `$CARGO`; host-side Rust work always passes `--target x86_64-unknown-linux-gnu`.

---

# Phase 1 — C daemon primitives

This phase is Stage B of the Phase 0 spike. **It must be validated on hardware before any Rust, ONVIF or WebUI work starts.** If `ak_osd_init` fails against a live VI handle, the whole design is void.

### Task 1: Wire `libmpi_osd.so` into the build

**Files:**
- Modify: `cross-compile/vendor-daemon/Makefile:73-96` (the `LDFLAGS` block)
- Already staged, no action needed: `cross-compile/vendor-daemon/lib/libmpi_osd.so`, `cross-compile/vendor-daemon/include/ak_osd.h`

**Step 1: Add the library to the link group**

In `LDFLAGS`, inside the `--start-group` / `--end-group` block, after `-lplat_vpss`, add:

```make
	  -lmpi_osd \
```

**Step 2: Document the trap directly above the group**

Add this comment immediately before `-Wl,--start-group`:

```make
# -lmpi_osd comes from the IOT-ANYKA-PTZdaemon bundle, not this SDK generation.
# It has two undefined symbols we cannot satisfy — ak_cmd_register_module and
# ak_cmd_unregister_module — because our libplat_ipcsrv.so exports
# ak_cmd_server_register instead. Disassembly shows both are reachable ONLY from
# osd_sys_ipc_register/osd_sys_ipc_unregister, and there is no BIND_NOW, so lazy
# binding never resolves them. NEVER call those two functions: doing so crashes
# at the PLT. Verified loading on .198 2026-08-24 (libmpi_osd V1.1.03).
```

**Step 3: Verify it links**

```bash
cd cross-compile/vendor-daemon && make clean && make 2>&1 | tail -20
```
Expected: builds to `build/vendor-daemon.bin` with no new warnings.

**Step 4: Verify the symbol is actually wanted**

```bash
readelf -d cross-compile/vendor-daemon/build/vendor-daemon.bin | grep -i mpi_osd
```
Expected: a `NEEDED` line for `libmpi_osd.so`.

**Step 5: Commit**

```bash
rtk git add cross-compile/vendor-daemon/Makefile
rtk git commit -m "build(vendor-daemon): link libmpi_osd for OSD support"
```

---

### Task 2: Add OSD command IDs to the wire protocol

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h:48` (after `CMD_VI_SET_FLIP_MIRROR = 21`)

**Step 1: Append the command IDs**

Append-only, immediately after `CMD_VI_SET_FLIP_MIRROR = 21,`:

```c
    /* OSD (appended — see the renumbering warning below).
     * Backed by libmpi_osd.so.  Rust owns all policy: timezone, string
     * formatting, layout math and the 1 Hz tick.  These are dumb primitives. */
    CMD_OSD_INIT                  = 22,
    CMD_OSD_SET_RECT              = 23,
    CMD_OSD_DRAW_STR              = 24,
    CMD_OSD_SET_ENABLE            = 25,
    CMD_OSD_SET_STYLE             = 26,
```

**Step 2: Verify it still compiles**

```bash
cd cross-compile/vendor-daemon && make 2>&1 | tail -5
```
Expected: clean build.

**Step 3: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/protocol.h
rtk git commit -m "feat(vendor-daemon): reserve OSD command IDs 22-26"
```

---

### Task 3: Implement the OSD handlers

**Files:**
- Create: `cross-compile/vendor-daemon/src/handlers_osd.h`
- Create: `cross-compile/vendor-daemon/src/handlers_osd.c`

Follow the shape of `handlers_vpss.c` (simplest handler) and `handlers_vi.c` (token resolution).

**Step 1: Write the header**

`handlers_osd.h`:

```c
#ifndef VENDOR_DAEMON_HANDLERS_OSD_H
#define VENDOR_DAEMON_HANDLERS_OSD_H

#include <stdint.h>

int handle_osd_init(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_set_rect(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_draw_str(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_set_enable(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_set_style(int fd, const uint8_t *req, uint32_t req_len);

/** Tear down OSD state.  Call from the VI close path, before ak_vi_close. */
void osd_shutdown(void);

#endif /* VENDOR_DAEMON_HANDLERS_OSD_H */
```

**Step 2: Write the implementation**

`handlers_osd.c`:

```c
/*
 * OSD handlers — thin wrappers over libmpi_osd.so.
 *
 * Deliberately dumb: no timers, no strftime, no timezone handling.  Rust owns
 * all of that because it already has the config, the timezone and chrono, and
 * because policy there is host-testable.  These handlers only draw what they
 * are told, where they are told.
 *
 * NEVER call osd_sys_ipc_register/osd_sys_ipc_unregister from this file — see
 * the warning in the Makefile LDFLAGS block.  They crash at the PLT.
 */
#include <string.h>
#include <stdlib.h>

#include "handlers_osd.h"
#include "globals.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"
#include "ak_osd.h"

#define OSD_FONT_PATH   "/usr/local/ak_font_16.bin"
#define OSD_FONT_SIZE   16
#define OSD_MAX_CHANNEL 1      /* channels are 0 (main) and 1 (sub) */
#define OSD_MAX_RECT    2      /* rects are 0..2 */
#define OSD_MAX_GLYPHS  128    /* bounds CMD_OSD_DRAW_STR; a rect cannot show more */

/* Set once by handle_osd_init so osd_shutdown() knows whether to destroy. */
static int g_osd_ready = 0;

/**
 * osd_args_valid - Bounds-check a channel/rect pair from an untrusted request.
 *
 * The vendor library does not validate these and indexes arrays with them, so
 * an out-of-range value is a memory-safety problem, not just a wrong picture.
 */
static int osd_args_valid(int32_t channel, int32_t rect)
{
    return channel >= 0 && channel <= OSD_MAX_CHANNEL &&
           rect >= 0 && rect <= OSD_MAX_RECT;
}

/**
 * handle_osd_init - IPC handler for CMD_OSD_INIT.
 *
 * Request: [u64 vi_token]
 * Response: [i32 main_w][i32 main_h][i32 sub_w][i32 sub_h] — the per-channel
 * max rect, which Rust needs for its layout math.
 *
 * Font file must be set BEFORE ak_osd_init; that ordering is what
 * platform/libmpi/demo/osd_demo does and it is load-bearing.
 */
int handle_osd_init(int fd, const uint8_t *req, uint32_t req_len)
{
    void *handle = NULL;
    int32_t dims[4] = { 0, 0, 0, 0 };
    int channel;

    if (req_len < 8) {
        log_error("[osd] init: short request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0) {
        log_error("[osd] init: bad VI token");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    if (ak_osd_set_font_file(OSD_FONT_SIZE, OSD_FONT_PATH) < 0) {
        log_error("[osd] init: set_font_file(%s) failed", OSD_FONT_PATH);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (ak_osd_init(handle) < 0) {
        log_error("[osd] init: ak_osd_init failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    for (channel = 0; channel <= OSD_MAX_CHANNEL; channel++) {
        int w = 0, h = 0;
        if (ak_osd_get_max_rect(channel, &w, &h) < 0) {
            log_error("[osd] init: get_max_rect(chn=%d) failed", channel);
            ak_osd_destroy();
            return send_response(fd, STATUS_ERROR, NULL, 0);
        }
        dims[channel * 2]     = (int32_t)w;
        dims[channel * 2 + 1] = (int32_t)h;
        log_info("[osd] init: chn=%d max_rect=%dx%d", channel, w, h);
    }

    g_osd_ready = 1;
    return send_response(fd, STATUS_OK, dims, sizeof(dims));
}

/**
 * handle_osd_set_rect - IPC handler for CMD_OSD_SET_RECT.
 *
 * Request: [u64 vi_token][i32 channel][i32 rect][i32 x][i32 y][i32 w][i32 h]
 */
int handle_osd_set_rect(int fd, const uint8_t *req, uint32_t req_len)
{
    void *handle = NULL;
    int32_t channel, rect, x, y, w, h;

    if (req_len < 8 + 6 * 4) {
        log_error("[osd] set_rect: short request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (vd_obj_resolve(req_read_u64(req, 0), VD_OBJ_KIND_VI, &handle) != 0)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    channel = req_read_i32(req, 8);
    rect    = req_read_i32(req, 12);
    x       = req_read_i32(req, 16);
    y       = req_read_i32(req, 20);
    w       = req_read_i32(req, 24);
    h       = req_read_i32(req, 28);

    if (!osd_args_valid(channel, rect)) {
        log_error("[osd] set_rect: bad chn=%d rect=%d", channel, rect);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    if (ak_osd_set_rect(handle, channel, rect, x, y, w, h) < 0) {
        log_error("[osd] set_rect: chn=%d rect=%d %dx%d@%d,%d failed",
                  channel, rect, w, h, x, y);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_osd_draw_str - IPC handler for CMD_OSD_DRAW_STR.
 *
 * Request: [i32 channel][i32 rect][i32 x][i32 y][u16 glyph_count][u16 glyphs...]
 *
 * Glyphs are already vendor-encoded by Rust (ASCII: u16 == byte).  Rust also
 * space-pads a shrinking string to its previous length, which is why there is
 * no CMD_OSD_CLEAN_STR — the vendor's own osd_disp_stat does exactly this.
 */
int handle_osd_draw_str(int fd, const uint8_t *req, uint32_t req_len)
{
    int32_t channel, rect, x, y;
    uint32_t count;
    unsigned short *glyphs;
    uint32_t i;
    int rc;

    if (req_len < 18) {
        log_error("[osd] draw_str: short request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    channel = req_read_i32(req, 0);
    rect    = req_read_i32(req, 4);
    x       = req_read_i32(req, 8);
    y       = req_read_i32(req, 12);
    count   = (uint32_t)req[16] | ((uint32_t)req[17] << 8);

    if (!osd_args_valid(channel, rect)) {
        log_error("[osd] draw_str: bad chn=%d rect=%d", channel, rect);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    if (count == 0 || count > OSD_MAX_GLYPHS || req_len < 18 + count * 2) {
        log_error("[osd] draw_str: bad glyph count %u (req_len=%u)", count, req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    glyphs = malloc(count * sizeof(unsigned short));
    if (!glyphs)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    /* Decode little-endian u16 explicitly rather than casting the request
     * buffer: it may not be 2-byte aligned, and armv5te faults on that. */
    for (i = 0; i < count; i++)
        glyphs[i] = (unsigned short)(req[18 + i * 2] |
                                     ((unsigned short)req[19 + i * 2] << 8));

    rc = ak_osd_draw_str(channel, rect, x, y, glyphs, (int)count);
    free(glyphs);

    if (rc < 0) {
        log_error("[osd] draw_str: chn=%d rect=%d failed", channel, rect);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_osd_set_enable - IPC handler for CMD_OSD_SET_ENABLE.
 *
 * Request: [i32 channel][i32 rect][i32 enable]
 */
int handle_osd_set_enable(int fd, const uint8_t *req, uint32_t req_len)
{
    int32_t channel, rect, enable;

    if (req_len < 12)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    channel = req_read_i32(req, 0);
    rect    = req_read_i32(req, 4);
    enable  = req_read_i32(req, 8);

    if (!osd_args_valid(channel, rect))
        return send_response(fd, STATUS_ERROR, NULL, 0);

    if (ak_osd_set_rect_enable(channel, rect, enable ? 1 : 0) < 0) {
        log_error("[osd] set_enable: chn=%d rect=%d en=%d failed",
                  channel, rect, enable);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_osd_set_style - IPC handler for CMD_OSD_SET_STYLE.
 *
 * Request: [i32 front_color][i32 bg_color][i32 edge_color][i32 alpha]
 *
 * All four are DEVICE-GLOBAL in the vendor API — no channel, no rect.  The
 * ONVIF layer advertises this honestly rather than faking per-OSD colour.
 */
int handle_osd_set_style(int fd, const uint8_t *req, uint32_t req_len)
{
    int32_t front, bg, edge, alpha;

    if (req_len < 16)
        return send_response(fd, STATUS_ERROR, NULL, 0);

    front = req_read_i32(req, 0);
    bg    = req_read_i32(req, 4);
    edge  = req_read_i32(req, 8);
    alpha = req_read_i32(req, 12);

    if (front < 0 || front > 15 || bg < 0 || bg > 15 ||
        edge < 0 || edge > 15 || alpha < 1 || alpha > 100) {
        log_error("[osd] set_style: out of range front=%d bg=%d edge=%d alpha=%d",
                  front, bg, edge, alpha);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }

    if (ak_osd_set_color(front, bg) < 0 ||
        ak_osd_set_edge_color(edge) < 0 ||
        ak_osd_set_alpha(alpha) < 0) {
        log_error("[osd] set_style: vendor call failed");
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    return send_response(fd, STATUS_OK, NULL, 0);
}

void osd_shutdown(void)
{
    if (g_osd_ready) {
        ak_osd_destroy();
        g_osd_ready = 0;
        log_info("[osd] destroyed");
    }
}
```

**Step 3: Verify it compiles**

```bash
cd cross-compile/vendor-daemon && make 2>&1 | tail -10
```
Expected: clean build, no warnings (the Makefile uses `-Wall -Wextra`).

**Step 4: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/handlers_osd.c cross-compile/vendor-daemon/src/handlers_osd.h
rtk git commit -m "feat(vendor-daemon): add OSD draw primitives"
```

---

### Task 4: Wire the handlers into the dispatcher

**Files:**
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c:16` (includes), `:32-57` (`is_lifecycle_cmd`), `:227` (dispatch switch)
- Modify: `cross-compile/vendor-daemon/src/handlers_vi.c` (call `osd_shutdown()` in the VI close path)

**Step 1: Add the include**

After `#include "handlers_vpss.h"`:

```c
#include "handlers_osd.h"
```

**Step 2: Mark `CMD_OSD_INIT` as a lifecycle command**

In `is_lifecycle_cmd`, add alongside the other VI/VPSS cases:

```c
    case CMD_OSD_INIT:
```

Only `CMD_OSD_INIT` is lifecycle — it allocates hardware buffers and takes a VI handle. The draw/enable/style commands are per-frame cosmetics and must stay available to non-control clients, matching how `CMD_VENC_SET_RC` is treated.

**Step 3: Add the dispatch arms**

After the VPSS block in the switch:

```c
    /* --- OSD --- */
    case CMD_OSD_INIT:
        ret = handle_osd_init(fd, req_buf, req_len);
        break;
    case CMD_OSD_SET_RECT:
        ret = handle_osd_set_rect(fd, req_buf, req_len);
        break;
    case CMD_OSD_DRAW_STR:
        ret = handle_osd_draw_str(fd, req_buf, req_len);
        break;
    case CMD_OSD_SET_ENABLE:
        ret = handle_osd_set_enable(fd, req_buf, req_len);
        break;
    case CMD_OSD_SET_STYLE:
        ret = handle_osd_set_style(fd, req_buf, req_len);
        break;
```

**Step 4: Tear down on VI close**

In `handlers_vi.c`, in `handle_vi_close`, call `osd_shutdown();` *before* `ak_vi_close`. OSD buffers are bound to the VI handle, so destroying after the handle goes away is a use-after-free. Add `#include "handlers_osd.h"` at the top.

**Step 5: Verify**

```bash
cd cross-compile/vendor-daemon && make clean && make 2>&1 | tail -10
```
Expected: clean build.

**Step 6: Commit**

```bash
rtk git add cross-compile/vendor-daemon/src/dispatcher.c cross-compile/vendor-daemon/src/handlers_vi.c
rtk git commit -m "feat(vendor-daemon): dispatch OSD commands, destroy on VI close"
```

---

### Task 5: 🚦 HARDWARE GATE — prove a string reaches the video

**This is the gate. Do not start Phase 2 until a drawn string is visible in the stream.**

**Files:** none — this is a deploy-and-observe task.

**Step 1: Deploy the daemon to `.198`**

Follow @anyka-embedded-build. Push `build/vendor-daemon.bin` and `lib/libmpi_osd.so` to `/mnt/anyka_hack/vendor-daemon/` and `/mnt/anyka_hack/vendor-daemon/lib/` respectively. Back up the existing `vendor-daemon.bin` first — there is already a `.bak` and `.prev` convention in that directory.

md5-verify both files after transfer. Silent NUL-byte writes on the SD card are a known failure mode here.

**Step 2: Restart the stack and confirm it comes up**

```bash
cd scripts/debugging && uv run python3 cam_exec.py --timeout 40 'pidof vendor-daemon.bin onvif-rust.bin' 'tail -30 /mnt/logs/vendor_daemon.log'
```
Expected: both PIDs present. Note that `onvif-rust` and `vendor-daemon` restart together — killing one kills both.

**Step 3: Drive a hardcoded draw**

Write a throwaway client that connects to `/tmp/vd-ctrl.sock`, performs the `CMD_HELLO` handshake, and issues `CMD_OSD_INIT` → `CMD_OSD_SET_RECT` → `CMD_OSD_SET_STYLE` → `CMD_OSD_DRAW_STR` with the ASCII glyphs for `HELLO OSD`. The VI token must come from the already-attached session, so the simplest route is a temporary debug command in `onvif-rust` rather than a separate process — only one client may hold VI.

**Step 4: Observe the stream**

Pull a snapshot or open the RTSP main stream and confirm `HELLO OSD` is visible and burned in.

Expected: text appears in the requested corner. If `ak_osd_init` returns `-1`, stop — capture `/mnt/logs/vendor_daemon.log` and reassess the design before proceeding.

**Step 5: Measure the memory cost**

```bash
cd scripts/debugging && uv run python3 cam_exec.py --timeout 30 'free' 'cat /proc/$(pidof vendor-daemon.bin)/status | grep -i vm'
```
Record the delta versus the pre-OSD baseline in the design doc. `ak_osd_init` calls `akuio_alloc_pmem` per channel; on a box with ~2.7 MB free this is the number that decides whether both channels can be enabled at once.

**Step 6: Record the result in the design doc and commit**

Update the "Stage B" section of `docs/plans/2026-08-24-osd-overlay-design.md` with the outcome and the measured memory cost.

```bash
rtk git add docs/plans/2026-08-24-osd-overlay-design.md
rtk git commit -m "spike: Stage B results — OSD draws on live video"
```

---

# Phase 2 — Rust pure logic (TDD, host-side)

No hardware needed. Every task here is `$CARGO test --target x86_64-unknown-linux-gnu`.

### Task 6: Glyph encoding

**Files:**
- Create: `cross-compile/onvif-rust/src/osd/mod.rs`
- Create: `cross-compile/onvif-rust/src/osd/encode.rs`
- Modify: `cross-compile/onvif-rust/src/lib.rs` (add `pub mod osd;`)

**Step 1: Write the failing tests**

In `encode.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_glyphs_ascii_maps_to_char_codes() {
        assert_eq!(encode_glyphs("AB").unwrap(), vec![0x41, 0x42]);
    }

    #[test]
    fn test_encode_glyphs_rejects_non_ascii() {
        // The vendor font is GB2312 and has no Latin diacritics, so this is a
        // hardware limit, not a policy choice.
        let err = encode_glyphs("Ogród").unwrap_err();
        assert!(err.contains("ASCII"), "error should explain why: {err}");
    }

    #[test]
    fn test_encode_glyphs_rejects_empty() {
        assert!(encode_glyphs("").is_err());
    }

    #[test]
    fn test_pad_to_erase_appends_spaces() {
        // A shrinking string must overwrite the tail of the previous one,
        // because the daemon has no clean_str command.
        assert_eq!(pad_to_erase(vec![0x41], 3), vec![0x41, 0x20, 0x20]);
    }

    #[test]
    fn test_pad_to_erase_leaves_longer_string_alone() {
        assert_eq!(pad_to_erase(vec![0x41, 0x42], 1), vec![0x41, 0x42]);
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::encode 2>&1 | tail -20
```
Expected: FAIL — `encode_glyphs` not found.

**Step 3: Implement**

```rust
//! Glyph encoding for the vendor OSD library.
//!
//! `ak_osd_draw_str` takes `unsigned short` codes, not bytes.  The vendor's own
//! `asc_to_short` (platform/libapp/src/osd_ex/ak_osd_ex.c) defines the mapping:
//! a byte below 0x80 becomes its own value, and a GBK pair packs as
//! `(hi << 8) | lo`.  We only ever emit the first case — see `encode_glyphs`.

/// Maximum glyphs the daemon will accept in one `CMD_OSD_DRAW_STR`.
pub const MAX_GLYPHS: usize = 128;

/// ASCII space, used to erase the tail of a previously longer string.
const GLYPH_SPACE: u16 = 0x20;

/// Encode text into vendor glyph codes.
///
/// ASCII only. `/usr/local/ak_font_16.bin` is a GB2312 font with no Latin
/// diacritic glyphs, so accepting non-ASCII would render garbage rather than
/// the user's text. Rejecting it here is the honest behaviour.
pub fn encode_glyphs(text: &str) -> Result<Vec<u16>, String> {
    if text.is_empty() {
        return Err("OSD text must not be empty".to_string());
    }
    if !text.is_ascii() {
        return Err(format!(
            "OSD text must be ASCII: the camera font is GB2312 and has no glyph \
             for the non-ASCII characters in {text:?}"
        ));
    }
    if text.len() > MAX_GLYPHS {
        return Err(format!(
            "OSD text is {} characters, maximum is {MAX_GLYPHS}",
            text.len()
        ));
    }
    Ok(text.bytes().map(u16::from).collect())
}

/// Pad `glyphs` with spaces up to `previous_len`.
///
/// The daemon has no clean_str command, so a shrinking string would otherwise
/// leave the tail of its predecessor on screen. This mirrors what the vendor's
/// `osd_disp_stat` does.
pub fn pad_to_erase(mut glyphs: Vec<u16>, previous_len: usize) -> Vec<u16> {
    while glyphs.len() < previous_len.min(MAX_GLYPHS) {
        glyphs.push(GLYPH_SPACE);
    }
    glyphs
}
```

`mod.rs`:

```rust
//! On-screen display: camera name and timestamp burned into the video.
//!
//! Policy lives here rather than in the C daemon — see
//! docs/plans/2026-08-24-osd-overlay-design.md.

pub mod encode;
pub mod layout;
```

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::encode 2>&1 | tail -20
```
Expected: 5 passed.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/osd/ cross-compile/onvif-rust/src/lib.rs
rtk git commit -m "feat(osd): ASCII glyph encoding for the vendor OSD library"
```

---

### Task 7: Layout math

**Files:**
- Create: `cross-compile/onvif-rust/src/osd/layout.rs`

The vendor math, from `osd_disp_name`: an ASCII glyph advances `font_size / 2` = 8px and is 16px tall. Right-aligned text starts at `width - 8 * len`; left-aligned at one font-size inset (16px).

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MAIN: ChannelDims = ChannelDims { width: 1280, height: 720 };

    #[test]
    fn test_place_upper_left_insets_by_one_font_size() {
        let p = place(Corner::UpperLeft, 9, MAIN);
        assert_eq!((p.x, p.y), (16, 0));
    }

    #[test]
    fn test_place_upper_right_is_right_aligned_by_glyph_width() {
        // 9 ASCII glyphs at 8px each = 72px wide.
        let p = place(Corner::UpperRight, 9, MAIN);
        assert_eq!((p.x, p.y), (1280 - 72, 0));
    }

    #[test]
    fn test_place_lower_left_sits_one_line_above_the_bottom() {
        let p = place(Corner::LowerLeft, 9, MAIN);
        assert_eq!((p.x, p.y), (16, 720 - 16));
    }

    #[test]
    fn test_place_lower_right_combines_both_edges() {
        let p = place(Corner::LowerRight, 9, MAIN);
        assert_eq!((p.x, p.y), (1280 - 72, 720 - 16));
    }

    #[test]
    fn test_place_clamps_overlong_text_to_zero_rather_than_negative() {
        // 400 glyphs would start at a negative x and the vendor library does
        // not bounds-check; clamping keeps the draw on screen.
        let p = place(Corner::UpperRight, 400, MAIN);
        assert_eq!(p.x, 0);
    }

    #[test]
    fn test_place_scales_to_the_sub_channel() {
        let sub = ChannelDims { width: 640, height: 360 };
        let p = place(Corner::LowerRight, 9, sub);
        assert_eq!((p.x, p.y), (640 - 72, 360 - 16));
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::layout 2>&1 | tail -20
```
Expected: FAIL — `place` not found.

**Step 3: Implement**

```rust
//! OSD placement math.
//!
//! Ports the vendor's own layout from `osd_disp_name`
//! (platform/libapp/src/osd_ex/ak_osd_ex.c): an ASCII glyph advances
//! `font_size / 2` pixels and occupies `font_size` vertically.  Pure functions
//! only, so this is fully testable without hardware.

use serde::{Deserialize, Serialize};

/// The only font size the camera has: `/usr/local/ak_font_16.bin`.
pub const FONT_SIZE: i32 = 16;

/// ASCII glyphs are half-width — 8px at a 16px font size.
pub const GLYPH_ADVANCE: i32 = FONT_SIZE / 2;

/// Which corner an OSD sits in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    UpperLeft,
    UpperRight,
    LowerLeft,
    LowerRight,
}

/// Usable dimensions of one video channel, from `CMD_OSD_INIT`'s max-rect reply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelDims {
    pub width: i32,
    pub height: i32,
}

/// Where to start drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Placement {
    pub x: i32,
    pub y: i32,
}

/// Compute the draw origin for `glyph_count` ASCII glyphs in `corner`.
///
/// Clamps to zero rather than returning a negative origin: the vendor library
/// does not bounds-check `ak_osd_draw_str`, so an overlong string would
/// otherwise index outside the OSD buffer.
pub fn place(corner: Corner, glyph_count: usize, dims: ChannelDims) -> Placement {
    let text_width = GLYPH_ADVANCE * glyph_count as i32;

    let x = match corner {
        Corner::UpperLeft | Corner::LowerLeft => FONT_SIZE,
        Corner::UpperRight | Corner::LowerRight => (dims.width - text_width).max(0),
    };
    let y = match corner {
        Corner::UpperLeft | Corner::UpperRight => 0,
        Corner::LowerLeft | Corner::LowerRight => (dims.height - FONT_SIZE).max(0),
    };

    Placement { x, y }
}
```

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::layout 2>&1 | tail -20
```
Expected: 6 passed.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/osd/layout.rs
rtk git commit -m "feat(osd): placement math ported from the vendor layout"
```

---

### Task 8: Timestamp formatting

**Files:**
- Create: `cross-compile/onvif-rust/src/osd/format.rs`
- Modify: `cross-compile/onvif-rust/src/osd/mod.rs`

**Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> chrono::DateTime<chrono::Utc> {
        chrono::Utc.with_ymd_and_hms(2026, 8, 24, 15, 4, 5).unwrap()
    }

    #[test]
    fn test_format_iso_date_with_24h_clock() {
        let s = format_datetime(sample(), DateFormat::Iso, TimeFormat::H24);
        assert_eq!(s, "2026-08-24 15:04:05");
    }

    #[test]
    fn test_format_european_date_with_12h_clock() {
        let s = format_datetime(sample(), DateFormat::European, TimeFormat::H12);
        assert_eq!(s, "24/08/2026 03:04:05 PM");
    }

    #[test]
    fn test_format_us_date() {
        let s = format_datetime(sample(), DateFormat::Us, TimeFormat::H24);
        assert_eq!(s, "08/24/2026 15:04:05");
    }

    #[test]
    fn test_formatted_output_is_always_ascii() {
        // Feeds straight into encode_glyphs, which rejects non-ASCII.
        for date in [DateFormat::Iso, DateFormat::European, DateFormat::Us] {
            for time in [TimeFormat::H12, TimeFormat::H24] {
                assert!(format_datetime(sample(), date, time).is_ascii());
            }
        }
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::format 2>&1 | tail -20
```
Expected: FAIL.

**Step 3: Implement**

```rust
//! Timestamp formatting for the OSD.
//!
//! Lives in Rust rather than the daemon so it can be tested on the host and so
//! the C side needs no strftime or TZ handling.

use chrono::{DateTime, TimeZone};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DateFormat {
    /// `2026-08-24`
    Iso,
    /// `24/08/2026`
    European,
    /// `08/24/2026`
    Us,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TimeFormat {
    /// `15:04:05`
    H24,
    /// `03:04:05 PM`
    H12,
}

/// Render `when` as an ASCII date-and-time string.
pub fn format_datetime<Tz: TimeZone>(
    when: DateTime<Tz>,
    date: DateFormat,
    time: TimeFormat,
) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let date_pattern = match date {
        DateFormat::Iso => "%Y-%m-%d",
        DateFormat::European => "%d/%m/%Y",
        DateFormat::Us => "%m/%d/%Y",
    };
    let time_pattern = match time {
        TimeFormat::H24 => "%H:%M:%S",
        TimeFormat::H12 => "%I:%M:%S %p",
    };
    when.format(&format!("{date_pattern} {time_pattern}")).to_string()
}
```

Add `pub mod format;` to `osd/mod.rs`.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::format 2>&1 | tail -20
```
Expected: 4 passed.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/osd/format.rs cross-compile/onvif-rust/src/osd/mod.rs
rtk git commit -m "feat(osd): ASCII timestamp formatting"
```

---

### Task 9: Config types

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/types.rs` (add `OsdConfig`, add the field to `AppConfig` at `:25`)
- Modify: `.deploy/anyka.toml` (add the `[osd]` section)

**Step 1: Write the failing test**

Add to the existing test module in `types.rs`:

```rust
#[test]
fn test_osd_config_defaults_are_sane() {
    let cfg = OsdConfig::default();
    assert!(cfg.enabled);
    assert_eq!(cfg.alpha, 80);
    assert_eq!(cfg.name.position, Corner::UpperLeft);
    assert_eq!(cfg.datetime.position, Corner::LowerRight);
}

#[test]
fn test_osd_config_round_trips_through_toml() {
    let cfg = OsdConfig::default();
    let text = toml::to_string(&cfg).unwrap();
    let back: OsdConfig = toml::from_str(&text).unwrap();
    assert_eq!(cfg, back);
}

#[test]
fn test_app_config_parses_without_an_osd_section() {
    // Existing deployed anyka.toml files have no [osd] section and must keep
    // loading — a missing section means "defaults", not "reject the config".
    let cfg: AppConfig = toml::from_str("").unwrap();
    assert!(cfg.osd.enabled);
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust config::types::tests::test_osd 2>&1 | tail -20
```
Expected: FAIL.

**Step 3: Implement**

Add to `types.rs`:

```rust
/// One text overlay: the camera name or the timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdItemConfig {
    pub enabled: bool,
    pub position: Corner,
}

/// On-screen display settings.
///
/// `color` and `alpha` are device-global, not per-item: the vendor API
/// (`ak_osd_set_color`, `ak_osd_set_alpha`) takes no channel or rect argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdConfig {
    pub enabled: bool,
    /// Index into the vendor's 16-entry colour table, 0..=15.
    pub color: u8,
    /// Overlay opacity, 1..=100.
    pub alpha: u8,
    pub name: OsdNameConfig,
    pub datetime: OsdDateTimeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdNameConfig {
    pub enabled: bool,
    pub position: Corner,
    /// Empty means "fall back to the ONVIF device name".
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OsdDateTimeConfig {
    pub enabled: bool,
    pub position: Corner,
    pub date_format: DateFormat,
    pub time_format: TimeFormat,
}
```

Write `Default` impls matching the assertions above. Add `#[serde(default)] pub osd: OsdConfig,` to `AppConfig`.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust config 2>&1 | tail -20
```
Expected: all config tests pass, including the pre-existing ones.

**Step 5: Add the section to the deployed config**

Append to `.deploy/anyka.toml` the `[osd]`, `[osd.name]` and `[osd.datetime]` blocks from the design doc.

**Step 6: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/config/types.rs .deploy/anyka.toml
rtk git commit -m "feat(config): add [osd] section with device-global colour and alpha"
```

---

# Phase 3 — Rust IPC client

### Task 10: OSD commands over the daemon socket

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs:263` (command constants), `:517-530` (`cmd_name`)
- Create: `cross-compile/onvif-rust/src/hal/anyka/ipc/osd.rs`
- Modify: `cross-compile/onvif-rust/src/hal/common/video.rs:45` (`VideoHalTrait`) or a new `OsdHalTrait` — prefer a **separate trait**, since OSD is orthogonal to video capture and adding five methods to `VideoHalTrait` forces every existing implementor and mock to grow stubs.

**Step 1: Write the failing tests**

In `osd.rs`, using the existing `FakeDaemon` helper (see `ipc/video.rs:275` for the pattern):

```rust
#[cfg(test)]
mod tests {
    use super::super::test_helpers::*;
    use super::*;

    #[test]
    fn test_osd_init_returns_channel_dims_from_daemon() {
        let daemon = FakeDaemon::start(|_cmd, _req| {
            let mut reply = Vec::new();
            for v in [1280i32, 720, 640, 360] {
                reply.extend_from_slice(&v.to_le_bytes());
            }
            (AK_SUCCESS_I32, reply)
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        let dims = ipc.osd_init(0x1234 as *mut c_void).unwrap();
        assert_eq!(dims[0].width, 1280);
        assert_eq!(dims[1].height, 360);
    }

    #[test]
    fn test_osd_draw_str_encodes_glyphs_little_endian() {
        let captured = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = captured.clone();
        let daemon = FakeDaemon::start(move |_cmd, req| {
            *sink.lock().unwrap() = req.to_vec();
            (AK_SUCCESS_I32, vec![])
        });
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        ipc.osd_draw_str(0, 1, 16, 0, &[0x41, 0x42]);

        let req = captured.lock().unwrap().clone();
        // [i32 chn][i32 rect][i32 x][i32 y][u16 count][u16 glyphs...]
        assert_eq!(&req[16..18], &2u16.to_le_bytes());
        assert_eq!(&req[18..22], &[0x41, 0x00, 0x42, 0x00]);
    }

    #[test]
    fn test_osd_set_style_rejects_out_of_range_alpha_before_ipc() {
        // Fail fast in Rust rather than burning a round trip on a value the
        // daemon will reject anyway.
        let daemon = FakeDaemon::start(|_cmd, _req| (AK_SUCCESS_I32, vec![]));
        let ipc = AnykaIpc::new_with_path(&daemon.socket_path).unwrap();
        ipc.set_epochs_for_test(1, 1);

        assert!(ipc.osd_set_style(1, 0, 0, 0).is_err());
        assert!(ipc.osd_set_style(1, 0, 0, 101).is_err());
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust hal::anyka::ipc::osd 2>&1 | tail -20
```
Expected: FAIL.

**Step 3: Implement**

Add the constants to `ipc/mod.rs` next to `CMD_VI_SET_FLIP_MIRROR`:

```rust
const CMD_OSD_INIT: i32 = 22;
const CMD_OSD_SET_RECT: i32 = 23;
const CMD_OSD_DRAW_STR: i32 = 24;
const CMD_OSD_SET_ENABLE: i32 = 25;
const CMD_OSD_SET_STYLE: i32 = 26;
```

Add matching arms to `cmd_name` so the tracing output stays readable.

Implement the five methods in `osd.rs` following the `vi_set_flip_mirror` shape at `ipc/video.rs:134`: build `req_data` with `to_le_bytes`, call `self.send_request(...)`, map errors through `error!` and a `Result`.

**Step 4: Run to verify they pass**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust hal::anyka::ipc::osd 2>&1 | tail -20
```
Expected: 3 passed.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/hal/
rtk git commit -m "feat(hal): OSD commands over the vendor-daemon IPC socket"
```

---

### Task 11: The 1 Hz renderer

**Files:**
- Create: `cross-compile/onvif-rust/src/osd/renderer.rs`
- Modify: `cross-compile/onvif-rust/src/osd/mod.rs`

The renderer holds the previous glyph count per rect so it can space-pad a shrinking string, and skips the IPC round trip entirely when nothing changed.

**Step 1: Write the failing tests**

Test the *decision* logic as a pure function — do not spin a real tokio timer in a unit test.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_plan_skips_unchanged_text() {
        let mut state = RenderState::default();
        let first = state.plan(OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        assert!(first.is_some(), "first render must draw");

        let second = state.plan(OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        assert!(second.is_none(), "unchanged text must not redraw");
    }

    #[test]
    fn test_render_plan_pads_a_shrinking_string() {
        let mut state = RenderState::default();
        state.plan(OsdRect::Name, "LONG NAME", Corner::UpperLeft, MAIN);
        let plan = state.plan(OsdRect::Name, "AB", Corner::UpperLeft, MAIN).unwrap();
        assert_eq!(plan.glyphs.len(), 9, "must overwrite the previous tail");
        assert_eq!(plan.glyphs[2], 0x20);
    }

    #[test]
    fn test_render_plan_redraws_when_the_corner_moves() {
        let mut state = RenderState::default();
        state.plan(OsdRect::Name, "CAM1", Corner::UpperLeft, MAIN);
        let moved = state.plan(OsdRect::Name, "CAM1", Corner::LowerRight, MAIN);
        assert!(moved.is_some(), "a position change must redraw");
    }

    #[test]
    fn test_render_plan_rejects_non_ascii_without_poisoning_state() {
        let mut state = RenderState::default();
        assert!(state.plan(OsdRect::Name, "Ogród", Corner::UpperLeft, MAIN).is_none());
        // The bad value must not be recorded as "last drawn".
        assert!(state.plan(OsdRect::Name, "OK", Corner::UpperLeft, MAIN).is_some());
    }
}
```

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu -p onvif-rust osd::renderer 2>&1 | tail -20
```

**Step 3: Implement**

`RenderState::plan` returns `Option<DrawPlan { channel, rect, x, y, glyphs }>`, combining `encode_glyphs`, `pad_to_erase` and `place`. The tokio task is a thin `interval(Duration::from_secs(1))` loop that calls `plan` for each rect on each channel and issues `osd_draw_str` for whatever comes back.

Mark the deliberate simplification:

```rust
// ponytail: one shared 1 Hz tick for both rects and both channels. Fine
// because only the timestamp changes per second; switch to per-rect timers
// only if a sub-second element is ever added.
```

**Step 4: Run to verify they pass**

Expected: 4 passed.

**Step 5: Commit**

```bash
rtk git add cross-compile/onvif-rust/src/osd/renderer.rs cross-compile/onvif-rust/src/osd/mod.rs
rtk git commit -m "feat(osd): 1 Hz renderer that redraws only what changed"
```

---

### Task 12: Lifecycle wiring

**Files:**
- Modify: `cross-compile/onvif-rust/src/app.rs` (stream pipeline bring-up, after VI open and VPSS init succeed)

**Step 1:** Call `osd_init`, then `osd_set_rect` for each enabled rect on each channel, then `osd_set_style`, then spawn the renderer task.

**Step 2:** Re-initialise on daemon restart. The epoch / `CMD_HELLO` handshake already signals this — find the existing reattach path rather than adding a second mechanism.

**Step 3:** If `osd_init` fails, log a warning and continue without OSD. A missing overlay must never take the video stream down with it.

**Step 4: Verify the full suite still passes**

```bash
cd cross-compile && $CARGO test --target x86_64-unknown-linux-gnu 2>&1 | tail -20
cd cross-compile && PATH="$(pwd)/../toolchain/arm-anykav200-crosstool-ng/bin:$PATH" $CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings 2>&1 | tail -20
```

**Step 5: Commit**

```bash
rtk git commit -am "feat(osd): initialise OSD with the stream pipeline"
```

---

# Phase 4 — ONVIF Media OSD operations

### Task 13: OSD types

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/types/media.rs`

Add the ONVIF 24.12 types: `OSDConfiguration`, `OSDTextConfiguration`, `OSDPosConfiguration`, `OSDColor`, `GetOSDs`/`GetOSDsResponse`, `GetOSD`/`GetOSDResponse`, `GetOSDOptions`/`GetOSDOptionsResponse`, `SetOSD`/`SetOSDResponse`.

Follow the existing serde attribute conventions in that file exactly — `@` prefixes for attributes, `skip_serializing_if` for optionals.

**Test:** serialization round-trip for `OSDConfiguration`, asserting the XML has `token` as an attribute and `Position` as a child element.

**Commit:** `feat(onvif): OSD configuration types`

---

### Task 14: OSD operations

**Files:**
- Create: `cross-compile/onvif-rust/src/onvif/media/ops/osd.rs`
- Modify: `cross-compile/onvif-rust/src/onvif/media/ops/mod.rs` (add `pub mod osd;`)

Follow `ops/video_sources.rs`: free functions taking `&ProfileManagerRef`, returning `OnvifResult<...>`.

**Tests to write first:**

```rust
#[test]
fn test_get_osds_returns_exactly_two_fixed_tokens() { /* osd_name, osd_datetime */ }

#[test]
fn test_get_osd_rejects_an_unknown_token() { /* ter:InvalidArgVal */ }

#[test]
fn test_get_osd_options_advertises_only_font_size_16() { /* the camera has one font */ }

#[test]
fn test_get_osd_options_advertises_sixteen_palette_colours() { }

#[test]
fn test_set_osd_rejects_non_ascii_plain_text() { }

#[test]
fn test_set_osd_rejects_a_font_size_other_than_16() { }

#[test]
fn test_set_osd_persists_and_returns_the_stored_value() { }

#[test]
fn test_create_osd_returns_action_not_supported() { /* fixed rects, honest fault */ }

#[test]
fn test_delete_osd_returns_action_not_supported() { }
```

**Commit:** `feat(onvif): GetOSDs, GetOSD, GetOSDOptions, SetOSD`

---

### Task 15: Dispatch and capability

**Files:**
- Modify: `cross-compile/onvif-rust/src/onvif/media/service.rs:684+` (dispatch arms), `ops/capabilities.rs:22`
- Modify: `cross-compile/onvif-rust/src/onvif/onvif/auth_requirements.rs` — OSD reads are `User` level, `SetOSD` is `Operator`, matching how `SetVideoSourceConfiguration` is classified.

Add dispatch arms for each operation following the `"GetVideoSources" =>` shape exactly, and flip the `@OSD` capability to `true`.

**Test:** a dispatch test asserting `GetOSDs` returns a well-formed SOAP body, and an auth test asserting `SetOSD` rejects an unauthenticated caller.

**Commit:** `feat(onvif): dispatch OSD ops and advertise the OSD capability`

---

# Phase 5 — WebUI

### Task 16: OSD service client

**Files:**
- Create: `cross-compile/www/src/services/osdService.ts`
- Create: `cross-compile/www/src/services/osdService.test.ts`

Mirror `imagingService.ts`: exported interfaces, `soapRequest` calls against `ENDPOINTS.media`, small pure parse helpers.

**Tests first** — mock `soapRequest`, assert the request body contains the right token and that a fault surfaces as a thrown `Error`.

```bash
cd cross-compile/www && npx vitest run src/services/osdService.test.ts
```

**Commit:** `feat(www): OSD SOAP service client`

---

### Task 17: OSD settings page

**Files:**
- Create: `cross-compile/www/src/pages/settings/OsdPage.tsx`
- Create: `cross-compile/www/src/pages/settings/OsdPage.test.tsx`

Follow `ImagingPage.tsx` for structure, TanStack Query usage and `data-testid` conventions. See @camera-webui-components and @anyka-webui-testing.

Sections:
1. **Camera name** — enable switch, text input, corner select. The text input must reject non-ASCII client-side with a message naming the reason (the camera font has no glyph for it), so users are not left guessing after a server fault.
2. **Date & time** — enable switch, corner select, date format select, time format select.
3. **Appearance** — 16 palette swatches and an alpha slider, in a section explicitly labelled as applying to the whole device.

**Tests:** renders current values; changing the corner fires the mutation; non-ASCII input shows the validation message and does *not* fire a mutation.

**Commit:** `feat(www): OSD settings page`

---

### Task 18: Route and navigation

**Files:**
- Modify: `cross-compile/www/src/router/index.tsx` (lazy import + `#/settings/osd` route)
- Modify: `cross-compile/www/src/router/index.test.tsx` (mock + route case)
- Modify: `cross-compile/www/src/Layout.tsx` (nav entry)

**Step: Run the full web gate**

```bash
cd cross-compile/www && npm run verify && npm run test
```

Note: run `prettier --check` via the raw binary and read `$?`. `rtk prettier --check` has printed "All files formatted correctly" on a real exit-1.

**Commit:** `feat(www): route and nav for the OSD settings page`

---

# Phase 6 — Deploy and validate

### Task 19: End-to-end on hardware

**Step 1:** Build the ARM binary from the crate dir:

```bash
cd cross-compile/onvif-rust && $CARGO build --release --target armv5te-unknown-linux-uclibceabi
```

**Step 2:** Build the WebUI and the daemon, then deploy per @anyka-embedded-build and @anyka-firmware-upgrade.

**Step 3:** Verify with `curl`, not `ls` — a deploy that looks present can still be serving the old bundle.

**Step 4:** Confirm on the live stream:
- name and timestamp appear in the configured corners on both main and sub;
- the timestamp advances once per second;
- changing the corner in the WebUI moves the text within a few seconds;
- disabling an OSD removes it and leaves no residue;
- a third-party ONVIF client (ONVIF Device Manager) sees both OSDs via `GetOSDs`.

**Step 5:** Re-measure memory and compare against the Task 5 baseline.

**Step 6:** Record results, update the design doc, open the PR.

---

## Definition of done

- [ ] Stage B hardware gate passed and recorded before Phase 2 began
- [ ] `$CARGO test --target x86_64-unknown-linux-gnu` green
- [ ] `$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings` clean (with the toolchain bin dir first on `PATH`, or clippy dies with E0514)
- [ ] `npm run verify && npm run test` green in `cross-compile/www`
- [ ] ARM release build succeeds **from `cross-compile/onvif-rust/`**
- [ ] OSD visible on main and sub, timestamp ticking, on `.198`
- [ ] `GetOSDs` answers a third-party ONVIF client
- [ ] Memory delta recorded; both channels fit
- [ ] `/mnt/anyka_hack/osd-spike/` cleaned off `.198`
- [ ] Design doc updated with Stage B results
