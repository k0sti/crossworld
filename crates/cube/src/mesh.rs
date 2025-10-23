use crate::octree::Octree;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshData {
    pub vertices: Vec<f32>,
    pub indices: Vec<u32>,
    pub normals: Vec<f32>,
    pub colors: Vec<f32>,
}

impl MeshData {
    pub fn new() -> Self {
        MeshData {
            vertices: Vec::new(),
            indices: Vec::new(),
            normals: Vec::new(),
            colors: Vec::new(),
        }
    }
}

impl Default for MeshData {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for mapping voxel indices to RGB colors
pub trait ColorMapper {
    fn map(&self, index: i32) -> [f32; 3];
}

/// HSV-based color mapper (existing behavior)
pub struct HsvColorMapper {
    pub saturation: f32,
    pub value: f32,
}

impl HsvColorMapper {
    pub fn new() -> Self {
        HsvColorMapper {
            saturation: 0.8,
            value: 0.9,
        }
    }

    pub fn with_params(saturation: f32, value: f32) -> Self {
        HsvColorMapper { saturation, value }
    }
}

impl Default for HsvColorMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorMapper for HsvColorMapper {
    fn map(&self, index: i32) -> [f32; 3] {
        if index < 0 {
            [1.0, 0.0, 0.0] // Red for negative
        } else if index == 0 {
            [0.0, 0.0, 0.0] // Black for zero
        } else {
            let hue = (index % 360) as f32;
            hsv_to_rgb(hue, self.saturation, self.value)
        }
    }
}

/// Palette-based color mapper
pub struct PaletteColorMapper {
    colors: Vec<[f32; 3]>,
}

impl PaletteColorMapper {
    pub fn new(colors: Vec<[f32; 3]>) -> Self {
        PaletteColorMapper { colors }
    }

    /// Load palette from image data (RGB/RGBA bytes)
    #[cfg(feature = "image")]
    pub fn from_image_bytes(bytes: &[u8]) -> Result<Self, String> {
        use image::GenericImageView;

        let img =
            image::load_from_memory(bytes).map_err(|e| format!("Failed to load image: {}", e))?;

        let mut colors = Vec::new();
        for pixel in img.pixels() {
            let rgba = pixel.2;
            colors.push([
                rgba[0] as f32 / 255.0,
                rgba[1] as f32 / 255.0,
                rgba[2] as f32 / 255.0,
            ]);
        }

        Ok(PaletteColorMapper { colors })
    }

    /// Load palette from image file path
    #[cfg(feature = "image")]
    pub fn from_image_path(path: &str) -> Result<Self, String> {
        use image::GenericImageView;

        let img =
            image::open(path).map_err(|e| format!("Failed to open image at {}: {}", path, e))?;

        let mut colors = Vec::new();
        for pixel in img.pixels() {
            let rgba = pixel.2;
            colors.push([
                rgba[0] as f32 / 255.0,
                rgba[1] as f32 / 255.0,
                rgba[2] as f32 / 255.0,
            ]);
        }

        Ok(PaletteColorMapper { colors })
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }
}

impl ColorMapper for PaletteColorMapper {
    fn map(&self, index: i32) -> [f32; 3] {
        if self.colors.is_empty() {
            return [1.0, 0.0, 1.0]; // Magenta for error
        }

        if index <= 0 {
            return [0.0, 0.0, 0.0]; // Black for zero/negative
        }

        let idx = ((index - 1) as usize) % self.colors.len();
        self.colors[idx]
    }
}

/// Generate a mesh from an octree with default HSV coloring
pub fn generate_mesh(octree: &Octree) -> MeshData {
    generate_mesh_with_mapper(octree, &HsvColorMapper::new())
}

/// Generate a mesh from an octree with custom color mapper
pub fn generate_mesh_with_mapper(octree: &Octree, mapper: &dyn ColorMapper) -> MeshData {
    let mut mesh = MeshData::new();

    // Collect all voxels from the octree
    let voxels = octree.collect_voxels();

    // For each voxel, generate a cube
    for (x, y, z, size, value) in voxels {
        if value == 0 {
            continue; // Skip empty voxels
        }

        let color = mapper.map(value);
        add_cube(&mut mesh, x, y, z, size, color);
    }

    mesh
}

/// Convert HSV to RGB
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h % 360.0;
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [r + m, g + m, b + m]
}

