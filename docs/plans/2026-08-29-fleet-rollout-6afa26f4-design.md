# Fleet Rollout of `6afa26f4` — Design

**Date:** 2026-08-29
**Goal:** Ship `main` @ `6afa26f4` to all four AK3918 cameras via the A/B upgrade
path, and close three pieces of device-local config drift in the same pass: event
audio, audio capture, and a missing `snmp.toml`.

Supersedes the approach in `docs/plans/2026-08-23-fleet-rollout-a013f167-design.md`
in two places: config edits move from telnet `sed` to an FTP round-trip, and the
inactive-slot `rm -rf` is dropped.

## Starting state (surveyed 2026-08-29)

All four cameras answer, all report `status: healthy`, all sit on slot `b` with a
clean `slots/{a,b}` — the `.aside` / `.preidr` leftovers from earlier rollouts are
gone.

| Camera | Reach | FS | Version | `[sound]` | `stream_profile_1.audio_enabled` | `snmp.toml` |
|---|---|---|---|---|---|---|
| 192.168.2.198 | direct | vfat | `a1660798-dirty` | yes, `volume = 6` | `true` | present |
| 192.168.30.146 | jumphost | exFAT | `a1660798-dirty` | **absent** | **`false`** | **absent** |
| 192.168.30.121 | jumphost | exFAT | `a1660798-dirty` | **absent** | **`false`** | **absent** |
| 192.168.30.127 | jumphost | exFAT | `a1660798-dirty` | **absent** | `true` | **absent** |

All three drifted settings live in files a bundle never overwrites, so no rollout
has ever corrected them and none ever will. Only `stream_profile_1` carries audio
on any camera; profiles 2–4 are `false` everywhere, so audio rides `/main` and
never `/sub`. Verified on the wire: `.198`'s `/main` reports
`stream,1,aac,audio,8000` next to h264.

SNMP is **already enabled everywhere** — `[services.snmp] enabled = true` in all
four `anyka.toml`, UDP 161 listening on all four, and `.198` answers
`iso.3.6.1.2.1.1.1.0 = "Anyka AK3918 IP camera (snmp-agent)"`. The missing
`snmp.toml` changes no behaviour today because the agent's built-in defaults are
identical to the file's contents; it only means a WebUI/ONVIF SNMP edit has
nowhere to persist.

Jumphost is `root@192.168.3.137`; the dev box has no route to `192.168.30.0/24`.
Per-camera telnet tunnel ports: 12421 → `.121`, 12446 → `.146`, 12427 → `.127`.

`.121`'s telnet is unreliable today — `cam_exec.py` times out repeatedly while its
HTTP answers normally. The design routes around telnet rather than fighting it.

Memory headroom is adequate: `.146` reports 36540 KB total / 33544 KB used, but
19 MB of that is buffers+cache, so ~21 MB is reclaimable.

## What the delta contains

`a1660798..HEAD` is 38 commits. Two things justify the rollout:

**The `/main` keyframe fix.** `6b713d50` sizes ring slots for real keyframes
(`VD_SHM_SLOT_SIZE` 128 KB → 256 KB) and `6da17401` gates it behind
`VD_SHM_VERSION` 3 → 4. This is the root cause behind `/main` emitting one IDR and
going quiet: 184 KB IDRs exceeded the 131008-byte slot and `push.c` dropped them
with no log. It is the reason recording coverage sits near 40%.

**Working event audio.** `acf8472f` (SPK_PA is active-high *shutdown*, not enable),
`bcf6b1ac` (duplicate mono to stereo before `ak_ao_send_frame`), `ffdcac4d`
(`libplat_ao.so` into the vendor-daemon payload), `2a925f99`/`ac1ab1a9` (clips
ship in bundles, with a build-time guard on the full set).

The rest is dependabot bumps and docs.

## Why the protocol bump is safe here

`VD_SHM_VERSION` 3 → 4 is a breaking change between `vendor-daemon` and
`onvif-rust`. It is safe because both binaries live inside `slots/{a,b}` and ship
in a single tar: a v3 daemon can never meet a v4 client across a reboot, because
there is no reboot in which half the slot is old. A partial upgrade is not
representable in this layout. Rollback flips the whole slot, so it too is
version-coherent.

`libplat_ao.so` is a new runtime dependency and does ship: it lives in
`vendor-daemon/lib/`, which `build_bundle.sh` copies wholesale with `cp -r`. The
`lib/` excluded from bundles is only the 31 MB top-level uClibc runtime.
`supervisor_loop.rs:82` rewrites `LD_LIBRARY_PATH` entry-by-entry into the active
slot, so the absolute path in `anyka.toml` resolves against `slots/<active>/`.

