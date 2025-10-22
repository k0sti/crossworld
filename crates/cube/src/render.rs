use crate::mesh::ColorMapper;
use crate::octree::{Cube, Octree};

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
}

/// Render octree to 2D image from specified view direction
pub fn render_orthographic(
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
    render_cube(
        &octree.root,
        (0.0, 0.0, 0.0),
        1.0,
        0,
        depth,
        direction,
        &mut image,
        mapper,
    );

    image
}

/// Recursively render a cube to the image
fn render_cube(
    cube: &Cube<i32>,
    position: (f32, f32, f32),
    size: f32,
    current_depth: usize,
    max_depth: usize,
    direction: ViewDirection,
    image: &mut RenderedImage,
    mapper: &dyn ColorMapper,
) {
    match cube {
        Cube::Solid(value) => {
            if *value != 0 {
                // Draw this voxel to the image
                draw_voxel(position, size, *value, direction, image, mapper);
            }
        }
        Cube::Cubes(children) if current_depth < max_depth => {
            // Recursively render children
            let half_size = size / 2.0;
            for (idx, child) in children.iter().enumerate() {
                let offset = octant_offset(idx);
                let child_pos = (
                    position.0 + offset.0 * size,
                    position.1 + offset.1 * size,
                    position.2 + offset.2 * size,
                );
                render_cube(
                    child,
                    child_pos,
                    half_size,
                    current_depth + 1,
                    max_depth,
                    direction,
                    image,
                    mapper,
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

/// Draw a voxel to the image buffer
fn draw_voxel(
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
    let color = mapper.map(value);
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
    use crate::mesh::HsvColorMapper;
    use crate::parser::parse_csm;

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
        let colored_count = image.pixels.iter().filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0).count();
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
            let colored_count = image.pixels.iter().filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0).count();
            assert!(colored_count > 0, "Direction {:?} has no colored pixels", direction);
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
            assert_eq!(image.height, 32, "Direction {:?} has wrong height", direction);

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
        image_orig.save_png("test_output/mirror/original.png").unwrap();

        // Test X mirror
        let csm_mirror_x = r#"
            >a [1 0 0 0 2 0 0 0]
            | >b /x <a
        "#;
        let tree_mx = parse_csm(csm_mirror_x).unwrap();
        let image_mx = render_orthographic(&tree_mx, ViewDirection::PosZ, Some(3), &mapper);
        image_mx.save_png("test_output/mirror/mirror_x.png").unwrap();

        // Test Y mirror
        let csm_mirror_y = r#"
            >a [1 0 0 0 2 0 0 0]
            | >b /y <a
        "#;
        let tree_my = parse_csm(csm_mirror_y).unwrap();
        let image_my = render_orthographic(&tree_my, ViewDirection::PosZ, Some(3), &mapper);
        image_my.save_png("test_output/mirror/mirror_y.png").unwrap();

        // Test Z mirror
        let csm_mirror_z = r#"
            >a [1 0 0 0 2 0 0 0]
            | >b /z <a
        "#;
        let tree_mz = parse_csm(csm_mirror_z).unwrap();
        let image_mz = render_orthographic(&tree_mz, ViewDirection::PosZ, Some(3), &mapper);
        image_mz.save_png("test_output/mirror/mirror_z.png").unwrap();

        // Verify images are different (mirrors should change appearance)
        assert_ne!(image_orig.pixels, image_mx.pixels, "X mirror should change appearance");
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
        image_base.save_png("test_output/swap_vs_mirror/base.png").unwrap();

        // Swap (non-recursive)
        let csm_swap = r#"
            >a [1 0 0 0 [10 0 0 0 0 0 0 0] 0 0 0]
            | >b ^x <a
        "#;
        let tree_swap = parse_csm(csm_swap).unwrap();
        let image_swap = render_orthographic(&tree_swap, ViewDirection::PosZ, Some(4), &mapper);
        image_swap.save_png("test_output/swap_vs_mirror/swap_x.png").unwrap();

        // Mirror (recursive)
        let csm_mirror = r#"
            >a [1 0 0 0 [10 0 0 0 0 0 0 0] 0 0 0]
            | >b /x <a
        "#;
        let tree_mirror = parse_csm(csm_mirror).unwrap();
        let image_mirror = render_orthographic(&tree_mirror, ViewDirection::PosZ, Some(4), &mapper);
        image_mirror.save_png("test_output/swap_vs_mirror/mirror_x.png").unwrap();

        // Verify they're different
        assert_ne!(image_swap.pixels, image_mirror.pixels, "Swap and mirror should be different");
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
                panic!("Depth {} should produce {}x{} image, got {}x{}",
                    depth, expected_size, expected_size, image.width, image.height);
            }

            let path = format!("test_output/depth/depth_{}_size_{}.png", depth, expected_size);
            image.save_png(&path).expect("Failed to save depth test output");
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
        for direction in [ViewDirection::PosX, ViewDirection::PosY, ViewDirection::PosZ] {
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
                save_on_failure(&image, &format!("deep_colorful_{}_low_variety", direction.name()));
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
                save_on_failure(&image, &format!("deep_colorful_{}_too_sparse", direction.name()));
                panic!(
                    "Deep colorful structure from {:?} has only {:.2}% colored pixels",
                    direction,
                    colored_ratio * 100.0
                );
            }

            let path = format!("test_output/deep_colorful/{}.png", direction.name());
            image.save_png(&path).expect("Failed to save deep colorful structure");

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
        image_hires.save_png("test_output/deep_colorful/pos_z_hires_64x64.png")
            .expect("Failed to save high-res image");

        // And at depth 7 for maximum detail (128x128)
        let image_ultra = render_orthographic(&tree, ViewDirection::PosZ, Some(7), &mapper);
        image_ultra.save_png("test_output/deep_colorful/pos_z_ultra_128x128.png")
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
}
