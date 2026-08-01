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
    pub wifi_reboots: u8,
}

impl StormState {
    fn field(src: &str, name: &str) -> Option<u8> {
        let key = format!("\"{name}\":");
        let rest = src.split_once(&key)?.1.trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        match rest[..end].parse::<u8>() {
            Ok(n) if n <= MAX_SANE_REBOOTS => Some(n),
            _ => None,
        }
    }

    /// Deliberately hand-rolled rather than pulling in serde_json for two
    /// integers. Torn or unrecognized input reads as zero.
    pub fn parse(src: &str) -> Self {
        let src = src.trim();
        if !src.starts_with('{') || !src.ends_with('}') {
            return Self::default();
        }
        Self {
            fast_reboots: Self::field(src, "fast_reboots").unwrap_or(0),
            wifi_reboots: Self::field(src, "wifi_reboots").unwrap_or(0),
        }
    }

    pub fn render(&self) -> String {
        format!(
            r#"{{"fast_reboots":{},"wifi_reboots":{}}}"#,
            self.fast_reboots, self.wifi_reboots
        )
    }

    /// Write via temp file + rename so a power cut leaves either the old
    /// contents or the new ones, never a half-written file.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(dir) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = format!("{path}.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(self.render().as_bytes())?;
            f.sync_all()?;
        }
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
    fn test_storm_state_parses_whitespace_after_colon() {
        assert_eq!(StormState::parse(r#"{"fast_reboots": 2}"#).fast_reboots, 2);
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
        let s = StormState {
            fast_reboots: 3,
            wifi_reboots: 0,
        };
        assert_eq!(StormState::parse(&s.render()).fast_reboots, 3);
    }

    #[test]
    fn test_storm_state_round_trips_both_counters() {
        let s = StormState {
            fast_reboots: 2,
            wifi_reboots: 1,
        };
        assert_eq!(StormState::parse(&s.render()), s);
    }

    #[test]
    fn test_storm_state_reads_legacy_single_field_file() {
        // Files written before this change must not read as a torn file and reset
        // the crash-loop guard.
        let legacy = r#"{"fast_reboots":2}"#;
        let s = StormState::parse(legacy);
        assert_eq!(s.fast_reboots, 2);
        assert_eq!(s.wifi_reboots, 0);
    }

    #[test]
    fn test_storm_state_torn_file_reads_as_zero() {
        for bad in [
            r#"{"fast_reboots":"#,
            "",
            "garbage",
            r#"{"fast_reboots":250}"#,
        ] {
            assert_eq!(StormState::parse(bad), StormState::default());
        }
    }

    #[test]
    fn test_should_enter_safe_mode_at_threshold() {
        assert!(!should_enter_safe_mode(0, 3));
        assert!(!should_enter_safe_mode(2, 3));
        assert!(should_enter_safe_mode(3, 3));
        assert!(should_enter_safe_mode(4, 3));
    }
}
