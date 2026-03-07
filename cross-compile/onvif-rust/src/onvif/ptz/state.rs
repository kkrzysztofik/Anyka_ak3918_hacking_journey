//! PTZ runtime state management.
//!
//! This module provides the runtime state management for PTZ operations including:
//! - Current position tracking
//! - Movement status tracking
//! - Home position management
//!
//! Preset storage is handled by the separate `store::PresetStore` module.

use std::sync::Arc;

use chrono::Utc;
use parking_lot::RwLock;

use crate::onvif::types::common::{
    MoveStatus, PTZMoveStatus, PTZStatus, PTZVector, Vector1D, Vector2D,
};

use super::store::PresetStore;
use super::types::{SPACE_ABSOLUTE_PAN_TILT, SPACE_ABSOLUTE_ZOOM};

// ============================================================================
// PTZ Runtime State
// ============================================================================

/// Internal PTZ position state.
#[derive(Debug, Clone, Default)]
pub struct Position {
    /// Pan value (-1.0 to 1.0).
    pub pan: f32,
    /// Tilt value (-1.0 to 1.0).
    pub tilt: f32,
    /// Zoom value (0.0 to 1.0).
    pub zoom: f32,
}

impl Position {
    /// Create a new position at home (center, no zoom).
    pub fn home() -> Self {
        Self {
            pan: 0.0,
            tilt: 0.0,
            zoom: 0.0,
        }
    }

    /// Convert to PTZ vector.
    pub fn to_ptz_vector(&self) -> PTZVector {
        PTZVector {
            pan_tilt: Some(Vector2D {
                x: self.pan,
                y: self.tilt,
                space: Some(SPACE_ABSOLUTE_PAN_TILT.to_string()),
            }),
            zoom: Some(Vector1D {
                x: self.zoom,
                space: Some(SPACE_ABSOLUTE_ZOOM.to_string()),
            }),
        }
    }

    /// Create from PTZ vector.
    pub fn from_ptz_vector(vector: &PTZVector) -> Self {
        let mut pos = Self::default();
        if let Some(pt) = &vector.pan_tilt {
            pos.pan = pt.x;
            pos.tilt = pt.y;
        }
        if let Some(z) = &vector.zoom {
            pos.zoom = z.x;
        }
        pos
    }
}

/// Movement state.
#[derive(Debug, Clone, Default)]
pub struct MovementState {
    /// Pan/tilt moving.
    pub pan_tilt_moving: bool,
    /// Zoom moving.
    pub zoom_moving: bool,
}

// ============================================================================
// PTZ State Manager
// ============================================================================

/// PTZ State Manager for managing PTZ position and presets.
///
/// Provides thread-safe access to PTZ state including:
/// - Current position and movement status
/// - Preset storage (up to 255 presets, in-memory only)
/// - Home position
///
/// # Example
///
/// ```rust,no_run
/// use onvif_rust::onvif::ptz::PTZStateManager;
///
/// let manager = PTZStateManager::new();
///
/// // Get current status
/// let status = manager.get_status();
///
/// // Set a preset
/// let token = manager.set_preset("MyPreset".to_string(), None).unwrap();
/// ```
pub struct PTZStateManager {
    /// Current position.
    position: RwLock<Position>,
    /// Movement state.
    movement: RwLock<MovementState>,
    /// Home position.
    home_position: RwLock<Position>,
    /// Preset store (persistent storage).
    pub(crate) presets: Arc<PresetStore>,
}

impl PTZStateManager {
    /// Create a new PTZ State Manager.
    pub fn new() -> Self {
        Self {
            position: RwLock::new(Position::home()),
            movement: RwLock::new(MovementState::default()),
            home_position: RwLock::new(Position::home()),
            presets: PresetStore::new_arc(),
        }
    }

