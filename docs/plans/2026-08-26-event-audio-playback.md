# Event Audio Playback Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Play a short PCM clip through the camera's built-in speaker when selected events fire (boot ready, network change, upgrade result) or when an operator triggers one from the WebUI.

**Architecture:** Mechanism in C, policy in Rust. The vendor-daemon gains a `CMD_AUDIO_PLAY` verb backed by `libplat_ao.so` — one detached worker thread that opens AO, sets volume, streams the file in chunks, and closes. It is deliberately dumb: *"play this file at this volume, or tell me you're BUSY."* All policy that is worth testing (debounce, event→clip mapping, drop-when-busy, config) lives in onvif-rust where it runs on the host without hardware.

**Tech Stack:** C99 (uClibc, armv5te), Rust (onvif-rust, tokio), Anyka `ak_ao` SDK, raw PCM s16le mono.

**Design doc:** `docs/plans/2026-08-26-event-audio-playback-design.md`

**Hardware facts already verified on `.198`** (do not re-litigate these):
- The speaker is real; raw PCM at 8 kHz mono s16le plays, exit 0.
- Playback does not disturb streaming — daemon holds `/dev/akpcm_cdev1` read-only for capture, playback is a separate fd.
- `libplat_ao.so` in the repo is md5-identical (`42abcbe4…`) to the camera's stock vendor lib.
- `ak_adec_demo` hardcodes `dac=6, aslc=2` — that is why playback was deafening, not a broken API.

---

## Task 1: Vendor `libplat_ao.so` and `ak_ao.h` into the daemon build

**Files:**
- Create: `cross-compile/vendor-daemon/lib/libplat_ao.so` (copy)
- Create: `cross-compile/vendor-daemon/include/ak_ao.h` (copy)
- Modify: `cross-compile/vendor-daemon/Makefile` (LDFLAGS, ~line 84)

**Step 1: Copy the lib and header from the reference tree**

```bash
cd /home/kmk/dev/anyka-dev
cp cross-compile/anyka_reference/IOT-ANYKA-PTZdaemon/libs/libplat_ao.so \
   cross-compile/vendor-daemon/lib/libplat_ao.so
cp cross-compile/anyka_reference/venc_demo/include/ak_ao.h \
   cross-compile/vendor-daemon/include/ak_ao.h
```

**Step 2: Verify the lib is the same build the camera runs**

```bash
md5sum cross-compile/vendor-daemon/lib/libplat_ao.so
```
Expected: `42abcbe47b3d3950493e8d07d91d53b1`

If it differs, STOP — you have the wrong lib generation and will hit the
version-gate failure mode that killed venc on `.121`.

**Step 3: Add `-lplat_ao` to the link group**

In `cross-compile/vendor-daemon/Makefile`, inside `-Wl,--start-group`, add after `-lplat_ai`:

```make
	  -lplat_ao \
```

**Step 4: Build and confirm it still links**

Run: `make -C cross-compile/vendor-daemon release`
Expected: builds clean. A new undefined-symbol error means the group order is wrong — `-lplat_ao` must be inside the `--start-group`/`--end-group` pair.

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/lib/libplat_ao.so \
        cross-compile/vendor-daemon/include/ak_ao.h \
        cross-compile/vendor-daemon/Makefile
git commit -m "build(vendor-daemon): link libplat_ao for audio output"
```

---

## Task 2: Protocol — add the play verbs and a BUSY status

**Files:**
- Modify: `cross-compile/vendor-daemon/src/protocol.h`

**Step 1: Add the BUSY status**

After `VD_STATUS_STALE_EPOCH` (~line 16):

```c
/* A play request arrived while the single AO worker was busy. Distinct from
 * STATUS_ERROR so the client can drop the sound quietly rather than log a fault:
 * there is one DAC, and a chime backlog is worse than a missed chime. */