/// Add a cube to the mesh
fn add_cube(mesh: &mut MeshData, x: f32, y: f32, z: f32, size: f32, color: [f32; 3]) {
    let base_index = (mesh.vertices.len() / 3) as u32;

    // Define 8 vertices of the cube
    let vertices = [
        [x, y, z],                      // 0: left-bottom-back
        [x + size, y, z],               // 1: right-bottom-back
        [x + size, y + size, z],        // 2: right-top-back
        [x, y + size, z],               // 3: left-top-back
        [x, y, z + size],               // 4: left-bottom-front
        [x + size, y, z + size],        // 5: right-bottom-front
        [x + size, y + size, z + size], // 6: right-top-front
        [x, y + size, z + size],        // 7: left-top-front
    ];

    // Add vertices and colors
    for vertex in &vertices {
        mesh.vertices.extend_from_slice(vertex);
        mesh.colors.extend_from_slice(&color);
    }

    // Add normals for each vertex
    // For a cube, we can use averaged normals or per-face normals
    // Using simple averaged normals pointing outward from cube center
    let center = [x + size / 2.0, y + size / 2.0, z + size / 2.0];
    for vertex in &vertices {
        let normal = [
            (vertex[0] - center[0]).signum(),
            (vertex[1] - center[1]).signum(),
            (vertex[2] - center[2]).signum(),
        ];
        // Normalize
        let len = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
        if len > 0.0 {
            mesh.normals.push(normal[0] / len);
            mesh.normals.push(normal[1] / len);
            mesh.normals.push(normal[2] / len);
        } else {
            mesh.normals.extend_from_slice(&[0.0, 1.0, 0.0]);
        }
    }

    // Define faces
    // Each face is defined by 4 vertices (2 triangles)
    let faces = [
        // Back face (z = 0)
        [0, 1, 2, 3],
        // Front face (z = size)
        [5, 4, 7, 6],
        // Left face (x = 0)
        [4, 0, 3, 7],
        // Right face (x = size)
        [1, 5, 6, 2],
        // Bottom face (y = 0)
        [4, 5, 1, 0],
        // Top face (y = size)
        [3, 2, 6, 7],
    ];

    for indices in &faces {
        // First triangle
        mesh.indices.push(base_index + indices[0]);
        mesh.indices.push(base_index + indices[1]);
        mesh.indices.push(base_index + indices[2]);

        // Second triangle
        mesh.indices.push(base_index + indices[0]);
        mesh.indices.push(base_index + indices[2]);
        mesh.indices.push(base_index + indices[3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::octree::{Cube, Octree};
    use std::rc::Rc;

    #[test]
    fn test_generate_mesh_simple() {
        let tree = Octree::new(Cube::Solid(42));
        let mesh = generate_mesh(&tree);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.indices.is_empty());
        assert_eq!(mesh.vertices.len() / 3, mesh.colors.len() / 3);
    }

    #[test]
    fn test_hsv_to_rgb() {
        let red = hsv_to_rgb(0.0, 1.0, 1.0);
        assert_eq!(red, [1.0, 0.0, 0.0]);

        let green = hsv_to_rgb(120.0, 1.0, 1.0);
        assert!((green[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_hsv_color_mapper() {
        let mapper = HsvColorMapper::new();

        let color1 = mapper.map(1);
        let color42 = mapper.map(42);

        // Different indices should give different colors
        assert_ne!(color1, color42);

        // Negative should be red
        assert_eq!(mapper.map(-1), [1.0, 0.0, 0.0]);

        // Zero should be black
        assert_eq!(mapper.map(0), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_palette_color_mapper() {
        let palette = vec![
            [1.0, 0.0, 0.0], // Red
            [0.0, 1.0, 0.0], // Green
            [0.0, 0.0, 1.0], // Blue
        ];

        let mapper = PaletteColorMapper::new(palette);

        // Index 1 -> first color (red)
        assert_eq!(mapper.map(1), [1.0, 0.0, 0.0]);
        // Index 2 -> second color (green)
        assert_eq!(mapper.map(2), [0.0, 1.0, 0.0]);
        // Index 3 -> third color (blue)
        assert_eq!(mapper.map(3), [0.0, 0.0, 1.0]);
        // Index 4 -> wraps to first color (red)
        assert_eq!(mapper.map(4), [1.0, 0.0, 0.0]);

        // Zero/negative should be black
        assert_eq!(mapper.map(0), [0.0, 0.0, 0.0]);
        assert_eq!(mapper.map(-1), [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_generate_mesh_with_palette() {
        let palette = vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let mapper = PaletteColorMapper::new(palette);

        let cube = Cube::cubes([
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
            Rc::new(Cube::Solid(3)),
            Rc::new(Cube::Solid(1)),
            Rc::new(Cube::Solid(2)),
        ]);

        let tree = Octree::new(cube);
        let mesh = generate_mesh_with_mapper(&tree, &mapper);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.colors.is_empty());
    }

    #[cfg(feature = "image")]
    #[test]
    fn test_dawnbringer_32_palette() {
        use crate::parser::parse_csm;

        // Path relative to workspace root
        let path = if std::path::Path::new("../../assets/palettes/dawnbringer-32.png").exists() {
            "../../assets/palettes/dawnbringer-32.png"
        } else {
            "assets/palettes/dawnbringer-32.png"
        };

        let palette = match PaletteColorMapper::from_image_path(path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: Could not load palette ({}). Skipping test.", e);
                eprintln!("Please place dawnbringer-32.png (32x1 or any size) in assets/palettes/");
                return; // Skip test if palette not found
            }
        };

        // Verify palette has colors
        assert!(palette.len() > 0, "Palette should have colors");

        // Create test octree using CSM
        let csm = r#"
            >a [1 2 3 4 5 6 7 8]
            >b [10 11 12 13 14 15 16 17]
            >c [20 21 22 23 24 25 26 27]
        "#;

        let tree = parse_csm(csm).expect("Failed to parse CSM");
        let mesh = generate_mesh_with_mapper(&tree, &palette);

        assert!(!mesh.vertices.is_empty());
        assert!(!mesh.colors.is_empty());
        assert_eq!(mesh.vertices.len() / 3, mesh.colors.len() / 3);

        // Verify colors are from palette (not black or magenta error colors)
        let mut has_valid_colors = false;
        for i in 0..mesh.colors.len() / 3 {
            let r = mesh.colors[i * 3];
            let g = mesh.colors[i * 3 + 1];
            let b = mesh.colors[i * 3 + 2];

            // Check it's not black (0,0,0) or magenta error (1,0,1)
            if (r > 0.01 || g > 0.01 || b > 0.01) && !(r > 0.99 && g < 0.01 && b > 0.99) {
                has_valid_colors = true;
                break;
            }
        }
        assert!(has_valid_colors, "Mesh should have valid palette colors");
    }
}
