//! Reboot-storm guard.
//!
//! The restart policy reboots the camera when a service exceeds its crash-loop
//! cap. Unguarded, a permanently broken service turns that into an unattended
//! power-cycle loop with no window to log in. This bounds it: after
//! `max_reboots` consecutive fast reboots the supervisor enters safe mode —
//! telnet, logging and the monitor thread only, no camera services — and waits
//! for a human.
//!
//! State lives on a vfat SD card and will occasionally be torn by a power cut.
//! Anything unparseable is read as zero; the cost of guessing wrong is three
//! extra reboots, and the cost of failing closed would be a camera that never
//! starts.

const MAX_SANE_REBOOTS: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StormState {
    pub fast_reboots: u8,
}

impl StormState {
    /// Deliberately hand-rolled rather than pulling in serde_json for one
    /// integer. Any input that is not exactly what `render` produces reads
    /// as zero.
    pub fn parse(src: &str) -> Self {
        let Some(rest) = src.trim().strip_prefix(r#"{"fast_reboots":"#) else {
            return Self::default();
        };
        let Some(num) = rest.strip_suffix('}') else {
            return Self::default();
        };
        match num.trim().parse::<u8>() {
            Ok(n) if n <= MAX_SANE_REBOOTS => Self { fast_reboots: n },
            _ => Self::default(),
        }
    }

    pub fn render(&self) -> String {
        format!(r#"{{"fast_reboots":{}}}"#, self.fast_reboots)
    }

    /// Write via temp file + rename so a power cut leaves either the old
    /// contents or the new ones, never a half-written file.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = format!("{path}.tmp");
        std::fs::write(&tmp, self.render())?;
        std::fs::rename(&tmp, path)?;
        // SAFETY: sync(2) takes no arguments and cannot fail.
        unsafe { libc::sync() };
        Ok(())
    }

    pub fn load(path: &str) -> Self {
        std::fs::read_to_string(path)
            .map(|s| Self::parse(&s))
            .unwrap_or_default()
    }
}

pub fn should_enter_safe_mode(fast_reboots: u8, max_reboots: u8) -> bool {
    fast_reboots >= max_reboots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storm_state_parses_valid_json() {
        assert_eq!(StormState::parse(r#"{"fast_reboots":2}"#).fast_reboots, 2);
    }

    #[test]
    fn test_storm_state_corrupt_input_is_treated_as_zero() {
        // vfat plus a power cut mid-write is expected, not exceptional.
        // Worst case of guessing zero is three extra reboots.
        assert_eq!(StormState::parse("").fast_reboots, 0);
        assert_eq!(StormState::parse("\0\0\0\0").fast_reboots, 0);
        assert_eq!(
            StormState::parse(r#"{"fast_reboots":"two"}"#).fast_reboots,
            0
        );
        assert_eq!(StormState::parse(r#"{"fast_reboots":999}"#).fast_reboots, 0);
    }

    #[test]
    fn test_storm_state_render_roundtrips() {
        let s = StormState { fast_reboots: 3 };
        assert_eq!(StormState::parse(&s.render()).fast_reboots, 3);
    }

    #[test]
    fn test_should_enter_safe_mode_at_threshold() {
        assert!(!should_enter_safe_mode(0, 3));
        assert!(!should_enter_safe_mode(2, 3));
        assert!(should_enter_safe_mode(3, 3));
        assert!(should_enter_safe_mode(4, 3));
    }
}