#define VD_STATUS_BUSY          (-3)
```

**Step 2: Add the command IDs**

After `CMD_AUDIO_STOP_PUSH = 58,`:

```c
    /* Audio playback (speaker).  Backed by libplat_ao.so.  Rust owns all policy:
     * event->clip mapping, debounce and drop-if-busy.  The daemon just plays a
     * file. Async: the response means "accepted", not "finished". */
    CMD_AUDIO_PLAY                = 59,
    CMD_AUDIO_STOP                = 60,
```

**Step 3: Verify it compiles**

Run: `make -C cross-compile/vendor-daemon release`
Expected: builds clean (nothing references the new IDs yet).

**Step 4: Commit**

```bash
git add cross-compile/vendor-daemon/src/protocol.h
git commit -m "feat(vendor-daemon): add CMD_AUDIO_PLAY/STOP and VD_STATUS_BUSY"
```

---

## Task 3: Request parsing (TDD — this is the real logic in C)

The wire format is
`[u32 sample_rate][u32 channel_num][i32 volume][u32 path_len][path bytes]`.
Parsing is where a malformed or hostile request does damage, so it is a pure
function with a test.

**Files:**
- Create: `cross-compile/vendor-daemon/src/sound.h`
- Create: `cross-compile/vendor-daemon/src/sound.c`
- Create: `cross-compile/vendor-daemon/tests/test_sound_parse.c`

**Step 1: Write the failing test**

Create `cross-compile/vendor-daemon/tests/test_sound_parse.c`:

```c
/* Host-compiled unit test for sound play-request parsing. */
#include <assert.h>
#include <stdio.h>
#include <string.h>
#include <stdint.h>

#include "sound.h"

static uint32_t put_u32(uint8_t *b, uint32_t off, uint32_t v)
{
    b[off] = v & 0xff; b[off+1] = (v >> 8) & 0xff;
    b[off+2] = (v >> 16) & 0xff; b[off+3] = (v >> 24) & 0xff;
    return off + 4;
}

int main(void)
{
    uint8_t buf[128];
    struct sound_req r;

    /* A well-formed request round-trips. */
    uint32_t o = 0;
    o = put_u32(buf, o, 8000);   /* rate */
    o = put_u32(buf, o, 1);      /* channels */
    o = put_u32(buf, o, 3);      /* volume */
    o = put_u32(buf, o, 9);      /* path_len */
    memcpy(buf + o, "/x/a.raw", 9); o += 9;
    assert(sound_parse_play_req(buf, o, &r) == 0);
    assert(r.sample_rate == 8000);
    assert(r.channel_num == 1);
    assert(r.volume == 3);
    assert(strcmp(r.path, "/x/a.raw") == 0);

    /* Truncated header is rejected. */
    assert(sound_parse_play_req(buf, 8, &r) != 0);

    /* path_len lying beyond the buffer is rejected (no overread). */
    o = put_u32(buf, 12, 999);
    assert(sound_parse_play_req(buf, 25, &r) != 0);
    put_u32(buf, 12, 9);

    /* Volume is clamped to the DAC's 0-6 range, not passed through. */
    put_u32(buf, 8, 99);
    assert(sound_parse_play_req(buf, 25, &r) == 0);
    assert(r.volume == 6);
    put_u32(buf, 8, (uint32_t)-5);
    assert(sound_parse_play_req(buf, 25, &r) == 0);
    assert(r.volume == 0);
    put_u32(buf, 8, 3);

    /* A path with no NUL terminator is rejected. */
    memcpy(buf + 16, "AAAAAAAAA", 9);
    assert(sound_parse_play_req(buf, 25, &r) != 0);

    printf("test_sound_parse: OK\n");
    return 0;
}
```

**Step 2: Create the header**

`cross-compile/vendor-daemon/src/sound.h`:

```c
#ifndef VENDOR_DAEMON_SOUND_H
#define VENDOR_DAEMON_SOUND_H

#include <stdint.h>

