/// Trait for mapping voxel indices to RGB colors
pub trait ColorMapper {
    fn map(&self, index: u8) -> [f32; 3];
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
    fn map(&self, index: u8) -> [f32; 3] {
        if index == 0 {
            [0.0, 0.0, 0.0] // Black for zero
        } else {
            let hue = (index as u32 % 360) as f32;
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
    fn map(&self, index: u8) -> [f32; 3] {
        if self.colors.is_empty() {
            return [1.0, 0.0, 1.0]; // Magenta for error
        }

        if index == 0 {
            return [0.0, 0.0, 0.0]; // Black for zero
        }

        let idx = ((index - 1) as usize) % self.colors.len();
        self.colors[idx]
    }
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

/// Vox model color mapper that decodes R2G3B2 encoding (range 128-255)
pub struct VoxColorMapper;

impl VoxColorMapper {
    pub fn new() -> Self {
        VoxColorMapper
    }
}

impl Default for VoxColorMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorMapper for VoxColorMapper {
    fn map(&self, index: u8) -> [f32; 3] {
        let color = crate::material::get_material_color(index as i32);
        [color.x, color.y, color.z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
