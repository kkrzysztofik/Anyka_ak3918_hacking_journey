//! PTZ preset store for persistent preset management.
//!
//! This module provides the persistent storage layer for PTZ presets.
//! It handles preset CRUD operations and maintains the preset numbering.
//!
//! The preset store is separate from runtime state (position/movement) to allow
//! for potential persistence to non-volatile storage in the future.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;

use crate::onvif::error::OnvifResult;
use crate::onvif::types::common::{PTZPreset, PTZVector};

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
}

impl PresetStore {
    /// Create a new preset store.
    pub fn new() -> Self {
        Self {
            presets: RwLock::new(HashMap::new()),
            next_preset_num: RwLock::new(1),
        }
    }

    /// Create a new preset store wrapped in Arc.
    pub fn new_arc() -> Arc<Self> {
        Arc::new(Self::new())
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
    fn test_max_presets() {
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
