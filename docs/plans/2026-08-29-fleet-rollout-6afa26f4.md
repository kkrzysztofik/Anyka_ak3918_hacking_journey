# Fleet Rollout of `6afa26f4` Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement
> this plan task-by-task. Load skill `anyka-firmware-upgrade` before Task 3, and
> `anyka-remote-debugging` for every telnet step.

**Goal:** Ship `main` @ `6afa26f4` to all four AK3918 cameras via the A/B upgrade
path, delivering the audio playback stack and closing three pieces of
device-local config drift that no bundle can reach.

**Architecture:** One `bundle.tar` serves all four cameras (identical boards).
Cameras are upgraded strictly sequentially — `.198` → `.146` → `.121` → `.127` —
with a full verification gate between each. Device-local config is edited by an
FTP round-trip validated with `tomllib`, before each camera's upload, so the
upgrade's own reboot activates it.

**Tech Stack:** Rust (`armv5te-unknown-linux-uclibceabi`), vendored crosstool-NG
toolchain, busybox on-device, `PUT /api/update` over HTTP Basic, FTP for config,
ssh jumphost at `root@192.168.3.137`.

**Design:** `docs/plans/2026-08-29-fleet-rollout-6afa26f4-design.md`

---

## Conventions used throughout

**Never run a device command without knowing which camera answered.** A stale ssh
forward silently wins the local port, and every "`.121`" command then lands on
`.146` with plausible-looking output. Per-camera local ports:

| Camera | Telnet port | RTSP port | Tunnel |
|---|---|---|---|
| 192.168.2.198 | n/a — direct | n/a — direct | none |
| 192.168.30.121 | 12421 | 15521 | `ssh -f -N -L <port>:192.168.30.121:<24\|554> root@192.168.3.137` |
| 192.168.30.146 | 12446 | 15546 | same, `.146` |
| 192.168.30.127 | 12427 | 15527 | same, `.127` |

Telnet command form (`uv run` is mandatory — a hook rejects bare `python3`):

```bash
uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 'CMD'
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 124NN 'CMD'
```

`.121`'s telnet is unreliable — it times out roughly half the time. **Retry, do
not conclude the camera is down.** Its HTTP and FTP are healthy. Prefer HTTP/FTP
for anything that has an HTTP/FTP equivalent.

HTTP against a `192.168.30.x` camera must originate on the jumphost — the dev box
has no route:

```bash
ssh -o BatchMode=yes root@192.168.3.137 'curl -s ... http://192.168.30.NNN/...'
```

`ffprobe` is the exception: it runs **locally through an SSH forward of port 554**,
because the jumphost has no ffmpeg. RTSP interleaved over TCP carries media on the
control connection, so one forwarded port suffices.

**Credentials.** Export before starting:

```bash
export CAMERA_PASS='admin'        # Administrator, for PUT /api/update
export ANYKA_FTP_PASS='www123'    # root, for config edits
```

`CAMERA_PASS` is never passed in argv: uploads read it from a `--pass-file`
process substitution, and `curl` reads it from a `--config` document on stdin.
`ANYKA_FTP_PASS` is read from the environment by the config script.
`ffprobe` has no credential-file mechanism, so its RTSP URL necessarily carries
the password in argv — run those checks only on a host whose process list you
trust.

---

## Task 1: Build the bundle

**Files:**
- Run: `scripts/build_upgrade_bundle.sh`
- Produces: `/tmp/bundle-fleet.tar`

**Step 1: Confirm the tree is clean and on the target commit**

```bash
git -C /home/kmk/dev/anyka-dev status --porcelain | grep -v '^??'
git -C /home/kmk/dev/anyka-dev describe --always --dirty
```

Expected: the first command prints nothing (untracked files are fine); the second
prints a clean hash with **no `-dirty` suffix**.

**Record that hash — it is `$STAMP` for the rest of this plan.** It is not
necessarily `6afa26f4`: the design and plan commits moved `HEAD`, and doc-only
commits change no binaries, so building from the current clean `HEAD` is correct.
As of 2026-08-29 it is `b2518aca`. Every gate below compares against `$STAMP`,
never against a hash written in this document.

**STOP if it prints any `-dirty` suffix.** The entire fleet already runs `-dirty`
versions, so a dirty stamp defeats every verification gate downstream. If the
build has already run once in this checkout, reset the artifacts it rewrites:

```bash
git -C /home/kmk/dev/anyka-dev checkout -- SD_card_contents/
```

**Step 2: Confirm the gitignored tower-http patch exists**

```bash
ls -d /home/kmk/dev/anyka-dev/cross-compile/patches/tower-http-0.7.0-full
```

Expected: the path prints. If missing, the armv5te build fails on `AtomicU64` —
`tower-http` 0.7.0's `ServeDir` `rate_limited!` macro uses
`std::sync::atomic::AtomicU64`, which does not exist on ARMv5TEJ. Regenerate the
patch; do not "fix" it by unpinning `tower-http`.

**Step 3: Build**

```bash
cd /home/kmk/dev/anyka-dev
rtk ./scripts/build_upgrade_bundle.sh /tmp/bundle-fleet.tar
```

