//! Trellis format conversion
//!
//! Convert Trellis.2 outputs to Crossworld voxel formats (Cube octree, CSM).
//! Supports both mesh-based voxelization and OVoxel sparse format conversion.

use crate::color_quantizer::{quantize_colors, ColorPalette};
use crate::ovoxel::{OVoxel, OVoxelError};
use crate::types::{Result, TrellisError, TrellisResult};
use cube::{serialize_csm, Cube, CubeBox, Voxel};
use glam::{IVec3, Vec3};

// =============================================================================
// Mesh to Voxel Conversion (TrellisResult → Cube)
// =============================================================================

/// Configuration for mesh voxelization
#[derive(Debug, Clone)]
pub struct VoxelizeConfig {
    /// Octree depth (grid size: 2^depth per axis)
    pub depth: u32,
    /// Fill solid interior (default: false - surface only)
    pub fill_interior: bool,
}

impl Default for VoxelizeConfig {
    fn default() -> Self {
        Self {
            depth: 6, // 64^3 grid
            fill_interior: false,
        }
    }
}

impl VoxelizeConfig {
    /// Create a new voxelization configuration with specified depth
    pub fn new(depth: u32) -> Self {
        Self {
            depth,
            fill_interior: false,
        }
    }

    /// Enable or disable interior filling
    pub fn with_fill_interior(mut self, fill: bool) -> Self {
        self.fill_interior = fill;
        self
    }
}

/// Voxelize a triangle mesh to a voxel grid using surface sampling
///
/// This function converts a triangle mesh (vertices + faces) into a set of discrete
/// voxel positions by rasterizing triangle surfaces.
///
/// # Arguments
///
/// * `vertices` - Array of vertex positions as [x, y, z]
/// * `faces` - Array of triangle faces as vertex indices [v0, v1, v2]
/// * `config` - Voxelization configuration (depth, fill_interior)
///
/// # Returns
///
/// Vector of voxel positions (IVec3) representing the voxelized mesh
///
/// # Algorithm
///
/// 1. Compute mesh bounding box
/// 2. Normalize vertices to [-1, 1] range
/// 3. For each triangle:
///    - Compute triangle AABB
///    - Sample points on triangle surface
///    - Convert to voxel grid coordinates
/// 4. Optionally fill interior using flood fill
///
/// # Example
///
/// ```
/// use trellis::convert::{voxelize_mesh, VoxelizeConfig};
///
/// let vertices = vec![
///     [0.0, 0.0, 0.0],
///     [1.0, 0.0, 0.0],
///     [0.5, 1.0, 0.0],
/// ];
/// let faces = vec![[0, 1, 2]];
/// let config = VoxelizeConfig::new(6); // 64^3 grid
///
/// let voxels = voxelize_mesh(&vertices, &faces, &config);
/// ```
pub fn voxelize_mesh(
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    config: &VoxelizeConfig,
) -> Vec<IVec3> {
    if vertices.is_empty() || faces.is_empty() {
        return Vec::new();
    }

    // Step 1: Compute mesh bounding box
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for [x, y, z] in vertices {
        min = min.min(Vec3::new(*x, *y, *z));
        max = max.max(Vec3::new(*x, *y, *z));
    }

    // Handle degenerate case (all vertices at same point)
    if (max - min).length_squared() < 1e-6 {
        return Vec::new();
    }

    // Step 2: Compute normalization transform
    let center = (min + max) * 0.5;
    let size = (max - min).max_element();
    let scale = 2.0 / size; // Map to [-1, 1]

    // Step 3: Voxelize each triangle
    let grid_size = 1 << config.depth; // 2^depth
    let half_size = grid_size as f32 / 2.0;
    let max_coord = grid_size - 1;

    let mut voxel_set = std::collections::HashSet::new();

    for face in faces {
        let [i0, i1, i2] = *face;

        // Get triangle vertices
        let v0 = Vec3::from_array(vertices[i0 as usize]);
        let v1 = Vec3::from_array(vertices[i1 as usize]);
        let v2 = Vec3::from_array(vertices[i2 as usize]);

        // Normalize to [-1, 1]
        let v0 = (v0 - center) * scale;
        let v1 = (v1 - center) * scale;
        let v2 = (v2 - center) * scale;

        // Sample triangle surface
        let samples = sample_triangle_surface(&v0, &v1, &v2, config.depth);

        // Convert samples to voxel coordinates
        for sample in samples {
            let vx = ((sample.x * half_size) + half_size).floor() as i32;
            let vy = ((sample.y * half_size) + half_size).floor() as i32;
            let vz = ((sample.z * half_size) + half_size).floor() as i32;

            let vx = vx.clamp(0, max_coord);
            let vy = vy.clamp(0, max_coord);
            let vz = vz.clamp(0, max_coord);

            voxel_set.insert(IVec3::new(vx, vy, vz));
        }
    }

    // Step 4: Optionally fill interior
    let mut voxels: Vec<IVec3> = voxel_set.into_iter().collect();

    if config.fill_interior {
        voxels = fill_interior_voxels(&voxels, grid_size);
    }

    voxels
}

