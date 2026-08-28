# Event Audio Playback Revision — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix the mono→stereo bug that made TTS clips silent, and delete the `system("ak_adec_demo")` workaround it caused.

**Architecture:** The vendor DA accepts stereo only. `sound_worker.c` must duplicate each mono sample into L+R and double the send length before `ak_ao_send_frame`, plus make the three setup calls the SDK header marks mandatory and drain to `AO_PLAY_STATUS_FINISHED` before close. With that fixed, the in-daemon `libplat_ao` path works and the shell-out, its ADEC libraries and its permanent teardown of live audio capture all get removed.

**Tech Stack:** C99 / uClibc / ARMv5TE (vendor-daemon), Rust (onvif-rust config + policy), Python 3 (clip generation), telnet + FTP for device verification.

**Reference implementation — read it before Task 1:** `cross-compile/anyka_reference/platform/libplat/demo/ao_demo/ak_ao_demo.c`. Specifically `copy_for_dual_channel` (:66), `wait_play_finished` (:80), and the setup block (:149-159). This is the *raw PCM* demo. Do **not** follow `ak_adec_demo.c` or `ak_voice_tips.c` — those route through ADEC, which is a dead end here.

**Design doc:** `docs/plans/2026-08-28-event-audio-playback-revision-design.md`

---

## Context you need before starting

**The bug in one sentence:** we called `ak_ao_send_frame(handle, mono_buffer, mono_len, 0)`; the DA read that buffer as interleaved stereo, so each channel got every other sample.

**How we know:** the daemon log on `.198` shows `ak_ao_send_frame` returned exactly 2× the bytes handed to it, on every clip (`boot 59600/29800`, `alert 95980/47990`, `ok 106436/53218`).

**Why tones worked:** a tone at double pitch is still a tone. Speech decimated 2:1 cancels when L+R are summed to one speaker.

**Build commands** (from `cross-compile/vendor-daemon/`):
- Host unit tests: `make test`
- ARM cross build: `make`

**IMPORTANT — this repo's git index is usually fully staged.** Always commit with an explicit pathspec (`git commit -m "..." -- path/one path/two`) and verify with `git show --stat`. A bare `git commit` will sweep in unrelated in-flight work.

**Baseline note:** the current `sound_worker.c` in the working tree is the shell-out version. The `ak_ao` version you are restoring is at `git show HEAD:cross-compile/vendor-daemon/src/sound_worker.c` — but it predates the fix, so restore it and then correct it.

---

## Task 1: Test-drive `sound_dup_mono_to_stereo()`

The channel duplication is the root-cause fix and the only new logic in C worth testing. It goes in `sound.c` (already compiled into the host test binary) rather than `sound_worker.c` (which links vendor libs and cannot run on the host).

**Files:**
- Modify: `cross-compile/vendor-daemon/src/sound.h`
- Modify: `cross-compile/vendor-daemon/src/sound.c`
- Test: `cross-compile/vendor-daemon/tests/test_sound_parse.c`

**Step 1: Write the failing test**

Append to `tests/test_sound_parse.c`, before `main()`:

```c
static void test_dup_mono_to_stereo_duplicates_each_sample(void)
{
    /* Three s16le mono samples: 0x0102, 0x0304, 0x0506 (little-endian bytes). */
    const unsigned char mono[6] = { 0x02, 0x01, 0x04, 0x03, 0x06, 0x05 };
    unsigned char stereo[12] = { 0 };

    int out_len = sound_dup_mono_to_stereo(mono, sizeof(mono), stereo);

    assert(out_len == 12);
    /* Sample 0 appears in both L and R. */
    assert(stereo[0] == 0x02 && stereo[1] == 0x01);
    assert(stereo[2] == 0x02 && stereo[3] == 0x01);
    /* Sample 1. */
    assert(stereo[4] == 0x04 && stereo[5] == 0x03);
    assert(stereo[6] == 0x04 && stereo[7] == 0x03);
    /* Sample 2. */
    assert(stereo[8] == 0x06 && stereo[9] == 0x05);
    assert(stereo[10] == 0x06 && stereo[11] == 0x05);
}

static void test_dup_mono_to_stereo_ignores_trailing_odd_byte(void)
{
    /* A truncated file can leave a dangling byte; it must not be half-copied. */
    const unsigned char mono[3] = { 0x02, 0x01, 0xFF };
    unsigned char stereo[8] = { 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA, 0xAA };

    int out_len = sound_dup_mono_to_stereo(mono, sizeof(mono), stereo);

    assert(out_len == 4);
    assert(stereo[0] == 0x02 && stereo[1] == 0x01);
    assert(stereo[2] == 0x02 && stereo[3] == 0x01);
    assert(stereo[4] == 0xAA);  /* untouched */
}
```

