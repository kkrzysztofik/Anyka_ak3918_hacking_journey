# Camera Firmware Upgrade Skill — Design

Date: 2026-08-13
Scope: Host-side skill + scripts for building and uploading A/B upgrade bundles (PR #74).

## Problem

PR #74 shipped the on-device applier, `PUT /api/update`, and `scripts/build_bundle.sh`
(package-only). Operators and agents still need a single documented path to:

1. Cross-compile + assemble + package a `bundle.tar`
2. Upload it (LAN or jumphost) with Admin auth
3. Verify commit vs automatic rollback, and recover when HTTP is dead

## Decisions

| Decision | Choice |
|---|---|
| Build entry point | New `scripts/build_upgrade_bundle.sh` = `build_sd_contents.sh` then `build_bundle.sh` |
| Package-only | Keep existing `scripts/build_bundle.sh` unchanged |
| Upload | Separate `scripts/upload_upgrade_bundle.sh` (Basic auth + optional jumphost) |
| Verify / recovery | Documented in the skill only — no wait-for-reboot wrapper script |
| Skill location | `.claude/skills/anyka-firmware-upgrade/` (project skill) |
| Credentials | Flags and/or env (`CAMERA_*`); never hardcoded |

## Scripts

### `build_upgrade_bundle.sh`

Passes `--skip-www`, `--skip-vendor`, `--debug` through to `build_sd_contents.sh`.
Optional positional `OUT` path (default `bundle.tar` at repo root), forwarded to
`build_bundle.sh`.

### `upload_upgrade_bundle.sh`

```text
--host / CAMERA_HOST
--user / CAMERA_USER
--pass / CAMERA_PASS
--jumphost / CAMERA_JUMPHOST   # e.g. root@192.168.3.137
bundle path (positional)
```

- LAN: `curl -T` → `http://<host>/api/update` with Basic auth
- Jumphost: `ssh <jumphost> 'curl -T - …' < bundle.tar`
- Success = HTTP **202** only; print body on failure
- Long transfer timeout (bundles ~19 MB; ceiling 64 MB on device)

## Skill playbook

1. Preconditions (slot layout migrated)
2. Build via `build_upgrade_bundle.sh`
3. Upload via `upload_upgrade_bundle.sh` → 202
4. Expect reboot (~90 s)
5. Verify: ports 80/554/8080, diagnostics `firmware_version`, no `state/trial-*`
6. On revert: confirm previous slot serving; fix before re-upload
7. Recovery: FTP spool + `bundle.trigger`; telnet via `cam_exec.py`; deadman → vendor path

## Out of scope

- Changes to applier / ONVIF / WebUI
- Auto-verify shell wrapper
- Replacing or deleting `deploy_onvif.sh`

## Success criteria

- One command produces uploadable `bundle.tar`
- One command uploads LAN or via jumphost and fails clearly on non-202
- Agent following the skill can verify commit vs rollback and recover via FTP spool