Expected: two log steps (`1/2 build ARM payloads`, `2/2 package bundle.tar`) and a
final line naming `/tmp/bundle-fleet.tar`. Several minutes; the WebUI npm build
dominates.

This is the **gate for the entire rollout**. PR CI never cross-builds ARM
(`armv5te` lives only in `release.yml`), so `main` being green proves nothing
about the camera target. If this fails, no camera is touched.

**Step 4: Verify the version stamp**

```bash
tar -xOf /tmp/bundle-fleet.tar ./manifest.meta
cat /home/kmk/dev/anyka-dev/SD_card_contents/anyka_hack/onvif/.build-version
```

Expected: `version=$STAMP` and `requires_config_schema=1`, with `.build-version`
reading the same `$STAMP`. The two must agree — `build_bundle.sh` reads
`.build-version` for the manifest precisely so the tar cannot claim a version the
binary does not report.

**STOP if the version is anything else.** Every gate downstream compares against
this string.

**Step 5: Verify the bundle carries the audio payload**

This is the actual payload of the rollout, so check it explicitly rather than
trusting the build.

```bash
tar -tf /tmp/bundle-fleet.tar | grep -E 'libplat_ao|sounds/'
```

Expected, all five:

```
./vendor-daemon/lib/libplat_ao.so
./onvif/sounds/alert.raw
./onvif/sounds/boot.raw
./onvif/sounds/ok.raw
./onvif/sounds/upgrade.raw
```

**STOP if any is missing.** `anyka_require_sound_clips` in `scripts/common.sh`
should have failed the build already; a missing entry here means the guard was
bypassed.

**Step 6: Confirm the size is sane**

```bash
ls -lh /tmp/bundle-fleet.tar
```

Expected: roughly 19–20 MB. The device ceiling is 64 MB; over that gets HTTP 413.

---

## Task 2: Write the config-edit tool

Three cameras need the same three edits. Hand-typing them three times is how the
wrong line gets written, and a malformed `config.toml` stops `onvif-rust` from
starting — which fails the trial and triggers an automatic revert. One idempotent,
self-validating script instead.

**Files:**
- Create: `scripts/deploy_camera_config.py`

**Step 1: Write the script**

