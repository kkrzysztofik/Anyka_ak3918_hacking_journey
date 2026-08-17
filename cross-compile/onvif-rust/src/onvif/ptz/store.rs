//! PTZ preset store for persistent preset management.
//!
//! Persistent storage for PTZ presets (`ptz_presets.toml`).
//!
//! Handles preset CRUD and token numbering. Kept separate from runtime
//! position/movement state so presets can be loaded/saved independently of
//! the live PTZ actor.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::config::{PendingWrite, PersistenceHandle, PersistenceService};
use crate::onvif::error::OnvifResult;
use crate::onvif::types::common::{PTZPreset, PTZVector, Vector1D, Vector2D};

use super::faults::{no_preset, too_many_presets};
use super::types::MAX_PRESETS;

/// Internal preset data storage.
#[derive(Debug, Clone)]
struct PresetData {
    /// Preset name.
    name: String,
    /// Preset position.
    position: PTZVector,
}

/// Disk file shape for the persisted presets.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PresetStoreFile {
    /// Presets (token -> preset data).
    #[serde(default)]
    presets: HashMap<String, PresetStoreEntry>,
    /// Next token number, so a fresh-process reload can keep issuing
    /// `Preset{N}` without colliding with already-stored tokens.
    #[serde(default = "default_next_preset_num")]
    next_preset_num: u32,
}

/// Per-preset entry on disk. Positions are kept in their TOML shape so the
/// file stays human-readable for diagnosis.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PresetStoreEntry {
    name: String,
    position: PersistedPosition,
}

/// TOML-friendly PTZ position. `PTZVector` uses the ONVIF namespace prefix
/// (`tt:PanTilt`) which is not a valid TOML key, so we serialise a flat shape
/// here and convert at the boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedPosition {
    pan_tilt_x: Option<f32>,
    pan_tilt_y: Option<f32>,
    zoom: Option<f32>,
}

impl PersistedPosition {
    fn from_vector(v: &PTZVector) -> Self {
        Self {
            pan_tilt_x: v.pan_tilt.as_ref().map(|pt| pt.x),
            pan_tilt_y: v.pan_tilt.as_ref().map(|pt| pt.y),
            zoom: v.zoom.as_ref().map(|z| z.x),
        }
    }

    fn into_vector(self) -> PTZVector {
        let pan_tilt = match (self.pan_tilt_x, self.pan_tilt_y) {
            (None, None) => None,
            (x, y) => Some(Vector2D {
                x: x.unwrap_or(0.0),
                y: y.unwrap_or(0.0),
                space: None,
            }),
        };
        PTZVector {
            pan_tilt,
            zoom: self.zoom.map(|x| Vector1D { x, space: None }),
        }
    }
}

fn default_next_preset_num() -> u32 {
    1
}

/// Persistent preset store for PTZ presets.
///
/// This store manages PTZ presets independently from runtime PTZ state.
/// It provides CRUD operations for presets and handles auto-generated
/// preset tokens.
///
/// # Example
///
/// ```rust,no_run
/// use onvif_rust::onvif::ptz::store::PresetStore;
/// use onvif_rust::onvif::types::common::{PTZVector, Vector1D, Vector2D};
///
/// let store = PresetStore::new();
///
/// // Get all presets
/// let presets = store.get_all();
///
/// // Set a preset at current position
/// let pos = PTZVector {
///     pan_tilt: Some(Vector2D { x: 0.5, y: 0.5, space: None }),
///     zoom: Some(Vector1D { x: 0.5, space: None }),
/// };
/// let token = store.set_preset("Home".to_string(), Some(pos), None).unwrap();
/// ```
pub struct PresetStore {
    /// Presets (token -> preset data).
    presets: RwLock<HashMap<String, PresetData>>,
    /// Next preset number for auto-generated tokens.
    next_preset_num: RwLock<u32>,
    /// Path the preset store serialises to. `None` disables persistence.
    persistence_path: Option<PathBuf>,
    /// Optional debounced, off-executor persistence handle.
    persistence: OnceLock<PersistenceHandle>,
}

impl PresetStore {
    /// Create a new preset store.
    pub fn new() -> Self {
        Self {
            presets: RwLock::new(HashMap::new()),
            next_preset_num: RwLock::new(1),
            persistence_path: None,
            persistence: OnceLock::new(),
        }
    }

