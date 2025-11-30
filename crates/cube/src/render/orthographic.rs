use crate::core::{Cube, Octree};
use crate::mesh::ColorMapper;
use glam::IVec3;

/// Type alias for voxel data: (position, size, color)
type VoxelData = ((f32, f32, f32), f32, [u8; 3]);

/// Parameters for 2D cube rendering
struct RenderParams2D<'a> {
    position: (f32, f32, f32),
    size: f32,
    current_depth: usize,
    max_depth: usize,
    direction: ViewDirection,
    image: &'a mut RenderedImage,
    mapper: &'a dyn ColorMapper,
}

/// Parameters for 3D mesh voxel drawing
struct DrawParams3D<'a> {
    position: (f32, f32, f32),
    size: f32,
    color: [u8; 3],
    #[allow(dead_code)]
    voxel_pixel_size: usize,
    direction: ViewDirection,
    image: &'a mut RenderedImage,
    min_bound: (f32, f32, f32),
    max_bound: (f32, f32, f32),
}

/// View direction for orthographic projection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewDirection {
    /// Looking from positive X towards negative X (right to left)
    PosX,
    /// Looking from negative X towards positive X (left to right)
    NegX,
    /// Looking from positive Y towards negative Y (top to bottom)
    PosY,
    /// Looking from negative Y towards positive Y (bottom to top)
    NegY,
    /// Looking from positive Z towards negative Z (front to back)
    PosZ,
    /// Looking from negative Z towards positive Z (back to front)
    NegZ,
}

impl ViewDirection {
    pub fn all() -> [ViewDirection; 6] {
        [
            ViewDirection::PosX,
            ViewDirection::NegX,
            ViewDirection::PosY,
            ViewDirection::NegY,
            ViewDirection::PosZ,
            ViewDirection::NegZ,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ViewDirection::PosX => "pos_x",
            ViewDirection::NegX => "neg_x",
            ViewDirection::PosY => "pos_y",
            ViewDirection::NegY => "neg_y",
            ViewDirection::PosZ => "pos_z",
            ViewDirection::NegZ => "neg_z",
        }
    }
}

/// Rendered image with pixel data
pub struct RenderedImage {
    pub width: usize,
    pub height: usize,
    /// RGB pixels in row-major order
    pub pixels: Vec<[u8; 3]>,
}

impl RenderedImage {
    pub fn new(width: usize, height: usize) -> Self {
        RenderedImage {
            width,
            height,
            pixels: vec![[0, 0, 0]; width * height],
        }
    }

    /// Save image to PNG file (requires "image" feature)
    #[cfg(feature = "image")]
    pub fn save_png(&self, path: &str) -> Result<(), String> {
        use image::{ImageBuffer, Rgb};

        let mut img = ImageBuffer::new(self.width as u32, self.height as u32);
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.pixels[y * self.width + x];
                img.put_pixel(x as u32, y as u32, Rgb(pixel));
            }
        }

        img.save(path)
            .map_err(|e| format!("Failed to save image: {}", e))
    }

    /// Create a side-by-side comparison image (left and right)
    pub fn side_by_side(left: &RenderedImage, right: &RenderedImage) -> Self {
        // Ensure both images have the same height
        let height = left.height.max(right.height);
        let width = left.width + right.width;

        let mut result = RenderedImage::new(width, height);

        // Copy left image
        for y in 0..left.height {
            for x in 0..left.width {
                let src_idx = y * left.width + x;
                let dst_idx = y * width + x;
                if src_idx < left.pixels.len() && dst_idx < result.pixels.len() {
                    result.pixels[dst_idx] = left.pixels[src_idx];
                }
            }
        }

        // Copy right image
        for y in 0..right.height {
            for x in 0..right.width {
                let src_idx = y * right.width + x;
                let dst_idx = y * width + (left.width + x);
                if src_idx < right.pixels.len() && dst_idx < result.pixels.len() {
                    result.pixels[dst_idx] = right.pixels[src_idx];
                }
            }
        }

        result
    }
}

/// Render octree to 2D image from specified view direction (2D rendering)
///
/// This renders at pixel-level resolution where each voxel at max_depth corresponds to 1 pixel.
/// For example, depth 5 produces a 32x32 image (2^5 = 32).
pub fn render_orthographic(
    octree: &Octree,
    direction: ViewDirection,
    max_depth: Option<usize>,
    mapper: &dyn ColorMapper,
) -> RenderedImage {
    render_orthographic_2d(octree, direction, max_depth, mapper)
}

/// Render octree to 2D image from specified view direction (2D rendering - pixel level)
pub fn render_orthographic_2d(
    octree: &Octree,
    direction: ViewDirection,
    max_depth: Option<usize>,
    mapper: &dyn ColorMapper,
) -> RenderedImage {
    // Calculate image size based on max depth
    // Default to depth 8 if not specified (256x256 image)
    let depth = max_depth.unwrap_or(8);
    let size = 1 << depth; // 2^depth

    let mut image = RenderedImage::new(size, size);

    // Render the octree
    render_cube_2d(
        &octree.root,
        RenderParams2D {
            position: (0.0, 0.0, 0.0),
            size: 1.0,
            current_depth: 0,
            max_depth: depth,
            direction,
            image: &mut image,
            mapper,
        },
    );

    image
}

