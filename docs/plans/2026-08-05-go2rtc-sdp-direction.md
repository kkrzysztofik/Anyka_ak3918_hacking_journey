# go2rtc SDP Direction Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Stop emitting `a=sendonly` in our RTSP SDP so go2rtc (and therefore Frigate) can consume the camera's `/main` and `/sub` streams.

**Architecture:** Three line deletions across the two SDP generators that ship in the camera binary, each guarded by a regression test asserting the SDP carries no direction attribute at all. Then cross-compile for ARM, deploy to `192.168.2.198`, and verify on the wire that go2rtc reaches `SETUP`/`PLAY` instead of tearing down after `DESCRIBE`.

**Tech Stack:** Rust (vendored `arm-anykav200-crosstool-ng` toolchain), `onvif-rust` + `streaming-lib` crates, FTP deploy over `scripts/deploy_onvif.sh`, telnet on port 24 for camera shell, `tcpdump` on the Frigate host for verification.

**Design doc:** `docs/plans/2026-08-05-go2rtc-sdp-direction-design.md`

---

## Background you need before starting

The bug is not a malformed SDP. Our SDP is valid and ffmpeg/VLC parse it happily.
go2rtc 1.9.10 stores SDP media direction from *its own* viewpoint rather than the
SDP author's, so it only accepts a producer track whose direction is `recvonly`.
Its `pkg/rtsp/helpers.go:94-97` fills that in **only when the attribute is
missing**. Our explicit `a=sendonly` survives that branch, go2rtc classifies the
track as a backchannel, and `internal/streams/play.go:138` drops it — leaving zero
tracks and the error `codecs not matched:  => video:ANY, audio:ANY`.

So the fix is deletion, not rewriting: emitting `a=recvonly` instead would be the
same diff size but semantically backwards under RFC 4566 §6, and would only work
against go2rtc's particular convention.

**Environment setup — run once per shell before any cargo command:**

```bash
cd /home/kmk/dev/anyka-dev
source ./setenv.sh
export PATH="$(pwd)/toolchain/arm-anykav200-crosstool-ng/bin:$PATH"
```