    /// Create a new preset store with a persistence path.
    pub fn with_persistence(path: impl AsRef<Path>) -> Self {
        Self {
            presets: RwLock::new(HashMap::new()),
            next_preset_num: RwLock::new(1),
            persistence_path: Some(path.as_ref().to_path_buf()),
            persistence: OnceLock::new(),
        }
    }

    /// Create a new preset store wrapped in Arc.
    pub fn new_arc() -> Arc<Self> {
        Arc::new(Self::new())
    }

    /// Attach a debounced persistence handle.
    pub fn set_persistence(&self, handle: PersistenceHandle) {
        if self.persistence.set(handle).is_err() {
            tracing::warn!("Preset store persistence handle already set; ignoring");
        }
    }

    /// Request a non-blocking, debounced save of the preset store.
    pub fn request_save(&self) {
        match self.persistence.get() {
            Some(handle) => handle.request_save(),
            None => {
                tracing::debug!("No preset persistence handle configured; skipping save request")
            }
        }
    }

    /// Build a persistence service that snapshots this store on debounce flush.
    pub fn persistence_service(
        self: &Arc<Self>,
        save_delay_ms: u64,
    ) -> (PersistenceService, PersistenceHandle) {
        let store = Arc::clone(self);
        let snapshot = Box::new(move || {
            let Some(path) = store.persistence_path.clone() else {
                tracing::debug!("No persistence path configured, skipping preset snapshot");
                return None;
            };
            match store.snapshot_toml() {
                Ok(bytes) => Some(PendingWrite {
                    path,
                    bytes,
                    mode: None,
                }),
                Err(e) => {
                    tracing::error!("Failed to serialize presets: {}", e);
                    None
                }
            }
        });

        PersistenceService::new("ptz_presets", save_delay_ms, snapshot)
    }

