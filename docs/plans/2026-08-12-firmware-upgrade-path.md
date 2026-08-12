# A/B Firmware Upgrade Path Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Let a camera accept a versioned bundle of `anyka-init` + `vendor-daemon` + `onvif-rust`, activate it atomically, and roll itself back automatically when the new build fails to come up.

**Architecture:** Two payload slots on the SD card with an `active` pointer file. `anyka-init` gains an applier thread that verifies a bundle dropped into `spool/`, stages it into the inactive slot, flips the pointer, and reboots. On the next boot a marker file signals an unconfirmed update; the supervisor watches `/proc/net/tcp` for the three service ports and either deletes the marker (commit) or flips the pointer back and reboots (revert). `SD_card_contents/Factory/config.sh` reads the pointer and falls back to the other slot, layered above the vendor-boot-path deadman it already has.

**Tech Stack:** Rust 2024 (`anyka-init`, no new crate dependencies), busybox `sha256sum` and `tar` invoked through the existing `Sys` trait, POSIX shell for `config.sh`, `mockall` + `tempfile` for tests.

**Design:** `docs/plans/2026-08-12-firmware-upgrade-path-design.md`

---

## Deviation from the design doc

The design says `manifest.json`. Use **`manifest.sha256`** in `sha256sum -c` format instead, plus a `manifest.meta` of `key=value` lines. Reasons:

- `anyka-init` has no `serde_json`, and `storm.rs:26-47` already documents the house rule ("deliberately hand-rolled rather than pulling in serde_json for two integers"). A per-file JSON hash list is past what hand-rolling should cover.
- `sha256sum -c` does the whole verify in one exit status, using a busybox applet already on the device, invoked through the existing `Sys::run_to_completion` (`sys.rs:241`). Zero new dependencies.
- It stays hand-checkable over telnet during a bad night.

Task 1 confirms the applet exists before anything is built on it.

---

## Increment 1 — the applier

Zero `onvif-rust` changes. Transport is an FTP drop into `spool/`, which works on the fleet today.

---

### Task 1: Confirm busybox `sha256sum` and `tar` on the device — **DONE, PASSED**

Verified on `192.168.2.198` on 2026-08-12. No work remains; this record exists so
the executor does not re-litigate the assumptions the later tasks rest on.

**busybox v1.24.1**, both applets present:

```
Usage: sha256sum [-c[sw]] [FILE]...
Usage: tar -[cxtzhmvO] [-X FILE] [-T FILE] [-f TARFILE] [-C DIR] [FILE]...
```

The verify mechanism was exercised end to end, not just probed for existence:

| Case | Result |
|---|---|
| Manifest matches every file | all `OK`, `rc=0` |
| One file corrupted | `FAILED`, `1 of 2 computed checksums did NOT match`, `rc=1` |
| One file deleted | `can't open ...`, `FAILED`, `rc=1` |
| `tar -cf` → `tar -xf -C` → `sha256sum -c` round trip | all `OK`, `rc=0` |

Both failure modes the applier depends on — corruption and absence — produce a
nonzero exit, which is the whole contract `verify_slot` is built on.

**Two findings that shape later tasks:**

1. **busybox `find` has no `-printf`** (GNU-only; it silently produced an empty
   manifest). Irrelevant on-device, since the camera only ever *verifies*. But
   `scripts/build_bundle.sh` in Task 12 must stay host-side, where GNU `find`
   provides it. Do not port manifest generation to the camera.
2. **Disk:** `/dev/mmcblk0p1`, vfat, 29.7 G with **28.8 G available**. `mount`
   shows a leftover `tmpfs on /mnt` that the vfat is mounted over; both report
   identical usage and `/mnt/anyka_hack` holds 721 MB of real content, so the
   card is what is live. Two ~19 MB slots are free in practice.
3. **RAM:** 36 MB total, 2.5 MB free (26 MB counting buffers/cache). This is why
   Task 14 streams the upload body straight to disk. Nothing may be buffered.

---

### Task 2: `netstat::listening` — read listening ports from `/proc/net/tcp`

**Files:**
- Modify: `cross-compile/anyka-init/src/netstat.rs` (append; follow the existing `/proc/net/route` and `/proc/net/arp` helpers at `:16` and `:43`)

**Step 1: Write the failing test**

Append to the `mod tests` block at the bottom of `netstat.rs`. The fixture is
real output captured from `192.168.2.198` on 2026-08-12 — 022A is RTSP, 1F90 is
HTTP-FLV, 0050 is ONVIF, 0015/0018 are FTP and telnet. Only the last line is
synthetic, added to cover a non-LISTEN state:

```rust
const TCP_FIXTURE: &str = "\
  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
   0: 00000000:022A 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 653 1 c2225680 100 0 0 10 -1
   1: 0100007F:224E 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 132 1 c2224900 100 0 0 10 -1
   2: 00000000:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 654 1 c2224d80 100 0 0 10 -1
   3: 00000000:0050 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 598 1 c2225200 100 0 0 10 -1
   4: 00000000:0015 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 112 1 c2224480 100 0 0 10 -1
   5: 00000000:0018 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 325 1 c2224000 100 0 0 10 -1
   6: 0100007F:1F91 0100007F:C350 01 00000000:00000000 00:00000000 00000000     0        0 999 1 c2224111 100 0 0 10 -1
";

#[test]
fn listening_finds_a_listening_port() {
    assert!(parse_listening(TCP_FIXTURE, 80));
}

#[test]
fn listening_finds_a_high_port_in_hex() {
    // 0x022A == 554
    assert!(parse_listening(TCP_FIXTURE, 554));
}

#[test]
fn listening_finds_every_trial_port() {
    for p in crate::update::TRIAL_PORTS {
        assert!(parse_listening(TCP_FIXTURE, p), "port {p} should be listening");
    }
}

#[test]
fn listening_rejects_an_established_socket() {
    // 0x1F91 == 8081, present but state 01 (ESTABLISHED), not 0A (LISTEN)
    assert!(!parse_listening(TCP_FIXTURE, 8081));
}

#[test]
fn listening_rejects_an_absent_port() {
    assert!(!parse_listening(TCP_FIXTURE, 12345));
}

#[test]
fn listening_tolerates_garbage() {
    assert!(!parse_listening("", 80));
    assert!(!parse_listening("not a proc file at all", 80));
}
```

**Step 2: Run the tests to verify they fail**

```bash
source setenv.sh
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu listening
```

Expected: FAIL, `cannot find function 'parse_listening' in this scope`.

**Step 3: Write the implementation**

Append to `netstat.rs`, above the tests:

```rust
/// TCP_LISTEN as `/proc/net/tcp` spells it.
const TCP_LISTEN: &str = "0A";

/// Is anything listening on `port`?
///
/// Reads `/proc/net/tcp` rather than connecting, so it costs no socket and
/// cannot be fooled by a half-open accept queue. IPv4 only — every service
/// this supervises binds v4.
pub fn listening(port: u16) -> bool {
    std::fs::read_to_string("/proc/net/tcp").is_ok_and(|s| parse_listening(&s, port))
}

/// Split out so the parse is testable without a real `/proc`.
///
/// Columns are `sl local_address rem_address st ...`, first line a header.
/// Destructuring all four up front is duller than chaining iterator adaptors
/// and does not invite a fencepost error on the one field that matters.
fn parse_listening(src: &str, port: u16) -> bool {
    src.lines().skip(1).any(|line| {
        let mut f = line.split_whitespace();
        let (Some(_sl), Some(local), Some(_rem), Some(state)) =
            (f.next(), f.next(), f.next(), f.next())
        else {
            return false;
        };
        state == TCP_LISTEN
            && local
                .rsplit(':')
                .next()
                .and_then(|hex| u16::from_str_radix(hex, 16).ok())
                == Some(port)
    })
}
```

