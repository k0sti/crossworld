/// Simple helper for determining how deep the returned cube should be.
pub fn clamp_depth(requested: u32, max_depth: u32) -> u32 {
    requested.min(max_depth)
}

/// Heuristic that maps distance in voxels to an octree depth hint.
pub fn depth_for_distance(distance: f32, macro_depth: u32, micro_depth: u32) -> u32 {
    if distance < 32.0 {
        macro_depth + micro_depth
    } else if distance < 128.0 {
        macro_depth + micro_depth.saturating_sub(1)
    } else {
        macro_depth
    }
}