Register both in `main()` alongside the existing calls, following whatever pattern is already there.

**Step 2: Run the test to verify it fails**

```bash
cd cross-compile/vendor-daemon && make test
```

Expected: compile error, `implicit declaration of function 'sound_dup_mono_to_stereo'`.

**Step 3: Write the minimal implementation**

Declare in `src/sound.h`, above the `#endif`:

```c
/* Duplicate s16le mono samples into interleaved stereo.
 *
 * The DA accepts stereo only: handing it a mono buffer makes each channel take
 * every other sample, which halves the effective rate and doubles the pitch.
 * See ak_ao_demo.c:66 (copy_for_dual_channel).
 *
 * `dest` must have room for `len * 2` bytes. A trailing odd byte is dropped.
 * Returns the number of bytes written to `dest`.
 */
int sound_dup_mono_to_stereo(const unsigned char *src, int len, unsigned char *dest);
```

Implement in `src/sound.c`:

```c
int sound_dup_mono_to_stereo(const unsigned char *src, int len, unsigned char *dest)
{
    int samples = len / 2;
    int j;

    for (j = 0; j < samples; ++j) {
        dest[j * 4]     = src[j * 2];
        dest[j * 4 + 1] = src[j * 2 + 1];
        dest[j * 4 + 2] = src[j * 2];
        dest[j * 4 + 3] = src[j * 2 + 1];
    }
    return samples * 4;
}
```

`sound.c` will need `#include <stddef.h>` only if not already present; it currently includes `<string.h>` and `<stdint.h>`, which suffice.

**Step 4: Run the tests to verify they pass**

```bash
cd cross-compile/vendor-daemon && make test
```

Expected: all three test binaries run, PASS.

**Step 5: Commit**

```bash
git commit -m "fix(vendor-daemon): duplicate mono to stereo before ak_ao_send_frame" \
  -- cross-compile/vendor-daemon/src/sound.h \
     cross-compile/vendor-daemon/src/sound.c \
     cross-compile/vendor-daemon/tests/test_sound_parse.c
git show --stat
```

Verify only three files are in the commit.

---

## Task 2: Restore the `ak_ao` worker and apply the fix

**Files:**
- Restore then modify: `cross-compile/vendor-daemon/src/sound_worker.c`
- Restore: `cross-compile/vendor-daemon/include/ak_ao.h`

**Step 1: Restore the pre-workaround worker and header**

```bash
git checkout HEAD -- cross-compile/vendor-daemon/include/ak_ao.h
git show HEAD:cross-compile/vendor-daemon/src/sound_worker.c > cross-compile/vendor-daemon/src/sound_worker.c
```

Read the restored file. It already has: the `lock`/`playing` mutex pattern, `elapsed_ms()`, the `SOUND_MAX_MS` watchdog, `ak_ao_open`, `ak_ao_set_dac_volume`, the chunked `fread` + `send_frame` loop, `notice_frame_end`, `close`. Keep all of that.

**Step 2: Apply the six changes**

1. **Keep `spk_pa_set()`** from the shell-out version — copy it across. It is a real discovery, orthogonal to the buffer bug:

```c
#define SPK_PA_SYSFS "/sys/user-gpio/SPK_PA"

static void spk_pa_set(int enabled)
{
    FILE *f = fopen(SPK_PA_SYSFS, "w");
    if (f == NULL) {
        log_warn("[sound] cannot open %s", SPK_PA_SYSFS);
        return;
    }
    fputc(enabled ? '1' : '0', f);
    fclose(f);
}
```

2. **Add the mandatory setup calls** immediately after `ak_ao_open` succeeds and before the send loop, matching `ak_ao_demo.c:149-159`:

```c
    /* Order and necessity per ak_ao_demo.c:149-159; the header marks all three
     * mandatory before the first ak_ao_send_frame. */
    ak_ao_enable_speaker(ao, AUDIO_FUNC_ENABLE);
    ak_ao_set_dac_volume(ao, current.volume);
    ak_ao_set_aslc_volume(ao, 0);
    ak_ao_set_resample(ao, AUDIO_FUNC_DISABLE);
    ak_ao_clear_frame_buffer(ao);
```

3. **Widen the send buffer and duplicate before sending.** Replace the buffer declaration and the inner send with:

```c
    unsigned char buf[SOUND_CHUNK_BYTES];
    unsigned char stereo[SOUND_CHUNK_BYTES * 2];
```

and inside the read loop, before the send-offset loop:

```c
        int stereo_len = sound_dup_mono_to_stereo(buf, (int)n, stereo);
```

then drive the offset loop over `stereo` / `stereo_len` instead of `buf` / `n`.

**This is the root-cause fix.** `sent` must now accumulate to `2 × filesize`, which is correct — it is DA-side stereo bytes for mono input.

4. **Drain before close.** After the read loop and `ak_ao_notice_frame_end(ao)`, before `ak_ao_close(ao)`:

```c
    /* Wait for the DA to actually play what we queued. send_frame returning >0
     * means "accepted", not "played", so closing here truncates the tail.
     * ak_ao_demo.c's wait_play_finished() counts elapsed time but never checks
     * it -- an unbounded poll. Ours reuses the watchdog deadline. */
    while (ak_ao_get_play_status(ao) != AO_PLAY_STATUS_FINISHED) {
        if (elapsed_ms(&start) > SOUND_MAX_MS) {
            log_warn("[sound] watchdog: drain timed out for %s", current.path);
            break;
        }
        struct timespec ts = { .tv_sec = 0, .tv_nsec = 10 * 1000 * 1000 };
        nanosleep(&ts, NULL);
    }
```

5. **Bracket playback with the amplifier**, and release it on *every* exit path including the watchdog abort: `spk_pa_set(1)` before the first send, `spk_pa_set(0)` after the drain, before `ak_ao_enable_speaker(ao, AUDIO_FUNC_DISABLE)` and `ak_ao_close(ao)`.

6. **Do NOT call `push_stop_audio()`.** The restored file does not have it; do not re-add it. See Task 3.

**Step 3: Cross-compile**

```bash
cd cross-compile/vendor-daemon && make
```

Expected: clean build. If `ak_ao_enable_speaker`, `ak_ao_set_resample`, `ak_ao_clear_frame_buffer`, `ak_ao_get_play_status` or `AUDIO_FUNC_ENABLE` are undeclared, the restored `include/ak_ao.h` is missing them — cross-check against `cross-compile/anyka_reference/platform/libplat/include/ak_ao.h` and add the declarations.

**Step 4: Run host tests**

```bash
cd cross-compile/vendor-daemon && make test
```

Expected: PASS (unchanged; `sound_worker.c` is not host-testable).

**Step 5: Commit**

```bash
git commit -m "fix(vendor-daemon): restore ak_ao playback worker with stereo fix and drain" \
  -- cross-compile/vendor-daemon/src/sound_worker.c \
     cross-compile/vendor-daemon/include/ak_ao.h
git show --stat
```

---

## Task 3: Stop killing live audio capture

`push_stop_audio()` tears down the AI → AENC chain. `start_audio_push` is called exactly once, from `Application::start_streaming`, and `hal/anyka/ipc/mod.rs:1637-1651` documents that nothing restarts it. One chime silences RTSP and HTTP-FLV audio until onvif-rust restarts.

Task 2 already dropped the call by restoring the older file. This task confirms nothing else reintroduces it.

**Files:**
- Verify: `cross-compile/vendor-daemon/src/sound_worker.c`

**Step 1: Verify no sound path stops capture**

```bash
rtk grep -rn "push_stop_audio" cross-compile/vendor-daemon/src/
```

Expected: matches in `main.c` (shutdown), `push.c` (definition), `push.h` (declaration) — and **none** in `sound_worker.c`.

**Step 2: No commit** unless the grep found something to remove.

---

## Task 4: Delete the shell-out's dependencies