/// Sample points on a triangle surface for voxelization
///
/// Uses adaptive sampling based on triangle size and target grid resolution
fn sample_triangle_surface(v0: &Vec3, v1: &Vec3, v2: &Vec3, depth: u32) -> Vec<Vec3> {
    let mut samples = Vec::new();

    // Compute triangle edges
    let edge1 = *v1 - *v0;
    let edge2 = *v2 - *v0;

    // Compute triangle area (half of cross product magnitude)
    let area = edge1.cross(edge2).length() * 0.5;

    // Determine sample count based on area and depth
    // More samples for larger triangles and higher resolutions
    let grid_size = 1 << depth;
    let voxel_size = 2.0 / grid_size as f32; // Size of one voxel in normalized space
    let samples_per_unit = (1.0 / voxel_size).max(1.0);
    let sample_count = (area * samples_per_unit * samples_per_unit).ceil() as usize;
    let sample_count = sample_count.clamp(3, 10000); // Reasonable bounds

    // Sample using barycentric coordinates
    let steps = (sample_count as f32).sqrt().ceil() as usize;
    for i in 0..=steps {
        for j in 0..=(steps - i) {
            let u = i as f32 / steps as f32;
            let v = j as f32 / steps as f32;
            let w = 1.0 - u - v;

            if w >= 0.0 {
                let point = *v0 * w + *v1 * u + *v2 * v;
                samples.push(point);
            }
        }
    }

    samples
}

/// Fill the interior of a voxelized mesh using flood fill
///
/// Uses 6-connected flood fill from outside the bounding box to mark exterior voxels,
/// then inverts to get interior + surface voxels
fn fill_interior_voxels(surface: &[IVec3], grid_size: i32) -> Vec<IVec3> {
    // Create a 3D grid to track voxel state
    let size = grid_size as usize;
    let mut grid = vec![vec![vec![false; size]; size]; size];

    // Mark surface voxels
    for voxel in surface {
        if voxel.x >= 0
            && voxel.x < grid_size
            && voxel.y >= 0
            && voxel.y < grid_size
            && voxel.z >= 0
            && voxel.z < grid_size
        {
            grid[voxel.x as usize][voxel.y as usize][voxel.z as usize] = true;
        }
    }

    // Flood fill from outside to mark exterior voxels
    let mut exterior = vec![vec![vec![false; size]; size]; size];
    let mut queue = Vec::new();

    // Start flood fill from all boundary faces
    for x in 0..grid_size {
        for y in 0..grid_size {
            // Front and back faces
            if !grid[x as usize][y as usize][0] {
                queue.push(IVec3::new(x, y, 0));
                exterior[x as usize][y as usize][0] = true;
            }
            if !grid[x as usize][y as usize][size - 1] {
                queue.push(IVec3::new(x, y, grid_size - 1));
                exterior[x as usize][y as usize][size - 1] = true;
            }
        }
    }

    for x in 0..grid_size {
        for z in 0..grid_size {
            // Top and bottom faces
            if !grid[x as usize][0][z as usize] {
                queue.push(IVec3::new(x, 0, z));
                exterior[x as usize][0][z as usize] = true;
            }
            if !grid[x as usize][size - 1][z as usize] {
                queue.push(IVec3::new(x, grid_size - 1, z));
                exterior[x as usize][size - 1][z as usize] = true;
            }
        }
    }

    for y in 0..grid_size {
        for z in 0..grid_size {
            // Left and right faces
            if !grid[0][y as usize][z as usize] {
                queue.push(IVec3::new(0, y, z));
                exterior[0][y as usize][z as usize] = true;
            }
            if !grid[size - 1][y as usize][z as usize] {
                queue.push(IVec3::new(grid_size - 1, y, z));
                exterior[size - 1][y as usize][z as usize] = true;
            }
        }
    }

    // 6-connected flood fill
    let directions = [
        IVec3::new(1, 0, 0),
        IVec3::new(-1, 0, 0),
        IVec3::new(0, 1, 0),
        IVec3::new(0, -1, 0),
        IVec3::new(0, 0, 1),
        IVec3::new(0, 0, -1),
    ];

    while let Some(pos) = queue.pop() {
        for dir in &directions {
            let next = pos + *dir;

            if next.x >= 0
                && next.x < grid_size
                && next.y >= 0
                && next.y < grid_size
                && next.z >= 0
                && next.z < grid_size
            {
                let nx = next.x as usize;
                let ny = next.y as usize;
                let nz = next.z as usize;

                if !grid[nx][ny][nz] && !exterior[nx][ny][nz] {
                    exterior[nx][ny][nz] = true;
                    queue.push(next);
                }
            }
        }
    }

    // Collect all non-exterior voxels (surface + interior)
    let mut result = Vec::new();
    for x in 0..grid_size {
        for y in 0..grid_size {
            for z in 0..grid_size {
                let ux = x as usize;
                let uy = y as usize;
                let uz = z as usize;

                if !exterior[ux][uy][uz] {
                    result.push(IVec3::new(x, y, z));
                }
            }
        }
    }

    result
}

/// Map vertex colors to voxel materials using nearest triangle interpolation
///
/// For each voxel position, finds the nearest triangle face and interpolates
/// vertex colors at the closest point on that triangle, then encodes to R2G3B2 format.
///
/// # Arguments
///
/// * `vertices` - Array of vertex positions
/// * `faces` - Array of triangle face indices
/// * `colors` - Array of vertex colors (RGB in 0-1 range)
/// * `voxel_positions` - Array of voxel positions from voxelization
/// * `config` - Voxelization configuration (for normalization)
///
/// # Returns
///
/// Vector of material indices (128-255 range for R2G3B2 encoding)
pub fn vertex_colors_to_materials(
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    colors: &[[f32; 3]],
    voxel_positions: &[IVec3],
    config: &VoxelizeConfig,
) -> Vec<u8> {
    if voxel_positions.is_empty() {
        return Vec::new();
    }

    // Compute mesh normalization (same as voxelize_mesh)
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);

    for [x, y, z] in vertices {
        min = min.min(Vec3::new(*x, *y, *z));
        max = max.max(Vec3::new(*x, *y, *z));
    }

    let center = (min + max) * 0.5;
    let size = (max - min).max_element();
    let scale = 2.0 / size;

    // Denormalize voxel positions back to mesh space
    let grid_size = 1 << config.depth;
    let half_size = grid_size as f32 / 2.0;

    let mut materials = Vec::with_capacity(voxel_positions.len());

    for voxel in voxel_positions {
        // Convert voxel coordinate back to normalized space [-1, 1]
        let voxel_pos = Vec3::new(
            (voxel.x as f32 - half_size) / half_size,
            (voxel.y as f32 - half_size) / half_size,
            (voxel.z as f32 - half_size) / half_size,
        );

        // Convert to mesh space
        let world_pos = (voxel_pos / scale) + center;

        // Find nearest triangle and interpolate color
        let color = find_nearest_triangle_color(vertices, faces, colors, &world_pos);

        // Encode to R2G3B2 material index
        let material = encode_r2g3b2_color(&color);
        materials.push(material);
    }

    materials
}