#define SOUND_PATH_MAX      256
#define SOUND_VOLUME_MAX    6     /* ak_ao dac range is [0,6]; 0 is mute */

struct sound_req {
    unsigned int sample_rate;
    unsigned int channel_num;
    int          volume;
    char         path[SOUND_PATH_MAX];
};

/* Parse a CMD_AUDIO_PLAY payload. Returns 0 on success, -1 on malformed input.
 * Volume is clamped to [0, SOUND_VOLUME_MAX]. */
int sound_parse_play_req(const uint8_t *req, uint32_t req_len, struct sound_req *out);

/* Start playback on the worker thread. Returns 0 if accepted, 1 if busy,
 * -1 on failure to start. */
int sound_play_async(const struct sound_req *req);

/* True if the worker is currently playing. */
int sound_is_playing(void);

#endif /* VENDOR_DAEMON_SOUND_H */
```

**Step 3: Run the test to verify it fails**

Run:
```bash
cd cross-compile/vendor-daemon
gcc -std=gnu99 -Isrc -Iinclude -o /tmp/test_sound_parse \
    tests/test_sound_parse.c src/sound.c && /tmp/test_sound_parse
```
Expected: FAIL — `src/sound.c` does not exist yet.

**Step 4: Write the minimal implementation**

`cross-compile/vendor-daemon/src/sound.c` (parsing only for now):

```c
#include <string.h>
#include <stdint.h>

#include "sound.h"

static uint32_t rd_u32(const uint8_t *b, uint32_t off)
{
    return (uint32_t)b[off]
         | ((uint32_t)b[off+1] << 8)
         | ((uint32_t)b[off+2] << 16)
         | ((uint32_t)b[off+3] << 24);
}

int sound_parse_play_req(const uint8_t *req, uint32_t req_len, struct sound_req *out)
{
    if (req == NULL || out == NULL || req_len < 16)
        return -1;

    out->sample_rate = rd_u32(req, 0);
    out->channel_num = rd_u32(req, 4);

    int vol = (int)rd_u32(req, 8);
    if (vol < 0)                 vol = 0;
    if (vol > SOUND_VOLUME_MAX)  vol = SOUND_VOLUME_MAX;
    out->volume = vol;

    uint32_t path_len = rd_u32(req, 12);
    /* Reject a length that overruns the buffer or the path field. Checked
     * before any copy, so a lying path_len can never overread. */
    if (path_len == 0 || path_len > SOUND_PATH_MAX || 16 + path_len > req_len)
        return -1;
    /* The sender must NUL-terminate; we never synthesise one. */
    if (req[16 + path_len - 1] != '\0')
        return -1;

    memcpy(out->path, req + 16, path_len);
    return 0;
}
```

**Step 5: Run the test to verify it passes**

Run:
```bash
gcc -std=gnu99 -Isrc -Iinclude -o /tmp/test_sound_parse \
    tests/test_sound_parse.c src/sound.c && /tmp/test_sound_parse
```
Expected: `test_sound_parse: OK`

**Step 6: Commit**

```bash
git add cross-compile/vendor-daemon/src/sound.h \
        cross-compile/vendor-daemon/src/sound.c \
        cross-compile/vendor-daemon/tests/test_sound_parse.c
git commit -m "feat(vendor-daemon): parse and validate sound play requests"
```

---

## Task 4: Add a `test` target so the C tests stop being tribal knowledge

The two existing tests (`test_push_slots.c`, `test_ring_epoch.c`) are in no
Makefile target and no CI job. Wire all three up now.

**Files:**
- Modify: `cross-compile/vendor-daemon/Makefile`

**Step 1: Add the target**

```make
HOST_CC ?= gcc
TESTS = test_sound_parse

test:
	@for t in $(TESTS); do \
	  $(HOST_CC) -std=gnu99 -Wall -Wextra -Isrc -Iinclude \
	      -o $(BUILD_DIR)/$$t tests/$$t.c src/sound.c || exit 1; \
	  $(BUILD_DIR)/$$t || exit 1; \
	done