Known limitation, deliberately not guarded: this matches on port only, so a
service bound to `127.0.0.1` alone would pass the trial while being externally
unreachable. The live `/proc/net/tcp` confirms every supervised service binds
`0.0.0.0` (only one unrelated listener sits on loopback), and checking the
address costs more than the failure is worth.

**Step 4: Run the tests to verify they pass**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu listening
```

Expected: PASS, 5 tests.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/netstat.rs
git commit -m "feat(anyka-init): read listening TCP ports from /proc/net/tcp"
```

---

### Task 3: Slot layout and the `active` pointer

**Files:**
- Create: `cross-compile/anyka-init/src/update.rs`
- Modify: `cross-compile/anyka-init/src/lib.rs` (add `pub mod update;`)

**Step 1: Write the failing test**

Create `update.rs` with only a `mod tests` block:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_defaults_to_a_when_the_pointer_is_missing() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(Slots::new(d.path()).active(), Slot::A);
    }

    #[test]
    fn slot_reads_the_pointer_file() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("active"), "b\n").unwrap();
        assert_eq!(Slots::new(d.path()).active(), Slot::B);
    }

    #[test]
    fn garbage_in_the_pointer_reads_as_a() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("active"), "\0\0\0\0").unwrap();
        assert_eq!(Slots::new(d.path()).active(), Slot::A);
    }

    #[test]
    fn set_active_round_trips() {
        let d = tempfile::tempdir().unwrap();
        let s = Slots::new(d.path());
        s.set_active(Slot::B).unwrap();
        assert_eq!(s.active(), Slot::B);
        assert_eq!(s.inactive(), Slot::A);
    }

    #[test]
    fn slot_dir_is_under_slots() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(Slots::new(d.path()).dir(Slot::B), d.path().join("slots/b"));
    }
}
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu update::
```

Expected: FAIL — `update` module not found (you have not added it to `lib.rs` yet) or `Slots` undefined.

**Step 3: Write the implementation**

Prepend to `update.rs`:

```rust
//! A/B slot activation and the update applier.
//!
//! Two payload trees live under `slots/a` and `slots/b`; a one-byte `active`
//! file selects which one boots. `/mnt` is vfat/exFAT, which has no symlinks,
//! so the pointer has to be a real file rather than a `current ->` link.
//!
//! Anything unreadable or unrecognized reads as slot A, matching the storm
//! guard's rule (`storm.rs:36-38`): a torn read on a card that just lost power
//! must not strand the camera.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    A,
    B,
}

impl Slot {
    pub fn name(self) -> &'static str {
        match self {
            Slot::A => "a",
            Slot::B => "b",
        }
    }

    pub fn other(self) -> Slot {
        match self {
            Slot::A => Slot::B,
            Slot::B => Slot::A,
        }
    }
}

/// The slot layout rooted at `/mnt/anyka_hack` in production.
pub struct Slots {
    root: PathBuf,
}

impl Slots {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn pointer(&self) -> PathBuf {
        self.root.join("active")
    }

    /// Currently selected slot. Unreadable or unrecognized reads as A.
    pub fn active(&self) -> Slot {
        match std::fs::read_to_string(self.pointer()) {
            Ok(s) if s.trim() == "b" => Slot::B,
            _ => Slot::A,
        }
    }

    pub fn inactive(&self) -> Slot {
        self.active().other()
    }

    pub fn dir(&self, slot: Slot) -> PathBuf {
        self.root.join("slots").join(slot.name())
    }

    /// Write the pointer via temp + rename + sync, the same durability dance
    /// `storm.rs:58-70` uses: a power cut leaves the old byte or the new one,
    /// never an empty file that would read as slot A by accident.
    pub fn set_active(&self, slot: Slot) -> std::io::Result<()> {
        use std::io::Write;
        std::fs::create_dir_all(&self.root)?;
        let tmp = self.root.join("active.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(slot.name().as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp, self.pointer())?;
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }
}
```

Add to `cross-compile/anyka-init/src/lib.rs`, in alphabetical position among the existing `pub mod` lines:

```rust
pub mod update;
```

**Step 4: Run to verify it passes**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu update::
```

Expected: PASS, 5 tests.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs cross-compile/anyka-init/src/lib.rs
git commit -m "feat(anyka-init): A/B slot layout and active pointer file"
```

---

### Task 4: Trial marker file

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs`

**Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn no_marker_means_no_trial() {
    let d = tempfile::tempdir().unwrap();
    assert_eq!(Trial::find(d.path()), None);
}

#[test]
fn marker_round_trips_the_previous_slot() {
    let d = tempfile::tempdir().unwrap();
    Trial::arm(d.path(), Slot::A).unwrap();
    assert_eq!(Trial::find(d.path()), Some(Slot::A));
}

#[test]
fn clearing_the_marker_ends_the_trial() {
    let d = tempfile::tempdir().unwrap();
    Trial::arm(d.path(), Slot::B).unwrap();
    Trial::clear(d.path()).unwrap();
    assert_eq!(Trial::find(d.path()), None);
}

#[test]
fn clearing_an_absent_marker_is_not_an_error() {
    let d = tempfile::tempdir().unwrap();
    Trial::clear(d.path()).unwrap();
}
```

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu update::
```

Expected: FAIL, `cannot find type 'Trial'`.

**Step 3: Write the implementation**

Add to `update.rs`:

```rust
/// An unconfirmed update, recorded as the *existence* of `state/trial-<slot>`.
///
/// Deliberately not a parsed file. There is no `serde_json` here, and the
/// storm guard already learned that reading structured state off exFAT after a
/// power cut is a hazard (`boot-runtime-rust-design.md:449`). A filename
/// cannot be half-parsed: it is there or it is not, and its last character is
/// the whole payload.
pub struct Trial;

impl Trial {
    fn dir(root: &Path) -> PathBuf {
        root.join("state")
    }

    /// The slot to fall back to, if an update is awaiting confirmation.
    pub fn find(root: &Path) -> Option<Slot> {
        let entries = std::fs::read_dir(Self::dir(root)).ok()?;
        for e in entries.flatten() {
            match e.file_name().to_str() {
                Some("trial-a") => return Some(Slot::A),
                Some("trial-b") => return Some(Slot::B),
                _ => {}
            }
        }
        None
    }

    /// Record that `prev` was active before the flip we are about to make.
    pub fn arm(root: &Path, prev: Slot) -> std::io::Result<()> {
        let dir = Self::dir(root);
        std::fs::create_dir_all(&dir)?;
        std::fs::File::create(dir.join(format!("trial-{}", prev.name())))?.sync_all()?;
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }

    /// Commit: the update proved itself.
    pub fn clear(root: &Path) -> std::io::Result<()> {
        let dir = Self::dir(root);
        for name in ["trial-a", "trial-b"] {
            match std::fs::remove_file(dir.join(name)) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
            }
        }
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 9 tests in `update::`.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs
git commit -m "feat(anyka-init): trial marker file for unconfirmed updates"
```

---

### Task 5: Bundle verification

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs`

A staged slot is valid when `busybox sha256sum -c manifest.sha256` succeeds with the slot directory as CWD, **and** `manifest.meta`'s `requires_config_schema` is at most the running `anyka.toml`'s `schema`.

**Step 1: Write the failing test**

```rust
#[test]
fn meta_parses_the_schema_requirement() {
    let m = Meta::parse("version=v1.2.3\nrequires_config_schema=2\n");
    assert_eq!(m.version.as_deref(), Some("v1.2.3"));
    assert_eq!(m.requires_config_schema, 2);
}

#[test]
fn missing_schema_requirement_reads_as_zero() {
    assert_eq!(Meta::parse("version=v1\n").requires_config_schema, 0);
}

#[test]
fn torn_meta_reads_as_zero() {
    let m = Meta::parse("\0\0\0garbage");
    assert_eq!(m.requires_config_schema, 0);
    assert_eq!(m.version, None);
}

#[test]
fn schema_newer_than_the_device_is_rejected() {
    assert!(!Meta { version: None, requires_config_schema: 3 }.compatible_with(2));
}

#[test]
fn schema_at_or_below_the_device_is_accepted() {
    assert!(Meta { version: None, requires_config_schema: 2 }.compatible_with(2));
    assert!(Meta { version: None, requires_config_schema: 1 }.compatible_with(2));
}
```

**Step 2: Run to verify it fails**

Expected: FAIL, `cannot find type 'Meta'`.

**Step 3: Write the implementation**

```rust
/// `manifest.meta`: `key=value` lines. Unparseable anything reads as zero,
/// which fails the compatibility check closed for a nonzero device schema and
/// open for schema 0 — matching the pre-schema bundles that predate this.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Meta {
    pub version: Option<String>,
    pub requires_config_schema: u32,
}

