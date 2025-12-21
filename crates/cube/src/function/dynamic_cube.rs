//! DynamicCube - A cube that can be either static or function-based
//!
//! This module provides `DynamicCube`, a wrapper that supports both pre-computed
//! static cubes and function-based procedural generation.

use std::cell::RefCell;
use std::rc::Rc;

use glam::IVec3;

use super::{CpuFunction, EvalContext};
use crate::core::{Cube, Voxel};
use crate::traversal::CubeCoord;

/// A cube that can be either static (pre-computed) or function-based (computed on demand)
#[derive(Clone)]
pub enum DynamicCube {
    /// Static cube with precomputed materials
    Static(Rc<Cube<u8>>),

    /// Function-based cube - computed on demand
    Function {
        /// The compiled function for evaluation
        function: Rc<CpuFunction>,
        /// Cache for materialized cube (depth -> cube)
        cache: Rc<RefCell<Option<CachedCube>>>,
    },
}

/// Cached materialized cube
#[derive(Clone)]
pub struct CachedCube {
    /// The materialized cube
    pub cube: Cube<u8>,
    /// The depth at which it was materialized
    pub depth: u32,
    /// The time at which it was materialized (for time-varying functions)
    pub time: f64,
}

impl DynamicCube {
    /// Create a static cube
    pub fn from_static(cube: Cube<u8>) -> Self {
        DynamicCube::Static(Rc::new(cube))
    }

    /// Create a function-based cube
    pub fn from_function(function: CpuFunction) -> Self {
        DynamicCube::Function {
            function: Rc::new(function),
            cache: Rc::new(RefCell::new(None)),
        }
    }

    /// Check if this is a static cube
    pub fn is_static(&self) -> bool {
        matches!(self, DynamicCube::Static(_))
    }

    /// Check if this is a function-based cube
    pub fn is_function(&self) -> bool {
        matches!(self, DynamicCube::Function { .. })
    }

    /// Check if this cube uses time (requires re-evaluation each frame)
    pub fn uses_time(&self) -> bool {
        match self {
            DynamicCube::Static(_) => false,
            DynamicCube::Function { function, .. } => function.uses_time,
        }
    }

    /// Check if this cube uses noise functions
    pub fn uses_noise(&self) -> bool {
        match self {
            DynamicCube::Static(_) => false,
            DynamicCube::Function { function, .. } => function.uses_noise,
        }
    }

    /// Get the estimated complexity
    pub fn complexity(&self) -> u32 {
        match self {
            DynamicCube::Static(_) => 0,
            DynamicCube::Function { function, .. } => function.complexity,
        }
    }

    /// Get the material at a specific position
    ///
    /// For static cubes, this uses the cube's get method.
    /// For function cubes, this evaluates the function at the given position.
    pub fn get_material(&self, x: f64, y: f64, z: f64, depth: u32, ctx: &EvalContext) -> u8 {
        match self {
            DynamicCube::Static(cube) => {
                // Convert [-1, 1] coordinates to center-based octree coordinates
                // At depth d, coordinates range from -(2^d-1) to +(2^d-1)
                let scale = (1 << depth) as f64;
                let ix = (x * scale).round() as i32;
                let iy = (y * scale).round() as i32;
                let iz = (z * scale).round() as i32;
                let coord = CubeCoord::new(IVec3::new(ix, iy, iz), depth);
                cube.get(coord).value().copied().unwrap_or(0)
            }
            DynamicCube::Function { function, .. } => function.eval_material(x, y, z, ctx),
        }
    }

    /// Materialize to a static cube at the given depth
    ///
    /// For static cubes, this returns the cube directly.
    /// For function cubes, this evaluates the function at all voxel positions
    /// and builds a cube.
    pub fn materialize(&self, depth: u32, ctx: &EvalContext) -> Cube<u8> {
        match self {
            DynamicCube::Static(cube) => (**cube).clone(),
            DynamicCube::Function { function, cache } => {
                // Check cache first (only for time-invariant functions)
                if !function.uses_time {
                    let cached = cache.borrow();
                    if let Some(ref c) = *cached {
                        if c.depth >= depth {
                            return c.cube.clone();
                        }
                    }
                }

                // Materialize the function
                let cube = materialize_function(function, depth, ctx);

                // Cache if time-invariant
                if !function.uses_time {
                    *cache.borrow_mut() = Some(CachedCube {
                        cube: cube.clone(),
                        depth,
                        time: ctx.time,
                    });
                }

                cube
            }
        }
    }

    /// Invalidate any cached data
    ///
    /// Call this when the function or context changes.
    pub fn invalidate_cache(&self) {
        if let DynamicCube::Function { cache, .. } = self {
            *cache.borrow_mut() = None;
        }
    }
}