```

Add `test` to the `.PHONY` line (~line 114).

**Step 2: Run it**

Run: `make -C cross-compile/vendor-daemon test`
Expected: `test_sound_parse: OK`

**Step 3: Commit**

```bash
git add cross-compile/vendor-daemon/Makefile
git commit -m "test(vendor-daemon): add a host test target"
```

---

## Task 5: The AO worker thread

**Files:**
- Modify: `cross-compile/vendor-daemon/src/sound.c`

**Step 1: Add the worker**

Append to `src/sound.c` (and add the includes at the top):

```c
#include <pthread.h>
#include <stdio.h>
#include <time.h>

#include "log.h"
#include "ak_ao.h"

/* One DAC, one worker. `playing` is guarded by `lock` rather than atomics
 * because the busy check and the thread spawn must be one critical section —
 * two racing CMD_AUDIO_PLAY calls must not both win. */
static pthread_mutex_t lock = PTHREAD_MUTEX_INITIALIZER;
static int             playing;
static struct sound_req current;

#define SOUND_CHUNK_BYTES   2048
/* A wedged play must not hold the DAC forever and block every later sound.
 * Clips are chimes; anything past this is a fault, not a long file. */
#define SOUND_MAX_MS        30000

static long elapsed_ms(const struct timespec *start)
{
    struct timespec now;
    clock_gettime(CLOCK_MONOTONIC, &now);
    return (now.tv_sec - start->tv_sec) * 1000
         + (now.tv_nsec - start->tv_nsec) / 1000000;
}

static void *sound_thread(void *arg)
{
    (void)arg;
    struct timespec start;
    clock_gettime(CLOCK_MONOTONIC, &start);

    FILE *fp = fopen(current.path, "rb");
    if (fp == NULL) {
        log_warn("[sound] cannot open %s", current.path);
        goto done;
    }

    struct pcm_param param;
    param.sample_rate = current.sample_rate;
    param.sample_bits = 16;               /* s16le; the only format we ship */
    param.channel_num = current.channel_num;

    void *ao = ak_ao_open(&param);
    if (ao == NULL) {
        log_error("[sound] ak_ao_open failed rate=%u ch=%u",
                  param.sample_rate, param.channel_num);
        fclose(fp);
        goto done;
    }

    /* The vendor demo pins this to 6 (max) plus 2 of ASLC gain, which is why
     * playback was deafening. We honour the configured level and leave ASLC off. */
    ak_ao_set_dac_volume(ao, current.volume);
    ak_ao_set_aslc_volume(ao, 0);

    unsigned char buf[SOUND_CHUNK_BYTES];
    size_t n;
    unsigned long long sent = 0;
    while ((n = fread(buf, 1, sizeof(buf), fp)) > 0) {
        if (elapsed_ms(&start) > SOUND_MAX_MS) {
            log_warn("[sound] watchdog: aborting %s after %d ms",
                     current.path, SOUND_MAX_MS);
            break;
        }
        if (ak_ao_send_frame(ao, buf, (int)n, 0) < 0) {
            log_warn("[sound] send_frame failed after %llu bytes", sent);
            break;
        }
        sent += n;
    }

    ak_ao_notice_frame_end(ao);
    ak_ao_close(ao);
    fclose(fp);
    log_info("event=sound_played path=%s bytes=%llu volume=%d",
             current.path, sent, current.volume);

done:
    pthread_mutex_lock(&lock);
    playing = 0;
    pthread_mutex_unlock(&lock);
    return NULL;
}

int sound_is_playing(void)
{
    pthread_mutex_lock(&lock);
    int p = playing;
    pthread_mutex_unlock(&lock);
    return p;
}

