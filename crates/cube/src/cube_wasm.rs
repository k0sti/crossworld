use serde::{Deserialize, Serialize};
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use crate::{
    generate_face_mesh,
    glam::{IVec3, Vec3},
    parse_csm, serialize_csm, ColorMapper, Cube, CubeCoord, DefaultMeshBuilder, Octree,
    PaletteColorMapper, VoxColorMapper,
};

// ============================================================================
// Result Types
// ============================================================================

#[derive(Serialize, Deserialize)]
pub struct MeshResult {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
}

#[derive(Serialize, Deserialize)]
pub struct ParseError {
    pub error: String,
}

#[derive(Serialize, Deserialize)]
pub struct RaycastResult {
    /// Octree coordinates of hit voxel
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub depth: u32,
    /// World position of hit (in normalized [0,1] space)
    pub world_x: f32,
    pub world_y: f32,
    pub world_z: f32,
    /// Surface normal at hit point
    pub normal_x: f32,
    pub normal_y: f32,
    pub normal_z: f32,
}

// ============================================================================
// Color Type
// ============================================================================

/// RGB color for palette
#[derive(Serialize, Deserialize, Clone)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

// ============================================================================
// WasmCube - Main Cube Interface
// ============================================================================

/// Immutable hierarchical voxel cube structure
///
/// This is the core data structure for voxel models in the new unified interface.
/// All operations return new cube instances (functional/immutable pattern).
#[wasm_bindgen]
pub struct WasmCube {
    inner: Rc<Cube<i32>>,
}

#[wasm_bindgen]
impl WasmCube {
    /// Create a solid cube with uniform value
    ///
    /// # Arguments
    /// * `value` - Voxel value (-1 = empty, 0+ = color index)
    #[wasm_bindgen(constructor)]
    pub fn new(value: i32) -> Self {
        Self {
            inner: Rc::new(Cube::Solid(value)),
        }
    }

    /// Create a solid cube (alias for constructor)
    pub fn solid(value: i32) -> Self {
        Self::new(value)
    }

    /// Get cube at specific coordinate
    ///
    /// # Arguments
    /// * `x, y, z` - Position coordinates
    /// * `depth` - Depth level
    ///
    /// # Returns
    /// New WasmCube instance representing the cube at that position
    pub fn get(&self, x: i32, y: i32, z: i32, depth: u32) -> WasmCube {
        let coord = CubeCoord::new(IVec3::new(x, y, z), depth);
        let cube = self.inner.get(coord);
        WasmCube {
            inner: Rc::new(cube.clone()),
        }
    }

    /// Update cube at specific coordinate
    ///
    /// Returns a new cube with the specified position updated.
    ///
    /// # Arguments
    /// * `x, y, z` - Position coordinates
    /// * `depth` - Depth level
    /// * `cube` - New cube to place at this position
    ///
    /// # Returns
    /// New WasmCube with the update applied
    pub fn update(&self, x: i32, y: i32, z: i32, depth: u32, cube: &WasmCube) -> WasmCube {
        let coord = CubeCoord::new(IVec3::new(x, y, z), depth);
        let new_cube = self.inner.update(coord, (*cube.inner).clone());
        WasmCube {
            inner: Rc::new(new_cube),
        }
    }

    /// Update this cube with another cube at specified depth and offset, scaled
    ///
    /// This is useful for placing scaled models within a larger cube.
    ///
    /// # Arguments
    /// * `depth` - Depth level at which to place the cube
    /// * `offset_x, offset_y, offset_z` - Position offset
    /// * `scale` - Scale factor (depth levels to scale)
    /// * `cube` - Cube to insert
    ///
    /// # Returns
    /// New WasmCube with the scaled cube inserted
    #[wasm_bindgen(js_name = updateDepth)]
    pub fn update_depth(
        &self,
        depth: u32,
        offset_x: i32,
        offset_y: i32,
        offset_z: i32,
        scale: u32,
        cube: &WasmCube,
    ) -> WasmCube {
        let offset = IVec3::new(offset_x, offset_y, offset_z);
        let new_cube = self
            .inner
            .update_depth(depth, offset, scale, (*cube.inner).clone());
        WasmCube {
            inner: Rc::new(new_cube),
        }
    }

