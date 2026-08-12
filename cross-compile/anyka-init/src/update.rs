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

/// An unconfirmed update, recorded as the *existence* of `state/trial-<slot>`.///
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
        assert!(
            !Meta {
                version: None,
                requires_config_schema: 3
            }
            .compatible_with(2)
        );
    }

    #[test]
    fn schema_at_or_below_the_device_is_accepted() {
        assert!(
            Meta {
                version: None,
                requires_config_schema: 2
            }
            .compatible_with(2)
        );
        assert!(
            Meta {
                version: None,
                requires_config_schema: 1
            }
            .compatible_with(2)
        );
    }

    fn exit_ok() -> crate::sys::ExitStatus {
        crate::sys::ExitStatus::Code(0)
    }

    fn exit_fail() -> crate::sys::ExitStatus {
        crate::sys::ExitStatus::Code(1)
    }

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
            .withf(|prog, args| prog == "busybox" && args[0] == "sh" && args[1] == "-c")
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
        sys.expect_run_to_completion()
            .returning(|_, _| Ok(exit_fail()));

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

    #[test]
    fn trial_passes_when_every_port_is_bound_for_the_hold() {
        let mut calls = 0;
        let outcome = evaluate_trial(
            &[80, 554, 8080],
            Policy {
                hold_secs: 3,
                deadline_secs: 10,
            },
            |_port| {
                calls += 1;
                true
            },
            |_| {},
        );
        assert_eq!(outcome, Outcome::Confirm);
        assert!(calls >= 3);
    }

    #[test]
    fn trial_fails_when_one_port_never_binds() {
        let outcome = evaluate_trial(
            &[80, 554, 8080],
            Policy {
                hold_secs: 3,
                deadline_secs: 6,
            },
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
            Policy {
                hold_secs: 2,
                deadline_secs: 20,
            },
            |_| {
                tick += 1;
                tick > 5
            },
            |_| {},
        );
        assert_eq!(outcome, Outcome::Confirm);
    }

    #[test]
    fn a_flapping_port_resets_the_hold_and_eventually_reverts() {
        let mut tick = 0;
        let outcome = evaluate_trial(
            &[80],
            Policy {
                hold_secs: 3,
                deadline_secs: 8,
            },
            |_| {
                tick += 1;
                tick % 2 == 0
            },
            |_| {},
        );
        assert_eq!(outcome, Outcome::Revert);
    }

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
        reconcile(
            &sys,
            d.path(),
            Policy {
                hold_secs: 1,
                deadline_secs: 3,
            },
            |_| true,
            |_| {},
        );

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
        reconcile(
            &sys,
            d.path(),
            Policy {
                hold_secs: 1,
                deadline_secs: 2,
            },
            |_| false,
            |_| {},
        );

        assert_eq!(s.active(), Slot::A, "must fall back before rebooting");
        assert_eq!(
            Trial::find(d.path()),
            None,
            "marker must not survive the revert"
        );
    }

    #[test]
    fn apply_stages_flips_and_reboots() {
        use crate::sys::MockSys;
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("spool")).unwrap();
        std::fs::write(root.join("spool/bundle.tar"), b"pretend tar").unwrap();
        std::fs::write(root.join("spool/bundle.trigger"), b"").unwrap();

        let mut sys = MockSys::new();
        // one untar, one sha256sum -c. The mocked untar cannot run for real, so
        // it materializes the slot content the verify step will look for by
        // extracting the target dir from the shell command line.
        sys.expect_run_to_completion()
            .times(2)
            .returning(|_, args| {
                if let Some(cmd) = args.iter().find(|a| a.contains("tar -xf")) {
                    let dir = cmd
                        .split("-C ")
                        .nth(1)
                        .map(|s| s.trim().trim_matches('\'').to_string())
                        .expect("untar -C dir");
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(
                        std::path::Path::new(&dir).join("manifest.sha256"),
                        "abc  anyka-init.bin\n",
                    )
                    .unwrap();
                    std::fs::write(
                        std::path::Path::new(&dir).join("manifest.meta"),
                        "requires_config_schema=1\n",
                    )
                    .unwrap();
                }
                Ok(exit_ok())
            });
        sys.expect_reboot().times(1).returning(|| Ok(()));

        apply(&sys, root, 1);

        assert_eq!(
            Slots::new(root).active(),
            Slot::B,
            "must flip to the staged slot"
        );
        assert_eq!(
            Trial::find(root),
            Some(Slot::A),
            "must arm the trial with the old slot"
        );
        assert!(
            !root.join("spool/bundle.trigger").exists(),
            "spool must be cleared"
        );
    }

    #[test]
    fn a_failed_verify_leaves_both_slots_alone() {
        use crate::sys::MockSys;
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        std::fs::create_dir_all(root.join("spool")).unwrap();
        std::fs::write(root.join("spool/bundle.tar"), b"corrupt").unwrap();
        std::fs::write(root.join("spool/bundle.trigger"), b"").unwrap();

        let mut sys = MockSys::new();
        sys.expect_run_to_completion().returning(|_, args| {
            if args.iter().any(|a| a.contains("sha256sum")) {
                return Ok(exit_fail());
            }
            // Materialize the staged slot the sha256sum step will reject.
            let cmd = args.iter().find(|a| a.contains("tar -xf")).unwrap();
            let dir = cmd
                .split("-C ")
                .nth(1)
                .map(|s| s.trim().trim_matches('\'').to_string())
                .expect("untar -C dir");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join("manifest.sha256"),
                "abc  anyka-init.bin\n",
            )
            .unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join("manifest.meta"),
                "requires_config_schema=1\n",
            )
            .unwrap();
            Ok(exit_ok())
        });
        sys.expect_reboot().never();

        apply(&sys, root, 1);

        assert_eq!(
            Slots::new(root).active(),
            Slot::A,
            "no flip on a bad bundle"
        );
        assert_eq!(Trial::find(root), None, "no trial armed");
        assert!(
            !root.join("spool/bundle.trigger").exists(),
            "spool must still be cleared"
        );
    }
}
