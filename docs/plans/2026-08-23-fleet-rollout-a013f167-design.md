# Fleet rollout of `a013f167` to all four cameras

Date: 2026-08-23
Status: design approved, plan pending

## Goal

Ship `main` @ `a013f167` to all four AK3918 cameras via the A/B upgrade path, and
clear three pieces of accumulated device-local drift found during the survey.

## Surveyed state (2026-08-23, all four answered live)

| Camera | Reach | `/mnt` | `active` | Running | Inactive slot | Uptime |
|---|---|---|---|---|---|---|
| 192.168.2.198 | direct | vfat | a | `b48e4847-dirty` | b: `2d88cf37-dirty` | 3d 16h |
| 192.168.30.121 | jumphost | exfat | b | `7737cfbc-dirty` | a: `89eb09d8-dirty` | 9d 10h |
| 192.168.30.146 | jumphost | exfat | b | `ec1d98ac-dirty` | a: `89eb09d8-dirty` | 8d 14h |
| 192.168.30.127 | jumphost | exfat | b | `ec1d98ac-dirty` | a: `89eb09d8-dirty` | 13 min |

Ports 80/554/8080/24 bind on all four.

Two stored assumptions were wrong and are corrected here:

- **`.127` is not on gergehack and does not need a site visit.** It runs the
  `anyka-init` slot layout on `slots/b`, and its `anyka-init.log` shows a single
  clean boot reaching "boot considered good". Its declining `mem_avail_kb` is
  convergence toward the ~2.7 MB steady state that `.121`/`.146` hold after 9
  days, not an OOM loop.
- **`.127` moved from `192.168.3.127` to `192.168.30.127`.** It is now behind the
  jumphost like `.121`/`.146`; `.198` is the only camera left with direct reach.

`.121` is *not* "primed" for the applier workaround as previously recorded — its
inactive slot `a` is populated, so it needs the slot cleared like the others.

## Build

One `bundle.tar` for all four: same board (`Cloud39EV2_AK3918E80PIN_MNBD`), same
`gc1084` sensor, byte-identical vendor libs.

```bash
./scripts/build_upgrade_bundle.sh /tmp/bundle-fleet.tar
```

The path is explicit so the rollout never picks up a stale repo-root
`bundle.tar`, and it is the same path the execution plan uploads.

With a clean working tree the stamp carries no `-dirty` suffix — the first
non-dirty version in the fleet, which makes `firmware_version` an unambiguous
pass/fail. **Do not hard-code the expected value here.** Committing this plan
and the `fix/www-typecheck-ts7` work both moved it; read the stamp the build
actually prints and compare every camera against that.

Build-time preconditions, both of which have bitten this repo before:

- `cross-compile/patches/tower-http-0.7.0-full` must exist. It is gitignored;
  without it the armv5te build fails on `AtomicU64`.
- PR CI never cross-builds ARM (`armv5te` lives only in `release.yml`), so a
  green `main` proves nothing about the camera target.

**The build is the gate.** No camera is touched until a bundle exists.

`CAMERA_PASS` is supplied at run time via env or `--pass-file`, never argv.

## exFAT inactive-slot hazard

The old applier's `stage_and_flip` does `let _ = remove_dir_all(dir)` followed by
`rename(&staging, dir)?`. Rust's `remove_dir_all` fails on exFAT, the error is
swallowed, and `rename` then fails with `File exists (os error 17)`. All three
exFAT cameras currently have a populated inactive slot, which is exactly the
trigger. `.121` runs `7737cfbc`, which predates the `remove_tree` fix, so it
provably hits this.

**Resolution: `busybox rm -rf` the inactive slot before uploading**, on all three
exFAT cameras. This works whether the running applier is old or fixed, so no
detection logic is needed, and it folds the leftover sweep into the same command.

This does not cost the automatic rollback. All three are `active=b`, so the
applier stages into `a` and flips to it; a failed trial flips back to `b`, which
still holds the build the camera is running now. Only `89eb09d8` is lost — two
generations stale and never a realistic recovery target.

**Derive the target from `active`, never from a hardcoded letter.** All three are
`active=b` today, but a camera that self-reverted mid-rollout flips to `a`, and an
`rm -rf slots/a` issued from a stale assumption would delete the running slot out
from under a live process.

`.198` is vfat and was never affected; it gets no slot prep.

## Device-local config drift

`anyka.toml` and `onvif/config.toml` are deliberately excluded from the bundle, so
these edits survive the upgrade. `[time]` is read by `anyka-init` at boot and the
upgrade reboots anyway, so sequencing edits *before* upload means one reboot does
both jobs.