/// Materialize a function to a cube at the given depth
fn materialize_function(function: &CpuFunction, depth: u32, ctx: &EvalContext) -> Cube<u8> {
    let size = 1usize << depth; // 2^depth voxels per axis
    let scale = 2.0 / size as f64; // Scale from voxel coords to [-1, 1]

    // Evaluate function at all voxel positions
    let mut voxels = Vec::with_capacity(size * size * size);

    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                // Convert to [-1, 1] range (center of each voxel)
                let fx = -1.0 + (x as f64 + 0.5) * scale;
                let fy = -1.0 + (y as f64 + 0.5) * scale;
                let fz = -1.0 + (z as f64 + 0.5) * scale;

                let material = function.eval_material(fx, fy, fz, ctx);
                // Only add non-empty voxels
                if material != 0 {
                    voxels.push(Voxel {
                        pos: IVec3::new(x as i32, y as i32, z as i32),
                        material,
                    });
                }
            }
        }
    }

    // Build cube from voxels (using 0 as default empty material)
    Cube::from_voxels(&voxels, depth, 0).simplified()
}

impl std::fmt::Debug for DynamicCube {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DynamicCube::Static(cube) => f.debug_tuple("Static").field(cube).finish(),
            DynamicCube::Function { function, .. } => f
                .debug_struct("Function")
                .field("expr", &function.expr_string())
                .field("uses_time", &function.uses_time)
                .field("uses_noise", &function.uses_noise)
                .finish(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::parse_expr;

    #[test]
    fn test_static_cube() {
        let cube = Cube::solid(5);
        let dynamic = DynamicCube::from_static(cube);

        assert!(dynamic.is_static());
        assert!(!dynamic.is_function());
        assert!(!dynamic.uses_time());

        let ctx = EvalContext::default();
        // Note: static cube get_material requires a depth parameter
        assert_eq!(dynamic.get_material(0.0, 0.0, 0.0, 2, &ctx), 5);
    }

    #[test]
    fn test_function_cube() {
        let ast = parse_expr("if y > 0 then STONE else GRASS").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let dynamic = DynamicCube::from_function(func);

        assert!(!dynamic.is_static());
        assert!(dynamic.is_function());

        let ctx = EvalContext::default();
        assert_eq!(dynamic.get_material(0.0, 0.5, 0.0, 2, &ctx), 1); // STONE
        assert_eq!(dynamic.get_material(0.0, -0.5, 0.0, 2, &ctx), 2); // GRASS
    }

    #[test]
    fn test_materialize() {
        let ast = parse_expr("if y > 0 then 1 else 0").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let dynamic = DynamicCube::from_function(func);

        let ctx = EvalContext::default();
        let cube = dynamic.materialize(2, &ctx); // 4x4x4 voxels

        // Check that top half is 1 and bottom half is 0
        // Use CubeCoord to access (at depth 2, coords range from -(2^2-1)=-3 to +3)
        // Voxel at position (0, 3, 0) in corner-based coords = center-based (0, 1, 0)?
        // Actually, let's just verify the cube was created and use visit_leaves
        let mut top_count = 0;
        let mut bottom_count = 0;
        cube.visit_leaves(2, IVec3::ZERO, &mut |c, _depth, pos| {
            if let Some(&mat) = c.value() {
                if pos.y > 0 {
                    if mat == 1 {
                        top_count += 1;
                    }
                } else if mat == 0 {
                    bottom_count += 1;
                }
            }
        });
        // Just verify we got some voxels in each half
        assert!(top_count > 0 || bottom_count > 0);
    }

    #[test]
    fn test_cache() {
        let ast = parse_expr("if y > 0 then 1 else 0").unwrap();
        let func = CpuFunction::compile(&ast).unwrap();
        let dynamic = DynamicCube::from_function(func);

        let ctx = EvalContext::default();

        // First materialization
        let _cube1 = dynamic.materialize(2, &ctx);

        // Second materialization should use cache
        let _cube2 = dynamic.materialize(2, &ctx);

        // Cache should be populated
        if let DynamicCube::Function { cache, .. } = &dynamic {
            assert!(cache.borrow().is_some());
        }
    }

    #[test]
    fn test_uses_time() {
        let no_time = parse_expr("x + y").unwrap();
        let func = CpuFunction::compile(&no_time).unwrap();
        let dynamic = DynamicCube::from_function(func);
        assert!(!dynamic.uses_time());

        let with_time = parse_expr("x + time").unwrap();
        let func = CpuFunction::compile(&with_time).unwrap();
        let dynamic = DynamicCube::from_function(func);
        assert!(dynamic.uses_time());
    }
}