    /// Create a new PTZ State Manager with existing preset store.
    pub fn with_preset_store(presets: Arc<PresetStore>) -> Self {
        Self {
            position: RwLock::new(Position::home()),
            movement: RwLock::new(MovementState::default()),
            home_position: RwLock::new(Position::home()),
            presets,
        }
    }

    // ========================================================================
    // Position Management
    // ========================================================================

    /// Get the current PTZ position as a PTZ vector.
    pub fn get_position(&self) -> PTZVector {
        self.position.read().to_ptz_vector()
    }

    /// Set the current PTZ position.
    pub fn set_position(&self, vector: &PTZVector) {
        let mut pos = self.position.write();
        *pos = Position::from_ptz_vector(vector);
    }

    /// Get the current PTZ status.
    pub fn get_status(&self) -> PTZStatus {
        let pos = self.position.read();
        let movement = self.movement.read();

        PTZStatus {
            position: Some(pos.to_ptz_vector()),
            move_status: Some(PTZMoveStatus {
                pan_tilt: Some(if movement.pan_tilt_moving {
                    MoveStatus::MOVING
                } else {
                    MoveStatus::IDLE
                }),
                zoom: Some(if movement.zoom_moving {
                    MoveStatus::MOVING
                } else {
                    MoveStatus::IDLE
                }),
            }),
            error: None,
            utc_time: Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        }
    }

    /// Set movement state.
    pub fn set_moving(&self, pan_tilt: bool, zoom: bool) {
        let mut movement = self.movement.write();
        movement.pan_tilt_moving = pan_tilt;
        movement.zoom_moving = zoom;
    }

    /// Stop all movement.
    pub fn stop(&self) {
        self.set_moving(false, false);
    }

    /// Check if PTZ is currently moving (any axis).
    pub fn is_moving(&self) -> bool {
        let movement = self.movement.read();
        movement.pan_tilt_moving || movement.zoom_moving
    }

    /// Check if pan/tilt is currently moving.
    pub fn is_pan_tilt_moving(&self) -> bool {
        self.movement.read().pan_tilt_moving
    }

    /// Check if zoom is currently moving.
    pub fn is_zoom_moving(&self) -> bool {
        self.movement.read().zoom_moving
    }

    // ========================================================================
    // Home Position
    // ========================================================================

    /// Get the home position.
    pub fn get_home_position(&self) -> PTZVector {
        self.home_position.read().to_ptz_vector()
    }

    /// Set the current position as home.
    pub fn set_home_position(&self) {
        let current = self.position.read().clone();
        let mut home = self.home_position.write();
        *home = current;
    }

    /// Go to the home position (updates current position).
    pub fn goto_home(&self) {
        let home = self.home_position.read().clone();
        let mut pos = self.position.write();
        *pos = home;
    }

    // ========================================================================
    // Preset Management (delegates to PresetStore)
    // ========================================================================

    /// Get all presets.
    pub fn get_presets(&self) -> Vec<crate::onvif::types::common::PTZPreset> {
        self.presets.get_all()
    }

    /// Get a specific preset.
    pub fn get_preset(
        &self,
        token: &str,
    ) -> crate::onvif::OnvifResult<crate::onvif::types::common::PTZPreset> {
        self.presets.get(token)
    }

    /// Set a preset at the current position.
    ///
    /// If `token` is provided, updates an existing preset.
    /// If `token` is None, creates a new preset with auto-generated token.
    ///
    /// Returns the preset token.
    pub fn set_preset(
        &self,
        name: String,
        token: Option<String>,
    ) -> crate::onvif::OnvifResult<String> {
        let position = self.get_position();
        self.presets.set_preset(name, position, token)
    }

    /// Go to a preset position.
    pub fn goto_preset(&self, token: &str) -> crate::onvif::OnvifResult<()> {
        let preset = self.presets.get(token)?;
        let position = preset.ptz_position.ok_or_else(|| {
            crate::onvif::OnvifError::NotFound("Preset has no position".to_string())
        })?;
        self.set_position(&position);
        Ok(())
    }

