---
name: anyka-firmware-upgrade
description: Build and push A/B firmware upgrade bundles to Anyka AK3918 cameras (build_bundle, push_bundle, package_bundle, build_payload, push_payload, PUT /api/update, slot rollback, spool recovery). Use when upgrading a camera installation, deploying a firmware bundle or full SD payload, verifying trial commit/revert, or recovering a failed update.
version: 1.0.0
---

# Anyka Firmware Upgrade (A/B slots)

Atomic upgrade of `anyka-init` + `vendor-daemon` + `onvif-rust` via versioned `bundle.tar`, with automatic rollback if ports 80/554/8080 never bind after reboot.

Canonical design: `docs/plans/2026-08-12-firmware-upgrade-path-design.md` (PR #74).

## When to use

- User asks to update/upgrade/install firmware on a camera
- Need to produce or upload `bundle.tar`
- Post-upload verify, rollback diagnosis, or FTP spool recovery

Do **not** use `scripts/push_binary_dev.sh` (per-binary FTP) for this path.

## Which path do I need?

This skill is the single entry point for getting code onto a camera.

| I want to… | Path |
|---|---|
| Ship a versioned change to a running camera | `build_bundle.sh` → `push_bundle.sh` — A/B, auto-rollback. **The default.** |
| Set up a fresh camera, or change `lib/` or Factory | `build_payload.sh` → `push_payload.sh` — see *Full SD payload* below |
| Move a flat camera onto slots (once per camera) | `scripts/migrate_to_slots.sh` |
| Iterate on one binary while debugging | `scripts/push_binary_dev.sh` + `scripts/run_binary_dev.sh` — dev only, never for upgrades |

There is no SSH on the camera: every path is FTP, a mounted SD card, or
`PUT /api/update`. The old `anyka-embedded-build/scripts/deploy.sh` was
ssh/scp-based, never worked, and has been removed.

## Full SD payload

Use when a bundle cannot carry what you need — a fresh camera, a toolchain bump
that changes `lib/`, or a change under `Factory/`. Build and copy are separate
steps:

```bash
./scripts/build_payload.sh                # compile into SD_card_contents/ only
#   --skip-www   skip the WebUI build (local iteration only)
#   --debug      debug binaries

./scripts/push_payload.sh --sd /path/to/mount   # mounted SD card
./scripts/push_payload.sh --ftp 192.168.2.198   # over FTP to /mnt/anyka_hack + /mnt/Factory
```

Unlike a bundle this ships `lib/` (31 MB of uClibc) and the Factory tree, and it
is **not** covered by `manifest.sha256` or the trial/rollback machinery. When
pushing with `tar | nc` instead, verify every file against an md5 manifest
afterwards — silent NUL-byte writes on exFAT are a known failure.

## Quick commands

```bash
# 1) Cross-compile + package
./scripts/build_bundle.sh
# optional: --skip-vendor | --debug | OUT path

# 2) Push (Administrator Basic auth) — expect HTTP 202.
# Password from env or --pass-file, never argv (keeps it out of history/`ps`).
CAMERA_PASS="$CAMERA_PASS" ./scripts/push_bundle.sh --host 192.168.2.198 \
  --user admin bundle.tar

# Behind jumphost
CAMERA_PASS="$CAMERA_PASS" ./scripts/push_bundle.sh --host 192.168.30.10 \
  --jumphost root@192.168.3.137 --user admin bundle.tar
```

Env fallbacks: `CAMERA_HOST`, `CAMERA_USER`, `CAMERA_PASS`, `CAMERA_JUMPHOST`.

**`build_bundle.sh` compiles then packages. `package_bundle.sh` only packages.**
Reach for `package_bundle.sh [OUT]` only when `SD_card_contents/` is already
fresh — on a stale tree it succeeds and ships last week's binaries.

Use `./scripts/build_payload.sh --skip-www` only for local payload iteration,
not for versioned upgrade bundle packaging.

## Agent workflow

Copy and track:

```
Upgrade progress:
- [ ] 1. Preconditions
- [ ] 2. Build bundle
- [ ] 3. Upload (202)
- [ ] 4. Wait for reboot
- [ ] 5. Verify commit
- [ ] 6. Handle revert / recovery if needed
```

### 1. Preconditions

Camera must already be on the slot layout (`/mnt/anyka_hack/active`, `slots/{a,b}`, `spool/`). Check via telnet:

```bash
scripts/debugging/cam_exec.py --host <ip> 'ls /mnt/anyka_hack/active /mnt/anyka_hack/slots /mnt/anyka_hack/spool'
```

If missing, stop — run the PR #74 migration first (do not upload onto the old flat layout).

Need Admin credentials for `PUT /api/update`.

### 2. Build

```bash
./scripts/build_bundle.sh
```

Note the printed bundle version (from `onvif/.build-version`, matching the
binary's `FirmwareVersion`). Bundle excludes `lib/` and device-local config
(`anyka.toml`, `onvif/config.toml`).

### 3. Upload

```bash
CAMERA_PASS="$CAMERA_PASS" ./scripts/push_bundle.sh --host <ip> --user admin bundle.tar
```

Require **HTTP 202**. Common failures: 401 (auth), 409 (upload/spool busy), 413 (too large).

### 4. Reboot window

Applier polls on the supervisor tick (~60 s), stages inactive slot, flips `active`, reboots. ~90 s downtime is normal. Do not re-upload during apply.

### 5. Verify commit

After the camera answers again:

```bash
# Version identity
curl -su admin:"$CAMERA_PASS" http://<ip>/api/diagnostics | jq .firmware_version
# expect git describe from the build

# Trial cleared + active slot
scripts/debugging/cam_exec.py --host <ip> \
  'cat /mnt/anyka_hack/active; ls /mnt/anyka_hack/state/trial-* 2>/dev/null; echo done'
```

Ports **80** (HTTP/ONVIF), **554** (RTSP), **8080** (HTTP-FLV) must be listening. Trial holds ~30 s within a ~120 s deadline; success unlinks `state/trial-<prev>`.

### 6. Revert / recovery

**Self-revert (trial failed):** device flips `active` back and reboots. Confirm previous version serves. Fix the bundle before uploading again — do not loop the same bad tar.

**HTTP dead (onvif-rust down):** FTP the file into spool, then trigger:

```text
/mnt/anyka_hack/spool/bundle.tar
/mnt/anyka_hack/spool/bundle.trigger   # touch last
```

**Telnet checks** (see skill `anyka-remote-debugging`):

```bash
scripts/debugging/cam_exec.py --host <ip> 'cat /mnt/anyka_hack/active'
scripts/debugging/cam_exec.py --host <ip> 'ls -la /mnt/anyka_hack/slots /mnt/anyka_hack/state /mnt/anyka_hack/spool'
```

**Both slots unusable:** `config.sh` falls back, then the existing 240 s deadman restores the vendor boot path — SD card / site visit.

## Do not

- Hardcode passwords in scripts, commits, or skill examples with real secrets
- Put `lib/` or live device config into the bundle
- Use per-binary `push_binary_dev.sh` for A/B upgrades
- Re-upload while `spool/bundle.trigger` or `bundle.tar.part` exists (409)
- Reach for ssh/scp — the camera runs no sshd; use FTP, SD card, or `PUT /api/update`

## Additional resources

- On-disk layout and failure matrix: [reference.md](reference.md)
- Design: `docs/plans/2026-08-12-firmware-upgrade-path-design.md`
- Skill design: `docs/plans/2026-08-13-camera-firmware-upgrade-skill-design.md`
- Telnet/FTP debugging: skill `anyka-remote-debugging`
- Cross-compile details: skill `anyka-embedded-build`