Ring memory grows from 8 × 128 KB to 8 × 256 KB — about 1 MB more, against ~21 MB
reclaimable.

## Decision 1 — version stamp

Build once from a clean tree. `SD_card_contents/*.bin` and `onvif/.build-version`
are tracked files that the build itself rewrites, so a second build in the same
checkout stamps `-dirty`; that is how the entire fleet came to run `-dirty`
versions. Run `git checkout -- SD_card_contents/` before building.

This matters beyond tidiness: every downstream gate compares
`/api/diagnostics.firmware_version` against the built stamp, and that comparison
is worthless while both sides read `-dirty`.

## Decision 2 — config edits by FTP round-trip

`[sound]` lives in `config.toml`. Bundles ship it only as `config.template.toml`
and never overwrite the live file, and `SoundConfig::default()` is
`enabled: false` with an empty event map (`cross-compile/onvif-rust/src/config/sound.rs:55`).
Without a per-camera edit, three of four cameras would receive a working audio
stack, the clips, and the library, and stay permanently silent.

The 2026-08-23 rollout made such edits with `busybox sed -i "75s|...|"` over
telnet, guarded by three separate "STOP if line 75 is not X" checks. Replace that
with a round-trip:

1. `curl` the live `config.toml` down over FTP (`root:www123`)
2. append `[sound]` and `[sound.events]` locally
3. validate the result with `tomllib`
4. `curl -T` it back

For `192.168.30.x` the curl runs on the jumphost. Verified working in both
directions on 2026-08-29: `.198` direct returned 3650 bytes, `.146` via jumphost
returned 3357 bytes.

Three reasons this beats the telnet edit: it does not depend on telnet, which is
currently failing on `.121`; it has no line-number coupling, so a shifted file
cannot cause a wrong-line write; and a malformed `config.toml` stops `onvif-rust`
from starting, which fails the trial and triggers an automatic revert — a parse at
a trust boundary earns a real validation step, not a hope.

Appending a new top-level table at EOF is positionally unambiguous in TOML.
`.146`'s file ends inside an OSD table, and `[sound]` closes it correctly.

### Edit A — append `[sound]` (`.146`, `.121`, `.127`)

```toml
[sound]
enabled = true
clip_dir = "sounds"
volume = 3
debounce_secs = 30

[sound.events]
boot_ready     = "boot.raw"
network_lost   = "alert.raw"
network_up     = "ok.raw"
upgrade_result = "upgrade.raw"
```

`.198` keeps its existing `volume = 6`; that value was tuned by ear and there is
no reason to harmonise it downward.

### Edit B — enable audio capture (`.121`, `.146` only)

Set `audio_enabled = true` under `[stream_profile_1]`, bringing both to parity
with `.198` and `.127`. The neighbouring `audio_encoding = "G711"`,
`audio_bitrate = 64`, `audio_sample_rate = 8000` already match `.198` exactly, so
only the one boolean moves.

This is a **textual** replacement of the first `audio_enabled = false`, not a
parse-and-reserialise: `tomllib` is read-only in the stdlib, and a round-trip
through any writer would reflow the file and destroy its comments. `tomllib` is
still what validates the result — parse the edited text and assert
`cfg["stream_profile_1"]["audio_enabled"] is True`. Read-only parsing is enough to
prove both that the file is still valid TOML and that the edit landed on the
intended key, which is the whole risk. No new dependency.

Profiles 2–4 stay `false`; `/sub` remains video-only.

### Edit C — push `snmp.toml` (`.146`, `.121`, `.127`)

One `curl -T` of the repo's `SD_card_contents/anyka_hack/snmp.toml` to
`/mnt/anyka_hack/snmp.toml`. Its contents equal the agent's compiled-in defaults,
so runtime behaviour is unchanged; the gain is that WebUI/ONVIF SNMP edits get a
file to persist into, and the fleet stops differing.

Not slot-scoped — `anyka.toml` points at `/mnt/anyka_hack/snmp.toml`, outside
`slots/`, so this survives every future upgrade and needs doing only once.

### Ordering

All three edits land **before** the upload, so the upgrade's own reboot activates
them — no separate restart — and the `boot_ready` clip becomes the first audible
proof.

## Decision 3 — no inactive-slot `rm -rf`