```python
#!/usr/bin/env python3
"""Bring one camera's device-local config up to the fleet standard.

Idempotent: each edit is skipped when already satisfied, so running this against
an already-correct camera is a no-op that reports what it found.

Edits, all in files a bundle never overwrites:
  A. append [sound] + [sound.events]           (event audio playback)
  B. stream_profile_1.audio_enabled = true     (audio capture on /main)
  C. upload snmp.toml when absent              (gives WebUI edits a home)

Stdlib only: it has to run on the jumphost, which has python3 and nothing else.
"""

import argparse
import difflib
import hashlib
import io
import os
import sys
import tomllib
from ftplib import FTP, error_perm

CONFIG_PATH = "/mnt/anyka_hack/onvif/config.toml"
BACKUP_PATH = "/mnt/anyka_hack/onvif/config.toml.pre-6afa26f4"
SNMP_PATH = "/mnt/anyka_hack/snmp.toml"

SOUND_BLOCK = """
[sound]
enabled = true
clip_dir = "sounds"
volume = 3
debounce_secs = 30

[sound.events]
boot_ready = "boot.raw"
network_lost = "alert.raw"
network_up = "ok.raw"
upgrade_result = "upgrade.raw"
"""

EXPECTED_EVENTS = {"boot_ready", "network_lost", "network_up", "upgrade_result"}


def add_sound(text):
    """Append the [sound] tables unless a [sound] table already parses out.

    Appending a new top-level table at EOF is positionally unambiguous in TOML,
    so this needs no knowledge of what the file currently ends with.
    """
    if "sound" in tomllib.loads(text):
        return text, False
    if not text.endswith("\n"):
        text += "\n"
    return text + SOUND_BLOCK, True


def set_audio_enabled(text):
    """Set audio_enabled = true inside [stream_profile_1] only.

    Textual, not parse-and-reserialise: tomllib is read-only, and any writer
    would reflow the file and destroy its comments. Correctness comes from
    validate() re-parsing the result, not from this function being clever.
    """
    out, changed, in_section = [], False, False
    for line in text.splitlines(keepends=True):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_section = stripped == "[stream_profile_1]"
        elif in_section and stripped.startswith("audio_enabled"):
            if stripped != "audio_enabled = true":
                line, changed = "audio_enabled = true\n", True
        out.append(line)
    return "".join(out), changed


def validate(text):
    """Parse the edited file and assert the edits landed where intended.

    Read-only parsing proves both that the file is still valid TOML and that the
    keys hold the intended values -- which is the entire risk, since a config
    onvif-rust cannot parse costs a reboot and an automatic revert.
    """
    cfg = tomllib.loads(text)
    if cfg["stream_profile_1"]["audio_enabled"] is not True:
        raise ValueError("stream_profile_1.audio_enabled is not true")
    if cfg["sound"]["enabled"] is not True:
        raise ValueError("sound.enabled is not true")
    if set(cfg["sound"]["events"]) != EXPECTED_EVENTS:
        raise ValueError(f"sound.events keys are {set(cfg['sound']['events'])}")
    return cfg


def self_test():
    """Runnable check for the two non-trivial edits. No network, no fixtures."""
    base = (
        '[stream_profile_1]\nname = "Profile1"\naudio_enabled = false\n'
        'audio_bitrate = 64\n\n[stream_profile_2]\naudio_enabled = false\n'
    )
    edited, changed = set_audio_enabled(base)
    assert changed
    cfg = tomllib.loads(edited)
    assert cfg["stream_profile_1"]["audio_enabled"] is True
    # The sibling profile must be untouched -- this is the whole point of
    # scoping the replace to a section rather than using a global substitution.
    assert cfg["stream_profile_2"]["audio_enabled"] is False
    assert "audio_bitrate = 64" in edited, "unrelated keys must survive"

    again, changed = set_audio_enabled(edited)
    assert not changed and again == edited, "must be idempotent"

    withsound, changed = add_sound(base)
    assert changed
    assert tomllib.loads(withsound)["sound"]["events"]["boot_ready"] == "boot.raw"
    again, changed = add_sound(withsound)
    assert not changed and again == withsound, "must be idempotent"

    validate(withsound.replace("audio_enabled = false", "audio_enabled = true", 1))

    try:
        validate(base)
    except (KeyError, ValueError):
        pass
    else:
        raise AssertionError("validate() must reject an unedited config")

    print("self-test OK")


def get(ftp, path):
    buf = io.BytesIO()
    ftp.retrbinary("RETR " + path, buf.write)
    return buf.getvalue()


def put(ftp, path, payload):
    """Upload, then read back and compare digests.

    Not paranoia: these cameras have silently written NUL bytes on exFAT before,
    and a truncated config.toml is indistinguishable from a good one until the
    next boot fails.
    """
    ftp.storbinary("STOR " + path, io.BytesIO(payload))
    back = get(ftp, path)
    if hashlib.sha256(back).hexdigest() != hashlib.sha256(payload).hexdigest():
        raise IOError(f"readback mismatch on {path}: wrote {len(payload)} B, "
                      f"read {len(back)} B")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("host", nargs="?", help="omit only with --self-test")
    ap.add_argument("--user", default="root")
    ap.add_argument("--snmp-toml", default=None,
                    help="local snmp.toml to upload when the camera lacks one")
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()

    if args.self_test:
        self_test()
        return 0
    if not args.host:
        ap.error("host is required unless --self-test is given")

    password = os.environ.get("ANYKA_FTP_PASS")
    if not password:
        print("ANYKA_FTP_PASS is not set", file=sys.stderr)
        return 2

    ftp = FTP(args.host, timeout=30)
    ftp.login(args.user, password)
    try:
        original = get(ftp, CONFIG_PATH)
        text = original.decode()

        text, sound_changed = add_sound(text)
        text, audio_changed = set_audio_enabled(text)
        validate(text)

        print(f"{args.host}: sound={'ADD' if sound_changed else 'present'} "
              f"audio_in={'ENABLE' if audio_changed else 'already on'}")

        if sound_changed or audio_changed:
            diff = difflib.unified_diff(
                original.decode().splitlines(), text.splitlines(),
                "before", "after", lineterm="", n=1)
            print("\n".join(diff))

        if args.dry_run:
            print(f"{args.host}: dry run, nothing written")
            return 0

        if sound_changed or audio_changed:
            put(ftp, BACKUP_PATH, original)
            put(ftp, CONFIG_PATH, text.encode())
            print(f"{args.host}: config.toml written and verified "
                  f"(backup at {BACKUP_PATH})")

        if args.snmp_toml:
            # Probe with RETR rather than NLST/SIZE: busybox ftpd's support for
            # the latter two is not guaranteed, and RETR is already proven to
            # work by the config fetch above.
            try:
                get(ftp, SNMP_PATH)
                present = True
            except error_perm:
                present = False
            if present:
                print(f"{args.host}: snmp.toml already present")
            else:
                with open(args.snmp_toml, "rb") as fh:
                    put(ftp, SNMP_PATH, fh.read())
                print(f"{args.host}: snmp.toml written and verified")
    finally:
        ftp.quit()
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

**Step 2: Run the self-test and verify it passes**

```bash
cd /home/kmk/dev/anyka-dev
uv run python3 scripts/deploy_camera_config.py --self-test
```

Expected: `self-test OK`, exit 0.

This is the check that the section-scoped replace does not touch
`[stream_profile_2]` — a global substitution would silently enable audio on all
four profiles, which is not what we want and would not show up until someone
opened the sub-stream.

**Step 3: Dry-run against `.198`, which needs nothing**

`.198` already has all three settings, so it is a free correctness check: the
script must report no changes and write nothing.

```bash
ANYKA_FTP_PASS="$ANYKA_FTP_PASS" uv run python3 scripts/deploy_camera_config.py \
  192.168.2.198 --dry-run