    /// Cast a ray through the cube to find voxel intersections
    ///
    /// # Arguments
    /// * `pos_x, pos_y, pos_z` - Ray origin in normalized [0,1] space
    /// * `dir_x, dir_y, dir_z` - Ray direction (should be normalized)
    /// * `far` - If true, returns the cube coord where ray hit (far side); if false, returns the neighbor coord on near side
    /// * `max_depth` - Maximum octree depth to traverse
    ///
    /// # Returns
    /// RaycastResult or null if no hit
    pub fn raycast(
        &self,
        pos_x: f32,
        pos_y: f32,
        pos_z: f32,
        dir_x: f32,
        dir_y: f32,
        dir_z: f32,
        far: bool,
        max_depth: u32,
    ) -> JsValue {
        let pos = Vec3::new(pos_x, pos_y, pos_z);
        let dir = Vec3::new(dir_x, dir_y, dir_z).normalize();

        // Define empty test: value == -1 (empty) or value == 0 (air)
        let is_empty = |v: &i32| *v == -1 || *v == 0;

        // Perform raycast first
        if let Some(hit) = self.inner.raycast(pos, dir, max_depth, &is_empty) {
            let coord = if far {
                // Return the cube coordinate where the ray hit (far side)
                hit.coord
            } else {
                // Return the cube coordinate on the near side (neighbor in opposite direction of normal)
                let offset = IVec3::new(
                    -hit.normal.x.signum() as i32,
                    -hit.normal.y.signum() as i32,
                    -hit.normal.z.signum() as i32,
                );
                CubeCoord::new(hit.coord.pos + offset, hit.coord.depth)
            };

            let result = RaycastResult {
                x: coord.pos.x,
                y: coord.pos.y,
                z: coord.pos.z,
                depth: coord.depth,
                world_x: hit.position.x,
                world_y: hit.position.y,
                world_z: hit.position.z,
                normal_x: hit.normal.x,
                normal_y: hit.normal.y,
                normal_z: hit.normal.z,
            };
            serde_wasm_bindgen::to_value(&result).unwrap()
        } else {
            JsValue::NULL
        }
    }