    /// Remove a preset.
    pub fn remove_preset(&self, token: &str) -> crate::onvif::OnvifResult<()> {
        self.presets.remove(token)
    }
}

impl Default for PTZStateManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Position Tests
    // ========================================================================

    #[test]
    fn test_position_to_ptz_vector() {
        let pos = Position {
            pan: 0.5,
            tilt: -0.3,
            zoom: 0.7,
        };

        let vector = pos.to_ptz_vector();

        let pt = vector.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.5);
        assert_eq!(pt.y, -0.3);

        let z = vector.zoom.unwrap();
        assert_eq!(z.x, 0.7);
    }

    #[test]
    fn test_position_from_ptz_vector() {
        let vector = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: -0.3,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.7,
                space: None,
            }),
        };

        let pos = Position::from_ptz_vector(&vector);

        assert_eq!(pos.pan, 0.5);
        assert_eq!(pos.tilt, -0.3);
        assert_eq!(pos.zoom, 0.7);
    }

    #[test]
    fn test_position_home() {
        let pos = Position::home();
        assert_eq!(pos.pan, 0.0);
        assert_eq!(pos.tilt, 0.0);
        assert_eq!(pos.zoom, 0.0);
    }

    // ========================================================================
    // State Manager Construction
    // ========================================================================

    #[test]
    fn test_new_state_manager() {
        let manager = PTZStateManager::new();

        // Should start at home position
        let status = manager.get_status();
        let pos = status.position.unwrap();

        let pt = pos.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.0);
        assert_eq!(pt.y, 0.0);

        let z = pos.zoom.unwrap();
        assert_eq!(z.x, 0.0);
    }

    #[test]
    fn test_default_not_moving() {
        let manager = PTZStateManager::new();
        assert!(!manager.is_moving());
    }

    #[test]
    fn test_with_preset_store() {
        let store = PresetStore::new_arc();
        let manager = PTZStateManager::with_preset_store(store.clone());

        // Should use the provided store
        assert!(Arc::ptr_eq(&manager.presets, &store));
    }

    // ========================================================================
    // Position Management
    // ========================================================================

    #[test]
    fn test_set_position() {
        let manager = PTZStateManager::new();

        let new_pos = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.5,
                y: -0.3,
                space: Some(SPACE_ABSOLUTE_PAN_TILT.to_string()),
            }),
            zoom: Some(Vector1D {
                x: 0.7,
                space: Some(SPACE_ABSOLUTE_ZOOM.to_string()),
            }),
        };

        manager.set_position(&new_pos);

        let pos = manager.get_position();
        let pt = pos.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.5);
        assert_eq!(pt.y, -0.3);

        let z = pos.zoom.unwrap();
        assert_eq!(z.x, 0.7);
    }

    #[test]
    fn test_get_status_returns_utc_time() {
        let manager = PTZStateManager::new();
        let status = manager.get_status();

        // Should have a valid UTC timestamp
        assert!(!status.utc_time.is_empty());
        assert!(status.utc_time.contains('T'));
        assert!(status.utc_time.ends_with('Z'));
    }

    // ========================================================================
    // Movement Status
    // ========================================================================

    #[test]
    fn test_set_moving() {
        let manager = PTZStateManager::new();

        manager.set_moving(true, false);
        assert!(manager.is_moving());

        let status = manager.get_status();
        let move_status = status.move_status.unwrap();
        assert_eq!(move_status.pan_tilt, Some(MoveStatus::MOVING));
        assert_eq!(move_status.zoom, Some(MoveStatus::IDLE));
    }

    #[test]
    fn test_stop() {
        let manager = PTZStateManager::new();

        manager.set_moving(true, true);
        assert!(manager.is_moving());

        manager.stop();
        assert!(!manager.is_moving());
    }

    // ========================================================================
    // Home Position
    // ========================================================================

    #[test]
    fn test_set_and_goto_home() {
        let manager = PTZStateManager::new();

        // Move to a position
        let pos1 = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.8,
                y: 0.4,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.5,
                space: None,
            }),
        };
        manager.set_position(&pos1);

        // Set as home
        manager.set_home_position();

        // Move somewhere else
        let pos2 = PTZVector {
            pan_tilt: Some(Vector2D {
                x: -0.5,
                y: -0.5,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        };
        manager.set_position(&pos2);

        // Go to home
        manager.goto_home();

        // Should be back at original position
        let current = manager.get_position();
        let pt = current.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.8);
        assert_eq!(pt.y, 0.4);
    }

    // ========================================================================
    // Preset Management (delegation tests)
    // ========================================================================

    #[test]
    fn test_set_and_get_preset() {
        let manager = PTZStateManager::new();

        // Move to a position
        let pos = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.3,
                y: -0.2,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.4,
                space: None,
            }),
        };
        manager.set_position(&pos);

        // Create preset
        let token = manager.set_preset("TestPreset".to_string(), None).unwrap();

        // Get preset
        let preset = manager.get_preset(&token).unwrap();
        assert_eq!(preset.name, Some("TestPreset".to_string()));

        let preset_pos = preset.ptz_position.unwrap();
        let pt = preset_pos.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.3);
        assert_eq!(pt.y, -0.2);
    }

    #[test]
    fn test_goto_preset() {
        let manager = PTZStateManager::new();

        // Create preset at specific position
        let pos = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.6,
                y: 0.7,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.8,
                space: None,
            }),
        };
        manager.set_position(&pos);
        let token = manager.set_preset("GotoTest".to_string(), None).unwrap();

        // Move somewhere else
        let pos2 = PTZVector {
            pan_tilt: Some(Vector2D {
                x: 0.0,
                y: 0.0,
                space: None,
            }),
            zoom: Some(Vector1D {
                x: 0.0,
                space: None,
            }),
        };
        manager.set_position(&pos2);

        // Go to preset
        manager.goto_preset(&token).unwrap();

        // Verify position
        let current = manager.get_position();
        let pt = current.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.6);
        assert_eq!(pt.y, 0.7);
    }

    #[test]
    fn test_remove_preset() {
        let manager = PTZStateManager::new();

        // Create preset
        let token = manager.set_preset("ToRemove".to_string(), None).unwrap();

        // Remove it
        manager.remove_preset(&token).unwrap();

        // Should fail to get it
        let result = manager.get_preset(&token);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove_nonexistent_preset() {
        let manager = PTZStateManager::new();

        let result = manager.remove_preset("NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_goto_nonexistent_preset() {
        let manager = PTZStateManager::new();

        let result = manager.goto_preset("NonExistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_presets() {
        let manager = PTZStateManager::new();

        // Create multiple presets
        manager.set_preset("Preset1".to_string(), None).unwrap();
        manager.set_preset("Preset2".to_string(), None).unwrap();
        manager.set_preset("Preset3".to_string(), None).unwrap();

        let presets = manager.get_presets();
        assert_eq!(presets.len(), 3);
    }

    #[test]
    fn test_update_existing_preset() {
        let manager = PTZStateManager::new();

        // Create initial preset
        let pos1 = PTZVector {
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
        manager.set_position(&pos1);
        let token = manager.set_preset("UpdateTest".to_string(), None).unwrap();

        // Move to new position
        let pos2 = PTZVector {
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
        manager.set_position(&pos2);

        // Update preset with same token
        manager
            .set_preset("UpdatedPreset".to_string(), Some(token.clone()))
            .unwrap();

        // Verify update
        let preset = manager.get_preset(&token).unwrap();
        assert_eq!(preset.name, Some("UpdatedPreset".to_string()));

        let preset_pos = preset.ptz_position.unwrap();
        let pt = preset_pos.pan_tilt.unwrap();
        assert_eq!(pt.x, 0.9);
        assert_eq!(pt.y, 0.9);
    }
}