The 2026-08-23 rollout deleted the inactive slot on every exFAT camera to dodge
the applier bug where Rust's `remove_dir_all` fails on exFAT, the error is
swallowed by `let _ =`, and the following `rename` fails with
`File exists (os error 17)`.

That bug was fixed in code on 2026-08-13. All four cameras run `a1660798`
(2026-08-27), so all four carry the fixed applier, and all four already show clean
`slots/{a,b}`. The step is no longer load-bearing and is dropped — one fewer
destructive command per camera.

It remains in the failure table: an upload failing with `File exists (os error 17)`
is diagnosed and fixed by exactly that `rm -rf`, derived from `active` at
execution time.

## Decision 4 — the verification gate

`push.c:348` drops oversized frames **silently, with no log line**. A running
daemon, an open socket, and a healthy `/api/diagnostics` are therefore all
compatible with the bug still being present. The gate must observe the stream.

Per camera, all five must hold:

1. `/api/diagnostics.firmware_version` equals the built stamp
2. `active` flipped `b` → `a`, and no `state/trial-*` remains
3. ports 80, 554, 8080 listening
4. **`/main` sustains keyframes** — over a 20 s sample, count frames with
   `key_frame=1`; at a ~2 s GOP expect roughly 10. **A single keyframe is a
   failure, not a pass** — one-IDR-then-silence is the precise signature of the
   bug being fixed.
5. `/live/main.flv` returns a non-zero byte count
6. **`/main` carries an audio track** — `ffprobe` reports a second stream,
   `audio`, 8000 Hz, matching `.198`'s `stream,1,aac,audio,8000`. On `.121` and
   `.146` this is the proof that Edit B took; on `.198` and `.127` it is a
   regression check that the upgrade did not lose the track.

Once, on `.198`: `POST /api/sound/play` returns success. `.198` is physically
next to the operator and reboots first, so hearing it announce itself proves the
whole audio chain — IPC verb, AO worker, amplifier polarity, clip decode — in a
way no remote check can.

## Order

`.198` → `.146` → `.121` → `.127`, strictly sequential, with the full gate between
each. The first failed gate halts the entire rollout.

- **`.198` first**: direct reach, vfat, physically accessible, and audible.
- **`.146` second**: healthiest of the three remote cameras.
- **`.121` third**: telnet is failing today; all of its work goes over FTP and
  HTTP, with telnet used only if a gate needs it.
- **`.127` last**: the most fragile history — it has gone dark and needed physical
  access before. If it surprises us, the rest of the fleet is already done.

Roughly 6 minutes per camera, dominated by the ~150 s apply-and-reboot window.

## Failure handling

| Symptom | Diagnosis | Action |
|---|---|---|
| HTTP 409 on upload | Spool busy | `ls -la /mnt/anyka_hack/spool`; clear a stale `bundle.tar.part`. If `bundle.trigger` exists an apply is queued — wait it out |
| HTTP 401 | Credentials | `CAMERA_PASS`; the account must be Administrator |
| HTTP 413 | Over the 64 MB ceiling | Rebuild; check nothing dragged top-level `lib/` in |
| `File exists (os error 17)` in the device log | Applier could not replace the inactive slot | Re-read `active`, `busybox rm -rf` the *other* slot, re-upload |
| Camera returns on the OLD version | Trial failed, self-reverted | **Halt the rollout.** Read `/mnt/logs/anyka-init.log` for the failing port. Do not re-upload the same tar |
| `/main` yields exactly one keyframe | SHM v4 not actually in effect | Confirm both binaries came from this bundle; check `hdr->version` |
| Cameras silent after upgrade | `[sound]` edit lost or rejected | `grep -c '^\[sound\]' config.toml` on the device; re-run the FTP round-trip |
| HTTP dead, telnet alive | `onvif-rust` down | FTP the bundle to `/mnt/anyka_hack/spool/bundle.tar`, then `touch spool/bundle.trigger` **last** |
| No telnet, no HTTP, no ARP | Both slots unusable | The 240 s deadman in `config.sh` restores the gergehack boot path. Power-cycle; then SD card |

## Out of scope

- **SNMP enablement.** Already on across the fleet; only the missing `snmp.toml`
  file is addressed (Edit C), and that is cosmetic today.
- **`audio_encoding`.** Left at the shipped `G711` everywhere even though `.198`'s
  wire format reports `aac`. The four cameras end up byte-identical in these keys,
  so whatever `.198` negotiates today, the others will too. Chasing the
  discrepancy is a separate investigation.
- **Profiles 2–4 audio.** Stays `false`; `/sub` is deliberately video-only.
