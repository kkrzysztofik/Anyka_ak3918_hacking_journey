//! A/B slot activation and the update applier.
//!
//! Two payload trees live under `slots/a` and `slots/b`; a one-byte `active`
//! file selects which one boots. `/mnt` is vfat/exFAT, which has no symlinks,
//! so the pointer has to be a real file rather than a `current ->` link.
//!
//! Anything unreadable or unrecognized reads as slot A, matching the storm
//! guard's rule (`storm.rs:36-38`): a torn read on a card that just lost power
//! must not strand the camera.

use std::path::PathBuf;

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