**Files:**
- Modify: `cross-compile/vendor-daemon/Makefile`
- Delete: `cross-compile/vendor-daemon/include/ak_adec.h`
- Delete: `cross-compile/vendor-daemon/lib/libmpi_adec.so`
- Delete: `cross-compile/vendor-daemon/lib/libakaudiocodec.so`
- Delete: `cross-compile/vendor-daemon/lib/libakaudiocodec.so.stock` (0 bytes — junk)

**Step 1: Restore `-lplat_ao` to the link line**

In the `LDFLAGS` block, re-add `-lplat_ao` above `-lmpi_aenc`. Keep the `HOST_TESTS` pattern-rule refactor from the working tree — that is an unrelated improvement worth keeping.

**Step 2: Remove the ADEC files**

```bash
git rm --cached cross-compile/vendor-daemon/include/ak_adec.h 2>/dev/null
rm -f cross-compile/vendor-daemon/include/ak_adec.h \
      cross-compile/vendor-daemon/lib/libmpi_adec.so \
      cross-compile/vendor-daemon/lib/libakaudiocodec.so \
      cross-compile/vendor-daemon/lib/libakaudiocodec.so.stock
```

Note: some of these are untracked; `git rm --cached` will fail harmlessly on those.

**Step 3: Confirm `libplat_ao.so` ships**

```bash
ls -la cross-compile/vendor-daemon/lib/libplat_ao.so \
       SD_card_contents/anyka_hack/vendor-daemon/lib/libplat_ao.so
```

Both must exist (32.4K). If the SD copy is missing, copy it across — the daemon cannot link without it on-device.

**Step 4: Rebuild**

```bash
cd cross-compile/vendor-daemon && make && make test
```

Expected: clean build, tests PASS.

**Step 5: Commit**

```bash
git commit -m "chore(vendor-daemon): drop the ADEC libs the shell-out needed" \
  -- cross-compile/vendor-daemon/Makefile cross-compile/vendor-daemon/include/ak_adec.h
git show --stat
```

---

## Task 5: Make `volume` real again

`[sound] volume` is currently parsed, clamped and shipped over IPC while nothing applies it — the demo hardcoded DAC 6 + ASLC 2. Task 2 restored `ak_ao_set_dac_volume(ao, current.volume)`, so it is live again. This task widens the range and restores the dropped test.

`ak_ao_demo.c:153` documents the DAC range as 0–12 (`/* volume is from 0 to 12, volume 0 is mute */`), while `ak_ao.h:114-120` documents `ak_ao_set_dac_volume` as 0–6 with the 0–12 range belonging to the combined `ak_ao_set_volume` macro. **The header is the more specific authority for the function we call: keep the clamp at 6.** Do not widen it on the strength of a demo comment.

So this task is only: restore the test that was deleted.

**Files:**
- Modify: `cross-compile/onvif-rust/src/config/sound.rs`

**Step 1: Restore the clamping deserializer and its test**

```bash
git diff HEAD -- cross-compile/onvif-rust/src/config/sound.rs
```

Re-apply the removed `deserialize_volume` function, the `SOUND_VOLUME_MAX` constant, the `#[serde(deserialize_with = ...)]` attribute, and the test:

```rust
#[test]
fn test_sound_config_volume_above_dac_range_is_clamped() {
    let c: SoundConfig = toml::from_str("enabled = true\nvolume = 99").unwrap();
    assert_eq!(c.volume, 6);
}
```

Rationale for keeping it despite the daemon also clamping: config is a trust boundary, and a silently-out-of-range value in `config.toml` should be visibly normalised, not left for the C side to catch.

**Step 2: Run the host tests**

```bash
cd cross-compile/onvif-rust && cargo test --target x86_64-unknown-linux-gnu config::sound
```

Expected: PASS.

**Step 3: Commit**

```bash
git commit -m "fix(sound): clamp configured volume to the DAC range again" \
  -- cross-compile/onvif-rust/src/config/sound.rs
```

---

## Task 6: Resync the shipped clip set

The repo holds the old tones (3 840–7 040 bytes); `.198` holds TTS (29 800–57 572 bytes). `make_speech.py` was run and deployed but only `upgrade.raw` was written back into the repo.