```

Expected exactly:

```
192.168.2.198: sound=present audio_in=already on
192.168.2.198: dry run, nothing written
```

**STOP if it reports a change.** That means an edit function is not idempotent,
and the same bug would corrupt the three cameras that do need editing.

**Step 4: Commit**

```bash
cd /home/kmk/dev/anyka-dev
git add scripts/deploy_camera_config.py
git commit -m "feat(deploy): idempotent device-local config tool

Brings one camera up to the fleet standard over FTP: appends [sound],
enables stream_profile_1.audio_enabled, and uploads snmp.toml when
absent. Every write is validated by re-parsing with tomllib and
verified by a sha256 readback.

Replaces line-addressed 'busybox sed -i \"75s|...|\"' over telnet, which
breaks silently the moment the file shifts and needs telnet to be
working -- it is not, reliably, on .121.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

Note: `git status` in this repo shows modifications in the *first* column, so a
bare `git commit` sweeps the whole tree. Always pass an explicit pathspec.

**Step 5: Re-confirm the version stamp is unaffected**

```bash
git -C /home/kmk/dev/anyka-dev describe --always --dirty
```

Expected: a new clean hash, different from `$STAMP`. This is fine and expected —
the commit moved `HEAD`, but `/tmp/bundle-fleet.tar` was already built and stamped
from `$STAMP`. Every gate compares against `$STAMP` as recorded in Task 1, not
against `git describe` at gate time.

**Do not rebuild to "fix" the mismatch.** Rebuilding here would stamp the bundle
with a hash whose only difference is this script commit, and would rewrite
`SD_card_contents/*.bin` — which are tracked, making the tree dirty and costing
you the clean stamp entirely.

---

## Task 3: Upgrade `.198` (canary, direct, vfat)

`.198` is the canary: direct reach, vfat, physically accessible, and — uniquely —
**audible**. It needs no config edits.

**Step 1: Record the pre-upgrade state**

```bash
cd /home/kmk/dev/anyka-dev
uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 \
  'ifconfig wlan0 | head -2 | tail -1; cat /mnt/anyka_hack/active; echo; ls /mnt/anyka_hack/slots; ls -l /tmp/vendor-frame-ring.shm'
```

Expected (as surveyed 2026-08-29): `inet addr:192.168.2.198`, `active` = `b`,
slots `a b`, shm `2097216`.

Write down the `active` value. If it is not `b`, the camera moved since the
survey — re-read the design's assumptions before continuing.

**Step 2: Upload**

```bash
cd /home/kmk/dev/anyka-dev
./scripts/upload_upgrade_bundle.sh --host 192.168.2.198 --user admin \
  --pass-file <(printf %s "$CAMERA_PASS") /tmp/bundle-fleet.tar
```

Expected: **HTTP 202**, nothing else counts as success.
`401` → wrong credentials. `409` → spool busy, see Task 8. `413` → too large.

**Step 3: Wait out the apply window**

Wait 150 seconds. The applier polls on the supervisor tick (~60 s), stages the
inactive slot, flips `active`, and reboots; ~90–120 s of downtime is normal.

**Do not re-upload during this window.**

**Listen.** If the upgrade succeeds, `.198` plays `boot.raw` when it comes back.
Hearing it is the single strongest proof available for the whole audio chain, and
it is only available on this camera. Note whether you heard it — a silent boot is
not yet a failure (the gate below is authoritative), but it is a strong hint.

**Step 4: Wait for the camera to answer**

```bash
alive=
for i in $(seq 1 30); do
  code=$(curl -s -o /dev/null -m 5 -w '%{http_code}' http://192.168.2.198/ || true)
  echo "attempt $i: $code"
  case "$code" in 200|401) alive=1; break;; esac
  sleep 10
done
[ -n "$alive" ] || { echo "NO ANSWER after 5 minutes - go to Task 8"; false; }
```

Expected: `200` or `401` within a few attempts — both prove the HTTP server is up.
The loop fails loudly rather than falling through.

**Step 5: Verification gate**

All seven must hold.