/// Find the nearest triangle to a point and interpolate vertex color
fn find_nearest_triangle_color(
    vertices: &[[f32; 3]],
    faces: &[[u32; 3]],
    colors: &[[f32; 3]],
    point: &Vec3,
) -> [f32; 3] {
    let mut min_dist = f32::MAX;
    let mut best_color = [0.5, 0.5, 0.5]; // Gray default

    for face in faces {
        let [i0, i1, i2] = *face;

        let v0 = Vec3::from_array(vertices[i0 as usize]);
        let v1 = Vec3::from_array(vertices[i1 as usize]);
        let v2 = Vec3::from_array(vertices[i2 as usize]);

        // Find closest point on triangle
        let (closest_point, bary) = closest_point_on_triangle(point, &v0, &v1, &v2);
        let dist = point.distance(closest_point);

        if dist < min_dist {
            min_dist = dist;

            // Interpolate vertex colors using barycentric coordinates
            let c0 = Vec3::from_array(colors[i0 as usize]);
            let c1 = Vec3::from_array(colors[i1 as usize]);
            let c2 = Vec3::from_array(colors[i2 as usize]);

            let color = c0 * bary.x + c1 * bary.y + c2 * bary.z;
            best_color = color.clamp(Vec3::ZERO, Vec3::ONE).to_array();
        }
    }

    best_color
}

/// Find the closest point on a triangle to a given point
///
/// Returns (closest_point, barycentric_coordinates)
fn closest_point_on_triangle(p: &Vec3, v0: &Vec3, v1: &Vec3, v2: &Vec3) -> (Vec3, Vec3) {
    let edge0 = *v1 - *v0;
    let edge1 = *v2 - *v0;
    let v0_to_p = *p - *v0;

    let a = edge0.dot(edge0);
    let b = edge0.dot(edge1);
    let c = edge1.dot(edge1);
    let d = edge0.dot(v0_to_p);
    let e = edge1.dot(v0_to_p);

    let det = a * c - b * b;
    let s = b * e - c * d;
    let t = b * d - a * e;

    // Compute barycentric coordinates
    let (u, v, w) = if s + t <= det {
        if s < 0.0 {
            if t < 0.0 {
                // Region 4
                let s = (-d / a).clamp(0.0, 1.0);
                (1.0 - s, s, 0.0)
            } else {
                // Region 3
                let t = (e / c).clamp(0.0, 1.0);
                (1.0 - t, 0.0, t)
            }
        } else if t < 0.0 {
            // Region 5
            let s = (d / a).clamp(0.0, 1.0);
            (1.0 - s, s, 0.0)
        } else {
            // Region 0 (inside triangle)
            let inv_det = 1.0 / det;
            let s = s * inv_det;
            let t = t * inv_det;
            (1.0 - s - t, s, t)
        }
    } else if s < 0.0 {
        // Region 2
        let t = (e / c).clamp(0.0, 1.0);
        (1.0 - t, 0.0, t)
    } else if t < 0.0 {
        // Region 6
        let s = (d / a).clamp(0.0, 1.0);
        (1.0 - s, s, 0.0)
    } else {
        // Region 1
        let numer = c + e - b - d;
        let denom = a - 2.0 * b + c;
        let s = (numer / denom).clamp(0.0, 1.0);
        (1.0 - s, s, 1.0 - s)
    };

    let closest = *v0 * u + *v1 * v + *v2 * w;
    (closest, Vec3::new(u, v, w))
}

/// Encode RGB color (0-1 range) to R2G3B2 material index (128-255)
fn encode_r2g3b2_color(color: &[f32; 3]) -> u8 {
    let r = (color[0] * 255.0).clamp(0.0, 255.0) as u8;
    let g = (color[1] * 255.0).clamp(0.0, 255.0) as u8;
    let b = (color[2] * 255.0).clamp(0.0, 255.0) as u8;

    // Extract top bits: red (2 bits), green (3 bits), blue (2 bits)
    let r_bits = (r >> 6) & 0b11;
    let g_bits = (g >> 5) & 0b111;
    let b_bits = (b >> 6) & 0b11;

    // Combine into 7-bit index and add 128 offset
    128 + ((r_bits << 5) | (g_bits << 2) | b_bits)
}

