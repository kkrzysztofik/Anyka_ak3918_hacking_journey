//! Event-audio playback settings (`[sound]` in config.toml).

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

/// DAC range is [0, 6]; 0 is mute. Matches `SOUND_VOLUME_MAX` in the daemon.
const SOUND_VOLUME_MAX: u8 = 6;

fn default_clip_dir() -> String {
    "sounds".to_string()
}

fn default_volume() -> u8 {
    3
}

fn default_debounce_secs() -> u64 {
    30
}

fn deserialize_volume<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let v = u32::deserialize(deserializer)?;
    Ok(v.min(u32::from(SOUND_VOLUME_MAX)) as u8)
}

/// Policy for playing short PCM clips on selected events.
///
/// Defaults are safe and quiet: sound is opt-in, volume stays in the DAC range,
/// and no events are mapped until configured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundConfig {
    pub enabled: bool,
    #[serde(default = "default_clip_dir")]
    pub clip_dir: String,
    #[serde(default = "default_volume", deserialize_with = "deserialize_volume")]
    pub volume: u8,
    #[serde(default = "default_debounce_secs")]
    pub debounce_secs: u64,
    #[serde(default)]
    pub events: BTreeMap<String, String>,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            clip_dir: default_clip_dir(),
            volume: default_volume(),
            debounce_secs: default_debounce_secs(),
            events: BTreeMap::new(),
        }
    }
}

impl SoundConfig {
    /// Clip filename for `event`, if mapped.
    pub fn clip_for(&self, event: &str) -> Option<&str> {
        self.events.get(event).map(String::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sound_config_defaults_are_safe_and_quiet() {
        let c = SoundConfig::default();
        assert!(!c.enabled, "sound must be opt-in");
        assert!(c.volume <= 6, "volume must stay in the DAC range");
        assert!(c.events.is_empty());
    }

    #[test]
    fn test_sound_config_volume_above_dac_range_is_clamped() {
        let c: SoundConfig = toml::from_str("enabled = true\nvolume = 99").unwrap();
        assert_eq!(c.volume, 6);
    }

    #[test]
    fn test_sound_config_unmapped_event_resolves_to_no_clip() {
        let c: SoundConfig =
            toml::from_str("enabled = true\n[events]\nboot_ready = \"boot.raw\"").unwrap();
        assert_eq!(c.clip_for("boot_ready"), Some("boot.raw"));
        assert_eq!(c.clip_for("network_lost"), None);
    }
}