```bash
# 1. version identity
printf 'user = "admin:%s"\n' "$CAMERA_PASS" \
  | curl -s --config - http://192.168.2.198/api/diagnostics | jq -r .firmware_version
# expect: $STAMP   (the stamp recorded in Task 1 Step 1/4)

# 2. active flipped, 3. trial cleared, 4. ring still 256 KB,
# plus the audio payload actually landed in the new slot
uv run python3 scripts/debugging/cam_exec.py --host 192.168.2.198 \
  'cat /mnt/anyka_hack/active; echo; ls /mnt/anyka_hack/state/trial-* 2>/dev/null; echo TRIAL_CHECK_DONE; ls -l /tmp/vendor-frame-ring.shm; a=$(cat /mnt/anyka_hack/active); ls /mnt/anyka_hack/slots/$a/vendor-daemon/lib/libplat_ao.so; ls /mnt/anyka_hack/slots/$a/onvif/sounds/'
# expect: active = a (flipped from b); no trial-* before TRIAL_CHECK_DONE;
#         shm exactly 2097216; libplat_ao.so present; FOUR .raw clips

# 5. ports
for p in 80 554 8080; do nc -z -w3 192.168.2.198 $p && echo "$p up" || echo "$p DOWN"; done

# 6. keyframes sustained, and 7. audio track present
timeout 40 ffprobe -v error -rtsp_transport tcp \
  -i "rtsp://admin:$CAMERA_PASS@192.168.2.198:554/main" \
  -select_streams v -show_entries frame=key_frame -read_intervals "%+20" -of csv 2>/dev/null \
  | awk -F, '{if($2==1)k++; n++} END{print "frames="n" keyframes="k}'
# expect: keyframes >= 3   (calibrated: .198 gave frames=202 keyframes=5)

timeout 30 ffprobe -v error -rtsp_transport tcp \
  -i "rtsp://admin:$CAMERA_PASS@192.168.2.198:554/main" \
  -show_entries stream=index,codec_type,codec_name,sample_rate -of csv 2>&1 | head -5
# expect two lines: h264 video, and an audio stream at 8000

curl -s -m 8 -o /dev/null -w 'flv:%{http_code} bytes:%{size_download}\n' \
  http://192.168.2.198:8080/live/main.flv
# expect: flv:200 with a non-zero byte count
```

**Step 6: Prove the sound API end to end**

```bash
printf 'user = "admin:%s"\n' "$CAMERA_PASS" \
  | curl -s --config - -X POST -H 'Content-Type: application/json' \
    -d '{"event":"boot_ready"}' http://192.168.2.198/api/sound/play -w '\nHTTP %{http_code}\n'
```

Expected: HTTP 200/204 and an audible clip from the camera.

**STOP the entire rollout if any gate fails.** Go to Task 8. `.198` passing is
what authorises Tasks 5–7.

---

## Task 4: Stage the config tool on the jumphost

The three remaining cameras are only reachable from the jumphost, and FTP does not
survive an SSH port forward cleanly (passive mode negotiates data connections on
arbitrary ports). Run the script *on* the jumphost instead.

**Step 1: Copy the script and the snmp.toml it uploads**

```bash
cd /home/kmk/dev/anyka-dev
ssh -o BatchMode=yes root@192.168.3.137 'mkdir -p /tmp/anyka-cfg'
scp -o BatchMode=yes scripts/deploy_camera_config.py \
    SD_card_contents/anyka_hack/snmp.toml \
    root@192.168.3.137:/tmp/anyka-cfg/
```

Copying the real `snmp.toml` rather than embedding its text in the script is what
stops the two drifting apart.

**Step 2: Verify the jumphost can run it**

```bash
ssh -o BatchMode=yes root@192.168.3.137 \
  'python3 /tmp/anyka-cfg/deploy_camera_config.py --self-test'
```

Expected: `self-test OK`.

**STOP if this fails on a missing `tomllib`** — it needs python ≥ 3.11. Check
`ssh root@192.168.3.137 python3 -V` before looking for anything more exotic.

---

## Task 5: Upgrade `.146` (jumphost, exFAT, healthiest)

`.146` goes first behind the jumphost: longest uptime, no known history of
trouble. It proves the bundle and the config script on exFAT with the least to
lose. It needs all three edits.

**Step 1: Open tunnels and confirm identity**

```bash
ssh -o BatchMode=yes -o ExitOnForwardFailure=yes -f -N \
  -L 12446:192.168.30.146:24 root@192.168.3.137
ssh -o BatchMode=yes -o ExitOnForwardFailure=yes -f -N \
  -L 15546:192.168.30.146:554 root@192.168.3.137
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12446 \
  'ifconfig wlan0 | head -2 | tail -1'
```

Expected: `inet addr:192.168.30.146`.

**STOP on any other address.** A pre-existing tunnel owns port 12446 and every
following command would silently hit the wrong camera. Kill it
(`pkill -f '12446:192.168.30'`) and reopen.

**Step 2: Record the pre-upgrade state**

```bash
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12446 \
  'cat /mnt/anyka_hack/active; echo; ls /mnt/anyka_hack/slots; ls -l /tmp/vendor-frame-ring.shm'
```

Expected: `b`; slots `a b`; shm `2097216`.

**Step 3: Dry-run the config edits and read the diff**

```bash
ssh -o BatchMode=yes root@192.168.3.137 \
  "ANYKA_FTP_PASS='$ANYKA_FTP_PASS' python3 /tmp/anyka-cfg/deploy_camera_config.py \
     192.168.30.146 --snmp-toml /tmp/anyka-cfg/snmp.toml --dry-run"
```

Expected: `sound=ADD audio_in=ENABLE`, followed by a unified diff showing exactly
two changes — one `audio_enabled` flip inside `[stream_profile_1]`, and the
appended `[sound]` block at EOF.

**Read the diff.** If it touches any other line, stop and fix the script.

**Step 4: Apply the config edits**

```bash
ssh -o BatchMode=yes root@192.168.3.137 \
  "ANYKA_FTP_PASS='$ANYKA_FTP_PASS' python3 /tmp/anyka-cfg/deploy_camera_config.py \
     192.168.30.146 --snmp-toml /tmp/anyka-cfg/snmp.toml"
```