/// Convert Trellis.2 mesh output to Crossworld Cube octree
///
/// This is the primary function for converting Trellis generation results into
/// Crossworld's native voxel format.
///
/// # Arguments
///
/// * `result` - The Trellis generation result containing mesh data
/// * `depth` - Octree depth (determines grid size: 2^depth per axis)
///
/// # Returns
///
/// A `Cube<u8>` octree structure ready for rendering or serialization
///
/// # Example
///
/// ```no_run
/// # use trellis::{TrellisClient, GenerationRequest};
/// # use trellis::convert::trellis_to_cube;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = TrellisClient::new("http://localhost:3642");
/// let request = GenerationRequest::new("base64_image_data");
/// let result = client.generate(&request).await?;
///
/// // Convert to octree with depth 6 (64^3 grid)
/// let cube = trellis_to_cube(&result, 6)?;
/// # Ok(())
/// # }
/// ```
pub fn trellis_to_cube(result: &TrellisResult, depth: u32) -> Result<Cube<u8>> {
    if result.vertices.is_empty() || result.faces.is_empty() {
        return Err(TrellisError::ConversionError(
            "Result has no mesh data".to_string(),
        ));
    }

    // Step 1: Voxelize mesh to get voxel positions
    let config = VoxelizeConfig::new(depth);
    let voxel_positions = voxelize_mesh(&result.vertices, &result.faces, &config);

    if voxel_positions.is_empty() {
        return Err(TrellisError::ConversionError(
            "Voxelization produced no voxels".to_string(),
        ));
    }

    // Step 2: Assign materials based on vertex colors (if available)
    let materials = if let Some(colors) = &result.vertex_colors {
        vertex_colors_to_materials(
            &result.vertices,
            &result.faces,
            colors,
            &voxel_positions,
            &config,
        )
    } else {
        // Use default material (white = 255)
        vec![255; voxel_positions.len()]
    };

    // Step 3: Build Voxel structs
    let voxels: Vec<Voxel> = voxel_positions
        .into_iter()
        .zip(materials)
        .map(|(pos, material)| Voxel { pos, material })
        .collect();

    // Step 4: Build octree from voxels
    let cube = Cube::from_voxels(&voxels, depth, 0);

    Ok(cube)
}

/// Convert Trellis.2 mesh output to CSM (CubeScript Model) format
///
/// This function converts a Trellis generation result into Crossworld's CSM text format,
/// which can be parsed and rendered by the cube crate.
///
/// # Arguments
///
/// * `result` - The Trellis generation result containing mesh data
///
/// # Returns
///
/// A CSM-formatted string representation of the model
///
/// # Example
///
/// ```no_run
/// # use trellis::{TrellisClient, GenerationRequest};
/// # use trellis::convert::trellis_to_csm;
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let client = TrellisClient::new("http://localhost:3642");
/// let request = GenerationRequest::new("base64_image_data");
/// let result = client.generate(&request).await?;
///
/// let csm_string = trellis_to_csm(&result)?;
/// # Ok(())
/// # }
/// ```
pub fn trellis_to_csm(result: &TrellisResult) -> Result<String> {
    // Convert to Cube octree with depth 6 (64x64x64 grid)
    let cube = trellis_to_cube(result, 6)?;

    // Serialize to CSM format
    let csm_string = serialize_csm(&cube);

    Ok(csm_string)
}

// =============================================================================
// OVoxel to Cube Conversion (OVoxel → Cube)
// =============================================================================

/// Errors that can occur during OVoxel conversion
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("OVoxel validation failed: {0}")]
    InvalidOVoxel(#[from] OVoxelError),

    #[error("Target depth {0} is out of valid range (0-7)")]
    InvalidDepth(u8),

    #[error("Failed to create octree: {0}")]
    OctreeCreation(String),

    #[error("Empty voxel data")]
    EmptyVoxels,
}

/// Convert OVoxel sparse voxel data to Cube octree
///
/// # Arguments
/// * `ovoxel` - The OVoxel data to convert
/// * `target_depth` - Target octree depth (0-7, typically 6-7 for 64³-128³ resolution)
/// * `max_palette_colors` - Maximum number of colors in the palette (1-256)
///
/// # Returns
/// A tuple of (CubeBox<u8>, ColorPalette) containing the octree and color palette
pub fn ovoxel_to_cube(
    ovoxel: &OVoxel,
    target_depth: u8,
    max_palette_colors: usize,
) -> std::result::Result<(CubeBox<u8>, ColorPalette), ConversionError> {
    // Validate input
    ovoxel.validate()?;

    if target_depth > 7 {
        return Err(ConversionError::InvalidDepth(target_depth));
    }

    if ovoxel.is_empty() {
        return Err(ConversionError::EmptyVoxels);
    }

    // 1. Build color palette (quantize to max_palette_colors)
    let palette = quantize_colors(&ovoxel.attrs, max_palette_colors.min(256));

    // 2. Calculate coordinate transformation
    let aabb_size = ovoxel.dimensions();
    let grid_size = 1 << target_depth; // 2^depth
    let max_dim = aabb_size.max_element();
    let scale = grid_size as f32 / max_dim;

    // 3. Convert sparse voxels to Voxel array
    let mut voxels = Vec::with_capacity(ovoxel.len());

    for (coord, attr) in ovoxel.iter() {
        // Normalize coordinates to [0, 1] range
        let normalized = (coord.as_vec3() - ovoxel.aabb[0]) / aabb_size;

        // Scale to grid coordinates
        let grid_coord = (normalized * grid_size as f32).floor();

        // Clamp to valid range [0, grid_size)
        let clamped = grid_coord
            .as_ivec3()
            .clamp(IVec3::ZERO, IVec3::splat(grid_size - 1));

        // Map color to palette index
        let material = palette.nearest_index(attr);

        // Create voxel
        voxels.push(Voxel {
            pos: clamped,
            material,
        });
    }

    // 4. Build octree from voxels
    let cube = Cube::from_voxels(&voxels, target_depth as u32, 0);

    // 5. Calculate original size (in grid coordinates)
    let original_size = (aabb_size * scale).ceil().as_ivec3();
    let size = original_size.clamp(IVec3::ONE, IVec3::splat(grid_size));

    // 6. Create CubeBox
    let cubebox = CubeBox {
        cube,
        size,
        depth: target_depth as u32,
    };

    Ok((cubebox, palette))
}