| Camera | File | Change | Why |
|---|---|---|---|
| `.127` | `anyka.toml` `[time]` | `enabled = false` → `true` | Leftover from the 2026-08-14 remote-recovery session, the same edit set that added the static IP and disabled `udhcpc`. Everything else from that session was already undone; this line was missed. It is the sole reason the clock reads 1970. |
| `.127` | live | `date -s` | Fixes the clock immediately without a reboot, so verification is not run against a 1970 clock. The config edit makes it stick. |
| `.127` | `onvif/config.toml:268` | `ir_cut_filter = "OFF"` → `"AUTO"` | Leftover from the IR strobe stopgap. Matches `.146`. Bundles never overwrite `config.toml`, so it persists silently otherwise. |
| `.146` | `anyka.toml` `[time] servers` | `192.168.3.1` → `192.168.30.1` | Gateway of a network it no longer lives on. Its clock is correct today only via the `0.ubuntu.pool.ntp.org` fallback. |

`192.168.30.1` was verified to answer NTP with correct time (probed from the
jumphost; an apparent 2h offset was the jumphost's own CEST display, not server
skew). `.127`'s `servers` list already points at it.

The clock is not cosmetic: `anyka.toml`'s own comment notes that at 1970,
`ws_security` rejects every authenticated ONVIF request on ±300 s skew. `PUT
/api/update` uses HTTP Basic and survives, but post-upgrade ONVIF verification
against `.127` would fail for reasons unrelated to the build.

## Order: `.198` → `.146` → `.121` → `.127`

- **`.198`** canary — direct reach, vfat, physically accessible.
- **`.146`** — healthiest jumphost candidate (8d uptime, newest build, applier
  already fixed). Proves the bundle on exFAT behind the jumphost.
- **`.121`** — biggest version jump and the only provably-old applier. Goes after
  the bundle is proven twice.
- **`.127`** — most unknown recent history (13 min uptime, just changed networks),
  it is the camera whose config we are also editing, and it is the one that has
  already gone dark and needed physical access. If anything surprises us, the rest
  of the fleet is already done.

Canary soak: **immediate**. Verify `.198` and move on.

## Per-camera sequence

1. Open the tunnel, then run `ifconfig wlan0` to **confirm which camera answered**.
   A stale forward silently wins the local port, and every "`.121`" command then
   lands on `.146` with plausible-looking output. Use a per-camera local port
   (`12421` → `.121`, `12446` → `.146`, `12427` → `.127`).
2. `cat /mnt/anyka_hack/active` → derive the inactive slot.
3. Apply any config edits for this camera.
4. exFAT only: `busybox rm -rf slots/<inactive> slots/*.aside slots/*.aside2
   slots/*.preidr && sync`.
5. Upload, require **HTTP 202**.
6. Wait out the apply window: ~60 s supervisor poll, then stage + flip + reboot,
   ~90–120 s dark. Do not re-upload during apply.
7. Verify before starting the next camera.

## Verification gate

Per camera, all must hold before moving on:

- `curl /api/diagnostics` → `firmware_version == a013f167`
- `cat /mnt/anyka_hack/active` flipped to the other slot
- no `state/trial-*` remaining (trial committed)
- ports 80, 554, 8080 listening
- one live RTSP pull and one HTTP-FLV pull

`.127` additionally: `date` returns real time, `ir_cut_filter` reads `AUTO`.

## Failure handling

| Symptom | Action |
|---|---|
| HTTP 409 | Spool busy — check `spool/bundle.trigger`, `spool/bundle.tar.part` |
| HTTP 401 | Credentials |
| `File exists (os error 17)` | The `rm` hit the wrong slot; re-derive from `active` |
| Trial reverted | Camera returns on its old version. **Stop the rollout.** Do not re-upload the same tar |
| HTTP dead, telnet alive | FTP into `spool/bundle.tar`, then `touch spool/bundle.trigger` last |
| Both slots dead | 240 s deadman restores the gergehack boot path — site visit |

## Risks

- **`.121` is the largest jump** (`7737cfbc` → `a013f167`) on the oldest applier.
  Mitigated by ordering it third, after the bundle is proven twice.
- **`.127` has previously gone off-network and needed physical access.** Mitigated
  by ordering it last and by never removing a watchdog trigger — the failure in
  August was caused by a static-IP change that left `anyka-init` running happily
  with no network and nothing to force the fallback.
- **No ARM coverage in PR CI.** Mitigated by treating the local cross-build as the
  gate.

## Out of scope

Leftover `slots/*.aside*` and `*.preidr` directories on `.198` — it has none.
