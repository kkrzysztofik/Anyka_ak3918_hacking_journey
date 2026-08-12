# Camera Firmware Upgrade Skill — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add a project skill and two host scripts so agents/operators can build and upload A/B firmware bundles from PR #74.

**Architecture:** Thin bash wrappers over existing `build_sd_contents.sh` / `build_bundle.sh`, plus a skill playbook for verify and recovery. No on-device code changes.

**Tech Stack:** bash, curl, ssh, vendored ARM toolchain (via existing build scripts)

---

### Task 1: `build_upgrade_bundle.sh`

**Files:**
- Create: `scripts/build_upgrade_bundle.sh`

**Steps:**
1. Source `scripts/common.sh`
2. Parse `--skip-www`, `--skip-vendor`, `--debug`, `-h/--help`, optional `OUT` positional
3. Run `scripts/build_sd_contents.sh` with forwarded flags
4. Run `scripts/build_bundle.sh "$OUT"`
5. `chmod +x`

**Verify:** `bash -n scripts/build_upgrade_bundle.sh` and `--help` prints usage.

### Task 2: `upload_upgrade_bundle.sh`

**Files:**
- Create: `scripts/upload_upgrade_bundle.sh`

**Steps:**
1. Source `scripts/common.sh`; require `curl` (and `ssh` when jumphost set)
2. Parse `--host`, `--user`, `--pass`, `--jumphost`, env fallbacks, bundle path
3. Reject missing host/user/pass/bundle
4. LAN or jumphost `curl -T`; capture HTTP status; exit 0 only on 202
5. `chmod +x`

**Verify:** `bash -n`; `--help`; missing-args exits non-zero.

### Task 3: Skill

**Files:**
- Create: `.claude/skills/anyka-firmware-upgrade/SKILL.md`
- Create: `.claude/skills/anyka-firmware-upgrade/reference.md` (layout + failure matrix)

**Steps:**
1. Frontmatter: name + description with trigger terms (firmware upgrade, bundle, A/B, `/api/update`, rollback)
2. Workflow checklist matching the design
3. Link to `reference.md` and PR #74 design/plan docs
4. Point at `anyka-remote-debugging` for telnet/`cam_exec.py`

**Verify:** SKILL.md under 500 lines; description has WHAT + WHEN.

### Task 4: Docs index (optional light touch)

**Files:**
- Modify: `docs/README.md` plans table only if listing recent firmware rows — add 2026-08-13 skill row when convenient.