int sound_play_async(const struct sound_req *req)
{
    pthread_mutex_lock(&lock);
    if (playing) {
        pthread_mutex_unlock(&lock);
        return 1;                       /* busy */
    }
    current = *req;
    playing = 1;
    pthread_mutex_unlock(&lock);

    pthread_t tid;
    if (pthread_create(&tid, NULL, sound_thread, NULL) != 0) {
        pthread_mutex_lock(&lock);
        playing = 0;
        pthread_mutex_unlock(&lock);
        log_error("[sound] pthread_create failed");
        return -1;
    }
    pthread_detach(tid);
    return 0;
}
```

**Step 2: Keep the host test compiling**

`sound.c` now includes `ak_ao.h` and `log.h`. The host test does not link the SDK,
so guard the worker with a compile-time switch **or** split it into `sound_worker.c`.
Prefer the split — the test target then compiles only the pure file:

- Move everything added in Step 1 into `cross-compile/vendor-daemon/src/sound_worker.c`.
- Leave `sound.c` as parsing only.

**Step 3: Verify both still build**

```bash
make -C cross-compile/vendor-daemon test      # host: parse test only
make -C cross-compile/vendor-daemon release   # ARM: full daemon
```
Expected: both succeed.

**Step 4: Commit**

```bash
git add cross-compile/vendor-daemon/src/sound_worker.c cross-compile/vendor-daemon/src/sound.c
git commit -m "feat(vendor-daemon): AO playback worker with watchdog"
```

---

## Task 6: IPC handlers and dispatcher wiring

**Files:**
- Modify: `cross-compile/vendor-daemon/src/handlers_audio.h`
- Modify: `cross-compile/vendor-daemon/src/handlers_audio.c`
- Modify: `cross-compile/vendor-daemon/src/dispatcher.c:~296`

**Step 1: Declare the handlers**

In `handlers_audio.h`, before the `#endif`:

```c
int handle_audio_play(int fd, const uint8_t *req, uint32_t req_len);
int handle_audio_stop(int fd, const uint8_t *req, uint32_t req_len);
```

**Step 2: Implement them**

Append to `handlers_audio.c` (add `#include "sound.h"` at the top):

```c
/*
 * CMD_AUDIO_PLAY — wire: [u32 rate][u32 ch][i32 volume][u32 path_len][path].
 * Async: STATUS_OK means "accepted", not "finished". VD_STATUS_BUSY means a
 * clip is already playing and this one was dropped — expected, not a fault.
 */
int handle_audio_play(int fd, const uint8_t *req, uint32_t req_len)
{
    struct sound_req sr;
    if (sound_parse_play_req(req, req_len, &sr) != 0) {
        log_warn("[sound] play: malformed request (%u bytes)", req_len);
        return send_response(fd, STATUS_ERROR, NULL, 0);
    }
    int rc = sound_play_async(&sr);
    if (rc == 1) {
        log_debug("[sound] play: busy, dropping %s", sr.path);
        return send_response(fd, VD_STATUS_BUSY, NULL, 0);
    }
    if (rc < 0)
        return send_response(fd, STATUS_ERROR, NULL, 0);
    return send_response(fd, STATUS_OK, NULL, 0);
}

/* CMD_AUDIO_STOP — reserved. There is no ak_ao cancel that is safe mid-frame;
 * the watchdog in the worker bounds playback instead. Reports current state. */
int handle_audio_stop(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[sound] stop: no-op (playing=%d)", sound_is_playing());
    return send_response(fd, STATUS_OK, NULL, 0);
}
```

**Step 3: Wire the dispatcher**

In `dispatcher.c`, after the `CMD_AUDIO_STOP_PUSH` case (~line 296):

```c
    /* --- Audio playback --- */
    case CMD_AUDIO_PLAY:
        ret = handle_audio_play(fd, req_buf, req_len);
        break;
    case CMD_AUDIO_STOP:
        ret = handle_audio_stop(fd, req_buf, req_len);
        break;
```

**Step 4: Build**