/// Render octree to 2D image from specified view direction (3D rendering with voxel size)
///
/// This uses the mesh generation infrastructure to render voxels as squares of pixels.
/// Each voxel corresponds to a square of 2^voxel_size_log2 pixels.
///
/// For example:
/// - voxel_size_log2=2: Each voxel rendered as 4x4 pixels
/// - voxel_size_log2=3: Each voxel rendered as 8x8 pixels
///
/// The image size is determined by the octree structure and voxel size.
pub fn render_orthographic_3d(
    octree: &Octree,
    direction: ViewDirection,
    voxel_size_log2: usize,
    mapper: &dyn ColorMapper,
) -> RenderedImage {
    // Extract voxels directly from octree (no need to generate full mesh)
    // Use max depth of 16 to ensure we traverse entire octree structure
    let voxels = extract_voxels_from_octree(octree, mapper, 16);

    if voxels.is_empty() {
        return RenderedImage::new(1 << voxel_size_log2, 1 << voxel_size_log2);
    }

    // Calculate bounds to determine image size
    let (min_bound, max_bound) = calculate_bounds(&voxels);

    // Calculate image size based on bounds and voxel size
    let voxel_pixel_size = 1 << voxel_size_log2;
    let (width, height) = calculate_image_size(min_bound, max_bound, direction, voxel_pixel_size);

    let mut image = RenderedImage::new(width, height);

    // Render each voxel cube from the mesh
    for (pos, size, color) in voxels {
        draw_mesh_voxel_3d(DrawParams3D {
            position: pos,
            size,
            color,
            voxel_pixel_size,
            direction,
            image: &mut image,
            min_bound,
            max_bound,
        });
    }

    image
}

/// Recursively render a cube to the image (2D pixel-level rendering)
fn render_cube_2d(cube: &Cube<i32>, params: RenderParams2D) {
    match cube {
        Cube::Solid(value) => {
            if *value != 0 {
                // Draw this voxel to the image
                draw_voxel_2d(
                    params.position,
                    params.size,
                    *value,
                    params.direction,
                    params.image,
                    params.mapper,
                );
            }
        }
        Cube::Cubes(children) if params.current_depth < params.max_depth => {
            // Recursively render children
            let half_size = params.size / 2.0;
            for (idx, child) in children.iter().enumerate() {
                let offset = octant_offset(idx);
                let child_pos = (
                    params.position.0 + offset.0 * params.size,
                    params.position.1 + offset.1 * params.size,
                    params.position.2 + offset.2 * params.size,
                );
                render_cube_2d(
                    child,
                    RenderParams2D {
                        position: child_pos,
                        size: half_size,
                        current_depth: params.current_depth + 1,
                        max_depth: params.max_depth,
                        direction: params.direction,
                        image: params.image,
                        mapper: params.mapper,
                    },
                );
            }
        }
        Cube::Cubes(_) => {
            // Max depth reached, treat as solid
            // Use a default value or skip
        }
        Cube::Planes { .. } | Cube::Slices { .. } => {
            // TODO: Implement rendering for Planes and Slices
        }
    }
}

/// Extract voxel information from mesh data
/// Returns vec of (position, size, color_rgb_u8)
/// Extract voxel data directly from octree
fn extract_voxels_from_octree(
    octree: &Octree,
    mapper: &dyn ColorMapper,
    max_depth: u32,
) -> Vec<VoxelData> {
    let mut voxels = Vec::new();

    // Use visitor pattern to traverse octree and extract voxel data
    octree
        .root
        .visit_leaves(max_depth, IVec3::ZERO, &mut |cube, depth, pos| {
            if let Cube::Solid(value) = cube {
                if *value == 0 {
                    return; // Skip empty voxels
                }

                // Calculate size and position in normalized [0,1] space
                let grid_size = 1 << max_depth; // 2^max_depth
                let voxel_size = 1.0 / grid_size as f32;

                // Scale factor based on remaining depth
                let scale_factor = 1 << depth;

                // Calculate world position
                let x = (pos.x * scale_factor) as f32 * voxel_size;
                let y = (pos.y * scale_factor) as f32 * voxel_size;
                let z = (pos.z * scale_factor) as f32 * voxel_size;
                let size = voxel_size * scale_factor as f32;

                // Get color and convert to u8
                let rgb = mapper.map(*value as u8);
                let color = [
                    (rgb[0] * 255.0) as u8,
                    (rgb[1] * 255.0) as u8,
                    (rgb[2] * 255.0) as u8,
                ];

                voxels.push(((x, y, z), size, color));
            }
        });

    voxels
}

/// Calculate bounding box of voxels
fn calculate_bounds(voxels: &[VoxelData]) -> ((f32, f32, f32), (f32, f32, f32)) {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut min_z = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    let mut max_z = f32::NEG_INFINITY;

    for ((x, y, z), size, _) in voxels {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        min_z = min_z.min(*z);
        max_x = max_x.max(x + size);
        max_y = max_y.max(y + size);
        max_z = max_z.max(z + size);
    }

    ((min_x, min_y, min_z), (max_x, max_y, max_z))
}

/// Calculate image size based on bounds and view direction
fn calculate_image_size(
    min_bound: (f32, f32, f32),
    max_bound: (f32, f32, f32),
    direction: ViewDirection,
    voxel_pixel_size: usize,
) -> (usize, usize) {
    let (min_x, min_y, min_z) = min_bound;
    let (max_x, max_y, max_z) = max_bound;

    let (u_span, v_span) = match direction {
        ViewDirection::PosX | ViewDirection::NegX => {
            // U=Z, V=Y
            (max_z - min_z, max_y - min_y)
        }
        ViewDirection::PosY | ViewDirection::NegY => {
            // U=X, V=Z
            (max_x - min_x, max_z - min_z)
        }
        ViewDirection::PosZ | ViewDirection::NegZ => {
            // U=X, V=Y
            (max_x - min_x, max_y - min_y)
        }
    };

    // Image size = span * voxel_pixel_size
    let width = (u_span * voxel_pixel_size as f32).ceil() as usize;
    let height = (v_span * voxel_pixel_size as f32).ceil() as usize;

    (width.max(1), height.max(1))
}