Expected three lines: `config.toml written and verified (backup at ...)`, and
`snmp.toml written and verified`. Any `readback mismatch` is a hard stop — the
file on the device does not match what we sent.

**Step 5: Confirm idempotency by running it again**

```bash
ssh -o BatchMode=yes root@192.168.3.137 \
  "ANYKA_FTP_PASS='$ANYKA_FTP_PASS' python3 /tmp/anyka-cfg/deploy_camera_config.py \
     192.168.30.146 --snmp-toml /tmp/anyka-cfg/snmp.toml"
```

Expected: `sound=present audio_in=already on`, `snmp.toml already present`, and no
write. This proves the device now genuinely holds the intended state, read back
through a fresh parse rather than assumed from the previous step's success.

**Step 6: Upload**

```bash
cd /home/kmk/dev/anyka-dev
./scripts/upload_upgrade_bundle.sh --host 192.168.30.146 \
  --jumphost root@192.168.3.137 --user admin \
  --pass-file <(printf %s "$CAMERA_PASS") /tmp/bundle-fleet.tar
```

Expected: **HTTP 202**. The script stages the credential in a netrc on the
jumphost, curls from there, and wipes it — no password reaches argv on either
host.

**Step 7: Wait, then poll from the jumphost**

Wait 150 seconds, then:

```bash
ssh -o BatchMode=yes root@192.168.3.137 'alive=; for i in $(seq 1 30); do code=$(curl -s -o /dev/null -m 5 -w "%{http_code}" http://192.168.30.146/ || true); echo "attempt $i: $code"; case "$code" in 200|401) alive=1; break;; esac; sleep 10; done; [ -n "$alive" ] || { echo "NO ANSWER after 5 minutes - go to Task 8"; false; }'
```

Both tunnels from Step 1 die with the reboot. Reopen them before the gate.

**Step 8: Verification gate**

```bash
# reopen both tunnels
ssh -o BatchMode=yes -o ExitOnForwardFailure=yes -f -N -L 12446:192.168.30.146:24 root@192.168.3.137
ssh -o BatchMode=yes -o ExitOnForwardFailure=yes -f -N -L 15546:192.168.30.146:554 root@192.168.3.137

# 1. version
printf 'user = "admin:%s"\n' "$CAMERA_PASS" \
  | ssh -o BatchMode=yes root@192.168.3.137 'cat > /tmp/curlrc; curl -s --config /tmp/curlrc http://192.168.30.146/api/diagnostics; rm -f /tmp/curlrc' \
  | jq -r .firmware_version
# expect: $STAMP

# 2-4 + identity + config survival + audio payload landed
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12446 \
  'ifconfig wlan0 | head -2 | tail -1; cat /mnt/anyka_hack/active; echo; ls /mnt/anyka_hack/state/trial-* 2>/dev/null; echo TRIAL_CHECK_DONE; ls -l /tmp/vendor-frame-ring.shm; a=$(cat /mnt/anyka_hack/active); ls /mnt/anyka_hack/slots/$a/vendor-daemon/lib/libplat_ao.so; ls /mnt/anyka_hack/slots/$a/onvif/sounds/; grep -c "^\[sound\]" /mnt/anyka_hack/onvif/config.toml'
# expect: wlan0 still .146; active = a; no trial-*; shm 2097216;
#         libplat_ao.so present (regression check -- it was already there);
#         FOUR .raw clips, against ZERO before the upgrade; sound count = 1

# 5. ports
ssh -o BatchMode=yes root@192.168.3.137 \
  'for p in 80 554 8080; do nc -z -w3 192.168.30.146 $p && echo "$p up" || echo "$p DOWN"; done'

# 6. keyframes, through the local RTSP forward
timeout 40 ffprobe -v error -rtsp_transport tcp \
  -i "rtsp://admin:$CAMERA_PASS@127.0.0.1:15546/main" \
  -select_streams v -show_entries frame=key_frame -read_intervals "%+20" -of csv 2>/dev/null \
  | awk -F, '{if($2==1)k++; n++} END{print "frames="n" keyframes="k}'
# expect: keyframes >= 3

# 7. audio track -- THE headline check for this camera
timeout 30 ffprobe -v error -rtsp_transport tcp \
  -i "rtsp://admin:$CAMERA_PASS@127.0.0.1:15546/main" \
  -show_entries stream=index,codec_type,codec_name,sample_rate -of csv 2>&1 | head -5
# expect TWO streams. Before this rollout .146 returned only
# "stream,0,h264,video" -- an audio line appearing is the proof Edit B took.

# FLV
ssh -o BatchMode=yes root@192.168.3.137 \
  'curl -s -m 8 -o /dev/null -w "flv:%{http_code} bytes:%{size_download}\n" http://192.168.30.146:8080/live/main.flv'
```

**Step 9: Prove the sound API**

```bash
printf 'user = "admin:%s"\n' "$CAMERA_PASS" \
  | ssh -o BatchMode=yes root@192.168.3.137 'cat > /tmp/curlrc; curl -s --config /tmp/curlrc -X POST -H "Content-Type: application/json" -d "{\"event\":\"boot_ready\"}" http://192.168.30.146/api/sound/play -w "\nHTTP %{http_code}\n"; rm -f /tmp/curlrc'
```