impl Meta {
    pub fn parse(src: &str) -> Self {
        let mut m = Self::default();
        for line in src.lines() {
            match line.split_once('=') {
                Some(("version", v)) => m.version = Some(v.trim().to_string()),
                Some(("requires_config_schema", v)) => {
                    m.requires_config_schema = v.trim().parse().unwrap_or(0);
                }
                _ => {}
            }
        }
        m
    }

    pub fn compatible_with(&self, device_schema: u32) -> bool {
        self.requires_config_schema <= device_schema
    }
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 14 tests in `update::`.

**Step 5: Add the checksum verification test**

`sha256sum` runs through `Sys`, so it mocks. Add:

```rust
#[test]
fn verify_runs_sha256sum_in_the_slot_directory() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    let slot = d.path().join("slots/b");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("manifest.sha256"), "abc  anyka-init.bin\n").unwrap();
    std::fs::write(slot.join("manifest.meta"), "requires_config_schema=1\n").unwrap();

    let mut sys = MockSys::new();
    sys.expect_run_to_completion()
        .withf(|prog, args| prog == "busybox" && args[0] == "sha256sum" && args[1] == "-c")
        .returning(|_, _| Ok(exit_ok()));

    assert!(verify_slot(&sys, &slot, 1).is_ok());
}

#[test]
fn verify_fails_when_a_checksum_does_not_match() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    let slot = d.path().join("slots/b");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("manifest.sha256"), "abc  anyka-init.bin\n").unwrap();
    std::fs::write(slot.join("manifest.meta"), "requires_config_schema=1\n").unwrap();

    let mut sys = MockSys::new();
    sys.expect_run_to_completion().returning(|_, _| Ok(exit_fail()));

    assert!(verify_slot(&sys, &slot, 1).is_err());
}

#[test]
fn verify_fails_on_an_incompatible_schema_without_hashing() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    let slot = d.path().join("slots/b");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("manifest.sha256"), "abc  anyka-init.bin\n").unwrap();
    std::fs::write(slot.join("manifest.meta"), "requires_config_schema=9\n").unwrap();

    let mut sys = MockSys::new();
    sys.expect_run_to_completion().never();

    assert!(verify_slot(&sys, &slot, 1).is_err());
}
```

You will need `exit_ok()` / `exit_fail()` helpers producing an `ExitStatus`. Check how the existing `anyka-init` tests build one — grep `ExitStatus` in `tests/supervision.rs` and `src/`, and reuse that construction rather than inventing a second one.

**Step 6: Implement `verify_slot`**

```rust
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    #[error("manifest unreadable in {0}")]
    NoManifest(PathBuf),
    #[error("bundle needs config schema {needs}, device has {has}")]
    Schema { needs: u32, has: u32 },
    #[error("checksum verification failed")]
    Checksum,
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Verify a staged slot. Schema first: it is free, and a bundle that can never
/// run should not cost a full pass over 19 MB of hashing.
pub fn verify_slot(
    sys: &dyn crate::sys::Sys,
    slot_dir: &Path,
    device_schema: u32,
) -> Result<Meta, UpdateError> {
    let meta_src = std::fs::read_to_string(slot_dir.join("manifest.meta"))
        .map_err(|_| UpdateError::NoManifest(slot_dir.to_path_buf()))?;
    let meta = Meta::parse(&meta_src);
    if !meta.compatible_with(device_schema) {
        return Err(UpdateError::Schema {
            needs: meta.requires_config_schema,
            has: device_schema,
        });
    }
    if !slot_dir.join("manifest.sha256").is_file() {
        return Err(UpdateError::NoManifest(slot_dir.to_path_buf()));
    }
    // busybox resolves the manifest's relative paths against CWD, so run it
    // from the slot root. This also catches the exFAT NUL-byte write artifact,
    // which is a write failure and therefore invisible to any transfer check.
    let args = [
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "cd {} && busybox sha256sum -c manifest.sha256",
            shell_quote(slot_dir)
        ),
    ];
    match sys.run_to_completion("busybox", &args) {
        Ok(st) if st.success() => Ok(meta),
        _ => Err(UpdateError::Checksum),
    }
}

/// Single-quote a path for `sh -c`. Slot paths are ours, not user input, but
/// quoting costs one line and removes the question.
fn shell_quote(p: &Path) -> String {
    format!("'{}'", p.to_string_lossy().replace('\'', r"'\''"))
}
```

**Step 7: Run to verify it passes**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu update::
```

Expected: PASS, 17 tests.

**Step 8: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs
git commit -m "feat(anyka-init): verify staged slots by manifest checksum and config schema"
```

---

### Task 6: The trial evaluation

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs`

**Step 1: Write the failing test**

The predicate must be injectable so tests do not need real sockets:

```rust
#[test]
fn trial_passes_when_every_port_is_bound_for_the_hold() {
    let mut calls = 0;
    let outcome = evaluate_trial(
        &[80, 554, 8080],
        Policy { hold_secs: 3, deadline_secs: 10 },
        |_port| { calls += 1; true },
        |_| {},
    );
    assert_eq!(outcome, Outcome::Confirm);
    assert!(calls >= 3);
}

#[test]
fn trial_fails_when_one_port_never_binds() {
    let outcome = evaluate_trial(
        &[80, 554, 8080],
        Policy { hold_secs: 3, deadline_secs: 6 },
        |port| port != 554,
        |_| {},
    );
    assert_eq!(outcome, Outcome::Revert);
}

#[test]
fn a_late_bind_still_confirms_inside_the_deadline() {
    let mut tick = 0;
    let outcome = evaluate_trial(
        &[554],
        Policy { hold_secs: 2, deadline_secs: 20 },
        |_| { tick += 1; tick > 5 },
        |_| {},
    );
    assert_eq!(outcome, Outcome::Confirm);
}

#[test]
fn a_flapping_port_resets_the_hold_and_eventually_reverts() {
    let mut tick = 0;
    let outcome = evaluate_trial(
        &[80],
        Policy { hold_secs: 3, deadline_secs: 8 },
        |_| { tick += 1; tick % 2 == 0 },
        |_| {},
    );
    assert_eq!(outcome, Outcome::Revert);
}
```

**Step 2: Run to verify it fails**

Expected: FAIL, `cannot find function 'evaluate_trial'`.

**Step 3: Write the implementation**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Confirm,
    Revert,
}

#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// Consecutive seconds every port must stay bound.
    pub hold_secs: u32,
    /// Give up after this long.
    pub deadline_secs: u32,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            hold_secs: 30,
            deadline_secs: 120,
        }
    }
}

/// Ports the trial requires. From
/// `SD_card_contents/anyka_hack/onvif/config.toml:105,183,186`.
pub const TRIAL_PORTS: [u16; 3] = [80, 554, 8080];

/// Watch `ports` once a second until all of them have been bound continuously
/// for `hold_secs`, or `deadline_secs` elapses.
///
/// `probe` and `sleep` are injected so this is testable without sockets or
/// wall-clock waits. Production passes `netstat::listening` and
/// `std::thread::sleep`.
///
/// ponytail: socket-liveness smoke test. Bound sockets do not prove frames
/// flow, so a broken vendor-daemon can pass. Add a frame-counter probe if a
/// silent-no-video regression ever ships.
pub fn evaluate_trial(
    ports: &[u16],
    policy: Policy,
    mut probe: impl FnMut(u16) -> bool,
    mut sleep: impl FnMut(std::time::Duration),
) -> Outcome {
    let mut held = 0u32;
    for _ in 0..policy.deadline_secs {
        if ports.iter().all(|p| probe(*p)) {
            held += 1;
            if held >= policy.hold_secs {
                return Outcome::Confirm;
            }
        } else {
            held = 0;
        }
        sleep(std::time::Duration::from_secs(1));
    }
    Outcome::Revert
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 21 tests in `update::`.

**Step 5: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs
git commit -m "feat(anyka-init): trial evaluation over listening ports"
```

---

### Task 7: Boot-time reconcile

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs`
- Modify: `cross-compile/anyka-init/src/main.rs` (after the supervisor loop starts services)

**Step 1: Write the failing test**

```rust
#[test]
fn reconcile_is_a_no_op_without_a_marker() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    let mut sys = MockSys::new();
    sys.expect_reboot().never();
    reconcile(&sys, d.path(), Policy::default(), |_| true, |_| {});
}