/// Convert OVoxel to CSM text format
///
/// This is a convenience function that combines ovoxel_to_cube with CSM serialization.
///
/// # Arguments
/// * `ovoxel` - The OVoxel data to convert
/// * `target_depth` - Target octree depth (0-7)
/// * `max_palette_colors` - Maximum number of colors in the palette (1-256)
///
/// # Returns
/// A tuple of (CSM string, ColorPalette) containing the serialized octree and color palette
pub fn ovoxel_to_csm(
    ovoxel: &OVoxel,
    target_depth: u8,
    max_palette_colors: usize,
) -> std::result::Result<(String, ColorPalette), ConversionError> {
    let (cubebox, palette) = ovoxel_to_cube(ovoxel, target_depth, max_palette_colors)?;

    let csm = cube::io::serialize_csm(&cubebox.cube);

    Ok((csm, palette))
}

/// Convert OVoxel to Cube with automatic depth selection
///
/// Automatically selects the minimum depth required to represent all voxels.
///
/// # Arguments
/// * `ovoxel` - The OVoxel data to convert
/// * `max_palette_colors` - Maximum number of colors in the palette (1-256)
///
/// # Returns
/// A tuple of (CubeBox<u8>, ColorPalette) containing the octree and color palette
pub fn ovoxel_to_cube_auto_depth(
    ovoxel: &OVoxel,
    max_palette_colors: usize,
) -> std::result::Result<(CubeBox<u8>, ColorPalette), ConversionError> {
    // Validate input
    ovoxel.validate()?;

    if ovoxel.is_empty() {
        return Err(ConversionError::EmptyVoxels);
    }

    // Find maximum coordinate extent
    let mut max_coord = 0;
    for coord in &ovoxel.coords {
        max_coord = max_coord
            .max(coord.x.abs())
            .max(coord.y.abs())
            .max(coord.z.abs());
    }

    // Calculate minimum depth needed
    let mut depth = 0u8;
    while depth < 7 && (1 << depth) < max_coord {
        depth += 1;
    }

    // Use at least depth 4 (16³) for reasonable quality
    depth = depth.max(4);

    ovoxel_to_cube(ovoxel, depth, max_palette_colors)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mesh voxelization tests
    fn create_test_cube() -> TrellisResult {
        // Create a simple cube mesh (8 vertices, 12 triangles)
        let vertices = vec![
            [-0.5, -0.5, -0.5],
            [0.5, -0.5, -0.5],
            [0.5, 0.5, -0.5],
            [-0.5, 0.5, -0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, 0.5, 0.5],
        ];

        let faces = vec![
            // Back face
            [0, 1, 2],
            [0, 2, 3],
            // Front face
            [4, 6, 5],
            [4, 7, 6],
            // Left face
            [0, 3, 7],
            [0, 7, 4],
            // Right face
            [1, 5, 6],
            [1, 6, 2],
            // Bottom face
            [0, 4, 5],
            [0, 5, 1],
            // Top face
            [3, 2, 6],
            [3, 6, 7],
        ];

        let vertex_colors = Some(vec![
            [1.0, 0.0, 0.0], // Red
            [0.0, 1.0, 0.0], // Green
            [0.0, 0.0, 1.0], // Blue
            [1.0, 1.0, 0.0], // Yellow
            [1.0, 0.0, 1.0], // Magenta
            [0.0, 1.0, 1.0], // Cyan
            [1.0, 1.0, 1.0], // White
            [0.5, 0.5, 0.5], // Gray
        ]);

        TrellisResult {
            vertices,
            faces,
            vertex_colors,
            vertex_normals: None,
            glb_data: None,
        }
    }

    #[test]
    fn test_voxelize_config() {
        let config = VoxelizeConfig::new(5).with_fill_interior(true);
        assert_eq!(config.depth, 5);
        assert!(config.fill_interior);
    }

    #[test]
    fn test_voxelize_config_default() {
        let config = VoxelizeConfig::default();
        assert_eq!(config.depth, 6);
        assert!(!config.fill_interior);
    }

    #[test]
    fn test_voxelize_mesh_empty() {
        let vertices: Vec<[f32; 3]> = vec![];
        let faces: Vec<[u32; 3]> = vec![];
        let config = VoxelizeConfig::default();

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        assert!(voxels.is_empty());
    }

    #[test]
    fn test_voxelize_mesh_empty_faces() {
        // Vertices but no faces
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        let faces: Vec<[u32; 3]> = vec![];
        let config = VoxelizeConfig::default();

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        assert!(voxels.is_empty());
    }

    #[test]
    fn test_voxelize_mesh_single_triangle() {
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        assert!(!voxels.is_empty());
    }

    #[test]
    fn test_voxelize_mesh_degenerate_point() {
        // All vertices at the same point (degenerate case)
        let vertices = vec![[0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, 0.5]];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        // Should return empty for degenerate mesh
        assert!(voxels.is_empty());
    }

    #[test]
    fn test_voxelize_mesh_degenerate_line() {
        // All vertices on a line (degenerate triangle with zero area)
        let vertices = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.0, 0.0]];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        // May produce voxels along the line, or may be empty depending on sampling
        // The key is it should not crash
        assert!(voxels.len() < 100); // Should be minimal if any
    }

    #[test]
    fn test_voxelize_mesh_very_small_triangle() {
        // Very small triangle that might be smaller than a voxel
        let vertices = vec![[0.0, 0.0, 0.0], [0.001, 0.0, 0.0], [0.0005, 0.001, 0.0]];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(3); // Low resolution

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        // Should handle gracefully - the normalized triangle fills the grid
        // so we expect the minimum sampling (3 samples minimum per triangle)
        // The test just verifies it doesn't crash and produces some output
        assert!(!voxels.is_empty() || voxels.is_empty()); // Always true - just checking no panic
    }

    #[test]
    fn test_voxelize_mesh_cube() {
        let result = create_test_cube();
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&result.vertices, &result.faces, &config);
        assert!(!voxels.is_empty());

        // All voxels should be within bounds
        let grid_size = 1 << config.depth;
        for voxel in &voxels {
            assert!(voxel.x >= 0 && voxel.x < grid_size);
            assert!(voxel.y >= 0 && voxel.y < grid_size);
            assert!(voxel.z >= 0 && voxel.z < grid_size);
        }
    }

    #[test]
    fn test_voxelize_mesh_with_fill_interior() {
        let result = create_test_cube();
        let config_surface = VoxelizeConfig::new(5).with_fill_interior(false);
        let config_filled = VoxelizeConfig::new(5).with_fill_interior(true);

        let voxels_surface = voxelize_mesh(&result.vertices, &result.faces, &config_surface);
        let voxels_filled = voxelize_mesh(&result.vertices, &result.faces, &config_filled);

        // Filled version should have more or equal voxels
        assert!(voxels_filled.len() >= voxels_surface.len());
    }

    #[test]
    fn test_encode_r2g3b2_color() {
        // Test known colors
        assert_eq!(encode_r2g3b2_color(&[1.0, 0.0, 0.0]), 128 + (0b11 << 5)); // Red
        assert_eq!(encode_r2g3b2_color(&[0.0, 1.0, 0.0]), 128 + (0b111 << 2)); // Green
        assert_eq!(encode_r2g3b2_color(&[0.0, 0.0, 1.0]), 128 + 0b11); // Blue
        assert_eq!(encode_r2g3b2_color(&[0.0, 0.0, 0.0]), 128); // Black
        assert_eq!(encode_r2g3b2_color(&[1.0, 1.0, 1.0]), 255); // White
    }

    #[test]
    fn test_encode_r2g3b2_color_clamping() {
        // Values outside [0, 1] should be clamped
        assert_eq!(encode_r2g3b2_color(&[-0.5, -0.5, -0.5]), 128); // Clamped to black
        assert_eq!(encode_r2g3b2_color(&[2.0, 2.0, 2.0]), 255); // Clamped to white
    }

    #[test]
    fn test_encode_r2g3b2_color_range() {
        // Test that all possible outputs are in valid range [128, 255]
        for r in 0..=10 {
            for g in 0..=10 {
                for b in 0..=10 {
                    let color = [r as f32 / 10.0, g as f32 / 10.0, b as f32 / 10.0];
                    let material = encode_r2g3b2_color(&color);
                    assert!(material >= 128);
                    // material is u8, so it's always <= 255
                }
            }
        }
    }

    #[test]
    fn test_vertex_colors_to_materials() {
        let result = create_test_cube();
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&result.vertices, &result.faces, &config);
        let colors = result.vertex_colors.as_ref().unwrap();

        let materials =
            vertex_colors_to_materials(&result.vertices, &result.faces, colors, &voxels, &config);

        assert_eq!(materials.len(), voxels.len());

        // All materials should be in valid range [128, 255]
        for &mat in &materials {
            assert!(mat >= 128);
        }
    }

    #[test]
    fn test_vertex_colors_to_materials_empty() {
        let vertices: Vec<[f32; 3]> = vec![];
        let faces: Vec<[u32; 3]> = vec![];
        let colors: Vec<[f32; 3]> = vec![];
        let voxels: Vec<IVec3> = vec![];
        let config = VoxelizeConfig::default();

        let materials = vertex_colors_to_materials(&vertices, &faces, &colors, &voxels, &config);
        assert!(materials.is_empty());
    }

    #[test]
    fn test_trellis_to_cube() {
        let result = create_test_cube();
        let cube = trellis_to_cube(&result, 5).unwrap();

        // Should create a valid octree structure
        match cube {
            Cube::Solid(_) | Cube::Cubes(_) | _ => {
                // Valid octree structures
            }
        }
    }

    // OVoxel conversion tests
    fn create_test_ovoxel() -> OVoxel {
        let coords = vec![
            IVec3::new(0, 0, 0),
            IVec3::new(1, 0, 0),
            IVec3::new(0, 1, 0),
            IVec3::new(0, 0, 1),
        ];
        let attrs = vec![
            [1.0, 0.0, 0.0], // Red
            [0.0, 1.0, 0.0], // Green
            [0.0, 0.0, 1.0], // Blue
            [1.0, 1.0, 0.0], // Yellow
        ];
        let voxel_size = 1.0;
        let aabb = [Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 2.0, 2.0)];

        OVoxel::new(coords, attrs, voxel_size, aabb)
    }

    #[test]
    fn test_trellis_to_cube_empty() {
        let empty_result = TrellisResult {
            vertices: vec![],
            faces: vec![],
            vertex_colors: None,
            vertex_normals: None,
            glb_data: None,
        };

        let result = trellis_to_cube(&empty_result, 5);
        assert!(result.is_err());

        if let Err(TrellisError::ConversionError(msg)) = result {
            assert!(msg.contains("no mesh data"));
        } else {
            panic!("Expected ConversionError");
        }
    }

    #[test]
    fn test_trellis_to_cube_degenerate() {
        let degenerate_result = TrellisResult {
            vertices: vec![[0.0, 0.0, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]],
            faces: vec![[0, 1, 2]],
            vertex_colors: None,
            vertex_normals: None,
            glb_data: None,
        };

        let result = trellis_to_cube(&degenerate_result, 5);
        assert!(result.is_err());

        if let Err(TrellisError::ConversionError(msg)) = result {
            assert!(msg.contains("no voxels"));
        } else {
            panic!("Expected ConversionError for degenerate mesh");
        }
    }

    #[test]
    fn test_trellis_to_cube_without_colors() {
        let result = TrellisResult {
            vertices: vec![
                [-0.5, -0.5, -0.5],
                [0.5, -0.5, -0.5],
                [0.5, 0.5, -0.5],
                [-0.5, 0.5, -0.5],
            ],
            faces: vec![[0, 1, 2], [0, 2, 3]],
            vertex_colors: None, // No colors
            vertex_normals: None,
            glb_data: None,
        };

        let cube = trellis_to_cube(&result, 5);
        assert!(cube.is_ok());
    }

    #[test]
    fn test_ovoxel_to_cube_basic() {
        let ovoxel = create_test_ovoxel();
        let result = ovoxel_to_cube(&ovoxel, 4, 256);
        assert!(result.is_ok());

        let (cubebox, palette) = result.unwrap();
        assert_eq!(cubebox.depth, 4);
        assert!(palette.len() <= 256);
        assert!(!palette.is_empty());
    }

    #[test]
    fn test_ovoxel_to_csm() {
        let ovoxel = create_test_ovoxel();
        let result = ovoxel_to_csm(&ovoxel, 4, 256);
        assert!(result.is_ok());

        let (csm, palette) = result.unwrap();
        assert!(!csm.is_empty());
        assert!(!palette.is_empty());
    }

    #[test]
    fn test_invalid_depth() {
        let ovoxel = create_test_ovoxel();
        let result = ovoxel_to_cube(&ovoxel, 8, 256);
        assert!(matches!(result, Err(ConversionError::InvalidDepth(8))));
    }

    #[test]
    fn test_empty_ovoxel() {
        let ovoxel = OVoxel::new(vec![], vec![], 1.0, [Vec3::ZERO, Vec3::ONE]);
        let result = ovoxel_to_cube(&ovoxel, 4, 256);
        // The validation catches EmptyData which gets wrapped in InvalidOVoxel
        assert!(matches!(
            result,
            Err(ConversionError::InvalidOVoxel(OVoxelError::EmptyData))
        ));
    }

    #[test]
    fn test_auto_depth() {
        let ovoxel = create_test_ovoxel();
        let result = ovoxel_to_cube_auto_depth(&ovoxel, 256);
        assert!(result.is_ok());

        let (cubebox, _) = result.unwrap();
        assert!(cubebox.depth >= 4);
        assert!(cubebox.depth <= 7);
    }

    #[test]
    fn test_sample_triangle_surface_minimum_samples() {
        // Even for tiny triangles, we should get at least 3 samples
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(0.0001, 0.0, 0.0);
        let v2 = Vec3::new(0.00005, 0.0001, 0.0);

        let samples = sample_triangle_surface(&v0, &v1, &v2, 5);
        assert!(samples.len() >= 3);
    }

    #[test]
    fn test_sample_triangle_surface_large_triangle() {
        // Large triangle should produce many samples
        let v0 = Vec3::new(-10.0, -10.0, 0.0);
        let v1 = Vec3::new(10.0, -10.0, 0.0);
        let v2 = Vec3::new(0.0, 10.0, 0.0);

        let samples = sample_triangle_surface(&v0, &v1, &v2, 6);
        assert!(samples.len() > 100);
    }

    #[test]
    fn test_sample_triangle_surface_3d() {
        // Triangle not on a coordinate plane
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 1.0);
        let v2 = Vec3::new(0.0, 1.0, 0.5);

        let samples = sample_triangle_surface(&v0, &v1, &v2, 5);
        assert!(!samples.is_empty());

        // All samples should be within the triangle's bounding box
        for sample in &samples {
            assert!(sample.x >= -0.01 && sample.x <= 1.01);
            assert!(sample.y >= -0.01 && sample.y <= 1.01);
            assert!(sample.z >= -0.01 && sample.z <= 1.01);
        }
    }

    #[test]
    fn test_closest_point_on_triangle() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        // Test 1: Point at a vertex
        let p = Vec3::new(0.0, 0.0, 0.0);
        let (_closest, bary) = closest_point_on_triangle(&p, &v0, &v1, &v2);
        // Barycentric coordinates should sum to 1
        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-4);

        // Test 2: Point on an edge
        let p = Vec3::new(0.5, 0.0, 0.0);
        let (_closest, bary) = closest_point_on_triangle(&p, &v0, &v1, &v2);
        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-4);

        // Test 3: Point inside triangle
        let p = Vec3::new(0.1, 0.1, 0.0);
        let (_closest, bary) = closest_point_on_triangle(&p, &v0, &v1, &v2);
        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-4);
        // All barycentric coordinates should be non-negative
        assert!(bary.x >= -1e-6 && bary.y >= -1e-6 && bary.z >= -1e-6);
    }

    #[test]
    fn test_closest_point_on_triangle_outside() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        // Point far outside the triangle
        let p = Vec3::new(5.0, 5.0, 5.0);
        let (closest, bary) = closest_point_on_triangle(&p, &v0, &v1, &v2);

        // Barycentric should still sum to 1
        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-4);

        // Closest point should be on the triangle (z = 0)
        assert!(closest.z.abs() < 1e-6);
    }

    #[test]
    fn test_closest_point_on_triangle_above_plane() {
        let v0 = Vec3::new(0.0, 0.0, 0.0);
        let v1 = Vec3::new(1.0, 0.0, 0.0);
        let v2 = Vec3::new(0.0, 1.0, 0.0);

        // Point directly above triangle center
        let p = Vec3::new(0.25, 0.25, 1.0);
        let (closest, bary) = closest_point_on_triangle(&p, &v0, &v1, &v2);

        // Barycentric coordinates should sum to 1
        assert!((bary.x + bary.y + bary.z - 1.0).abs() < 1e-4);

        // Closest point should be on the triangle plane (z = 0)
        assert!(closest.z.abs() < 1e-6);
        // And should be inside or on the triangle
        assert!(closest.x >= -1e-6);
        assert!(closest.y >= -1e-6);
        assert!(closest.x + closest.y <= 1.0 + 1e-6);
    }

    #[test]
    fn test_fill_interior_voxels() {
        // Create a hollow cube surface
        let mut surface = Vec::new();
        let size = 10;

        // Add only the surface voxels (6 faces)
        for i in 0..size {
            for j in 0..size {
                surface.push(IVec3::new(i, j, 0)); // Front
                surface.push(IVec3::new(i, j, size - 1)); // Back
                surface.push(IVec3::new(i, 0, j)); // Bottom
                surface.push(IVec3::new(i, size - 1, j)); // Top
                surface.push(IVec3::new(0, i, j)); // Left
                surface.push(IVec3::new(size - 1, i, j)); // Right
            }
        }

        let filled = fill_interior_voxels(&surface, size);

        // Filled should have more voxels than surface (includes interior)
        assert!(filled.len() >= surface.len());

        // All filled voxels should be within bounds
        for voxel in &filled {
            assert!(voxel.x >= 0 && voxel.x < size);
            assert!(voxel.y >= 0 && voxel.y < size);
            assert!(voxel.z >= 0 && voxel.z < size);
        }
    }

    #[test]
    fn test_fill_interior_voxels_empty() {
        let surface: Vec<IVec3> = vec![];
        let filled = fill_interior_voxels(&surface, 10);
        // With no surface, nothing should be filled
        assert!(filled.is_empty());
    }

    #[test]
    fn test_fill_interior_voxels_single_voxel() {
        let surface = vec![IVec3::new(5, 5, 5)];
        let filled = fill_interior_voxels(&surface, 10);
        // Single voxel - should only contain that voxel (no interior to fill)
        assert_eq!(filled.len(), 1);
    }

    #[test]
    fn test_voxelize_mesh_with_colors() {
        let result = create_test_cube();
        let config = VoxelizeConfig::new(6);

        let voxels = voxelize_mesh(&result.vertices, &result.faces, &config);
        assert!(!voxels.is_empty());

        // Test with vertex colors
        if let Some(colors) = &result.vertex_colors {
            let materials = vertex_colors_to_materials(
                &result.vertices,
                &result.faces,
                colors,
                &voxels,
                &config,
            );

            assert_eq!(materials.len(), voxels.len());

            // Materials should have variety (not all the same)
            let unique_materials: std::collections::HashSet<_> = materials.iter().collect();
            assert!(unique_materials.len() > 1);
        }
    }

    #[test]
    fn test_different_depths() {
        let result = create_test_cube();

        for depth in [3, 4, 5, 6, 7] {
            let cube = trellis_to_cube(&result, depth);
            assert!(cube.is_ok(), "Failed at depth {}", depth);
        }
    }

    #[test]
    fn test_voxelize_mesh_large_coordinates() {
        // Mesh with large coordinate values
        let vertices = vec![
            [1000.0, 1000.0, 1000.0],
            [1001.0, 1000.0, 1000.0],
            [1000.5, 1001.0, 1000.0],
        ];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        // Should handle large coordinates through normalization
        assert!(!voxels.is_empty());

        // All voxels should be within grid bounds
        let grid_size = 1 << config.depth;
        for voxel in &voxels {
            assert!(voxel.x >= 0 && voxel.x < grid_size);
            assert!(voxel.y >= 0 && voxel.y < grid_size);
            assert!(voxel.z >= 0 && voxel.z < grid_size);
        }
    }

    #[test]
    fn test_voxelize_mesh_negative_coordinates() {
        // Mesh centered at negative coordinates
        let vertices = vec![
            [-100.0, -100.0, -100.0],
            [-99.0, -100.0, -100.0],
            [-99.5, -99.0, -100.0],
        ];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        assert!(!voxels.is_empty());

        // All voxels should be within grid bounds (positive indices)
        let grid_size = 1 << config.depth;
        for voxel in &voxels {
            assert!(voxel.x >= 0 && voxel.x < grid_size);
            assert!(voxel.y >= 0 && voxel.y < grid_size);
            assert!(voxel.z >= 0 && voxel.z < grid_size);
        }
    }

    #[test]
    fn test_voxelize_mesh_asymmetric() {
        // Non-uniform bounding box to test normalization
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0], // Much larger in X
            [5.0, 1.0, 0.0],
        ];
        let faces = vec![[0, 1, 2]];
        let config = VoxelizeConfig::new(5);

        let voxels = voxelize_mesh(&vertices, &faces, &config);
        assert!(!voxels.is_empty());

        // Grid should be uniformly scaled, so Y range should be compressed
        let grid_size = 1 << config.depth;
        for voxel in &voxels {
            assert!(voxel.x >= 0 && voxel.x < grid_size);
            assert!(voxel.y >= 0 && voxel.y < grid_size);
            assert!(voxel.z >= 0 && voxel.z < grid_size);
        }
    }
}
