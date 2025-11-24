/// Errors that can occur during raycasting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RaycastError {
    /// Ray start position is outside the bounds of the octree
    StartOutOfBounds,
    /// Ray direction is zero or invalid
    InvalidDirection,
    /// Maximum recursion depth exceeded (should not happen in normal operation)
    MaxDepthExceeded,
}

impl std::fmt::Display for RaycastError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaycastError::StartOutOfBounds => write!(f, "Ray start position is out of bounds"),
            RaycastError::InvalidDirection => write!(f, "Ray direction is invalid"),
            RaycastError::MaxDepthExceeded => write!(f, "Maximum recursion depth exceeded"),
        }
    }
}

impl std::error::Error for RaycastError {}