Expected: HTTP 200/204. Nobody is there to hear it, so this plus the clips and
`libplat_ao.so` being present in the slot is as far as remote proof goes.

**STOP the rollout if any check fails.** Go to Task 8.

---

## Task 6: Upgrade `.121` (jumphost, exFAT — check the link first)

Same as Task 5 with `192.168.30.121`, telnet port `12421`, RTSP port `15521`.
Three differences.

**Difference 1: run the link pre-flight before anything else.** On 2026-08-29
`.121` was found at **-82 dBm / 28/70**, dropping 40-60% of packets across ping,
HTTP and FTP — an FTP connection died with `OSError: [Errno 113] No route to host`
minutes after an identical one succeeded. A reboot took it to **-44 dBm / 66/70**
with **0% loss over 20 pings**, making it the best-signal camera in the fleet.
The association had degraded over 2 d 6 h of uptime; the radio and its placement
were never the problem.

```bash
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12421 \
  'iwconfig wlan0 2>/dev/null | grep -io "link quality=[0-9/]*\|signal level=[-0-9]* dBm" | tr "\n" " "; echo; uptime'
```

Reference values measured the same day: `.198` -52 dBm, `.146` -65 dBm.

**If signal is worse than about -70 dBm, reboot and re-measure before upgrading.**
Do not attempt a 19 MB upload over a degraded link: `PUT /api/update` can die
mid-transfer and leave a partial `bundle.tar.part` in the spool, after which every
retry returns HTTP 409.

Rebooting this camera is itself unreliable while the link is bad — the telnet
session often dies before the command is sent. **Verify by `uptime`, never by
"HTTP came back"**: a flapping link produces exactly the same `000` → `200`
transition as a reboot, and it is easy to convince yourself a reboot happened when
it did not.

```bash
# retry until uptime resets; an empty exit=0 response means it took
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12421 'reboot'
# wait ~90 s, reopen the tunnel, then confirm:
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12421 'uptime'
```

There is no HTTP reboot path — `handle_system_reboot` returns
`ActionNotSupported` (`onvif/device/ops/system.rs:251`). Telnet is the only way.

Also ignore the load average here: it reads 3-4 on this board because wifi threads
park in D state. It is not CPU pressure.

**Difference 2: the shm size.** Verified `2097216` post-reboot on 2026-08-29, so
`.121` matches the rest of the fleet and the ring check is a plain regression
check like everywhere else.

**Difference 3: it needs all three edits**, same as `.146`. Dry run confirmed
2026-08-29: `sound=ADD audio_in=ENABLE`.

Run every step of Task 5 with the substituted host and ports. The post-upgrade
gate expects `active` = `a` and `firmware_version` = `$STAMP`.

**STOP the rollout if any check fails.**

---

## Task 7: Upgrade `.127` (jumphost, exFAT, zt9101)

`.127` goes last: it is the only camera on the zt9101 wifi chip, it has
previously gone dark and needed physical access, and it is the least like the
others. If it surprises us, the rest of the fleet is already done.

Same as Task 5 with `192.168.30.127`, telnet port `12427`, RTSP port `15527`.
Three differences:

**Difference 1: it needs only two of the three edits.** `.127` already has
`stream_profile_1.audio_enabled = true`. The Step 3 dry run must report:

```
192.168.30.127: sound=ADD audio_in=already on
```

Confirmed by a dry run on 2026-08-29. **STOP if it says `audio_in=ENABLE`** —
that would mean the survey was wrong about this camera, or you are talking to a
different one. Check `ifconfig wlan0`.

**Difference 2: check the clock before trusting any ONVIF result.** This camera
has booted at 1970 before. At that skew `ws_security` rejects every authenticated
ONVIF request on ±300 s, so ONVIF checks fail for reasons unrelated to the build —
while `PUT /api/update` uses HTTP Basic and succeeds regardless, which is exactly
how a broken clock stays hidden.

```bash
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12427 \
  'date -u; sed -n "74,76p" /mnt/anyka_hack/anyka.toml'
```

Expected: real UTC, and a `[time]` block with `enabled = true`. Both were fixed on
2026-08-23 and should have held. If `date` reads 1970, set it before continuing:

```bash
NOW=$(date -u '+%Y-%m-%d %H:%M:%S')
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12427 \
  "date -u -s '$NOW'; date -u"
```

**Difference 3: check the uptime first.** If it is only a few minutes, the camera
is on a reboot cycle — investigate before upgrading, because an upgrade into a
reboot loop cannot pass its trial window.

Post-gate, add one `.127`-specific check that the clock survived the reboot on its
own:

```bash
uv run python3 scripts/debugging/cam_exec.py --host 127.0.0.1 --port 12427 \
  'date; grep -i "stepped system clock" /mnt/logs/anyka-init.log | tail -3'
```

Expected: real time plus a `stepped system clock` line, which proves NTP ran
rather than the clock merely surviving a warm reboot.