    /// Generate mesh geometry from this cube
    ///
    /// # Arguments
    /// * `palette` - Array of {r, g, b} color objects (0.0-1.0 range), or null to use HSV
    /// * `max_depth` - Maximum octree depth (determines unit size: 2^max_depth)
    ///
    /// # Returns
    /// MeshResult with vertices, indices, normals, colors or ParseError
    #[wasm_bindgen(js_name = generateMesh)]
    pub fn generate_mesh(&self, palette: JsValue, max_depth: u32) -> JsValue {
        let octree = Octree::new((*self.inner).clone());
        let mut builder = DefaultMeshBuilder::new();

        // Border materials for avatars: all empty (0)
        let border_materials = [0, 0, 0, 0];

        // Parse palette if provided, otherwise use VoxColorMapper for R2G3B2 decoding
        if palette.is_null() || palette.is_undefined() {
            let mapper = VoxColorMapper::new();
            generate_face_mesh(
                &octree.root,
                &mut builder,
                |v| mapper.map(v),
                max_depth,
                border_materials,
            );
        } else {
            // Try to deserialize palette
            match serde_wasm_bindgen::from_value::<Vec<Color>>(palette) {
                Ok(colors) => {
                    let palette_colors: Vec<[f32; 3]> =
                        colors.iter().map(|c| [c.r, c.g, c.b]).collect();
                    let mapper = PaletteColorMapper::new(palette_colors);
                    generate_face_mesh(
                        &octree.root,
                        &mut builder,
                        |v| mapper.map(v),
                        max_depth,
                        border_materials,
                    );
                }
                Err(e) => {
                    let error = ParseError {
                        error: format!("Invalid palette format: {}", e),
                    };
                    return serde_wasm_bindgen::to_value(&error).unwrap();
                }
            }
        }

        // Scale mesh from [0,1] space to [0, 2^max_depth] world space
        let scale = (1 << max_depth) as f32;
        for i in 0..builder.vertices.len() {
            builder.vertices[i] *= scale;
        }

        let result = MeshResult {
            vertices: builder.vertices,
            indices: builder.indices,
            normals: builder.normals,
            colors: builder.colors,
        };
        serde_wasm_bindgen::to_value(&result).unwrap_or_else(|e| {
            let error = ParseError {
                error: format!("Serialization error: {}", e),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        })
    }

    /// Generate Cubescript model script
    ///
    /// # Arguments
    /// * `optimize` - If true, optimize by finding common subtrees (TODO: not yet implemented)
    ///
    /// # Returns
    /// Cubescript format string
    #[wasm_bindgen(js_name = printScript)]
    pub fn print_script(&self, optimize: bool) -> String {
        let octree = Octree::new((*self.inner).clone());

        if optimize {
            // TODO: Implement optimization
            // For now, just serialize normally and add a comment
            let script = serialize_csm(&octree);
            format!("// Optimized serialization not yet implemented\n{}", script)
        } else {
            serialize_csm(&octree)
        }
    }
}

// ============================================================================
// Standalone Functions
// ============================================================================

/// Load Cubescript (CSM) code into a Cube
///
/// # Arguments
/// * `cubescript` - CSM format text
///
/// # Returns
/// WasmCube on success
///
/// # Errors
/// Throws JS error if parsing fails
#[wasm_bindgen(js_name = loadCsm)]
pub fn load_csm(cubescript: &str) -> Result<WasmCube, JsValue> {
    match parse_csm(cubescript) {
        Ok(octree) => Ok(WasmCube {
            inner: Rc::new(octree.root),
        }),
        Err(e) => {
            let error = ParseError {
                error: format!("Parse error: {}", e),
            };
            Err(serde_wasm_bindgen::to_value(&error).unwrap())
        }
    }
}

/// Validate Cubescript code without creating a cube
///
/// # Arguments
/// * `cubescript` - CSM format text
///
/// # Returns
/// null if valid, ParseError if invalid
#[wasm_bindgen(js_name = validateCsm)]
pub fn validate_csm(cubescript: &str) -> JsValue {
    match parse_csm(cubescript) {
        Ok(_) => JsValue::NULL,
        Err(e) => {
            let error = ParseError {
                error: format!("{}", e),
            };
            serde_wasm_bindgen::to_value(&error).unwrap()
        }
    }
}

/// Load a .vox file from bytes into a WasmCube
///
/// # Arguments
/// * `bytes` - .vox file bytes
/// * `align_x` - X alignment (0.0-1.0, typically 0.5 for center)
/// * `align_y` - Y alignment (0.0-1.0, typically 0.5 for center)
/// * `align_z` - Z alignment (0.0-1.0, typically 0.5 for center)
///
/// # Returns
/// WasmCube on success
///
/// # Errors
/// Throws JS error if loading fails
#[wasm_bindgen(js_name = loadVox)]
pub fn load_vox(
    bytes: &[u8],
    align_x: f32,
    align_y: f32,
    align_z: f32,
) -> Result<WasmCube, JsValue> {
    let align = Vec3::new(align_x, align_y, align_z);
    match crate::load_vox_to_cube(bytes, align) {
        Ok(cube) => Ok(WasmCube {
            inner: Rc::new(cube),
        }),
        Err(e) => {
            let error = ParseError {
                error: format!("Failed to load .vox file: {}", e),
            };
            Err(serde_wasm_bindgen::to_value(&error).unwrap())
        }
    }
}
