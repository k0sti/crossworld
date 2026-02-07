//! Touch input support
//!
//! This module provides types for handling touch input on mobile devices
//! and touch-enabled displays. This is currently a placeholder for future
//! mobile support.

use glam::Vec2;

/// Touch event phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// A finger touched the screen
    Started,
    /// A finger moved on the screen
    Moved,
    /// A finger was lifted from the screen
    Ended,
    /// The touch was cancelled (e.g., by a phone call)
    Cancelled,
}

/// A single touch point
#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
    /// Unique identifier for this touch (for multi-touch tracking)
    pub id: u64,
    /// Current position of the touch in window coordinates
    pub position: Vec2,
    /// Previous position of the touch (if moved)
    pub previous_position: Vec2,
    /// The phase of this touch event
    pub phase: TouchPhase,
    /// Touch pressure (if available, 0.0-1.0)
    pub pressure: f32,
    /// Touch radius in pixels (if available)
    pub radius: f32,
}

impl TouchPoint {
    /// Create a new touch point
    pub fn new(id: u64, position: Vec2, phase: TouchPhase) -> Self {
        Self {
            id,
            position,
            previous_position: position,
            phase,
            pressure: 1.0,
            radius: 1.0,
        }
    }

    /// Get the delta movement since the last update
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }
}

/// Touch state tracker for multi-touch input
///
/// Tracks all active touch points and provides query methods.
#[derive(Debug, Clone, Default)]
pub struct TouchState {
    /// Currently active touch points
    touches: Vec<TouchPoint>,
    /// Maximum number of simultaneous touches seen
    max_touches: usize,
}

impl TouchState {
    /// Create a new empty touch state
    pub fn new() -> Self {
        Self::default()
    }

    /// Update touch state with a new touch event
    pub fn update_touch(&mut self, touch: TouchPoint) {
        match touch.phase {
            TouchPhase::Started => {
                self.touches.push(touch);
                self.max_touches = self.max_touches.max(self.touches.len());
            }
            TouchPhase::Moved => {
                if let Some(existing) = self.touches.iter_mut().find(|t| t.id == touch.id) {
                    existing.previous_position = existing.position;
                    existing.position = touch.position;
                    existing.pressure = touch.pressure;
                    existing.radius = touch.radius;
                    existing.phase = TouchPhase::Moved;
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.touches.retain(|t| t.id != touch.id);
            }
        }
    }

    /// Get all currently active touch points
    pub fn touches(&self) -> &[TouchPoint] {
        &self.touches
    }

    /// Get a specific touch point by ID
    pub fn get_touch(&self, id: u64) -> Option<&TouchPoint> {
        self.touches.iter().find(|t| t.id == id)
    }

    /// Get the number of currently active touches
    pub fn touch_count(&self) -> usize {
        self.touches.len()
    }

    /// Check if there are any active touches
    pub fn is_touching(&self) -> bool {
        !self.touches.is_empty()
    }

    /// Get the first touch point (convenience for single-touch scenarios)
    pub fn primary_touch(&self) -> Option<&TouchPoint> {
        self.touches.first()
    }

    /// Calculate the center point of all active touches
    pub fn center(&self) -> Option<Vec2> {
        if self.touches.is_empty() {
            return None;
        }

        let sum: Vec2 = self.touches.iter().map(|t| t.position).sum();
        Some(sum / self.touches.len() as f32)
    }

    /// Calculate the average delta movement of all touches
    pub fn average_delta(&self) -> Vec2 {
        if self.touches.is_empty() {
            return Vec2::ZERO;
        }

        let sum: Vec2 = self.touches.iter().map(|t| t.delta()).sum();
        sum / self.touches.len() as f32
    }

    /// Calculate pinch/zoom scale factor (for two-finger gestures)
    ///
    /// Returns Some(scale) if exactly two touches are active, where scale > 1.0
    /// means fingers are moving apart (zoom in) and scale < 1.0 means fingers
    /// are moving together (zoom out). Returns None otherwise.
    pub fn pinch_scale(&self) -> Option<f32> {
        if self.touches.len() != 2 {
            return None;
        }

        let t0 = &self.touches[0];
        let t1 = &self.touches[1];

        let current_dist = t0.position.distance(t1.position);
        let prev_dist = t0.previous_position.distance(t1.previous_position);

        if prev_dist > 0.001 {
            Some(current_dist / prev_dist)
        } else {
            Some(1.0)
        }
    }

    /// Clear all touch state
    pub fn clear(&mut self) {
        self.touches.clear();
    }

    /// Get the maximum number of simultaneous touches recorded
    pub fn max_touches_recorded(&self) -> usize {
        self.max_touches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_touch_point_delta() {
        let mut touch = TouchPoint::new(1, Vec2::new(100.0, 100.0), TouchPhase::Started);
        assert_eq!(touch.delta(), Vec2::ZERO);

        touch.previous_position = touch.position;
        touch.position = Vec2::new(150.0, 120.0);
        assert_eq!(touch.delta(), Vec2::new(50.0, 20.0));
    }

    #[test]
    fn test_touch_state_lifecycle() {
        let mut state = TouchState::new();
        assert!(!state.is_touching());

        // Start touch
        state.update_touch(TouchPoint::new(
            1,
            Vec2::new(100.0, 100.0),
            TouchPhase::Started,
        ));
        assert!(state.is_touching());
        assert_eq!(state.touch_count(), 1);

        // Move touch
        let mut moved_touch = TouchPoint::new(1, Vec2::new(150.0, 150.0), TouchPhase::Moved);
        moved_touch.previous_position = Vec2::new(100.0, 100.0);
        state.update_touch(moved_touch);
        assert_eq!(state.touch_count(), 1);

        // End touch
        state.update_touch(TouchPoint::new(
            1,
            Vec2::new(150.0, 150.0),
            TouchPhase::Ended,
        ));
        assert!(!state.is_touching());
    }

    #[test]
    fn test_multi_touch() {
        let mut state = TouchState::new();

        state.update_touch(TouchPoint::new(
            1,
            Vec2::new(100.0, 100.0),
            TouchPhase::Started,
        ));
        state.update_touch(TouchPoint::new(
            2,
            Vec2::new(200.0, 200.0),
            TouchPhase::Started,
        ));

        assert_eq!(state.touch_count(), 2);
        assert_eq!(state.max_touches_recorded(), 2);

        let center = state.center().unwrap();
        assert_eq!(center, Vec2::new(150.0, 150.0));
    }

    #[test]
    fn test_pinch_scale() {
        let mut state = TouchState::new();

        // No pinch with single touch
        state.update_touch(TouchPoint::new(
            1,
            Vec2::new(100.0, 100.0),
            TouchPhase::Started,
        ));
        assert!(state.pinch_scale().is_none());

        // Add second touch
        state.update_touch(TouchPoint::new(
            2,
            Vec2::new(200.0, 100.0),
            TouchPhase::Started,
        ));

        // Initial scale should be 1.0
        assert!((state.pinch_scale().unwrap() - 1.0).abs() < 0.001);
    }
}
