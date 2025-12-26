//! Active region tracking for terrain collision
//!
//! Determines which regions need triangle-level indexing based on
//! dynamic body positions.

use glam::Vec3;
use rapier3d::parry::bounding_volume::{Aabb, BoundingVolume};

/// Tracks the active region for triangle QBVH generation
///
/// Only builds the fine-grained triangle QBVH for regions near dynamic objects.
/// Uses margin-based hysteresis to avoid rebuilding on small movements.
pub struct ActiveRegionTracker {
    /// Current active region AABB
    current_aabb: Aabb,
    /// Margin added to dynamic AABBs for velocity/prediction
    margin: f32,
    /// Whether the tracker has been initialized
    initialized: bool,
}

impl ActiveRegionTracker {
    /// Create a new tracker with the given margin
    ///
    /// # Arguments
    /// * `margin` - Distance added around dynamic bodies for safety margin
    pub fn new(margin: f32) -> Self {
        Self {
            current_aabb: Aabb::new_invalid(),
            margin,
            initialized: false,
        }
    }

    /// Update active region based on dynamic body positions
    ///
    /// Returns Some(new_aabb) if the region changed significantly and
    /// triangle QBVH should be rebuilt.
    ///
    /// # Arguments
    /// * `dynamic_aabbs` - AABBs of all dynamic bodies
    ///
    /// # Returns
    /// The new active AABB if rebuild is needed, None otherwise
    pub fn update(&mut self, dynamic_aabbs: &[Aabb]) -> Option<Aabb> {
        if dynamic_aabbs.is_empty() {
            self.initialized = false;
            return None;
        }

        // Compute union of all dynamic AABBs
        let mut new_aabb = dynamic_aabbs[0];
        for aabb in &dynamic_aabbs[1..] {
            new_aabb = new_aabb.merged(aabb);
        }

        // Add margin for velocity/prediction
        let margin_vec = rapier3d::math::Vector::new(self.margin, self.margin, self.margin);
        new_aabb.mins -= margin_vec;
        new_aabb.maxs += margin_vec;

        // First time initialization or significant change
        if !self.initialized || !self.contains(&new_aabb) {
            // Expand current region with additional margin for hysteresis
            self.current_aabb = Aabb::new(
                new_aabb.mins - margin_vec,
                new_aabb.maxs + margin_vec,
            );
            self.initialized = true;
            Some(self.current_aabb)
        } else {
            None
        }
    }

    /// Force a rebuild on next update
    pub fn invalidate(&mut self) {
        self.initialized = false;
    }

    /// Get the current active AABB
    pub fn current_aabb(&self) -> Option<&Aabb> {
        if self.initialized {
            Some(&self.current_aabb)
        } else {
            None
        }
    }

    /// Check if the current region contains the given AABB
    fn contains(&self, aabb: &Aabb) -> bool {
        self.current_aabb.mins.x <= aabb.mins.x
            && self.current_aabb.mins.y <= aabb.mins.y
            && self.current_aabb.mins.z <= aabb.mins.z
            && self.current_aabb.maxs.x >= aabb.maxs.x
            && self.current_aabb.maxs.y >= aabb.maxs.y
            && self.current_aabb.maxs.z >= aabb.maxs.z
    }
}

/// Convert glam Vec3 to Rapier AABB
pub fn aabb_from_center_half_extents(center: Vec3, half_extents: Vec3) -> Aabb {
    let min = center - half_extents;
    let max = center + half_extents;
    Aabb::new(min.to_array().into(), max.to_array().into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_aabb(min: [f32; 3], max: [f32; 3]) -> Aabb {
        Aabb::new(min.into(), max.into())
    }

    #[test]
    fn test_empty_dynamics_returns_none() {
        let mut tracker = ActiveRegionTracker::new(1.0);
        let result = tracker.update(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_first_update_triggers_rebuild() {
        let mut tracker = ActiveRegionTracker::new(1.0);
        let aabb = make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);

        let result = tracker.update(&[aabb]);
        assert!(result.is_some());
    }

    #[test]
    fn test_small_movement_no_rebuild() {
        let mut tracker = ActiveRegionTracker::new(5.0);

        // Initial position
        let aabb1 = make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let result1 = tracker.update(&[aabb1]);
        assert!(result1.is_some());

        // Small movement within margin - should not trigger rebuild
        let aabb2 = make_aabb([0.5, 0.5, 0.5], [1.5, 1.5, 1.5]);
        let result2 = tracker.update(&[aabb2]);
        assert!(result2.is_none(), "Small movement should not trigger rebuild");
    }

    #[test]
    fn test_large_movement_triggers_rebuild() {
        let mut tracker = ActiveRegionTracker::new(1.0);

        // Initial position
        let aabb1 = make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let result1 = tracker.update(&[aabb1]);
        assert!(result1.is_some());

        // Large movement outside current region
        let aabb2 = make_aabb([100.0, 100.0, 100.0], [101.0, 101.0, 101.0]);
        let result2 = tracker.update(&[aabb2]);
        assert!(result2.is_some(), "Large movement should trigger rebuild");
    }

    #[test]
    fn test_multiple_bodies_union() {
        let mut tracker = ActiveRegionTracker::new(0.0);

        let aabb1 = make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        let aabb2 = make_aabb([5.0, 5.0, 5.0], [6.0, 6.0, 6.0]);

        let result = tracker.update(&[aabb1, aabb2]);
        assert!(result.is_some());

        let active = result.unwrap();
        // Should contain both AABBs (with margin applied)
        assert!(active.mins.x <= 0.0);
        assert!(active.mins.y <= 0.0);
        assert!(active.mins.z <= 0.0);
        assert!(active.maxs.x >= 6.0);
        assert!(active.maxs.y >= 6.0);
        assert!(active.maxs.z >= 6.0);
    }

    #[test]
    fn test_invalidate_forces_rebuild() {
        let mut tracker = ActiveRegionTracker::new(5.0);

        // Initial update
        let aabb = make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        tracker.update(&[aabb]);

        // Same position - no rebuild
        let result1 = tracker.update(&[aabb]);
        assert!(result1.is_none());

        // Invalidate and try again
        tracker.invalidate();
        let result2 = tracker.update(&[aabb]);
        assert!(result2.is_some(), "Should rebuild after invalidation");
    }

    #[test]
    fn test_current_aabb() {
        let mut tracker = ActiveRegionTracker::new(1.0);

        assert!(tracker.current_aabb().is_none());

        let aabb = make_aabb([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]);
        tracker.update(&[aabb]);

        assert!(tracker.current_aabb().is_some());
    }

    #[test]
    fn test_aabb_from_center_half_extents() {
        let center = Vec3::new(5.0, 10.0, 15.0);
        let half_extents = Vec3::new(1.0, 2.0, 3.0);

        let aabb = aabb_from_center_half_extents(center, half_extents);

        assert!((aabb.mins.x - 4.0).abs() < 1e-6);
        assert!((aabb.mins.y - 8.0).abs() < 1e-6);
        assert!((aabb.mins.z - 12.0).abs() < 1e-6);
        assert!((aabb.maxs.x - 6.0).abs() < 1e-6);
        assert!((aabb.maxs.y - 12.0).abs() < 1e-6);
        assert!((aabb.maxs.z - 18.0).abs() < 1e-6);
    }
}