Run: `make -C cross-compile/vendor-daemon release`
Expected: builds clean.

**Step 5: Commit**

```bash
git add cross-compile/vendor-daemon/src/handlers_audio.h \
        cross-compile/vendor-daemon/src/handlers_audio.c \
        cross-compile/vendor-daemon/src/dispatcher.c
git commit -m "feat(vendor-daemon): wire CMD_AUDIO_PLAY to the AO worker"
```

---

## Task 7: Generate the clip set

**Files:**
- Create: `scripts/make_sounds.py`
- Create: `SD_card_contents/anyka_hack/onvif/sounds/{boot,ok,alert}.raw`

**Step 1: Write the generator (stdlib only — no ffmpeg dependency)**

`scripts/make_sounds.py`:

```python
#!/usr/bin/env python3
"""Generate the shipped PCM clip set (s16le mono, 8 kHz).

8 kHz is the rate verified on hardware. Amplitude is deliberately low: the
speaker is loud enough to resonate the plastic casing at the vendor demo's
hardcoded max, so loudness is a property of the file as well as the DAC volume.
"""
import math
import struct
import pathlib

RATE = 8000
OUT = pathlib.Path(__file__).parent.parent / "SD_card_contents/anyka_hack/onvif/sounds"

def tone(freqs, secs, amp=0.25):
    """Sequence of (freq, duration) notes with a short fade to avoid DAC clicks."""
    buf = bytearray()
    for freq, dur in freqs:
        n_total = int(RATE * dur)
        fade = min(400, n_total // 4)
        for n in range(n_total):
            env = min(1.0, n / fade, (n_total - n) / fade) if fade else 1.0
            v = int(32767 * amp * env * math.sin(2 * math.pi * freq * n / RATE))
            buf += struct.pack("<h", v)
    return bytes(buf)

CLIPS = {
    "boot.raw":  [(660, 0.12), (880, 0.18)],   # rising: up and running
    "ok.raw":    [(880, 0.10), (1170, 0.14)],  # short confirmation
    "alert.raw": [(520, 0.18), (420, 0.26)],   # falling: something is wrong
}

if __name__ == "__main__":
    OUT.mkdir(parents=True, exist_ok=True)
    for name, notes in CLIPS.items():
        data = tone(notes, None)
        (OUT / name).write_bytes(data)
        print(f"{name}: {len(data)} bytes ({len(data)/2/RATE:.2f}s)")
```

Note the `tone(notes, None)` call takes the note list; drop the unused `secs`
parameter when you write it for real.

**Step 2: Generate and check the sizes**

Run: `uv run python3 scripts/make_sounds.py`
Expected: three files, each well under 10 KB.

**Step 3: Commit**

```bash
git add scripts/make_sounds.py SD_card_contents/anyka_hack/onvif/sounds/
git commit -m "feat(sound): generate the shipped PCM clip set"
```

---

## Task 8: Device gate — prove the daemon path works before writing Rust

Do not proceed to the Rust side until this passes. This is the checkpoint that
catches an AO-open failure or a wrong link before it is buried under policy code.

**Step 1: Deploy the daemon and a clip**

```bash
uv run python3 - <<'PY'
import ftplib
f = ftplib.FTP("192.168.2.198", timeout=30); f.login("root", "www123")
f.cwd("/mnt/anyka_hack/slots/a/vendor-daemon")
for local, remote in [
    ("cross-compile/vendor-daemon/build/vendor-daemon.bin", "vendor-daemon.bin.new"),
    ("cross-compile/vendor-daemon/lib/libplat_ao.so", "lib/libplat_ao.so"),
]:
    with open(local, "rb") as fh:
        f.storbinary(f"STOR {remote}", fh)
print("uploaded")
f.quit()
PY
```

**Step 2: Record the pre-state**

Run: `uv run python3 scripts/debugging/cam_exec.py 'ps | grep -E "onvif|vendor" | grep -v grep'`
Expected: two PIDs. Write them down — the post-check compares against these.