**STOP the rollout if any check fails.**

---

## Task 8: Failure handling (reference — do not run unless a gate fails)

| Symptom | Diagnosis | Action |
|---|---|---|
| HTTP 409 on upload | Spool busy | `ls -la /mnt/anyka_hack/spool` — remove a stale `bundle.tar.part`; if `bundle.trigger` exists an apply is queued, wait it out |
| HTTP 401 | Credentials | Check `CAMERA_PASS`; the account must be Administrator |
| HTTP 413 | Over the 64 MB ceiling | Rebuild; check nothing dragged top-level `lib/` in |
| `File exists (os error 17)` in the device log | Applier could not replace the inactive slot | Re-read `active`, then: `cd /mnt/anyka_hack && act=$(cat active) && case "$act" in a\|b) ;; *) echo "REFUSING: active=[$act]"; exit 1;; esac && cd slots && for e in *; do [ "$e" = "$act" ] \|\| busybox rm -rf "$e"; done && sync && ls` — derives the target from `active` at execution time so it cannot delete the running slot. Re-upload |
| `readback mismatch` from the config script | FTP wrote a truncated or corrupted file | **Do not reboot.** Re-run the script; if it repeats, restore `config.toml.pre-6afa26f4` and stop |
| Camera returns on the OLD version | Trial failed, self-reverted | **Halt the rollout.** Read `/mnt/logs/anyka-init.log` for the failing port. Do not re-upload the same tar |
| Camera up but silent, `sounds/` absent in the new slot | Clips missing from the bundle | Re-check Task 1 Step 5; `anyka_require_sound_clips` should have caught it |
| shm file reads `1048640` after upgrade | Build regressed to 128 KB slots | Halt — the bundle was built from the wrong tree. Do not roll it further |
| Sound config present but no audio | Amplifier or clip issue, not config | `SPK_PA` is active-high **shutdown**: writing 1 silences the amp while every layer reports success. Never write 1 |
| HTTP dead, telnet alive | `onvif-rust` down | FTP the bundle to `/mnt/anyka_hack/spool/bundle.tar`, then `touch /mnt/anyka_hack/spool/bundle.trigger` **last** |
| No telnet, no HTTP, no ARP | Both slots unusable | The 240 s deadman in `config.sh` restores the gergehack boot path. Power-cycle; if still dark, pull the SD card — `Factory/config.sh.gerge.bak` and `onvif/config.toml.pre-6afa26f4` are on the card |

**Never remove a remote camera's watchdog trigger.** `.127` went dark in August
precisely because a static-IP change let `anyka-init` run happily with no working
network, so nothing forced the gergehack fallback.

---

## Task 9: Close out

**Step 1: Confirm the whole fleet reports the same version**

```bash
printf 'user = "admin:%s"\n' "$CAMERA_PASS" \
  | curl -s --config - http://192.168.2.198/api/diagnostics | jq -r .firmware_version
printf 'user = "admin:%s"\n' "$CAMERA_PASS" \
  | ssh -o BatchMode=yes root@192.168.3.137 'cat > /tmp/curlrc; for ip in 192.168.30.121 192.168.30.146 192.168.30.127; do printf "%s " $ip; curl -s --config /tmp/curlrc http://$ip/api/diagnostics | jq -r .firmware_version; done; rm -f /tmp/curlrc'
```

Expected: `$STAMP` four times, with **no `-dirty` suffix anywhere**. That is the
first time the fleet has been on a reproducible build.

**Step 2: Confirm audio capture is uniform**

```bash
for hp in 192.168.2.198:554 127.0.0.1:15521 127.0.0.1:15546 127.0.0.1:15527; do
  printf '%s ' "$hp"
  timeout 30 ffprobe -v error -rtsp_transport tcp -i "rtsp://admin:$CAMERA_PASS@$hp/main" \
    -show_entries stream=codec_type -of csv=p=0 2>/dev/null | tr '\n' ' '
  echo
done
```

Expected: `video audio` for all four.

**Step 3: Clean up the jumphost and close the tunnels**

```bash
ssh -o BatchMode=yes root@192.168.3.137 'rm -rf /tmp/anyka-cfg /tmp/c1*.toml /tmp/a1*.toml /tmp/cfg*.toml'
pkill -f 'L 124[0-9][0-9]:192.168.30' || true
pkill -f 'L 155[0-9][0-9]:192.168.30' || true
```

The staged script carried no credential (it reads `ANYKA_FTP_PASS` from the
environment), but the downloaded configs did sit in `/tmp` — remove them. Stale
forwards are a documented foot-gun for the *next* session.

**Step 4: Record the outcome**

Append a "Rollout log" section to
`docs/plans/2026-08-29-fleet-rollout-6afa26f4-design.md`: per camera, the old
version, new version, new `active` slot, `.121`'s pre-upgrade shm size, whether
`.198`'s boot clip was audible, and anything that surprised you.

```bash
cd /home/kmk/dev/anyka-dev
git add docs/plans/2026-08-29-fleet-rollout-6afa26f4-design.md
git commit -m "docs(deploy): record the 6afa26f4 rollout outcome

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```