The `PATH` prefix is load-bearing: `cargo clippy` dies with E0514 ("found crate
compiled by an incompatible version of rustc") without it, because it resolves a
different `rustc` than the vendored one.

---

## Task 1: Regression test for `generate_av_sdp`

**Files:**
- Modify: `cross-compile/onvif-rust/src/streaming/helpers.rs` (test module starts at line 266)

**Step 1: Write the failing test**

Add this to the `mod tests` block in `cross-compile/onvif-rust/src/streaming/helpers.rs`,
after `test_generate_av_sdp_no_framerate_when_none` (around line 325):

```rust
    /// go2rtc only normalises media direction to `recvonly` when the SDP omits
    /// the attribute entirely (`pkg/rtsp/helpers.go:94-97`). Any explicit
    /// direction survives, and go2rtc then treats the track as a backchannel and
    /// drops it — leaving Frigate with "codecs not matched:  => video:ANY".
    /// RFC 4566 §6 defaults an absent direction to `sendrecv`, which every RTSP
    /// client already reads as "the server sends media", so emitting nothing is
    /// both the compatible and the correct choice.
    #[test]
    fn test_generate_av_sdp_omits_direction_attribute() {
        let sps = vec![0x67, 0x42, 0x00, 0x1e];
        let pps = vec![0x68, 0xce, 0x06, 0xe2];
        let audio_config = vec![0x12, 0x10];

        // With audio, so both the video and the audio media section are covered.
        let sdp = generate_av_sdp(&sps, &pps, Some(&audio_config), 48000, Some(15.0));

        for direction in ["sendonly", "recvonly", "sendrecv", "inactive"] {
            assert!(
                !sdp.contains(direction),
                "SDP must carry no direction attribute, found {direction}:\n{sdp}"
            );
        }
    }
```

**Step 2: Run test to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu --lib test_generate_av_sdp_omits_direction_attribute
```

Expected: FAIL — `SDP must carry no direction attribute, found sendonly:` followed by the SDP dump.

**Step 3: Write minimal implementation**

In `cross-compile/onvif-rust/src/streaming/helpers.rs`, delete line 189:

```rust
    sdp.push_str("a=sendonly\r\n");
```

(the one immediately after the `if let Some(fps) = video_framerate { ... }` block)

and delete line 205:

```rust
        sdp.push_str("a=sendonly\r\n");
```

(the one immediately after `sdp.push_str("a=control:trackID=1\r\n");` inside the
`if let Some(config) = audio_config` block)

Delete only those two lines. Everything else in `generate_av_sdp` stays as-is —
in particular do **not** touch the `s=H264 Validation Stream` or
`a=tool:onvif-validation` lines, which are cosmetic and out of scope.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_generate_av_sdp
```

Expected: PASS, 6 tests (the 5 pre-existing `generate_av_sdp` tests plus the new one).

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/onvif-rust/src/streaming/helpers.rs
git commit -m "fix(sdp): drop a=sendonly so go2rtc keeps our video track"
```

---

## Task 2: Regression test for the mock publisher

`streaming-lib`'s `MockVideoPublisher` has its own SDP generator with the same
bug, and it ships in the camera binary — `onvif-rust/src/main.rs:578` instantiates
it for H.264-file playback. Fixing only Task 1 leaves this sibling broken.

There is an existing test that asserts the bug; it gets inverted.

**Files:**
- Modify: `cross-compile/streaming-lib/src/hub/mock_publisher.rs:431` (the generator)
- Modify: `cross-compile/streaming-lib/src/hub/mock_publisher.rs:1729-1735` (the test)

**Step 1: Write the failing test**

Replace the existing test at `cross-compile/streaming-lib/src/hub/mock_publisher.rs:1729-1735`:

```rust
    #[test]
    fn test_generate_sdp_contains_sendonly() {
        let sps = [0x67, 0x42, 0xE0, 0x1E];
        let pps = [0x68, 0xCE];
        let sdp = generate_sdp_from_sps_pps(&sps, &pps);
        assert!(sdp.contains("a=sendonly"));
    }
```

with:

```rust
    /// See `onvif-rust`'s `generate_av_sdp`: go2rtc only defaults media direction
    /// to `recvonly` when the SDP omits it (`pkg/rtsp/helpers.go:94-97`), and
    /// treats an explicit direction on a producer as a backchannel. The mock
    /// stands in for the camera in host-side testing, so it must not reintroduce
    /// the attribute the real generator dropped.
    #[test]
    fn test_generate_sdp_omits_direction_attribute() {
        let sps = [0x67, 0x42, 0xE0, 0x1E];
        let pps = [0x68, 0xCE];
        let sdp = generate_sdp_from_sps_pps(&sps, &pps);
        for direction in ["sendonly", "recvonly", "sendrecv", "inactive"] {
            assert!(
                !sdp.contains(direction),
                "SDP must carry no direction attribute, found {direction}:\n{sdp}"
            );
        }
    }
```

**Step 2: Run test to verify it fails**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/streaming-lib
$CARGO test --target x86_64-unknown-linux-gnu --lib test_generate_sdp_omits_direction_attribute
```

Expected: FAIL — `SDP must carry no direction attribute, found sendonly:`.

**Step 3: Write minimal implementation**

In `cross-compile/streaming-lib/src/hub/mock_publisher.rs`, delete line 431:

```rust
    sdp.push_str("a=sendonly\r\n");
```

(the last `push_str` in `generate_sdp_from_sps_pps`, immediately after
`sdp.push_str("a=control:trackID=0\r\n");` and immediately before `sdp`)

Do **not** touch the SDP *parser* tests elsewhere in `streaming-lib`
(`src/protocol/rtsp/sdp/mod.rs`, `src/common/http.rs`) that reference `sendonly`.
Parsing an attribute that other servers legitimately send is still correct
behaviour; only our own emission was wrong.

**Step 4: Run test to verify it passes**

```bash
$CARGO test --target x86_64-unknown-linux-gnu --lib test_generate_sdp
```

Expected: PASS — all `generate_sdp` tests including `..._omits_direction_attribute`.

**Step 5: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add cross-compile/streaming-lib/src/hub/mock_publisher.rs
git commit -m "fix(sdp): drop a=sendonly from mock publisher too"
```

---

## Task 3: Quality gates

**Files:** none modified unless `fmt` or `clippy` complains.

**Step 1: Run the full host-side test suite for both crates**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO test --target x86_64-unknown-linux-gnu
cd /home/kmk/dev/anyka-dev/cross-compile/streaming-lib
$CARGO test --target x86_64-unknown-linux-gnu
```

Expected: all tests pass. If anything unrelated to SDP fails, stop and report —
do not "fix while you're here".

**Step 2: Lint**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
cd /home/kmk/dev/anyka-dev/cross-compile/streaming-lib
$CARGO clippy --target x86_64-unknown-linux-gnu -- -D warnings
```

Expected: no warnings. If this fails with E0514, the `PATH` prefix from the setup
block above is missing.

**Step 3: Format check**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust && $CARGO fmt --check
cd /home/kmk/dev/anyka-dev/cross-compile/streaming-lib && $CARGO fmt --check
```

Expected: no output. If it reports diffs, run `$CARGO fmt` and commit the result.

**Step 4: Commit only if formatting changed**

```bash
cd /home/kmk/dev/anyka-dev
git add -u && git commit -m "style: cargo fmt after SDP direction fix"
```

---

## Task 4: Cross-compile for ARM

**Files:**
- Produces: `cross-compile/target/armv5te-unknown-linux-uclibceabi/release/onvif-rust`

**Step 1: Build**

```bash
cd /home/kmk/dev/anyka-dev/cross-compile/onvif-rust
$CARGO build --release
```

Note the absence of `--target`: `cross-compile/onvif-rust/.cargo/config.toml:16-18`
pins `armv5te-unknown-linux-uclibceabi` as the default target. Note also that this
is a **workspace** — artifacts land in the shared `cross-compile/target/`, not in
a per-crate `onvif-rust/target/`. This build takes about six minutes.

**Step 2: Verify the artifact is ARM and contains the fix**

```bash
B=cross-compile/target/armv5te-unknown-linux-uclibceabi/release/onvif-rust
file $B
strings $B | grep -c 'a=sendonly'                    # expect 0
strings $B | grep -c 'a=rtpmap:96 H264/90000'        # expect >0
```

Expected: `ELF 32-bit LSB executable, ARM, EABI5 ... interpreter
/mnt/anyka_hack/lib/ld-uClibc.so.1, stripped`, a count of `0` for `a=sendonly`,
and a non-zero count for the rtpmap marker. That second check matters: a `0` on
both would mean the SDP code did not make it into the binary at all, which looks
identical to success if you only grep for the removed string.

**Step 3: Refresh the SD card copy**

Record the checksum of the deployed pre-build artifact *before* overwriting it —
`git show HEAD~1` is NOT the deployed image when the working tree already carries
uncommitted work (see the note below), so Task 5 compares against this recorded
value instead of guessing at HEAD~1.

```bash
# What is currently committed on the SD-card copy (this is the prior release):
git show HEAD:SD_card_contents/anyka_hack/onvif/onvif-rust.bin | md5sum
# The camera currently runs this (if already deployed): keep this exact value
# for Task 5 step 1 — save it, e.g.:
md5sum SD_card_contents/anyka_hack/onvif/onvif-rust.bin > /tmp/deployed_onvif.md5

cp cross-compile/target/armv5te-unknown-linux-uclibceabi/release/onvif-rust \
   SD_card_contents/anyka_hack/onvif/onvif-rust.bin
```

Note: this file is *already* dirty in the working tree (uncommitted IR-LED work),
so the rebuild carries that work along with this fix. That is expected — Task 5
step 1 confirms it matches what `.198` is already running.

**Step 4: Commit the binary**

```bash
git add SD_card_contents/anyka_hack/onvif/onvif-rust.bin
git commit -m "build(onvif): rebuild ARM binary with SDP direction fix"
```

---

## Task 5: Deploy to 192.168.2.198

The camera is on WiFi with 110-1140 ms RTT spikes; expect FTP and telnet
operations to be slow, and give commands generous timeouts.

**Step 1: Confirm what is currently running, then back it up**

```bash
cd /home/kmk/dev/anyka-dev
uv run python3 scripts/debugging/cam_exec.py --timeout 60 \
  'md5sum /mnt/anyka_hack/onvif/onvif-rust.bin; cp /mnt/anyka_hack/onvif/onvif-rust.bin /mnt/anyka_hack/onvif/onvif-rust.bin.bak; ls -la /mnt/anyka_hack/onvif/'
```

Compare against the checksum recorded in Task 4 step 3 (the artifact `.198` is
actually running), e.g. `cat /tmp/deployed_onvif.md5`. Do **not** use `git show
HEAD~1` — Task 4 just committed the *new* binary, and HEAD~1 is neither the new
commit nor necessarily the deployed image when the working tree carried
uncommitted work. They should match. If they do not, stop and report — something
other than this branch's work is on the camera, and overwriting it blind would
lose it.

**Step 2: Deploy**

```bash
./scripts/deploy_onvif.sh 192.168.2.198 admin admin
```

This FTPs the binary to `/mnt/anyka_hack/onvif/` twice, as `onvif-rust` and as
`onvif-rust.bin`. The supervisor launches `onvif-rust.bin` per `.deploy/anyka.toml`.

**Known defect:** `scripts/deploy_onvif.sh:24` still points `SOURCE_DIR` at
`cross-compile/onvif-rust/target/arm-anykav200-crosstool-ng/release`, a path that
no longer exists after the move to a workspace target dir and the
`armv5te-unknown-linux-uclibceabi` triple. The script aborts with "Binary not
found" before contacting the camera — a safe failure, but it must be corrected to
`$PROJECT_ROOT/cross-compile/target/armv5te-unknown-linux-uclibceabi/release`
before this step works.

**Step 3: Restart BOTH daemons — not onvif-rust alone**

```bash
uv run python3 scripts/debugging/cam_exec.py --timeout 260 'killall vendor-daemon.bin onvif-rust.bin; sleep 35; ps | grep -v grep | grep -E "onvif|vendor"'
```

Expected: *new* PIDs for both `/mnt/anyka_hack/vendor-daemon/vendor-daemon.bin`
and `/mnt/anyka_hack/onvif/onvif-rust.bin`. The anyka-init supervisor respawns
both automatically — do not start either by hand. Allow ~45 s afterwards for the
sensor/ISP/VENC to come up before probing.

**Restarting `onvif-rust` alone does not work.** Observed on 2026-08-05: after
`killall onvif-rust.bin`, `/sub` served fine but `/main` returned
`DESCRIBE failed: 404 Not Found`, and a second lone restart lost `/sub` as well —
each restart degraded things further. Killing both daemons together restored both
streams immediately. The vendor-daemon does not release per-channel IPC attach
state when its onvif-rust peer dies, so the new process cannot re-attach. This
matches the known vendor-daemon restart-resilience weakness. A 404 on DESCRIBE
means the stream was never published to the streamhub — it is an IPC/publisher
problem, not an SDP problem, and no amount of SDP work will fix it.

**Step 4: Confirm the camera serves the new SDP**

From the dev host:

```bash
timeout 25 ffprobe -rtsp_transport tcp -v error \
  -show_entries stream=codec_name,width,height -of default=nw=1 \
  -i "rtsp://admin:admin@192.168.2.198:554/sub"
```

Expected: `codec_name=h264`, `width=640`, `height=360`. This confirms the stream
still works for lenient clients; Task 6 confirms it now works for go2rtc.

**Rollback if anything above goes wrong:**

```bash
uv run python3 scripts/debugging/cam_exec.py --timeout 60 \
  'mv /mnt/anyka_hack/onvif/onvif-rust.bin.bak /mnt/anyka_hack/onvif/onvif-rust.bin; killall onvif-rust.bin'
```

---

## Task 6: Verify on the hardware

This is the acceptance gate. Do not claim success from source inspection.

**Step 1: Capture the go2rtc → camera RTSP handshake**

```bash
ssh root@192.168.2.6 "rm -f /tmp/rtsp.pcap; timeout 45 tcpdump -i any -Z root -s 1500 -w /tmp/rtsp.pcap 'tcp port 554 and host 192.168.2.198' 2>&1 | tail -2"
ssh root@192.168.2.6 "tcpdump -Z root -A -r /tmp/rtsp.pcap 2>/dev/null | grep -aoE '(DESCRIBE|SETUP|PLAY|TEARDOWN) rtsp://[^ ]*' | sort -u"
```

`-Z root` is required: without it tcpdump exits immediately trying to drop
privileges to a non-existent `tcpdump` user, and writes an empty 24-byte pcap that
looks exactly like "no traffic".

Expected: `DESCRIBE`, `SETUP ...trackID=0`, and `PLAY` present.
Failure signature: `DESCRIBE` followed by `TEARDOWN` with no `SETUP`.

**Step 2: Confirm go2rtc now holds real tracks**

```bash
ssh root@192.168.2.6 "docker exec frigate curl -s 'http://127.0.0.1:1984/api/streams'"
```

Expected: `salon` and `salon-detect` each list a producer with a populated
`medias` array naming H264, not just a bare `url`.

**Step 3: Confirm Frigate is quiet**

```bash
ssh root@192.168.2.6 "docker logs frigate --since 90s 2>&1 | grep -cE 'codecs not matched|404 Not Found'"
```

Expected: `0`. If Frigate had been crash-looping, give it a minute to settle after
the camera restart before running this.

Do **not** include `Unable to read frames` in that grep, as an earlier draft did:
it also fires for failures downstream of this fix and will mask a clean result.
Confirmed on 2026-08-05 — once the SDP fix landed, `codecs not matched` and
`404 Not Found` both went to zero while ffmpeg still failed later, at hardware
decoder setup:

```
No VA display found for device /dev/dri/renderD128
No device available for decoder: device type vaapi needed for codec h264
```

That is a **separate, pre-existing** problem on the Frigate host, not a camera or
SDP issue: `/config/config.yaml` sets `hwaccel_args: preset-vaapi`, but `.6` has
only `/dev/dri/card0` and no `renderD128` render node. It was previously latent
because ffmpeg died at input-open and never reached decoder init. Fixing it means
dropping `hwaccel_args` (CPU decode) or exposing a render node — out of scope
here, and explicitly NOT a reason to consider this task unfinished.

**Step 4: Commit nothing; report the evidence**

Paste the actual command output — the RTSP method list, the `medias` array, the
zero count — rather than asserting it worked.

---

## Out of scope — do not do these

- Renaming `s=H264 Validation Stream` / `a=tool:onvif-validation` on the live path (cosmetic)
- Camera `.121` (blocked on the libakstreamenc version mismatch)
- Changing Frigate's config on `.6`
- Touching the SDP parser tests that reference `sendonly`
- Any `sdp_direction` config option