**Step 3: Swap in the binary and let the supervisor restart the pair**

Note: killing onvif-rust kills the daemon too; they always restart together.

**Step 4: Trigger a play over IPC and confirm sound**

Expected: the clip is audible, `event=sound_played` appears in
`/mnt/logs/vendor_daemon.log`, and `bytes=` matches the clip size.

**Step 5: Confirm nothing else broke**

```bash
uv run python3 scripts/debugging/cam_exec.py \
  'ps | grep -E "onvif|vendor" | grep -v grep' 'free | head -2'
```
Expected: both processes alive, RSS not materially changed. Confirm RTSP still streams.

**Step 6: Verify the BUSY path**

Fire two plays back to back. Expected: the second returns `VD_STATUS_BUSY` and is
dropped; the first finishes cleanly. Exactly one clip is heard.

---

## Task 9: Rust config (TDD)

**Files:**
- Create: `cross-compile/onvif-rust/src/config/sound.rs`
- Modify: `cross-compile/onvif-rust/src/config/mod.rs`
- Modify: `cross-compile/onvif-rust/src/config/storage.rs:42`
- Modify: `SD_card_contents/anyka_hack/onvif/config.toml`

Run all Rust commands from `cross-compile/` with `--target x86_64-unknown-linux-gnu`.

**Step 1: Write the failing test**

In `src/config/sound.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_safe_and_quiet() {
        let c = SoundConfig::default();
        assert!(!c.enabled, "sound must be opt-in");
        assert!(c.volume <= 6, "volume must stay in the DAC range");
        assert!(c.events.is_empty());
    }

    #[test]
    fn volume_above_dac_range_is_clamped() {
        let c: SoundConfig = toml::from_str("enabled = true\nvolume = 99").unwrap();
        assert_eq!(c.volume, 6);
    }

    #[test]
    fn unmapped_event_resolves_to_no_clip() {
        let c: SoundConfig = toml::from_str(
            "enabled = true\n[events]\nboot_ready = \"boot.raw\"",
        ).unwrap();
        assert_eq!(c.clip_for("boot_ready"), Some("boot.raw"));
        assert_eq!(c.clip_for("network_lost"), None);
    }
}
```

**Step 2: Run it and watch it fail**

Run: `toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust sound`
Expected: FAIL — `SoundConfig` does not exist.

**Step 3: Implement `SoundConfig`**

Fields: `enabled: bool`, `clip_dir: String` (default `"sounds"`), `volume: u8`
(default 3, clamped to 6 on deserialize), `debounce_secs: u64` (default 30),
`events: BTreeMap<String, String>`. Add `clip_for(&self, event: &str) -> Option<&str>`.

Register it on `ConfigStorage` as `#[serde(default)] pub sound: SoundConfig`.

**Step 4: Run the tests**

Expected: PASS.

**Step 5: Add the stanza to the shipped config**

Append to `SD_card_contents/anyka_hack/onvif/config.toml`:

```toml
[sound]
enabled = true
clip_dir = "sounds"
volume = 3
debounce_secs = 30

[sound.events]
boot_ready = "boot.raw"
network_lost = "alert.raw"
network_up = "ok.raw"
upgrade_result = "ok.raw"
```

**Step 6: Commit**

```bash
git add cross-compile/onvif-rust/src/config/ SD_card_contents/anyka_hack/onvif/config.toml
git commit -m "feat(sound): add [sound] config with clamped volume"
```

---

## Task 10: `SoundPlayer` policy (TDD)

**Files:**
- Create: `cross-compile/onvif-rust/src/platform/anyka/sound.rs`
- Modify: `cross-compile/onvif-rust/src/platform/anyka/mod.rs`

**Step 1: Write the failing tests**

Cover exactly the behaviours that can be wrong, against a fake sink that records calls:

```rust
#[test] fn disabled_config_plays_nothing() { }
#[test] fn unmapped_event_plays_nothing() { }
#[test] fn first_event_plays() { }
#[test] fn repeat_within_debounce_is_dropped() { }
#[test] fn repeat_after_debounce_plays_again() { }
#[test] fn debounce_is_per_event_not_global() { }
#[test] fn busy_response_is_not_an_error() { }
#[test] fn clip_path_is_joined_under_clip_dir() { }
```

`debounce_is_per_event_not_global` is the one that matters: a network chime must
not suppress the upgrade chime.

**Step 2: Run and watch them fail**

Run: `toolchain/arm-anykav200-crosstool-ng/bin/cargo test --target x86_64-unknown-linux-gnu -p onvif-rust sound`

**Step 3: Implement**

`SoundPlayer` holds the config, a clock, and `Mutex<HashMap<String, Instant>>` of
last-played times. `play(&self, event: &str)`:
1. return if disabled;
2. return if the event has no clip;
3. return if within `debounce_secs` of that event's last play;
4. record the time, send `CMD_AUDIO_PLAY`, treat `VD_STATUS_BUSY` as a normal drop.

Inject the clock so debounce is testable without sleeping.

**Step 4: Tests pass. Step 5: Commit.**

```bash
git commit -m "feat(sound): SoundPlayer with per-event debounce"
```

---

## Task 11: IPC send path

**Files:**
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/mod.rs` (command constants)
- Modify: `cross-compile/onvif-rust/src/hal/anyka/ipc/audio.rs`

Add `CMD_AUDIO_PLAY = 59` / `CMD_AUDIO_STOP = 60` alongside the existing
`CMD_AI_*` constants, and a method encoding
`[u32 rate][u32 ch][i32 volume][u32 path_len][path\0]` — mirroring
`ai_open`'s use of `send_request`. Map `-3` to a `Busy` outcome rather than an error.

Commit: `feat(sound): send CMD_AUDIO_PLAY over IPC`

---

## Task 12: Wire the four events

**Files:**
- Modify: `cross-compile/onvif-rust/src/platform/anyka/supervisor.rs` (boot ready)
- Modify: the network state-change path (`network_lost` / `network_up`)
- Modify: the upgrade applier (`upgrade_result`)

One `player.play("boot_ready")`-style call per site. Fire-and-forget: no call site
blocks or awaits on audio — a yielding await costs ~12 ms on this hardware.

Add one test per site asserting the call happens on transition **and not on every
poll** — a level-triggered network check would otherwise chime forever.

Commit: `feat(sound): play clips on boot, network change and upgrade result`

---

## Task 13: Device verification

Run through all four events on `.198`:

| Event | How to trigger | Expected |
| --- | --- | --- |
| `boot_ready` | restart the pair | one chime shortly after start |
| `network_lost` / `network_up` | drop Wi-Fi briefly | one chime each way, **no repeat storm** |
| `upgrade_result` | trial-commit an A/B bundle | one chime on commit |

After each: daemon PIDs unchanged, RTSP still streaming, `free` not materially
changed, `event=sound_played` in the daemon log.

Then verify on `.127` (zt9101, different rootfs) before any fleet rollout.

**Deploying the config:** the device's `config.toml` is the DEBUG variant and
diverges from the repo copy. Do **not** push the repo file wholesale — it would
overwrite live logging and stream-profile settings. Append the `[sound]` stanza in
place with `sed -i`, back up first, and re-read the file to confirm.

---

## Phase 3 (defer): WebUI trigger

Add a "play test sound" control to the WebUI, backed by an endpoint that calls
`SoundPlayer::play`. Follow `camera-webui-components` for the component and
`anyka-webui-testing` for the tests. Not needed for the event feature to ship.

---

## Out of scope

Motion detection, ONVIF Events build-out, clip upload UI, per-event volume,
two-way audio. All additive; none blocks this.