**Files:**
- Modify: `SD_card_contents/anyka_hack/onvif/sounds/{boot,ok,alert,upgrade}.raw`
- Delete: `scripts/make_sounds.py` (already staged as deleted; keep it deleted)

**Step 1: Regenerate**

```bash
uv run python3 scripts/make_speech.py
```

Requires `espeak-ng` and `ffmpeg` on the host. Expected output: four lines with byte counts and durations.

**Step 2: Verify the set is coherent**

```bash
ls -la SD_card_contents/anyka_hack/onvif/sounds/
```

All four files should now be TTS-sized (tens of KB), not the 3.8–7.0 KB tones. Sizes need not match `.198` byte-for-byte — espeak output varies with version — but the *shape* must match: four clips, all speech.

**Step 3: Commit**

```bash
git commit -m "feat(sound): ship the generated speech clip set" \
  -- SD_card_contents/anyka_hack/onvif/sounds/ scripts/make_speech.py scripts/make_sounds.py
git show --stat
```

---

## Task 7: Device verification on `.198`

**This is the gate. The root cause is source-grounded but unproven on hardware.**

**Step 1: Deploy**

Use the project's normal deploy path (`@anyka-embedded-build` / `scripts/deploy_onvif.sh`). Note the camera boots slot `b`; deploy the daemon and the clips to the active slot.

**Step 2: Capture the pre-play audio state**

```bash
uv run python3 scripts/debugging/cam_exec.py \
  'pidof vendor-daemon; pidof onvif-rust' \
  'grep -c "\[audio\] push stopped" /mnt/logs/vendor_daemon.log'
```

Record both numbers.

**Step 3: Trigger a play and read the instrumentation**

Trigger via the WebUI Diagnostics sound card or `POST /api/sound/play`. Then:

```bash
uv run python3 scripts/debugging/cam_exec.py \
  'tail -20 /mnt/logs/vendor_daemon.log | grep -i sound'
```

**Assertions — all must hold:**

| Check | Expected | Meaning if it fails |
| --- | --- | --- |
| `bytes=` in the log | **exactly 2 × file size** | With duplication this is now *correct*. If it equals 1×, duplication is not running. |
| wall-clock play duration | ≈ clip duration (e.g. 3.60 s for `upgrade.raw`) | Much shorter ⇒ the drain is not working. |
| no `send_frame failed` warning | absent | — |
| `pidof vendor-daemon` / `onvif-rust` | unchanged from Step 2 | Playback destabilised the daemon. |
| `[audio] push stopped` count | unchanged from Step 2 | Something still tears down capture. |

**Step 4: Listen**

Play a TTS clip. It must be **intelligible speech at normal speed and pitch** — not a chipmunk, not a beep, not silence. This is the check that the whole revision exists to satisfy.

**Step 5: Confirm live audio survived**

Open the RTSP or HTTP-FLV stream and confirm audio is still present *after* a clip has played. This is the `push_stop_audio()` regression check.

**Step 6: Record the result**

Append measured numbers to the design doc's Risks section — specifically whether removing `push_stop_audio()` caused any audible contention between capture and playback. Commit that edit.

---

## Task 8: Full quality gate

**Step 1: Run everything**

```bash
cd cross-compile/vendor-daemon && make test
cd cross-compile/onvif-rust && cargo test --target x86_64-unknown-linux-gnu
cd cross-compile/www && npx vitest run
```

Note: per prior experience, do **not** trust `rtk prettier --check` — run the raw binary and read `$?`.

**Step 2: Lint**

Clippy needs the vendored toolchain first on `PATH`:

```bash
PATH="$PWD/toolchain/arm-anykav200-crosstool-ng/bin:$PATH" cargo clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

**Step 3: Request code review**

REQUIRED SUB-SKILL: `superpowers:requesting-code-review`.

**Step 4: Finish the branch**

REQUIRED SUB-SKILL: `superpowers:finishing-a-development-branch`.

---

## Out of scope

Deliberately not touched, from the original design's non-goals: motion detection, ONVIF Events build-out, clip upload UI, two-way audio, per-event volume, playlists, looping.

Also deliberately not touched: the `HOST_TESTS` Makefile refactor and the Rust `exe_dir()` clip-path resolution in the working tree — both are genuine improvements from the workaround branch and should survive.