#[test]
fn a_healthy_trial_clears_the_marker_and_does_not_reboot() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    Trial::arm(d.path(), Slot::A).unwrap();
    let s = Slots::new(d.path());
    s.set_active(Slot::B).unwrap();

    let mut sys = MockSys::new();
    sys.expect_reboot().never();
    reconcile(&sys, d.path(), Policy { hold_secs: 1, deadline_secs: 3 }, |_| true, |_| {});

    assert_eq!(Trial::find(d.path()), None);
    assert_eq!(s.active(), Slot::B);
}

#[test]
fn a_failed_trial_restores_the_previous_slot_and_reboots() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    Trial::arm(d.path(), Slot::A).unwrap();
    let s = Slots::new(d.path());
    s.set_active(Slot::B).unwrap();

    let mut sys = MockSys::new();
    sys.expect_reboot().times(1).returning(|| Ok(()));
    reconcile(&sys, d.path(), Policy { hold_secs: 1, deadline_secs: 2 }, |_| false, |_| {});

    assert_eq!(s.active(), Slot::A, "must fall back before rebooting");
    assert_eq!(Trial::find(d.path()), None, "marker must not survive the revert");
}
```

That last assertion matters: a marker surviving a revert would put the camera in a revert loop, flipping and rebooting forever.

**Step 2: Run to verify it fails**

Expected: FAIL, `cannot find function 'reconcile'`.

**Step 3: Write the implementation**

```rust
/// Resolve an unconfirmed update, if there is one. Called once per boot, after
/// services have been started.
pub fn reconcile(
    sys: &dyn crate::sys::Sys,
    root: &Path,
    policy: Policy,
    probe: impl FnMut(u16) -> bool,
    sleep: impl FnMut(std::time::Duration),
) {
    let Some(prev) = Trial::find(root) else {
        return;
    };
    let slots = Slots::new(root);
    tracing::info!(
        active = slots.active().name(),
        prev = prev.name(),
        "unconfirmed update: starting trial"
    );

    match evaluate_trial(&TRIAL_PORTS, policy, probe, sleep) {
        Outcome::Confirm => {
            if let Err(e) = Trial::clear(root) {
                tracing::error!(error = %e, "could not clear the trial marker");
            } else {
                tracing::info!(slot = slots.active().name(), "update confirmed");
            }
        }
        Outcome::Revert => {
            tracing::error!(
                ports = ?TRIAL_PORTS,
                "trial failed: reverting to slot {}",
                prev.name()
            );
            // Order matters. Restore the pointer, then clear the marker, then
            // reboot: if power is lost between the first two the next boot
            // repeats a revert that is already correct, whereas clearing first
            // would boot the broken slot with no marker and no way back.
            if let Err(e) = slots.set_active(prev) {
                tracing::error!(error = %e, "could not restore the previous slot");
                return;
            }
            if let Err(e) = Trial::clear(root) {
                tracing::error!(error = %e, "could not clear the trial marker");
            }
            let _ = sys.reboot();
        }
    }
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 24 tests in `update::`.

**Step 5: Wire it into `main.rs`**

`supervisor_loop::run` blocks, so the reconcile has to run on its own thread. Add just before the `supervisor_loop::run(sysimpl, &cfg, rx);` line at the end of `main`, following the `monitor` / `timesync` thread pattern already there:

```rust
{
    let s = Arc::clone(&sysimpl);
    let root = cfg.update.root.clone();
    let policy = anyka_init::update::Policy {
        hold_secs: cfg.update.trial_hold_sec,
        deadline_secs: cfg.update.trial_deadline_sec,
    };
    let _ = std::thread::Builder::new()
        .name("update-trial".into())
        .stack_size(supervisor_loop::thread_stack())
        .spawn(move || {
            anyka_init::update::reconcile(
                s.as_ref(),
                std::path::Path::new(&root),
                policy,
                anyka_init::netstat::listening,
                std::thread::sleep,
            );
        });
}
```

`cfg.update` does not exist yet — Task 9 adds it. Until then this will not compile, so **do Task 9 before running the build**, or stub the three values as literals and replace them in Task 9. Prefer doing Task 9 first if you hit this.

**Step 6: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): reconcile unconfirmed updates at boot"
```

---

### Task 8: The applier — stage, flip, reboot

**Files:**
- Modify: `cross-compile/anyka-init/src/update.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn apply_stages_flips_and_reboots() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    std::fs::create_dir_all(root.join("spool")).unwrap();
    std::fs::write(root.join("spool/bundle.tar"), b"pretend tar").unwrap();
    std::fs::write(root.join("spool/bundle.trigger"), b"").unwrap();

    let mut sys = MockSys::new();
    // one untar, one sha256sum -c
    sys.expect_run_to_completion().times(2).returning(|_prog, args| {
        // Stage the manifest the verify step will look for.
        if args.iter().any(|a| a.contains("tar")) { /* untar is faked below */ }
        Ok(exit_ok())
    });
    sys.expect_reboot().times(1).returning(|| Ok(()));

    // The fake untar cannot write files from inside the mock, so pre-create
    // what a real one would produce in the inactive slot.
    let staged = root.join("slots/b");
    std::fs::create_dir_all(&staged).unwrap();
    std::fs::write(staged.join("manifest.sha256"), "abc  anyka-init.bin\n").unwrap();
    std::fs::write(staged.join("manifest.meta"), "requires_config_schema=1\n").unwrap();

    apply(&sys, root, 1);

    assert_eq!(Slots::new(root).active(), Slot::B, "must flip to the staged slot");
    assert_eq!(Trial::find(root), Some(Slot::A), "must arm the trial with the old slot");
    assert!(!root.join("spool/bundle.trigger").exists(), "spool must be cleared");
}

