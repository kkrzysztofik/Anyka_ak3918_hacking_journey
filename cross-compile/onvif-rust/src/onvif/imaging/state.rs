//! Imaging Service runtime state.
//!
//! This module provides runtime state management for imaging operations.
//! Currently provides scaffold for focus runtime state (for future hardware support).

use parking_lot::RwLock;
use std::sync::Arc;

/// Focus runtime state.
///
/// This structure holds runtime state for focus operations.
/// Currently a scaffold for future hardware support.
#[derive(Debug, Clone, Default)]
pub struct FocusState {
    /// Whether focus is currently moving.
    pub is_moving: bool,
    /// Current focus position (0.0 - 100.0).
    pub position: Option<f32>,
    /// Focus move mode (absolute, relative, continuous).
    pub move_mode: Option<FocusMoveMode>,
}

/// Focus move mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusMoveMode {
    /// Absolute focus positioning.
    Absolute,
    /// Relative focus positioning.
    Relative,
    /// Continuous focus movement.
    #[default]
    Continuous,
}

/// Imaging runtime state.
///
/// Holds all runtime state for the imaging service.
#[derive(Debug, Default)]
pub struct ImagingState {
    /// Focus runtime state per video source.
    pub focus_states: RwLock<std::collections::HashMap<String, FocusState>>,
}

impl ImagingState {
    /// Create a new imaging state.
    pub fn new() -> Self {
        Self {
            focus_states: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Get focus state for a video source.
    pub fn get_focus_state(&self, token: &str) -> Option<FocusState> {
        let states = self.focus_states.read();
        states.get(token).cloned()
    }

    /// Set focus state for a video source.
    pub fn set_focus_state(&self, token: String, state: FocusState) {
        let mut states = self.focus_states.write();
        states.insert(token, state);
    }

    /// Clear focus state for a video source.
    pub fn clear_focus_state(&self, token: &str) {
        let mut states = self.focus_states.write();
        states.remove(token);
    }
}

/// Create a new imaging state wrapped in Arc.
pub fn new_state() -> Arc<ImagingState> {
    Arc::new(ImagingState::new())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = new_state();
        assert!(state.get_focus_state("VideoSource_1").is_none());
    }

    #[test]
    fn test_set_focus_state() {
        let state = new_state();
        let focus_state = FocusState {
            is_moving: true,
            position: Some(50.0),
            move_mode: Some(FocusMoveMode::Absolute),
        };
        state.set_focus_state("VideoSource_1".to_string(), focus_state.clone());

        let retrieved = state.get_focus_state("VideoSource_1");
        assert!(retrieved.is_some());
        assert!(retrieved.unwrap().is_moving);
    }

    #[test]
    fn test_clear_focus_state() {
        let state = new_state();
        let focus_state = FocusState {
            is_moving: true,
            position: Some(50.0),
            move_mode: Some(FocusMoveMode::Absolute),
        };
        state.set_focus_state("VideoSource_1".to_string(), focus_state);
        state.clear_focus_state("VideoSource_1");

        assert!(state.get_focus_state("VideoSource_1").is_none());
    }

    #[test]
    fn test_focus_state_default() {
        let state = FocusState::default();
        assert!(!state.is_moving);
        assert!(state.position.is_none());
        assert!(state.move_mode.is_none());
    }
}