    /// Load presets from the persistence file. Missing/unreadable files are
    /// not fatal — the store boots empty and the next save overwrites the path.
    pub fn load_from_file(&self) -> OnvifResult<()> {
        let Some(path) = &self.persistence_path else {
            tracing::debug!("No persistence path configured, skipping load");
            return Ok(());
        };
        if !path.exists() {
            tracing::info!("Preset store file does not exist: {:?}", path);
            return Ok(());
        }

        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    "Failed to read preset file {}: {}; booting with empty store",
                    path.display(),
                    e
                );
                return Ok(());
            }
        };
        let file: PresetStoreFile = match toml::from_str(&raw) {
            Ok(f) => f,
            Err(e) => {
                tracing::warn!(
                    "Failed to parse preset file {}: {}; booting with empty store",
                    path.display(),
                    e
                );
                return Ok(());
            }
        };

        let mut max_loaded_num = 0u32;
        {
            let mut presets = self.presets.write();
            presets.clear();
            for (token, entry) in file.presets {
                if let Some(n) = token
                    .strip_prefix("Preset")
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    max_loaded_num = max_loaded_num.max(n);
                }
                presets.insert(
                    token,
                    PresetData {
                        name: entry.name,
                        position: entry.position.into_vector(),
                    },
                );
            }
        }
        {
            let mut next = self.next_preset_num.write();
            // Advance past any PresetN already on disk so a subsequent
            // set_preset(existing_token=None) cannot collide with a restored token.
            *next = file.next_preset_num.max(max_loaded_num.saturating_add(1));
        }

        tracing::info!(
            "Loaded {} presets from {:?}",
            self.presets.read().len(),
            path
        );
        Ok(())
    }

    /// Serialize the in-memory store to TOML bytes.
    pub fn snapshot_toml(&self) -> OnvifResult<Vec<u8>> {
        let file = {
            let presets = self.presets.read();
            let next = *self.next_preset_num.read();
            let mut entries = HashMap::with_capacity(presets.len());
            for (token, data) in presets.iter() {
                entries.insert(
                    token.clone(),
                    PresetStoreEntry {
                        name: data.name.clone(),
                        position: PersistedPosition::from_vector(&data.position),
                    },
                );
            }
            PresetStoreFile {
                presets: entries,
                next_preset_num: next,
            }
        };
        let content = toml::to_string_pretty(&file).map_err(|e| {
            crate::onvif::error::OnvifError::Internal(format!("Failed to serialize presets: {}", e))
        })?;
        Ok(content.into_bytes())
    }

    /// Get all presets.
    ///
    /// # Returns
    ///
    /// A vector of all stored presets.
    pub fn get_all(&self) -> Vec<PTZPreset> {
        let presets = self.presets.read();
        presets
            .iter()
            .map(|(token, data)| PTZPreset {
                token: Some(token.clone()),
                name: Some(data.name.clone()),
                ptz_position: Some(data.position.clone()),
            })
            .collect()
    }

    /// Get a specific preset by token.
    ///
    /// # Arguments
    ///
    /// * `token` - The preset token to retrieve
    ///
    /// # Returns
    ///
    /// The preset if found, or an error.
    pub fn get(&self, token: &str) -> OnvifResult<PTZPreset> {
        let presets = self.presets.read();
        presets
            .get(token)
            .map(|data| PTZPreset {
                token: Some(token.to_string()),
                name: Some(data.name.clone()),
                ptz_position: Some(data.position.clone()),
            })
            .ok_or_else(|| no_preset(token))
    }

    /// Set a preset at the given position.
    ///
    /// If `existing_token` is provided, updates an existing preset.
    /// If `existing_token` is None, creates a new preset with auto-generated token.
    ///
    /// # Arguments
    ///
    /// * `name` - The preset name
    /// * `position` - The PTZ position for the preset
    /// * `existing_token` - Optional token to update an existing preset
    ///
    /// # Returns
    ///
    /// The preset token (new or existing).
    pub fn set_preset(
        &self,
        name: String,
        position: PTZVector,
        existing_token: Option<String>,
    ) -> OnvifResult<String> {
        let mut presets = self.presets.write();

        // Check if we're at max presets (for new presets only)
        if existing_token.is_none() && presets.len() >= MAX_PRESETS as usize {
            return Err(too_many_presets(MAX_PRESETS));
        }

        let preset_token = match existing_token {
            Some(t) => {
                // Verify preset exists for updates
                if !presets.contains_key(&t) {
                    return Err(no_preset(&t));
                }
                t
            }
            None => {
                // Generate new token
                let mut num = self.next_preset_num.write();
                let new_token = format!("Preset{}", *num);
                *num += 1;
                new_token
            }
        };

        presets.insert(preset_token.clone(), PresetData { name, position });
        drop(presets);
        self.request_save();

        Ok(preset_token)
    }

    /// Remove a preset by token.
    ///
    /// # Arguments
    ///
    /// * `token` - The preset token to remove
    ///
    /// # Returns
    ///
    /// Ok if removed, error if not found.
    pub fn remove(&self, token: &str) -> OnvifResult<()> {
        let mut presets = self.presets.write();
        presets.remove(token).ok_or_else(|| no_preset(token))?;
        drop(presets);
        self.request_save();
        Ok(())
    }

    /// Check if a preset exists.
    ///
    /// # Arguments
    ///
    /// * `token` - The preset token to check
    ///
    /// # Returns
    ///
    /// True if the preset exists.
    pub fn contains(&self, token: &str) -> bool {
        self.presets.read().contains_key(token)
    }

    /// Get the current count of presets.
    ///
    /// # Returns
    ///
    /// The number of stored presets.
    pub fn len(&self) -> usize {
        self.presets.read().len()
    }

    /// Check if the store is empty.
    ///
    /// # Returns
    ///
    /// True if there are no presets.
    pub fn is_empty(&self) -> bool {
        self.presets.read().is_empty()
    }
}