#[test]
fn a_failed_verify_leaves_both_slots_alone() {
    use crate::sys::MockSys;
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    std::fs::create_dir_all(root.join("spool")).unwrap();
    std::fs::write(root.join("spool/bundle.tar"), b"corrupt").unwrap();
    std::fs::write(root.join("spool/bundle.trigger"), b"").unwrap();

    let staged = root.join("slots/b");
    std::fs::create_dir_all(&staged).unwrap();
    std::fs::write(staged.join("manifest.sha256"), "abc  anyka-init.bin\n").unwrap();
    std::fs::write(staged.join("manifest.meta"), "requires_config_schema=1\n").unwrap();

    let mut sys = MockSys::new();
    sys.expect_run_to_completion().returning(|_, args| {
        if args.iter().any(|a| a.contains("sha256sum")) { Ok(exit_fail()) } else { Ok(exit_ok()) }
    });
    sys.expect_reboot().never();

    apply(&sys, root, 1);

    assert_eq!(Slots::new(root).active(), Slot::A, "no flip on a bad bundle");
    assert_eq!(Trial::find(root), None, "no trial armed");
    assert!(!root.join("spool/bundle.trigger").exists(), "spool must still be cleared");
}
```

**Step 2: Run to verify it fails**

Expected: FAIL, `cannot find function 'apply'`.

**Step 3: Write the implementation**

```rust
/// Is a complete bundle waiting?
///
/// The trigger file is written after the tar, so its presence means the
/// transfer finished. A tar without a trigger is an upload still in flight.
pub fn pending(root: &Path) -> bool {
    root.join("spool/bundle.trigger").is_file() && root.join("spool/bundle.tar").is_file()
}