/// Draw a voxel from mesh data to the image buffer (3D rendering with voxel size)
fn draw_mesh_voxel_3d(params: DrawParams3D) {
    // Normalize position to [0, 1] range based on bounds
    let (min_x, min_y, min_z) = params.min_bound;
    let (max_x, max_y, max_z) = params.max_bound;

    let (x, y, z) = params.position;

    // Project to 2D based on view direction and normalize
    let (u_norm, v_norm, u_size, v_size) = match params.direction {
        ViewDirection::PosX | ViewDirection::NegX => {
            // U=Z, V=Y
            let u = (z - min_z) / (max_z - min_z);
            let v = (y - min_y) / (max_y - min_y);
            let u_s = params.size / (max_z - min_z);
            let v_s = params.size / (max_y - min_y);
            if matches!(params.direction, ViewDirection::NegX) {
                (1.0 - u - u_s, v, u_s, v_s)
            } else {
                (u, v, u_s, v_s)
            }
        }
        ViewDirection::PosY | ViewDirection::NegY => {
            // U=X, V=Z
            let u = (x - min_x) / (max_x - min_x);
            let v = (z - min_z) / (max_z - min_z);
            let u_s = params.size / (max_x - min_x);
            let v_s = params.size / (max_z - min_z);
            if matches!(params.direction, ViewDirection::NegY) {
                (u, 1.0 - v - v_s, u_s, v_s)
            } else {
                (u, v, u_s, v_s)
            }
        }
        ViewDirection::PosZ | ViewDirection::NegZ => {
            // U=X, V=Y
            let u = (x - min_x) / (max_x - min_x);
            let v = (y - min_y) / (max_y - min_y);
            let u_s = params.size / (max_x - min_x);
            let v_s = params.size / (max_y - min_y);
            if matches!(params.direction, ViewDirection::NegZ) {
                (1.0 - u - u_s, v, u_s, v_s)
            } else {
                (u, v, u_s, v_s)
            }
        }
    };

    // Convert to pixel coordinates
    let u_pixels = (u_norm * params.image.width as f32) as usize;
    let v_pixels = (v_norm * params.image.height as f32) as usize;
    let u_size_pixels = (u_size * params.image.width as f32).ceil() as usize;
    let v_size_pixels = (v_size * params.image.height as f32).ceil() as usize;

    // Draw the voxel
    for v in v_pixels..usize::min(v_pixels + v_size_pixels, params.image.height) {
        for u in u_pixels..usize::min(u_pixels + u_size_pixels, params.image.width) {
            let idx = v * params.image.width + u;
            if idx < params.image.pixels.len() {
                params.image.pixels[idx] = params.color;
            }
        }
    }
}

/// Draw a voxel to the image buffer (2D pixel-level rendering)
fn draw_voxel_2d(
    position: (f32, f32, f32),
    size: f32,
    value: i32,
    direction: ViewDirection,
    image: &mut RenderedImage,
    mapper: &dyn ColorMapper,
) {
    // Project 3D position to 2D based on view direction
    let (u_range, v_range, _depth) = project_voxel(position, size, direction);

    // Get color from mapper
    let color = mapper.map(value as u8);
    let color_u8 = [
        (color[0] * 255.0) as u8,
        (color[1] * 255.0) as u8,
        (color[2] * 255.0) as u8,
    ];

    // Calculate pixel coordinates
    let img_size = image.width;
    let u_min = (u_range.0 * img_size as f32) as usize;
    let u_max = (u_range.1 * img_size as f32) as usize;
    let v_min = (v_range.0 * img_size as f32) as usize;
    let v_max = (v_range.1 * img_size as f32) as usize;

    // Draw pixels (simple depth test - closer = smaller depth value)
    for v in v_min..usize::min(v_max, img_size) {
        for u in u_min..usize::min(u_max, img_size) {
            let idx = v * img_size + u;
            if idx < image.pixels.len() {
                // Simple overwrite for now (no proper depth buffer)
                // In a real renderer, we'd check depth
                image.pixels[idx] = color_u8;
            }
        }
    }
}

/// Project 3D voxel to 2D coordinates based on view direction
/// Returns (u_range, v_range, depth) where ranges are in [0, 1]
fn project_voxel(
    position: (f32, f32, f32),
    size: f32,
    direction: ViewDirection,
) -> ((f32, f32), (f32, f32), f32) {
    let (x, y, z) = position;

    match direction {
        ViewDirection::PosX => {
            // Looking from +X towards -X: U=Z, V=Y, depth=X
            ((z, z + size), (y, y + size), x)
        }
        ViewDirection::NegX => {
            // Looking from -X towards +X: U=-Z (flipped), V=Y, depth=-X
            ((1.0 - (z + size), 1.0 - z), (y, y + size), -x)
        }
        ViewDirection::PosY => {
            // Looking from +Y towards -Y: U=X, V=Z, depth=Y
            ((x, x + size), (z, z + size), y)
        }
        ViewDirection::NegY => {
            // Looking from -Y towards +Y: U=X, V=-Z (flipped), depth=-Y
            ((x, x + size), (1.0 - (z + size), 1.0 - z), -y)
        }
        ViewDirection::PosZ => {
            // Looking from +Z towards -Z: U=X, V=Y, depth=Z
            ((x, x + size), (y, y + size), z)
        }
        ViewDirection::NegZ => {
            // Looking from -Z towards +Z: U=-X (flipped), V=Y, depth=-Z
            ((1.0 - (x + size), 1.0 - x), (y, y + size), -z)
        }
    }
}