impl Default for PresetStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::onvif::types::common::{Vector1D, Vector2D};
    use tempfile::tempdir;

    fn create_test_position() -> PTZVector {
        PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: 0.5,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        }
    }

    #[test]
    fn test_preset_store_roundtrips_through_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ptz_presets.toml");
        let store = PresetStore::with_persistence(&path);
        let token = store
            .set_preset("Front Door".into(), create_test_position(), None)
            .unwrap();
        std::fs::write(&path, store.snapshot_toml().unwrap()).unwrap();

        let reloaded = PresetStore::with_persistence(&path);
        reloaded.load_from_file().unwrap();
        assert_eq!(
            reloaded.get(&token).unwrap().name,
            Some("Front Door".into())
        );
    }

    #[test]
    fn test_preset_store_roundtrips_a_position_without_pan_tilt() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ptz_presets.toml");
        let store = PresetStore::with_persistence(&path);
        // A zoom-only preset must not come back aimed at (0, 0) — that would
        // silently re-point the camera on the next GotoPreset.
        let token = store
            .set_preset(
                "Zoom Only".into(),
                PTZVector {
                    pan_tilt: None,
                    zoom: Some(Vector1D {
                        x: 0.25,
                        space: None,
                    }),
                },
                None,
            )
            .unwrap();
        std::fs::write(&path, store.snapshot_toml().unwrap()).unwrap();

        let reloaded = PresetStore::with_persistence(&path);
        reloaded.load_from_file().unwrap();
        let position = reloaded.get(&token).unwrap().ptz_position.unwrap();
        assert!(position.pan_tilt.is_none());
        assert_eq!(position.zoom.unwrap().x, 0.25);
    }

    #[test]
    fn test_preset_store_survives_corrupt_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ptz_presets.toml");
        std::fs::write(&path, b"this is not toml {{{").unwrap();

        // Garbage in the file must leave an empty, usable store — never a boot failure.
        let store = PresetStore::with_persistence(&path);
        store.load_from_file().unwrap();
        assert!(store.is_empty());
    }

    #[test]
    fn test_preset_store_preserves_next_token_number_across_reload() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ptz_presets.toml");
        let store = PresetStore::with_persistence(&path);
        let _ = store
            .set_preset("First".into(), create_test_position(), None)
            .unwrap();
        let _ = store
            .set_preset("Second".into(), create_test_position(), None)
            .unwrap();
        std::fs::write(&path, store.snapshot_toml().unwrap()).unwrap();

        let reloaded = PresetStore::with_persistence(&path);
        reloaded.load_from_file().unwrap();
        let token = reloaded
            .set_preset("Third".into(), create_test_position(), None)
            .unwrap();
        // Two presets occupied "Preset1" and "Preset2" before reload; the third
        // must be "Preset3", not "Preset1" — otherwise we'd overwrite.
        assert_eq!(token, "Preset3");
    }

    #[test]
    fn test_load_from_file_advances_counter_past_highest_preset_token() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("ptz_presets.toml");
        // Stale/low next_preset_num with a high PresetN token must not collide.
        std::fs::write(
            &path,
            r#"
next_preset_num = 1

[presets.Preset7]
name = "High"

[presets.Preset7.position]
pan_tilt_x = 0.1
pan_tilt_y = 0.2
zoom = 0.0
"#,
        )
        .unwrap();

        let store = PresetStore::with_persistence(&path);
        store.load_from_file().unwrap();
        let token = store
            .set_preset("Next".into(), create_test_position(), None)
            .unwrap();
        assert_eq!(token, "Preset8");
        assert!(store.get("Preset7").is_ok());
    }

    // ========================================================================
    // Construction
    // ========================================================================

    #[test]
    fn test_new_preset_store() {
        let store = PresetStore::new();
        assert!(store.is_empty());
    }

    #[test]
    fn test_new_preset_store_arc() {
        let store = PresetStore::new_arc();
        assert!(store.is_empty());
    }

    // ========================================================================
    // Get Operations
    // ========================================================================

    #[test]
    fn test_get_all_empty() {
        let store = PresetStore::new();
        let presets = store.get_all();
        assert!(presets.is_empty());
    }

    #[test]
    fn test_get_preset() {
        let store = PresetStore::new();
        let position = create_test_position();

        let token = store
            .set_preset("TestPreset".to_string(), position.clone(), None)
            .unwrap();

        let preset = store.get(&token).unwrap();
        assert_eq!(preset.name, Some("TestPreset".to_string()));
    }

    #[test]
    fn test_get_nonexistent_preset() {
        let store = PresetStore::new();
        let result = store.get("NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_contains() {
        let store = PresetStore::new();
        let position = create_test_position();

        let token = store
            .set_preset("Test".to_string(), position, None)
            .unwrap();

        assert!(store.contains(&token));
        assert!(!store.contains("NonExistent"));
    }

    // ========================================================================
    // Set Operations
    // ========================================================================

    #[test]
    fn test_set_new_preset() {
        let store = PresetStore::new();
        let position = create_test_position();

        let token = store
            .set_preset("NewPreset".to_string(), position, None)
            .unwrap();

        assert!(token.starts_with("Preset"));
    }

    #[test]
    fn test_set_preset_auto_increment() {
        let store = PresetStore::new();
        let position = create_test_position();

        let token1 = store
            .set_preset("Preset1".to_string(), position.clone(), None)
            .unwrap();
        let token2 = store
            .set_preset("Preset2".to_string(), position.clone(), None)
            .unwrap();
        let token3 = store
            .set_preset("Preset3".to_string(), position.clone(), None)
            .unwrap();

        assert_eq!(token1, "Preset1");
        assert_eq!(token2, "Preset2");
        assert_eq!(token3, "Preset3");
    }

    #[test]
    fn test_update_existing_preset() {
        let store = PresetStore::new();
        let position1 = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.1,
                y: 0.1,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.1,
                space: None,
            }),
        };
        let position2 = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.9,
                y: 0.9,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.9,
                space: None,
            }),
        };

        let token = store
            .set_preset("Original".to_string(), position1, None)
            .unwrap();

        // Update with new position and name
        store
            .set_preset("Updated".to_string(), position2, Some(token.clone()))
            .unwrap();

        let preset = store.get(&token).unwrap();
        assert_eq!(preset.name, Some("Updated".to_string()));

        let pos = preset.ptz_position.unwrap();
        assert_eq!(pos.pan_tilt.unwrap().x, 0.9);
    }

    #[test]
    fn test_update_nonexistent_preset() {
        let store = PresetStore::new();
        let position = create_test_position();

        let result = store.set_preset(
            "NewName".to_string(),
            position,
            Some("NonExistent".to_string()),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_create_preset_at_limit_succeeds() {
        let store = PresetStore::new();
        let position = create_test_position();
        for i in 0..MAX_PRESETS {
            assert!(
                store
                    .set_preset(format!("Preset{}", i), position.clone(), None)
                    .is_ok(),
                "Failed to create preset {}",
                i
            );
        }
        assert_eq!(store.len(), MAX_PRESETS as usize);
    }

    #[test]
    fn test_preset_set_exceeds_limit_returns_error() {
        let store = PresetStore::new();
        let position = create_test_position();

        // Create max presets
        for i in 0..MAX_PRESETS {
            let result = store.set_preset(format!("Preset{}", i), position.clone(), None);
            assert!(result.is_ok(), "Failed to create preset {}", i);
        }

        // Try to create one more
        let result = store.set_preset("OneMore".to_string(), position, None);
        assert!(result.is_err());
    }

    // ========================================================================
    // Remove Operations
    // ========================================================================

    #[test]
    fn test_remove_preset() {
        let store = PresetStore::new();
        let position = create_test_position();

        let token = store
            .set_preset("ToRemove".to_string(), position, None)
            .unwrap();

        store.remove(&token).unwrap();
        assert!(!store.contains(&token));
    }

    #[test]
    fn test_remove_nonexistent_preset() {
        let store = PresetStore::new();
        let result = store.remove("NonExistent");
        assert!(result.is_err());
    }

    // ========================================================================
    // Get All
    // ========================================================================

    #[test]
    fn test_get_all_presets() {
        let store = PresetStore::new();
        let position = create_test_position();

        store
            .set_preset("Preset1".to_string(), position.clone(), None)
            .unwrap();
        store
            .set_preset("Preset2".to_string(), position.clone(), None)
            .unwrap();
        store
            .set_preset("Preset3".to_string(), position.clone(), None)
            .unwrap();

        let all = store.get_all();
        assert_eq!(all.len(), 3);
    }

    // ========================================================================
    // Length
    // ========================================================================

    #[test]
    fn test_len() {
        let store = PresetStore::new();
        let position = create_test_position();

        assert_eq!(store.len(), 0);

        store
            .set_preset("One".to_string(), position.clone(), None)
            .unwrap();
        assert_eq!(store.len(), 1);

        store
            .set_preset("Two".to_string(), position.clone(), None)
            .unwrap();
        assert_eq!(store.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        let store = PresetStore::new();
        assert!(store.is_empty());

        let position = create_test_position();
        store.set_preset("One".to_string(), position, None).unwrap();
        assert!(!store.is_empty());
    }
}