/// Apply the bundle in `spool/`. Returns having either rebooted or done
/// nothing durable.
pub fn apply(sys: &dyn crate::sys::Sys, root: &Path, device_schema: u32) {
    let slots = Slots::new(root);
    let target = slots.inactive();
    let dir = slots.dir(target);

    let result = stage_and_flip(sys, root, &slots, target, &dir, device_schema);

    // Clear the spool either way. A bundle that failed verification will fail
    // it again on the next tick, and retrying forever would keep the supervisor
    // busy hashing 19 MB every minute.
    let _ = std::fs::remove_file(root.join("spool/bundle.trigger"));
    let _ = std::fs::remove_file(root.join("spool/bundle.tar"));

    match result {
        Ok(meta) => {
            tracing::info!(
                slot = target.name(),
                version = meta.version.as_deref().unwrap_or("unknown"),
                "update staged; rebooting into it"
            );
            let _ = sys.reboot();
        }
        Err(e) => {
            tracing::error!(error = %e, slot = target.name(), "update rejected; both slots untouched");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

fn stage_and_flip(
    sys: &dyn crate::sys::Sys,
    root: &Path,
    slots: &Slots,
    target: Slot,
    dir: &Path,
    device_schema: u32,
) -> Result<Meta, UpdateError> {
    // Wipe first: a slot half-written by a previous interrupted apply would
    // otherwise contribute stale files that the manifest never mentions.
    let _ = std::fs::remove_dir_all(dir);
    std::fs::create_dir_all(dir)?;

    let untar = [
        "sh".to_string(),
        "-c".to_string(),
        format!(
            "busybox tar -xf {} -C {}",
            shell_quote(&root.join("spool/bundle.tar")),
            shell_quote(dir)
        ),
    ];
    match sys.run_to_completion("busybox", &untar) {
        Ok(st) if st.success() => {}
        _ => return Err(UpdateError::Checksum),
    }
    // SAFETY: sync(2) takes no arguments and cannot fail.
    unsafe { libc::sync() };

    let meta = verify_slot(sys, dir, device_schema)?;

    // Arm before flipping. A power cut between the two leaves the old slot
    // active with a stale marker, which the next boot resolves by confirming a
    // slot that is already good — harmless. The reverse order could flip
    // without a marker and lose the way back.
    Trial::arm(root, slots.active())?;
    slots.set_active(target)?;
    Ok(meta)
}
```

**Step 4: Run to verify it passes**

Expected: PASS, 26 tests in `update::`.

**Step 5: Wire the poller into `main.rs`**

Alongside the trial thread from Task 7, add a poller. Reuse `cfg.monitor.interval_sec` for the cadence — a `stat` per minute costs nothing and adds no new tunable:

```rust
{
    let s = Arc::clone(&sysimpl);
    let root = cfg.update.root.clone();
    let schema = cfg.schema;
    let interval = Duration::from_secs(cfg.monitor.interval_sec);
    let _ = std::thread::Builder::new()
        .name("update-poll".into())
        .stack_size(supervisor_loop::thread_stack())
        .spawn(move || {
            let root = std::path::Path::new(&root);
            loop {
                std::thread::sleep(interval);
                if anyka_init::update::pending(root) {
                    anyka_init::update::apply(s.as_ref(), root, schema);
                }
            }
        });
}
```

**Step 6: Commit**

```bash
git add cross-compile/anyka-init/src/update.rs cross-compile/anyka-init/src/main.rs
git commit -m "feat(anyka-init): apply bundles from the spool directory"
```

---

### Task 9: Config — `schema` and `[update]`

**Files:**
- Modify: `cross-compile/anyka-init/src/config.rs`
- Modify: `SD_card_contents/anyka_hack/anyka.toml` (tracked template)
- Modify: `.deploy/anyka.toml`, `.deploy/anyka-121.toml`, `.deploy/anyka-127.toml`, `.deploy/anyka-146.toml`

**Step 1: Write the failing test**

In `config.rs`'s test module, following the existing config tests:

```rust
#[test]
fn schema_defaults_to_zero_for_configs_that_predate_it() {
    let c: Config = toml::from_str(MINIMAL_CONFIG).unwrap();
    assert_eq!(c.schema, 0);
}

#[test]
fn schema_is_read_from_the_top_level() {
    let src = format!("schema = 2\n{MINIMAL_CONFIG}");
    let c: Config = toml::from_str(&src).unwrap();
    assert_eq!(c.schema, 2);
}

#[test]
fn update_section_has_working_defaults() {
    let c: Config = toml::from_str(MINIMAL_CONFIG).unwrap();
    assert_eq!(c.update.root, "/mnt/anyka_hack");
    assert_eq!(c.update.trial_hold_sec, 30);
    assert_eq!(c.update.trial_deadline_sec, 120);
}
```

Reuse whatever minimal-config fixture `config.rs`'s tests already define; if there is none, build one from the smallest valid `anyka.toml`.

**Step 2: Run to verify it fails**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu config::
```

Expected: FAIL, `no field 'schema' on type 'Config'`.

**Step 3: Write the implementation**

Add to the `Config` struct:

```rust
/// Config schema generation. A bundle declares the minimum it needs in
/// `manifest.meta`; the *running* supervisor compares that against this
/// number before flipping, so a build that needs keys this file lacks is
/// rejected with both slots intact instead of crashlooping into a revert.
///
/// Zero means "predates the schema key", which accepts every bundle that
/// does not ask for one.
#[serde(default)]
pub schema: u32,

#[serde(default)]
pub update: Update,
```

And the section:

```rust
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    /// Root holding `active`, `slots/`, `state/` and `spool/`.
    #[serde(default = "default_update_root")]
    pub root: String,
    /// Consecutive seconds all trial ports must stay bound.
    #[serde(default = "default_trial_hold")]
    pub trial_hold_sec: u32,
    /// Give up and revert after this long.
    #[serde(default = "default_trial_deadline")]
    pub trial_deadline_sec: u32,
}

fn default_update_root() -> String { "/mnt/anyka_hack".to_string() }
fn default_trial_hold() -> u32 { 30 }
fn default_trial_deadline() -> u32 { 120 }

impl Default for Update {
    fn default() -> Self {
        Self {
            root: default_update_root(),
            trial_hold_sec: default_trial_hold(),
            trial_deadline_sec: default_trial_deadline(),
        }
    }
}
```

Match the surrounding style — check whether existing sections use `deny_unknown_fields` and mirror whatever they do.

Add to `Config::validate` (`config.rs:439`), so a misconfiguration parks at boot rather than surfacing mid-update:

```rust
if self.update.trial_hold_sec >= self.update.trial_deadline_sec {
    return Err(ConfigError::Invalid(
        "update.trial_hold_sec must be less than update.trial_deadline_sec".into(),
    ));
}
```

Use whatever `ConfigError` variant the neighbouring validations use.

**Step 4: Add a test for the validation**

```rust
#[test]
fn a_hold_longer_than_the_deadline_is_rejected() {
    let src = format!("{MINIMAL_CONFIG}\n[update]\ntrial_hold_sec = 200\ntrial_deadline_sec = 120\n");
    let c: Config = toml::from_str(&src).unwrap();
    assert!(c.validate().is_err());
}
```

**Step 5: Run to verify it passes**

Expected: PASS.

**Step 6: Update the five TOML files**

Add `schema = 1` as the first line of `SD_card_contents/anyka_hack/anyka.toml` and each of the four `.deploy/anyka*.toml` files, above the first `[section]` header. TOML requires top-level keys before any table.

Add a comment in the tracked template only:

```toml
# Config schema generation. Bump this when a key is added that a new build
# requires, and set `requires_config_schema` to the same number in that
# build's manifest.meta. The running supervisor refuses to flip to a bundle
# asking for more than this.
schema = 1
```

**Step 7: Verify the deployed configs still parse**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu
```

If there is an existing test that parses the real `.deploy/*.toml` files, make sure it still passes. If there is not, that is out of scope here.

**Step 8: Commit**

```bash
git add cross-compile/anyka-init/src/config.rs SD_card_contents/anyka_hack/anyka.toml .deploy/anyka*.toml
git commit -m "feat(anyka-init): config schema generation and [update] section"
```

---

### Task 10: Slot-relative service paths

**Files:**
- Modify: `cross-compile/anyka-init/src/sys.rs:108-113`
- Modify: `cross-compile/anyka-init/src/supervisor_loop.rs` (exec path resolution)

**Step 1: Coredump consequence — CHECKED 2026-08-12, CLEAR**

`spec.core_dump` services write cores to CWD, so giving each service a working
directory could have moved 30 MB cores onto vfat on every crash. Checked on
`192.168.2.198`:

```
/proc/sys/kernel/core_pattern = /mnt/core_%e_%p_%t
```

**Absolute**, so the kernel ignores CWD entirely for core placement and
`current_dir` has no effect on where cores land. Proceed with the implementation
as written. Re-check if a camera turns up with a relative pattern.

**Step 2: Write the failing test**

In `sys.rs`'s tests, assert the child's working directory is the executable's parent:

```rust
#[test]
fn a_spawned_child_runs_in_its_executable_directory() {
    let d = tempfile::tempdir().unwrap();
    let script = d.path().join("probe.sh");
    std::fs::write(&script, "#!/bin/sh\npwd\n").unwrap();
    // set the exec bit; check how existing sys.rs tests do this and match
    let spec = SpawnSpec {
        exec: script.clone(),
        log: d.path().join("out.log"),
        ..Default::default()
    };
    RealSys::new().spawn(&spec).unwrap();
    // poll the log briefly, then assert it contains d.path()
}
```

If `SpawnSpec` has no `Default`, construct it fully — match the construction in `tests/supervision.rs`.

**Step 3: Run to verify it fails**

Expected: FAIL — the log contains `/`, not the temp directory.

**Step 4: Write the implementation**

In `sys.rs`, inside `Sys::spawn` for `RealSys`, after the `.stderr(...)` line:

```rust
// Run each service from its own directory. This makes relative paths in a
// service's own config resolve inside its slot: onvif-rust's
// `static_root = "www"` (config_debug.toml:129) resolved against CWD=/
// before, which is how the WebUI came up serving nothing.
if let Some(parent) = spec.exec.parent() {
    cmd.current_dir(parent);
}
```

**Step 5: Make the configured exec paths slot-relative**

`[services.*] exec` in `anyka.toml` is absolute (`/mnt/anyka_hack/onvif/onvif-rust.bin`). Under A/B it must resolve inside the active slot. In `supervisor_loop.rs`, where the `SpawnSpec` is built from the config, rewrite paths that fall under the update root's old layout:

```rust
/// Rewrite a configured exec path into the active slot.
///
/// Config keeps writing `/mnt/anyka_hack/onvif/onvif-rust.bin`; this maps it to
/// `/mnt/anyka_hack/slots/<active>/onvif/onvif-rust.bin`. Paths outside the
/// update root (`/bin/busybox`, `/tmp/wpa_supplicant`) pass through untouched,
/// which is what keeps udhcpc and wpa_supplicant working.
pub fn slot_path(root: &Path, active: Slot, exec: &Path) -> PathBuf {
    match exec.strip_prefix(root) {
        Ok(rest) => root.join("slots").join(active.name()).join(rest),
        Err(_) => exec.to_path_buf(),
    }
}
```

Put it in `update.rs` next to `Slots`, and test it there:

```rust
#[test]
fn paths_under_the_root_move_into_the_active_slot() {
    assert_eq!(
        slot_path(Path::new("/mnt/anyka_hack"), Slot::B,
                  Path::new("/mnt/anyka_hack/onvif/onvif-rust.bin")),
        Path::new("/mnt/anyka_hack/slots/b/onvif/onvif-rust.bin")
    );
}

#[test]
fn paths_outside_the_root_are_untouched() {
    assert_eq!(
        slot_path(Path::new("/mnt/anyka_hack"), Slot::B, Path::new("/bin/busybox")),
        Path::new("/bin/busybox")
    );
}
```

Apply the same rewrite to `LD_LIBRARY_PATH` in `[services.vendor-daemon] env`, which also points at `/mnt/anyka_hack/vendor-daemon/lib`.

**Step 6: Run the full suite**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu
```

Expected: PASS.

**Step 7: Commit**

```bash
git add cross-compile/anyka-init/src/sys.rs cross-compile/anyka-init/src/supervisor_loop.rs cross-compile/anyka-init/src/update.rs
git commit -m "feat(anyka-init): resolve service paths inside the active slot"
```

---

### Task 11: `config.sh` slot selection

**Files:**
- Modify: `SD_card_contents/Factory/config.sh:16-21`
- Modify: `cross-compile/anyka-init/tests/p0_wrapper.rs`

This is the file that must never fail. Change only the `BIN` resolution; leave the deadman at `:45-57` and the respawn loop at `:67-70` exactly as they are.

**Step 1: Write the failing tests**

`p0_wrapper.rs` already drives `config.sh` through `ANYKA_INIT_BIN`. Add a slot root override in the same style and three cases:

```rust
#[test]
fn boots_the_slot_named_by_the_pointer() { /* active="b" -> runs slots/b/anyka-init.bin */ }

#[test]
fn falls_back_to_the_other_slot_when_the_named_one_is_missing() { /* active="b", only slots/a exists */ }

#[test]
fn defaults_to_slot_a_when_the_pointer_is_absent() { /* no active file -> slots/a */ }
```

Follow the existing harness in that file for how it stubs paths and observes which binary ran — do not invent a second mechanism.

**Step 2: Run to verify they fail**

```bash
cd cross-compile && $CARGO test -p anyka-init --target x86_64-unknown-linux-gnu --test p0_wrapper
```

Expected: FAIL.

**Step 3: Write the implementation**

Replace `config.sh:18`:

```sh
# A/B slot selection. /mnt is vfat and has no symlinks, so the pointer is a
# one-byte file. Unreadable or unrecognized reads as slot a.
#
# Second line is the fallback: a slot whose supervisor will not exec is the
# exact failure an update must survive, and it costs one test to handle here
# rather than waiting 240s for the deadman below to restore the vendor path.
SLOT_ROOT=${ANYKA_SLOT_ROOT:-/mnt/anyka_hack}
SLOT=$(cat "$SLOT_ROOT/active" 2>/dev/null) || SLOT=a
[ "$SLOT" = b ] || SLOT=a
BIN=${ANYKA_INIT_BIN:-$SLOT_ROOT/slots/$SLOT/anyka-init.bin}
if [ ! -x "$BIN" ]; then
  OTHER=$([ "$SLOT" = a ] && echo b || echo a)
  BIN=$SLOT_ROOT/slots/$OTHER/anyka-init.bin
fi
```

Keep `ANYKA_INIT_BIN` honoured so the existing tests in `p0_wrapper.rs` keep working unchanged.

**Step 4: Run to verify they pass**

Expected: PASS, including every pre-existing test in that file.

**Step 5: Lint the shell**

```bash
shellcheck SD_card_contents/Factory/config.sh
```

Expected: clean, or no new findings versus before the change.

**Step 6: Commit**

```bash
git add SD_card_contents/Factory/config.sh cross-compile/anyka-init/tests/p0_wrapper.rs
git commit -m "feat(p0): select the boot slot from the active pointer with a fallback"
```

---

### Task 12: Bundle builder

**Files:**
- Create: `scripts/build_bundle.sh`

Sits beside `scripts/build_sd_contents.sh`; source `scripts/common.sh` for `log_info` / `log_error` / `ANYKA_REPO_ROOT` the way the existing scripts do.

**Step 1: Write the script**

```bash
#!/bin/bash
# Build an upgrade bundle: the three components plus a checksum manifest.
#
# Deliberately not the whole anyka_hack tree. lib/ is 31 MB of uClibc runtime
# that changes only on a toolchain bump; the manifest does not cover it, so a
# toolchain change is a separate deliberate push, not an accidental one.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/common.sh
source "${SCRIPT_DIR}/common.sh"

SRC="${ANYKA_REPO_ROOT}/SD_card_contents/anyka_hack"
OUT="${1:-${ANYKA_REPO_ROOT}/bundle.tar}"
SCHEMA="${ANYKA_CONFIG_SCHEMA:-1}"
VERSION="$(git -C "${ANYKA_REPO_ROOT}" describe --tags --always --dirty)"

STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

cp "${SRC}/anyka-init.bin"        "${STAGE}/"
cp -r "${SRC}/vendor-daemon"      "${STAGE}/"
mkdir -p "${STAGE}/onvif"
cp "${SRC}/onvif/onvif-rust.bin"  "${STAGE}/onvif/"
cp -r "${SRC}/onvif/www"          "${STAGE}/onvif/"
cp "${SRC}/onvif/config.toml"     "${STAGE}/onvif/config.template.toml"

cat > "${STAGE}/manifest.meta" <<EOF
version=${VERSION}
requires_config_schema=${SCHEMA}
EOF

# sha256sum format, so `busybox sha256sum -c manifest.sha256` verifies it on the
# device and a human can verify it by hand over telnet.
( cd "${STAGE}" && find . -type f ! -name manifest.sha256 -printf '%P\n' \
    | sort | xargs sha256sum > manifest.sha256 )

tar -cf "${OUT}" -C "${STAGE}" .

log_success "bundle ${VERSION} -> ${OUT} ($(du -h "${OUT}" | cut -f1))"
log_info "deploy: curl -T ${OUT} http://<camera>/api/update   # increment 2"
log_info "or drop it in /mnt/anyka_hack/spool/ over FTP, then touch bundle.trigger"
```

**Step 2: Verify it produces a self-consistent bundle**

```bash
chmod +x scripts/build_bundle.sh
./scripts/build_bundle.sh /tmp/b.tar
mkdir -p /tmp/verify && tar -xf /tmp/b.tar -C /tmp/verify
cd /tmp/verify && sha256sum -c manifest.sha256 && cat manifest.meta
```

Expected: every line `OK`, and a `manifest.meta` with a version and `requires_config_schema=1`.

**Step 3: Lint**

```bash
shellcheck scripts/build_bundle.sh
```

Expected: clean.

**Step 4: Commit**

```bash
git add scripts/build_bundle.sh
git commit -m "feat(scripts): build upgrade bundles with a checksum manifest"
```

---

### Task 13: Quality gates and hardware validation

**Files:** none

**Step 1: Run the full host gate**

Per `AGENTS.md` and the `anyka-embedded-build` skill:

```bash
source setenv.sh
cd cross-compile
$CARGO fmt --all -- --check
PATH="${ANYKA_TOOLCHAIN_BIN}:$PATH" $CARGO clippy -p anyka-init --target x86_64-unknown-linux-gnu --all-targets -- -D warnings
$CARGO test -p anyka-init --target x86_64-unknown-linux-gnu
```

The `PATH` prefix on clippy is not optional — without it clippy dies with E0514.

**Step 2: Cross-compile**

```bash
$CARGO build --release --target armv5te-unknown-linux-uclibceabi -p anyka-init
```

Expected: success, and a binary in `cross-compile/target/armv5te-unknown-linux-uclibceabi/release/`.

**Step 3: Migrate `192.168.2.198` to the slot layout by hand — once**

This is a one-time conversion; there is no automation for it, and it must be done before the new `config.sh` can boot anything. Over telnet and FTP:

```sh
mkdir -p /mnt/anyka_hack/slots/a /mnt/anyka_hack/spool /mnt/anyka_hack/state
cp /mnt/anyka_hack/anyka-init.bin /mnt/anyka_hack/slots/a/
cp -r /mnt/anyka_hack/vendor-daemon /mnt/anyka_hack/slots/a/
mkdir -p /mnt/anyka_hack/slots/a/onvif
cp /mnt/anyka_hack/onvif/onvif-rust.bin /mnt/anyka_hack/slots/a/onvif/
cp -r /mnt/anyka_hack/onvif/www /mnt/anyka_hack/slots/a/onvif/
printf a > /mnt/anyka_hack/active
sync
```

Leave the originals in place: they are the fallback if the new `config.sh` misbehaves, and 19 MB is nothing on a 57 GB card.

**Step 4: Deploy and reboot**

Push the new `anyka-init.bin` into `slots/a/` and the new `config.sh` to `/mnt/Factory/`, then reboot. Confirm over telnet that `/mnt/logs/` shows `anyka-init starting` and that ports 80, 554 and 8080 are listening.

**Step 5: Validate a good update end to end**

Build a bundle, FTP it into `/mnt/anyka_hack/spool/bundle.tar`, then `touch /mnt/anyka_hack/spool/bundle.trigger`. Within one monitor interval the camera should stage into `slots/b`, flip, and reboot. After it returns, confirm:

- `cat /mnt/anyka_hack/active` reads `b`
- `ls /mnt/anyka_hack/state/` shows no `trial-*` file
- the log records `update confirmed`

**Step 6: Validate a bad update end to end**

This is the test that matters — everything else is theatre if this does not work. Build a bundle whose `onvif-rust.bin` is deliberately broken (truncate it), push it, and confirm the camera reverts to `slots/b`... that is, to whichever slot was active, reboots, and comes back serving on all three ports. Confirm the log shows `trial failed`.

**Do this on `192.168.2.198` only.** It is the camera with direct telnet access. Do not touch `.121`, `.127` or `.146` until a bad update has demonstrably self-recovered on `.198`.

**Step 7: Commit nothing**

Record the results in the session notes and in the PR description.

---

## Increment 2 — HTTP transport

Only start this after Increment 1 has survived a deliberate bad update on hardware.

---

### Task 14: `PUT /api/update`

**Files:**
- Modify: `cross-compile/onvif-rust/src/diagnostics/http.rs` (route + auth, alongside `:89-90`)
- Create: the handler module beside the existing diagnostics handlers

**Step 1: Write the failing tests**

Follow the existing diagnostics HTTP tests. Cover:

- `PUT /api/update` without admin credentials returns 401.
- A successful body write lands at `spool/bundle.tar` and creates `spool/bundle.trigger`.
- A body that ends early (simulated stream error) leaves **no** trigger file — this is the property the whole scheme rests on, since the trigger is what tells the applier the transfer is complete.
- A second upload while one is in flight is rejected rather than interleaved.

**Step 2: Implement**

Register the route at `AuthLevel::Administrator`, matching `"/logs"` at `diagnostics/http.rs:89`. Stream the raw request body to `spool/bundle.tar.part`, `fsync`, rename to `bundle.tar`, then create `bundle.trigger`. Raw body, not multipart: with 36 MB of RAM nothing may be buffered, and a raw `PUT` needs no parser.

Return `202 Accepted` — the update has not happened yet, only been queued.

**Step 3: Run tests, then commit**

```bash
git commit -m "feat(onvif): accept upgrade bundles at PUT /api/update"
```

---

### Task 15: `FirmwareVersion`

**Files:**
- Create: `cross-compile/onvif-rust/build.rs`
- Modify: the `GetDeviceInformation` handler and the `/api/diagnostics` payload

**Step 1: Emit the version at build time**

```rust
// build.rs
fn main() {
    let v = std::process::Command::new("git")
        .args(["describe", "--tags", "--always", "--dirty"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=ANYKA_BUILD_VERSION={v}");
    println!("cargo:rerun-if-changed=.git/HEAD");
}
```

**Step 2: Use it**

`env!("ANYKA_BUILD_VERSION")` in `GetDeviceInformation`'s `FirmwareVersion` and in the `/api/diagnostics` payload. Update the tests that assert on the current firmware string.

**Step 3: Commit**

```bash
git commit -m "feat(onvif): report the build version as FirmwareVersion"
```

---

### Task 16: WebUI upload

**Files:**
- `cross-compile/www/` — a control on the existing diagnostics or settings page

**Step 1: Implement**

A file input plus a button that does `fetch(url, { method: 'PUT', body: file })`. Show upload progress, then a message saying the camera will reboot and return in about two minutes. Use the existing shadcn/ui components and the `camera-webui-components` skill's conventions; do not introduce an upload library.

**Step 2: Test**

Per the `anyka-webui-testing` skill: `data-testid` on the input and button, mock the service function, assert the PUT fires with the file as the body and that failure surfaces an error state.

**Step 3: Commit**

```bash
git commit -m "feat(www): upload an upgrade bundle from the diagnostics page"
```

---

## Notes for the executor

- **Toolchain.** Always `source setenv.sh` first and use `$CARGO`. Host-side work needs `--target x86_64-unknown-linux-gnu`; clippy additionally needs `PATH="${ANYKA_TOOLCHAIN_BIN}:$PATH"` or it fails with E0514.
- **Never `killall busybox` on the camera.** `udhcpc` and your own telnet shell are busybox.
- **Both hardware gates are resolved** (2026-08-12, on `192.168.2.198`): busybox
  ships `sha256sum -c` and `tar -C`, verified end to end including the corrupt
  and missing-file cases; `core_pattern` is absolute, so per-service `current_dir`
  cannot relocate coredumps. Neither needs re-checking.
- **`.198`'s `/mnt/anyka_hack` is 721 MB and does not match `SD_card_contents/`** —
  it still carries `gergehack.sh`, `web_interface`, `xiu`, `rtsp`, `ptz` and the
  vendor demos. The Task 13 migration copies only the three components into
  `slots/a`, so this is harmless, but it is why that step leaves the originals
  in place rather than moving them.
- **Task 13 Step 6 is the acceptance test.** An upgrade system that has never been observed to roll back has not been tested.