/// Get octant offset
fn octant_offset(index: usize) -> (f32, f32, f32) {
    let x = if index & 0b100 != 0 { 0.5 } else { 0.0 };
    let y = if index & 0b010 != 0 { 0.5 } else { 0.0 };
    let z = if index & 0b001 != 0 { 0.5 } else { 0.0 };
    (x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::csm::parser::parse_csm;
    use crate::mesh::HsvColorMapper;

    #[test]
    fn test_render_simple_cube() {
        let tree = Octree::new(Cube::Solid(42));
        let mapper = HsvColorMapper::new();

        let image = render_orthographic(&tree, ViewDirection::PosZ, Some(1), &mapper);

        assert_eq!(image.width, 2);
        assert_eq!(image.height, 2);
        assert_eq!(image.pixels.len(), 4);

        // All pixels should be colored (not black)
        for pixel in &image.pixels {
            assert!(pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0);
        }
    }

    #[test]
    fn test_render_subdivided() {
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        let image = render_orthographic(&tree, ViewDirection::PosZ, Some(2), &mapper);

        assert_eq!(image.width, 4);
        assert_eq!(image.height, 4);
        assert_eq!(image.pixels.len(), 16);

        // Should have non-black pixels
        let colored_count = image
            .pixels
            .iter()
            .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
            .count();
        assert!(colored_count > 0);
    }

    #[test]
    fn test_render_all_directions() {
        let csm = r#"
            >a [1 0 0 0 0 0 0 0]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        for direction in ViewDirection::all() {
            let image = render_orthographic(&tree, direction, Some(2), &mapper);
            assert_eq!(image.width, 4);
            assert_eq!(image.height, 4);

            // Each direction should render something
            let colored_count = image
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();
            assert!(
                colored_count > 0,
                "Direction {:?} has no colored pixels",
                direction
            );
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_and_save() {
        use std::fs;

        let _ = fs::create_dir_all("test_output");

        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        let image = render_orthographic(&tree, ViewDirection::PosZ, Some(4), &mapper);
        assert_eq!(image.width, 16);
        assert_eq!(image.height, 16);

        // Save for manual inspection
        let _ = image.save_png("test_output/test_render_basic.png");
    }

    /// Helper to save image on test failure
    #[cfg(feature = "image")]
    fn save_on_failure(image: &RenderedImage, name: &str) {
        use std::fs;
        let _ = fs::create_dir_all("test_output/failures");
        let path = format!("test_output/failures/{}.png", name);
        if let Err(e) = image.save_png(&path) {
            eprintln!("Failed to save failure image {}: {}", path, e);
        } else {
            eprintln!("Saved failure image: {}", path);
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_all_directions_with_output() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/directions");

        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        for direction in ViewDirection::all() {
            let image = render_orthographic(&tree, direction, Some(5), &mapper);

            // Verify image properties
            assert_eq!(image.width, 32, "Direction {:?} has wrong width", direction);
            assert_eq!(
                image.height, 32,
                "Direction {:?} has wrong height",
                direction
            );

            // Check that we have colored pixels
            let colored_count = image
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();

            if colored_count == 0 {
                save_on_failure(&image, &format!("direction_{}_no_pixels", direction.name()));
                panic!("Direction {:?} has no colored pixels", direction);
            }

            // Save output for manual inspection
            let path = format!("test_output/directions/{}.png", direction.name());
            image.save_png(&path).expect("Failed to save test output");
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_mirror_symmetry() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/mirror");

        // Create asymmetric pattern
        let csm = r#"
            >a [1 0 0 0 2 0 0 0]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        let image_orig = render_orthographic(&tree, ViewDirection::PosZ, Some(3), &mapper);
        image_orig
            .save_png("test_output/mirror/original.png")
            .unwrap();

        // Test X mirror
        let csm_mirror_x = r#"
            >a [1 0 0 0 2 0 0 0]
            | >b /x <a
        "#;
        let tree_mx = parse_csm(csm_mirror_x).unwrap();
        let image_mx = render_orthographic(&tree_mx, ViewDirection::PosZ, Some(3), &mapper);
        image_mx
            .save_png("test_output/mirror/mirror_x.png")
            .unwrap();

        // Test Y mirror
        let csm_mirror_y = r#"
            >a [1 0 0 0 2 0 0 0]
            | >b /y <a
        "#;
        let tree_my = parse_csm(csm_mirror_y).unwrap();
        let image_my = render_orthographic(&tree_my, ViewDirection::PosZ, Some(3), &mapper);
        image_my
            .save_png("test_output/mirror/mirror_y.png")
            .unwrap();

        // Test Z mirror
        let csm_mirror_z = r#"
            >a [1 0 0 0 2 0 0 0]
            | >b /z <a
        "#;
        let tree_mz = parse_csm(csm_mirror_z).unwrap();
        let image_mz = render_orthographic(&tree_mz, ViewDirection::PosZ, Some(3), &mapper);
        image_mz
            .save_png("test_output/mirror/mirror_z.png")
            .unwrap();

        // Verify images are different (mirrors should change appearance)
        assert_ne!(
            image_orig.pixels, image_mx.pixels,
            "X mirror should change appearance"
        );
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_swap_vs_mirror() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/swap_vs_mirror");

        // Nested structure to show difference between swap and mirror
        let csm_base = r#"
            >a [1 0 0 0 [10 0 0 0 0 0 0 0] 0 0 0]
        "#;
        let tree_base = parse_csm(csm_base).unwrap();
        let mapper = HsvColorMapper::new();

        let image_base = render_orthographic(&tree_base, ViewDirection::PosZ, Some(4), &mapper);
        image_base
            .save_png("test_output/swap_vs_mirror/base.png")
            .unwrap();

        // Swap (non-recursive)
        let csm_swap = r#"
            >a [1 0 0 0 [10 0 0 0 0 0 0 0] 0 0 0]
            | >b ^x <a
        "#;
        let tree_swap = parse_csm(csm_swap).unwrap();
        let image_swap = render_orthographic(&tree_swap, ViewDirection::PosZ, Some(4), &mapper);
        image_swap
            .save_png("test_output/swap_vs_mirror/swap_x.png")
            .unwrap();

        // Mirror (recursive)
        let csm_mirror = r#"
            >a [1 0 0 0 [10 0 0 0 0 0 0 0] 0 0 0]
            | >b /x <a
        "#;
        let tree_mirror = parse_csm(csm_mirror).unwrap();
        let image_mirror = render_orthographic(&tree_mirror, ViewDirection::PosZ, Some(4), &mapper);
        image_mirror
            .save_png("test_output/swap_vs_mirror/mirror_x.png")
            .unwrap();

        // Verify they're different
        assert_ne!(
            image_swap.pixels, image_mirror.pixels,
            "Swap and mirror should be different"
        );
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_depth_levels() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/depth");

        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
            >aaa [20 21 22 23 24 25 26 27]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        for depth in [2, 3, 4, 5, 6] {
            let image = render_orthographic(&tree, ViewDirection::PosZ, Some(depth), &mapper);
            let expected_size = 1 << depth;

            if image.width != expected_size {
                save_on_failure(&image, &format!("depth_{}_wrong_size", depth));
                panic!(
                    "Depth {} should produce {}x{} image, got {}x{}",
                    depth, expected_size, expected_size, image.width, image.height
                );
            }

            let path = format!(
                "test_output/depth/depth_{}_size_{}.png",
                depth, expected_size
            );
            image
                .save_png(&path)
                .expect("Failed to save depth test output");
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_complex_scene() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/complex");

        // More complex CSM with multiple operations
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
            >ab [20 21 22 23 24 25 26 27]
            >ac [30 31 32 33 34 35 36 37]
            | >b /x <a
            | >c /y <b
        "#;

        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Render from multiple angles
        for direction in [
            ViewDirection::PosX,
            ViewDirection::PosY,
            ViewDirection::PosZ,
        ] {
            let image = render_orthographic(&tree, direction, Some(6), &mapper);

            // Check for reasonable color distribution
            let colored_count = image
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();

            let colored_ratio = colored_count as f32 / image.pixels.len() as f32;

            if colored_ratio < 0.01 {
                save_on_failure(&image, &format!("complex_{}_too_empty", direction.name()));
                panic!(
                    "Complex scene from {:?} has too few colored pixels: {:.2}%",
                    direction,
                    colored_ratio * 100.0
                );
            }

            let path = format!("test_output/complex/{}.png", direction.name());
            image.save_png(&path).expect("Failed to save complex scene");
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_deep_colorful_structure() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/deep_colorful");

        // Create a 3-level deep structure with many different colors
        // Use all 8 children at each level for better visibility from all angles
        let csm = r#"
            # Level 1: 8 children with distinct colors
            >a [10 20 30 40 50 60 70 80]

            # Level 2: Subdivide all 8 children
            >aa [11 12 13 14 15 16 17 18]
            >ab [21 22 23 24 25 26 27 28]
            >ac [31 32 33 34 35 36 37 38]
            >ad [41 42 43 44 45 46 47 48]
            >ae [51 52 53 54 55 56 57 58]
            >af [61 62 63 64 65 66 67 68]
            >ag [71 72 73 74 75 76 77 78]
            >ah [81 82 83 84 85 86 87 88]

            # Level 3: Subdivide first child of each level-2 octant
            >aaa [110 111 112 113 114 115 116 117]
            >aba [210 211 212 213 214 215 216 217]
            >aca [310 311 312 313 314 315 316 317]
            >ada [410 411 412 413 414 415 416 417]
            >aea [510 511 512 513 514 515 516 517]
            >afa [610 611 612 613 614 615 616 617]
            >aga [710 711 712 713 714 715 716 717]
            >aha [810 811 812 813 814 815 816 817]
        "#;

        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Render from all 6 directions at depth 5 (32x32 pixels)
        for direction in ViewDirection::all() {
            let image = render_orthographic(&tree, direction, Some(5), &mapper);

            // Verify size
            assert_eq!(image.width, 32);
            assert_eq!(image.height, 32);

            // Count unique colors to verify we have variety
            use std::collections::HashSet;
            let unique_colors: HashSet<[u8; 3]> = image.pixels.iter().copied().collect();

            // Should have multiple unique colors
            // Note: From some angles we see fewer colors due to occlusion
            let non_black_colors = unique_colors
                .iter()
                .filter(|c| c[0] > 0 || c[1] > 0 || c[2] > 0)
                .count();

            // Require at least 3 different colors (lenient for orthographic views)
            if non_black_colors < 3 {
                save_on_failure(
                    &image,
                    &format!("deep_colorful_{}_low_variety", direction.name()),
                );
                panic!(
                    "Deep colorful structure from {:?} has only {} unique colors, expected at least 3",
                    direction, non_black_colors
                );
            }

            // Check that reasonable portion is filled
            let colored_count = image
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();
            let colored_ratio = colored_count as f32 / image.pixels.len() as f32;

            if colored_ratio < 0.1 {
                save_on_failure(
                    &image,
                    &format!("deep_colorful_{}_too_sparse", direction.name()),
                );
                panic!(
                    "Deep colorful structure from {:?} has only {:.2}% colored pixels",
                    direction,
                    colored_ratio * 100.0
                );
            }

            let path = format!("test_output/deep_colorful/{}.png", direction.name());
            image
                .save_png(&path)
                .expect("Failed to save deep colorful structure");

            println!(
                "Rendered {} from {:?}: {} unique colors, {:.1}% filled",
                path,
                direction,
                non_black_colors,
                colored_ratio * 100.0
            );
        }

        // Also render at higher resolution (depth 6 = 64x64) for better detail
        let image_hires = render_orthographic(&tree, ViewDirection::PosZ, Some(6), &mapper);
        image_hires
            .save_png("test_output/deep_colorful/pos_z_hires_64x64.png")
            .expect("Failed to save high-res image");

        // And at depth 7 for maximum detail (128x128)
        let image_ultra = render_orthographic(&tree, ViewDirection::PosZ, Some(7), &mapper);
        image_ultra
            .save_png("test_output/deep_colorful/pos_z_ultra_128x128.png")
            .expect("Failed to save ultra-res image");
    }

    #[test]
    fn test_depth_limits() {
        let tree = Octree::new(Cube::Solid(1));

        // Depth 1 = 2x2
        let img1 = render_orthographic(&tree, ViewDirection::PosZ, Some(1), &HsvColorMapper::new());
        assert_eq!(img1.width, 2);

        // Depth 3 = 8x8
        let img3 = render_orthographic(&tree, ViewDirection::PosZ, Some(3), &HsvColorMapper::new());
        assert_eq!(img3.width, 8);

        // Depth 5 = 32x32
        let img5 = render_orthographic(&tree, ViewDirection::PosZ, Some(5), &HsvColorMapper::new());
        assert_eq!(img5.width, 32);
    }

    #[test]
    fn test_render_3d_basic() {
        let tree = Octree::new(Cube::Solid(42));
        let mapper = HsvColorMapper::new();

        // Render with voxel size 2^2 (4x4 pixels per voxel)
        // Single solid cube at position (0,0,0) with size 1.0
        // Should produce 4x4 image
        let image = render_orthographic_3d(&tree, ViewDirection::PosZ, 2, &mapper);

        assert_eq!(image.width, 4);
        assert_eq!(image.height, 4);

        // All pixels should be colored (solid cube)
        let colored_count = image
            .pixels
            .iter()
            .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
            .count();
        assert_eq!(colored_count, 16, "Solid cube should fill entire image");
    }

    #[test]
    fn test_render_3d_voxel_sizes() {
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Test different voxel sizes
        // The mesh contains 8 voxels, each at positions that span [0, 1]
        for voxel_size_log2 in [1, 2, 3, 4] {
            let image =
                render_orthographic_3d(&tree, ViewDirection::PosZ, voxel_size_log2, &mapper);

            // Image size depends on voxel bounds from mesh
            // For subdivided octree [0,1] range: each half-cube gets drawn
            assert!(image.width > 0, "Should have non-zero width");
            assert!(image.height > 0, "Should have non-zero height");

            // Should have colored pixels
            let colored_count = image
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();
            assert!(
                colored_count > 0,
                "Voxel size 2^{} should have colored pixels",
                voxel_size_log2
            );
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_2d_vs_3d_comparison() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/comparison");

        // Create a simple test structure
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Render with 2D at depth 5 (32x32)
        let image_2d = render_orthographic_2d(&tree, ViewDirection::PosZ, Some(5), &mapper);
        assert_eq!(image_2d.width, 32);

        // Render with 3D using mesh
        // Voxel size 2^5 means each mesh voxel becomes 32 pixels
        // The mesh spans [0,1], so image will be 32x32
        let image_3d = render_orthographic_3d(&tree, ViewDirection::PosZ, 5, &mapper);

        // Create side-by-side comparison
        let comparison = RenderedImage::side_by_side(&image_2d, &image_3d);

        // Save outputs
        image_2d
            .save_png("test_output/comparison/2d_render.png")
            .unwrap();
        image_3d
            .save_png("test_output/comparison/3d_render.png")
            .unwrap();
        comparison
            .save_png("test_output/comparison/side_by_side.png")
            .unwrap();

        println!(
            "Saved comparison images to test_output/comparison/ (2D: {}x{}, 3D: {}x{})",
            image_2d.width, image_2d.height, image_3d.width, image_3d.height
        );
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_comparison_all_directions() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/comparison_dirs");

        // Create a 3-level deep colorful structure
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >aa [10 11 12 13 14 15 16 17]
            >ab [20 21 22 23 24 25 26 27]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Test all 6 directions
        for direction in ViewDirection::all() {
            // 2D render at depth 5 (32x32)
            let image_2d = render_orthographic_2d(&tree, direction, Some(5), &mapper);

            // 3D render using mesh
            let image_3d = render_orthographic_3d(&tree, direction, 5, &mapper);

            // Create comparison image (2D left, 3D right)
            let comparison = RenderedImage::side_by_side(&image_2d, &image_3d);

            let path = format!("test_output/comparison_dirs/{}.png", direction.name());
            comparison.save_png(&path).unwrap();

            println!(
                "Saved {} (2D: {}x{}, 3D: {}x{}, total: {}x{})",
                path,
                image_2d.width,
                image_2d.height,
                image_3d.width,
                image_3d.height,
                comparison.width,
                comparison.height
            );
        }
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_3d_large_voxels() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/3d_render");

        // Create complex structure
        let csm = r#"
            >a [10 20 30 40 50 60 70 80]
            >aa [11 12 13 14 15 16 17 18]
            >ab [21 22 23 24 25 26 27 28]
            >ac [31 32 33 34 35 36 37 38]
        "#;
        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Render with large voxels for visibility (8x8 pixels per voxel)
        let image = render_orthographic_3d(&tree, ViewDirection::PosZ, 3, &mapper);

        // Verify we have colored pixels
        let colored_count = image
            .pixels
            .iter()
            .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
            .count();

        let colored_ratio = colored_count as f32 / image.pixels.len() as f32;
        assert!(
            colored_ratio > 0.1,
            "Should have reasonable fill ratio, got {:.2}%",
            colored_ratio * 100.0
        );

        image
            .save_png("test_output/3d_render/large_voxels_64x64.png")
            .unwrap();
        println!(
            "3D render saved: {}x{} with {:.1}% filled",
            image.width,
            image.height,
            colored_ratio * 100.0
        );

        // Also test at higher resolution (16x16 pixels per voxel)
        let image_hires = render_orthographic_3d(&tree, ViewDirection::PosZ, 4, &mapper);
        image_hires
            .save_png("test_output/3d_render/large_voxels_128x128.png")
            .unwrap();
        println!(
            "3D hi-res render saved: {}x{}",
            image_hires.width, image_hires.height
        );
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_side_by_side_helper() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/side_by_side");

        // Create two different renders
        let tree1 = Octree::new(Cube::Solid(10));
        let mapper = HsvColorMapper::new();

        let img1 = render_orthographic_2d(&tree1, ViewDirection::PosZ, Some(4), &mapper);

        let csm2 = r#">a [1 2 3 4 5 6 7 8]"#;
        let tree2 = parse_csm(csm2).unwrap();
        let img2 = render_orthographic_2d(&tree2, ViewDirection::PosZ, Some(4), &mapper);

        // Create side-by-side
        let combined = RenderedImage::side_by_side(&img1, &img2);

        assert_eq!(combined.width, img1.width + img2.width);
        assert_eq!(combined.height, img1.height.max(img2.height));

        combined
            .save_png("test_output/side_by_side/combined.png")
            .unwrap();
    }

    /// Verify that mesh normals are correct

    #[cfg(feature = "image")]
    #[test]
    fn test_render_3d_deep_all_directions() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/3d_deep_directions");

        // Create a deep octree (depth 3) with varied colors
        let csm = r#"
            # Root subdivided (level 1)
            >a [1 2 3 4 5 6 7 8]

            # Level 2: Subdivide children a, b, c, d
            >aa [10 11 12 13 14 15 16 17]
            >ab [20 21 22 23 24 25 26 27]
            >ac [30 31 32 33 34 35 36 37]
            >ad [40 41 42 43 44 45 46 47]

            # Level 3: Subdivide some level-2 children for more detail
            >aaa [110 111 112 113 114 115 116 117]
            >aab [120 121 122 123 124 125 126 127]
            >aba [210 211 212 213 214 215 216 217]
            >abb [220 221 222 223 224 225 226 227]
        "#;

        let tree = parse_csm(csm).unwrap();
        let mapper = HsvColorMapper::new();

        // Render from all 6 directions with voxel_size_log2=4 (16x16 pixels per voxel)
        // This should produce images that are at least 4x4 and show detail
        for direction in ViewDirection::all() {
            let image_3d = render_orthographic_3d(&tree, direction, 4, &mapper);

            // Verify minimum size
            assert!(
                image_3d.width >= 4,
                "{:?}: width {} should be >= 4",
                direction,
                image_3d.width
            );
            assert!(
                image_3d.height >= 4,
                "{:?}: height {} should be >= 4",
                direction,
                image_3d.height
            );

            // Verify we have colored pixels
            let colored_count = image_3d
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();
            let colored_ratio = colored_count as f32 / image_3d.pixels.len() as f32;

            assert!(
                colored_count > 0,
                "{:?}: should have colored pixels",
                direction
            );

            let path = format!("test_output/3d_deep_directions/{}.png", direction.name());
            image_3d.save_png(&path).unwrap();

            println!(
                "3D deep render {}: {}x{} ({:.1}% filled, {} colored pixels)",
                direction.name(),
                image_3d.width,
                image_3d.height,
                colored_ratio * 100.0,
                colored_count
            );
        }

        // Also create high-resolution versions
        let image_hires = render_orthographic_3d(&tree, ViewDirection::PosZ, 6, &mapper);
        image_hires
            .save_png("test_output/3d_deep_directions/pos_z_hires.png")
            .unwrap();
        println!(
            "High-res 3D render: {}x{}",
            image_hires.width, image_hires.height
        );

        let image_ultra = render_orthographic_3d(&tree, ViewDirection::PosZ, 7, &mapper);
        image_ultra
            .save_png("test_output/3d_deep_directions/pos_z_ultra.png")
            .unwrap();
        println!(
            "Ultra-res 3D render: {}x{}",
            image_ultra.width, image_ultra.height
        );
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_render_transparency_depth2() {
        use std::fs;

        let _ = fs::create_dir_all("test_output/transparency");

        // Create depth-2 octree with pseudorandom cell data and 50% transparency
        // Depth 2 means: 1 root subdivided into 8 children, each subdivided into 8 = 64 leaf voxels

        // Pseudorandom pattern using simple deterministic sequence
        // Use pattern: 0 for transparent, 1-100 for colored
        // Alternate transparent/opaque in a pseudorandom pattern
        let transparent_pattern = [
            // Level 1: 8 children (indices 0-7)
            // Each child is subdivided in level 2

            // Child 0 (aaa-aah): mix of transparent and opaque
            0, 1, 0, 2, 3, 0, 4, 0, // 4 transparent, 4 opaque
            // Child 1 (aba-abh): mostly opaque
            5, 6, 0, 7, 8, 0, 9, 10, // 2 transparent, 6 opaque
            // Child 2 (aca-ach): mostly transparent
            0, 0, 11, 0, 0, 12, 0, 0, // 6 transparent, 2 opaque
            // Child 3 (ada-adh): mixed
            13, 0, 14, 0, 15, 0, 16, 0, // 4 transparent, 4 opaque
            // Child 4 (aea-aeh): mixed
            0, 17, 0, 18, 0, 19, 0, 20, // 4 transparent, 4 opaque
            // Child 5 (afa-afh): mostly opaque
            21, 22, 23, 0, 24, 25, 0, 26, // 2 transparent, 6 opaque
            // Child 6 (aga-agh): mostly transparent
            0, 0, 0, 27, 0, 0, 0, 28, // 6 transparent, 2 opaque
            // Child 7 (aha-ahh): mixed
            29, 0, 30, 0, 31, 0, 32, 0, // 4 transparent, 4 opaque
        ];

        // Count transparency: should be 32 transparent out of 64 (50%)
        let transparent_count = transparent_pattern.iter().filter(|&&v| v == 0).count();
        let opaque_count = transparent_pattern.iter().filter(|&&v| v != 0).count();
        assert_eq!(transparent_count, 32, "Should have 32 transparent voxels");
        assert_eq!(opaque_count, 32, "Should have 32 opaque voxels");

        // Build CSM with this pattern
        // Define each child first with 8 values
        let mut csm = String::new();
        for i in 0..8 {
            let child_name = format!("a{}", (b'a' + i as u8) as char);
            csm.push_str(&format!(">a{} [", child_name));

            for j in 0..8 {
                let idx = i * 8 + j;
                csm.push_str(&format!("{}", transparent_pattern[idx]));
                if j < 7 {
                    csm.push(' ');
                }
            }
            csm.push_str("]\n");
        }

        // Add root that references all children
        csm.push_str("\n>a [");
        for i in 0..8 {
            let child_name = format!("a{}", (b'a' + i as u8) as char);
            csm.push_str(&format!("<{}", child_name));
            if i < 7 {
                csm.push(' ');
            }
        }
        csm.push_str("]\n");

        let tree = parse_csm(&csm).expect("Failed to parse transparency test CSM");
        let mapper = HsvColorMapper::new();

        // Test all 6 directions
        for direction in ViewDirection::all() {
            // 2D render at depth 6 (64x64 pixels, one pixel per leaf voxel)
            let image_2d = render_orthographic_2d(&tree, direction, Some(6), &mapper);

            // 3D render with 8 pixels per voxel for better visibility
            let image_3d = render_orthographic_3d(&tree, direction, 3, &mapper);

            // Verify that we have black (transparent) pixels
            let black_2d = image_2d
                .pixels
                .iter()
                .filter(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
                .count();
            let colored_2d = image_2d
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();

            let black_3d = image_3d
                .pixels
                .iter()
                .filter(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
                .count();
            let colored_3d = image_3d
                .pixels
                .iter()
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();

            // 2D rendering should always show both transparent and colored
            // (since we render at pixel level with no occlusion within a view plane)
            assert!(
                black_2d > 0,
                "{:?}: 2D should have transparent pixels",
                direction
            );
            assert!(
                colored_2d > 0,
                "{:?}: 2D should have colored pixels",
                direction
            );

            // 3D rendering might not show transparent pixels from all angles due to occlusion,
            // but should have colored pixels
            assert!(
                colored_3d > 0,
                "{:?}: 3D should have colored pixels",
                direction
            );

            // Create side-by-side comparison
            let comparison = RenderedImage::side_by_side(&image_2d, &image_3d);

            let path = format!("test_output/transparency/{}.png", direction.name());
            comparison.save_png(&path).unwrap();

            println!("Transparency test {}: 2D={}x{} ({} transparent, {} colored), 3D={}x{} ({} transparent, {} colored)",
                direction.name(),
                image_2d.width, image_2d.height, black_2d, colored_2d,
                image_3d.width, image_3d.height, black_3d, colored_3d);
        }

        // Also save a high-resolution version for detailed inspection
        let image_2d_hires = render_orthographic_2d(&tree, ViewDirection::PosZ, Some(8), &mapper);
        let image_3d_hires = render_orthographic_3d(&tree, ViewDirection::PosZ, 5, &mapper);

        let comparison_hires = RenderedImage::side_by_side(&image_2d_hires, &image_3d_hires);
        comparison_hires
            .save_png("test_output/transparency/pos_z_hires.png")
            .unwrap();

        println!(
            "High-res transparency test saved: 2D={}x{}, 3D={}x{}",
            image_2d_hires.width,
            image_2d_hires.height,
            image_3d_hires.width,
            image_3d_hires.height
        );
    }
}
