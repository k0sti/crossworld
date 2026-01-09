//! Color palette for the voxel editor
//!
//! Provides three palette systems:
//! - `MaterialPalette`: Named materials from MATERIAL_REGISTRY (indices 0-127)
//! - `ColorPalette`: R2G3B2 encoded colors (indices 128-255)
//! - `ModelPalette`: Loaded VOX models with metadata
//!
//! The R2G3B2 encoding uses 2 bits for red, 3 bits for green, and 2 bits for blue,
//! giving 128 unique colors (4 × 8 × 4 = 128).

use cube::io::vox::load_vox_to_cubebox_compact;
use cube::material::{Material, MATERIAL_REGISTRY};
use cube::CubeBox;
use glam::{IVec3, Vec3};
use image::{ImageBuffer, Rgb};

/// Color entry in the palette
#[derive(Debug, Clone, Copy)]
pub struct PaletteColor {
    /// Material index (128-255)
    pub index: u8,
    /// Normalized RGB color (0.0-1.0)
    pub color: Vec3,
    /// R2G3B2 component values
    pub r_bits: u8,
    pub g_bits: u8,
    pub b_bits: u8,
}

impl PaletteColor {
    /// Create a new palette color from R2G3B2 components
    pub const fn new(r_bits: u8, g_bits: u8, b_bits: u8) -> Self {
        let index = (128 + ((r_bits & 0b11) << 5)) | ((g_bits & 0b111) << 2) | (b_bits & 0b11);
        let color = decode_r2g3b2_const(r_bits, g_bits, b_bits);
        Self {
            index,
            color,
            r_bits,
            g_bits,
            b_bits,
        }
    }

    /// Create a palette color from a material index (128-255)
    pub const fn from_index(index: u8) -> Self {
        let bits = index.saturating_sub(128);
        let r_bits = (bits >> 5) & 0b11;
        let g_bits = (bits >> 2) & 0b111;
        let b_bits = bits & 0b11;
        let color = decode_r2g3b2_const(r_bits, g_bits, b_bits);
        Self {
            index,
            color,
            r_bits,
            g_bits,
            b_bits,
        }
    }
}

/// Material palette for MATERIAL_REGISTRY materials (indices 0-127)
///
/// Provides selection and organization of named materials like stone, wood, metals, etc.
#[derive(Debug, Clone)]
pub struct MaterialPalette {
    /// Currently selected material index (0-127)
    selected_index: usize,
}

impl Default for MaterialPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl MaterialPalette {
    /// Number of materials in the registry
    pub const PALETTE_SIZE: usize = 128;

    /// First material index
    pub const FIRST_INDEX: u8 = 0;

    /// Last material index
    pub const LAST_INDEX: u8 = 127;

    /// Create a new material palette with stone (index 20) selected by default
    pub fn new() -> Self {
        Self { selected_index: 20 } // Default to stone
    }

    /// Get the currently selected material
    pub fn selected(&self) -> &'static Material {
        self.get(self.selected_index)
    }

    /// Get the selected index within the palette (0-127)
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get the material index of the selected material (0-127)
    pub fn selected_material_index(&self) -> u8 {
        self.selected_index as u8
    }

    /// Set the selected material by palette index (0-127)
    pub fn select(&mut self, index: usize) {
        if index < Self::PALETTE_SIZE {
            self.selected_index = index;
        }
    }

    /// Set the selected material by material index (0-127)
    pub fn select_by_material(&mut self, material_index: u8) {
        if material_index <= Self::LAST_INDEX {
            self.selected_index = material_index as usize;
        }
    }

    /// Get a material by palette index (0-127)
    pub fn get(&self, index: usize) -> &'static Material {
        let clamped = index.min(Self::PALETTE_SIZE - 1);
        &MATERIAL_REGISTRY[clamped]
    }

    /// Get a material by material index (0-127)
    pub fn get_by_material(&self, material_index: u8) -> Option<&'static Material> {
        if material_index <= Self::LAST_INDEX {
            Some(&MATERIAL_REGISTRY[material_index as usize])
        } else {
            None
        }
    }

    /// Find a material by its string ID
    pub fn find_by_id(&self, id: &str) -> Option<&'static Material> {
        MATERIAL_REGISTRY.iter().find(|m| m.id == id)
    }

    /// Iterate over all materials
    pub fn iter(&self) -> impl Iterator<Item = &'static Material> {
        MATERIAL_REGISTRY.iter()
    }

    /// Check if a material index is a registry material
    pub fn is_material(material_index: u8) -> bool {
        material_index <= Self::LAST_INDEX
    }

    /// Get materials in a specific category
    ///
    /// Categories:
    /// - "transparent": indices 0-15 (empty, glass, etc.)
    /// - "terrain": indices 16-35 (dirt, stone, sand, etc.)
    /// - "wood": indices 36-47 (oak, spruce, leaves, etc.)
    /// - "metal": indices 48-55 (coal, iron, gold, etc.)
    /// - "building": indices 56-63 (brick, concrete, etc.)
    /// - "organic": indices 64-95 (skin, fabric, plants, etc.)
    /// - "misc": indices 96-115 (bone, magma, paper, etc.)
    /// - "gems": indices 116-127 (glowstone, emerald, diamond, etc.)
    pub fn get_category(&self, category: &str) -> Vec<&'static Material> {
        let range = match category {
            "transparent" => 0..16,
            "terrain" => 16..36,
            "wood" => 36..48,
            "metal" => 48..56,
            "building" => 56..64,
            "organic" => 64..96,
            "misc" => 96..116,
            "gems" => 116..128,
            _ => 0..0,
        };
        range.map(|i| &MATERIAL_REGISTRY[i]).collect()
    }

    /// Get all category names
    pub fn categories() -> &'static [&'static str] {
        &[
            "transparent",
            "terrain",
            "wood",
            "metal",
            "building",
            "organic",
            "misc",
            "gems",
        ]
    }

    /// Get the category for a material index
    pub fn category_for_index(index: u8) -> &'static str {
        match index {
            0..=15 => "transparent",
            16..=35 => "terrain",
            36..=47 => "wood",
            48..=55 => "metal",
            56..=63 => "building",
            64..=95 => "organic",
            96..=115 => "misc",
            116..=127 => "gems",
            _ => "unknown",
        }
    }
}

/// Decode R2G3B2 components to normalized RGB color (const fn compatible)
const fn decode_r2g3b2_const(r_bits: u8, g_bits: u8, b_bits: u8) -> Vec3 {
    // 2-bit expansion: 0 -> 0.0, 1 -> 0.333, 2 -> 0.667, 3 -> 1.0
    let r = match r_bits & 0b11 {
        0 => 0.0,
        1 => 0.333,
        2 => 0.667,
        3 => 1.0,
        _ => 0.0,
    };

    // 3-bit expansion: 0 -> 0.0, 1 -> 0.143, ..., 7 -> 1.0
    let g = match g_bits & 0b111 {
        0 => 0.0,
        1 => 0.143,
        2 => 0.286,
        3 => 0.429,
        4 => 0.571,
        5 => 0.714,
        6 => 0.857,
        7 => 1.0,
        _ => 0.0,
    };

    // 2-bit expansion: 0 -> 0.0, 1 -> 0.333, 2 -> 0.667, 3 -> 1.0
    let b = match b_bits & 0b11 {
        0 => 0.0,
        1 => 0.333,
        2 => 0.667,
        3 => 1.0,
        _ => 0.0,
    };

    Vec3::new(r, g, b)
}

/// Color palette for R2G3B2 encoded colors (indices 128-255)
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// Currently selected color index
    selected_index: usize,
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorPalette {
    /// Number of colors in the R2G3B2 palette
    pub const PALETTE_SIZE: usize = 128;

    /// First material index for R2G3B2 colors
    pub const FIRST_INDEX: u8 = 128;

    /// Last material index for R2G3B2 colors
    pub const LAST_INDEX: u8 = 255;

    /// Create a new color palette with first color selected
    pub fn new() -> Self {
        Self { selected_index: 0 }
    }

    /// Get the currently selected palette color
    pub fn selected(&self) -> PaletteColor {
        self.get(self.selected_index)
    }

    /// Get the selected index within the palette (0-127)
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Get the material index of the selected color (128-255)
    pub fn selected_material_index(&self) -> u8 {
        Self::FIRST_INDEX + self.selected_index as u8
    }

    /// Set the selected color by palette index (0-127)
    pub fn select(&mut self, index: usize) {
        if index < Self::PALETTE_SIZE {
            self.selected_index = index;
        }
    }

    /// Set the selected color by material index (128-255)
    pub fn select_by_material(&mut self, material_index: u8) {
        if material_index >= Self::FIRST_INDEX {
            let index = (material_index - Self::FIRST_INDEX) as usize;
            if index < Self::PALETTE_SIZE {
                self.selected_index = index;
            }
        }
    }

    /// Get a color by palette index (0-127)
    pub fn get(&self, index: usize) -> PaletteColor {
        let material_index = Self::FIRST_INDEX + (index as u8).min(127);
        PaletteColor::from_index(material_index)
    }

    /// Get a color by material index (128-255)
    pub fn get_by_material(&self, material_index: u8) -> Option<PaletteColor> {
        if material_index >= Self::FIRST_INDEX {
            Some(PaletteColor::from_index(material_index))
        } else {
            None
        }
    }

    /// Iterate over all palette colors
    pub fn iter(&self) -> impl Iterator<Item = PaletteColor> {
        (0..Self::PALETTE_SIZE).map(|i| PaletteColor::from_index(Self::FIRST_INDEX + i as u8))
    }

    /// Get colors organized by rows (for grid display)
    /// Returns colors grouped by red value (4 rows of 32 colors each)
    pub fn rows_by_red(&self) -> [[PaletteColor; 32]; 4] {
        let mut rows = [[PaletteColor::from_index(128); 32]; 4];
        for r in 0..4u8 {
            for idx in 0..32u8 {
                let g = (idx >> 2) & 0b111;
                let b = idx & 0b11;
                rows[r as usize][idx as usize] = PaletteColor::new(r, g, b);
            }
        }
        rows
    }

    /// Get colors organized by hue-like grouping
    /// Returns colors in a visually organized grid (8 rows of 16 colors)
    pub fn rows_by_hue(&self) -> [[PaletteColor; 16]; 8] {
        let mut rows = [[PaletteColor::from_index(128); 16]; 8];
        for g in 0..8u8 {
            for idx in 0..16u8 {
                let r = (idx >> 2) & 0b11;
                let b = idx & 0b11;
                rows[g as usize][idx as usize] = PaletteColor::new(r, g, b);
            }
        }
        rows
    }

    /// Check if a material index is a palette color
    pub fn is_palette_color(material_index: u8) -> bool {
        material_index >= Self::FIRST_INDEX
    }
}

/// A VOX model entry with lazy loading support
#[derive(Debug, Clone)]
pub struct PaletteModel {
    /// Unique identifier for the model within the palette
    pub id: usize,
    /// Display name for the model (typically filename without extension)
    pub name: String,
    /// File path to the .vox file
    pub file_path: std::path::PathBuf,
    /// Cached size (loaded on demand)
    pub size: Option<IVec3>,
    /// The loaded CubeBox containing the voxel data (None if not loaded)
    pub cubebox: Option<CubeBox<u8>>,
    /// Thumbnail image (generated on demand)
    pub thumbnail: Option<ImageBuffer<Rgb<u8>, Vec<u8>>>,
}

impl PaletteModel {
    /// Create a new palette model entry (not yet loaded)
    pub fn new(id: usize, name: impl Into<String>, file_path: std::path::PathBuf) -> Self {
        // Generate placeholder thumbnail
        let hue = (id as u8).wrapping_mul(37); // Spread colors across hue spectrum
        let placeholder = renderer::thumbnail::generate_placeholder(
            renderer::thumbnail::DEFAULT_THUMBNAIL_SIZE,
            hue,
        );

        Self {
            id,
            name: name.into(),
            file_path,
            size: None,
            cubebox: None,
            thumbnail: Some(placeholder),
        }
    }

    /// Get the model dimensions (size in voxels), loading if necessary
    pub fn size(&self) -> IVec3 {
        self.size.unwrap_or(IVec3::ZERO)
    }

    /// Check if the model data is loaded in memory
    pub fn is_loaded(&self) -> bool {
        self.cubebox.is_some()
    }

    /// Load only the size from disk (lightweight)
    pub fn load_size(&mut self) -> Result<(), String> {
        if self.size.is_some() {
            return Ok(());
        }

        let bytes = std::fs::read(&self.file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let cubebox = load_vox_to_cubebox_compact(&bytes)?;
        self.size = Some(cubebox.size);

        Ok(())
    }

    /// Load size and generate thumbnail
    pub fn load_size_and_thumbnail(&mut self) -> Result<(), String> {
        if self.size.is_some() {
            // Already loaded, but might still have placeholder thumbnail
            if self.cubebox.is_some() {
                return Ok(());
            }
        }

        // Simulate loading delay for testing
        std::thread::sleep(std::time::Duration::from_millis(10));

        let bytes = std::fs::read(&self.file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let cubebox = load_vox_to_cubebox_compact(&bytes)?;
        self.size = Some(cubebox.size);

        // Generate thumbnail using renderer
        let cube_rc = std::rc::Rc::new(cubebox.cube.clone());
        let thumbnail = renderer::thumbnail::generate_thumbnail_default(cube_rc);
        self.thumbnail = Some(thumbnail);

        Ok(())
    }

    /// Load the model data from disk
    pub fn load(&mut self) -> Result<(), String> {
        if self.is_loaded() {
            return Ok(());
        }

        let bytes = std::fs::read(&self.file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let cubebox = load_vox_to_cubebox_compact(&bytes)?;
        self.size = Some(cubebox.size);
        self.cubebox = Some(cubebox);

        Ok(())
    }

    /// Unload the model data from memory (keep metadata)
    pub fn unload(&mut self) {
        self.cubebox = None;
    }
}

/// Model palette for loaded VOX models
///
/// Stores loaded MagicaVoxel (.vox) models with metadata for use in the editor.
/// Models can be placed into the editing cube using the cursor system.
#[derive(Debug, Clone, Default)]
pub struct ModelPalette {
    /// Loaded models
    models: Vec<PaletteModel>,
    /// Currently selected model index (None if no models loaded)
    selected_index: Option<usize>,
    /// Next ID to assign to a model
    next_id: usize,
}

impl ModelPalette {
    /// Maximum number of models that can be loaded
    pub const MAX_MODELS: usize = 2048;

    /// Create a new empty model palette
    pub fn new() -> Self {
        Self {
            models: Vec::new(),
            selected_index: None,
            next_id: 0,
        }
    }

    /// Get the number of loaded models
    pub fn len(&self) -> usize {
        self.models.len()
    }

    /// Check if the palette is empty
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }

    /// Get the currently selected model (if any)
    pub fn selected(&self) -> Option<&PaletteModel> {
        self.selected_index.and_then(|idx| self.models.get(idx))
    }

    /// Get the selected index within the palette (None if no selection)
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set the selected model by palette index
    pub fn select(&mut self, index: usize) {
        if index < self.models.len() {
            self.selected_index = Some(index);
        }
    }

    /// Clear the selection
    pub fn clear_selection(&mut self) {
        self.selected_index = None;
    }

    /// Add a VOX model entry to the palette (without loading it)
    ///
    /// # Arguments
    /// * `file_path` - Path to the .vox file
    /// * `name` - Display name for the model
    ///
    /// # Returns
    /// The ID of the newly added model, or an error string
    pub fn add_model(&mut self, file_path: std::path::PathBuf, name: impl Into<String>) -> Result<usize, String> {
        if self.models.len() >= Self::MAX_MODELS {
            return Err(format!(
                "Cannot add more than {} models",
                Self::MAX_MODELS
            ));
        }

        let id = self.next_id;
        self.next_id += 1;

        let model = PaletteModel::new(id, name, file_path);
        self.models.push(model);

        // Auto-select the first loaded model
        if self.selected_index.is_none() {
            self.selected_index = Some(self.models.len() - 1);
        }

        Ok(id)
    }

    /// Get a mutable reference to the currently selected model
    pub fn selected_mut(&mut self) -> Option<&mut PaletteModel> {
        self.selected_index.and_then(|idx| self.models.get_mut(idx))
    }

    /// Get a model by palette index
    pub fn get(&self, index: usize) -> Option<&PaletteModel> {
        self.models.get(index)
    }

    /// Get a mutable reference to a model by palette index
    pub fn get_mut(&mut self, index: usize) -> Option<&mut PaletteModel> {
        self.models.get_mut(index)
    }

    /// Unload all models that are not currently selected
    /// Call this when keep_models_in_memory is false
    pub fn unload_unused_models(&mut self) {
        for (idx, model) in self.models.iter_mut().enumerate() {
            if Some(idx) != self.selected_index {
                model.unload();
            }
        }
    }

    /// Load sizes for all models in the background (batch operation)
    /// Returns the number of models whose sizes were loaded
    pub fn load_all_sizes(&mut self) -> usize {
        let mut loaded = 0;
        for model in &mut self.models {
            if model.size.is_none() {
                if model.load_size().is_ok() {
                    loaded += 1;
                }
            }
        }
        loaded
    }

    /// Load sizes and generate thumbnails for all models
    /// Returns the number of models processed
    pub fn load_all_thumbnails(&mut self) -> usize {
        let mut loaded = 0;
        for model in &mut self.models {
            if model.thumbnail.is_none() {
                if model.load_size_and_thumbnail().is_ok() {
                    loaded += 1;
                }
            }
        }
        loaded
    }

    /// Get a model by its unique ID
    pub fn get_by_id(&self, id: usize) -> Option<&PaletteModel> {
        self.models.iter().find(|m| m.id == id)
    }

    /// Get a mutable reference to a model by its unique ID
    pub fn get_model_by_id_mut(&mut self, id: usize) -> Option<&mut PaletteModel> {
        self.models.iter_mut().find(|m| m.id == id)
    }

    /// Find a model by name (case-insensitive)
    pub fn find_by_name(&self, name: &str) -> Option<&PaletteModel> {
        let name_lower = name.to_lowercase();
        self.models.iter().find(|m| m.name.to_lowercase() == name_lower)
    }

    /// Remove a model by palette index
    ///
    /// Returns the removed model if successful
    pub fn remove(&mut self, index: usize) -> Option<PaletteModel> {
        if index >= self.models.len() {
            return None;
        }

        let removed = self.models.remove(index);

        // Adjust selection if needed
        if let Some(selected) = self.selected_index {
            if selected == index {
                // Selected model was removed
                self.selected_index = if self.models.is_empty() {
                    None
                } else {
                    Some(selected.min(self.models.len() - 1))
                };
            } else if selected > index {
                // Selection shifts down
                self.selected_index = Some(selected - 1);
            }
        }

        Some(removed)
    }

    /// Remove a model by its unique ID
    ///
    /// Returns the removed model if successful
    pub fn remove_by_id(&mut self, id: usize) -> Option<PaletteModel> {
        if let Some(index) = self.models.iter().position(|m| m.id == id) {
            self.remove(index)
        } else {
            None
        }
    }

    /// Clear all models from the palette
    pub fn clear(&mut self) {
        self.models.clear();
        self.selected_index = None;
    }

    /// Iterate over all models
    pub fn iter(&self) -> impl Iterator<Item = &PaletteModel> {
        self.models.iter()
    }

    /// Get models sorted by name
    pub fn sorted_by_name(&self) -> Vec<&PaletteModel> {
        let mut sorted: Vec<_> = self.models.iter().collect();
        sorted.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        sorted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_palette_color_from_index() {
        // Black (index 128)
        let black = PaletteColor::from_index(128);
        assert_eq!(black.index, 128);
        assert_eq!(black.r_bits, 0);
        assert_eq!(black.g_bits, 0);
        assert_eq!(black.b_bits, 0);
        assert!((black.color.x - 0.0).abs() < 0.01);
        assert!((black.color.y - 0.0).abs() < 0.01);
        assert!((black.color.z - 0.0).abs() < 0.01);

        // White (index 255)
        let white = PaletteColor::from_index(255);
        assert_eq!(white.index, 255);
        assert_eq!(white.r_bits, 3);
        assert_eq!(white.g_bits, 7);
        assert_eq!(white.b_bits, 3);
        assert!((white.color.x - 1.0).abs() < 0.01);
        assert!((white.color.y - 1.0).abs() < 0.01);
        assert!((white.color.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_palette_color_new() {
        // Pure red: r=3, g=0, b=0 => index = 128 + (3<<5) = 224
        let red = PaletteColor::new(3, 0, 0);
        assert_eq!(red.index, 224);
        assert!((red.color.x - 1.0).abs() < 0.01);
        assert!((red.color.y - 0.0).abs() < 0.01);
        assert!((red.color.z - 0.0).abs() < 0.01);

        // Pure green: r=0, g=7, b=0 => index = 128 + (7<<2) = 156
        let green = PaletteColor::new(0, 7, 0);
        assert_eq!(green.index, 156);
        assert!((green.color.x - 0.0).abs() < 0.01);
        assert!((green.color.y - 1.0).abs() < 0.01);
        assert!((green.color.z - 0.0).abs() < 0.01);

        // Pure blue: r=0, g=0, b=3 => index = 128 + 3 = 131
        let blue = PaletteColor::new(0, 0, 3);
        assert_eq!(blue.index, 131);
        assert!((blue.color.x - 0.0).abs() < 0.01);
        assert!((blue.color.y - 0.0).abs() < 0.01);
        assert!((blue.color.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_palette_selection() {
        let mut palette = ColorPalette::new();

        // Default selection is 0
        assert_eq!(palette.selected_index(), 0);
        assert_eq!(palette.selected_material_index(), 128);

        // Select by palette index
        palette.select(64);
        assert_eq!(palette.selected_index(), 64);
        assert_eq!(palette.selected_material_index(), 192);

        // Select by material index
        palette.select_by_material(200);
        assert_eq!(palette.selected_index(), 72);
        assert_eq!(palette.selected_material_index(), 200);

        // Out of bounds selection should be ignored
        palette.select(200);
        assert_eq!(palette.selected_index(), 72); // Unchanged
    }

    #[test]
    fn test_color_palette_iter() {
        let palette = ColorPalette::new();
        let colors: Vec<_> = palette.iter().collect();
        assert_eq!(colors.len(), 128);
        assert_eq!(colors[0].index, 128);
        assert_eq!(colors[127].index, 255);
    }

    #[test]
    fn test_is_palette_color() {
        assert!(!ColorPalette::is_palette_color(0));
        assert!(!ColorPalette::is_palette_color(127));
        assert!(ColorPalette::is_palette_color(128));
        assert!(ColorPalette::is_palette_color(200));
        assert!(ColorPalette::is_palette_color(255));
    }

    #[test]
    fn test_rows_by_red() {
        let palette = ColorPalette::new();
        let rows = palette.rows_by_red();

        // Row 0 should have r_bits = 0
        for color in &rows[0] {
            assert_eq!(color.r_bits, 0);
        }

        // Row 3 should have r_bits = 3
        for color in &rows[3] {
            assert_eq!(color.r_bits, 3);
        }
    }

    #[test]
    fn test_rows_by_hue() {
        let palette = ColorPalette::new();
        let rows = palette.rows_by_hue();

        // Row 0 should have g_bits = 0
        for color in &rows[0] {
            assert_eq!(color.g_bits, 0);
        }

        // Row 7 should have g_bits = 7
        for color in &rows[7] {
            assert_eq!(color.g_bits, 7);
        }
    }

    #[test]
    fn test_material_palette_new() {
        let palette = MaterialPalette::new();
        // Default selection is stone (index 20)
        assert_eq!(palette.selected_index(), 20);
        assert_eq!(palette.selected_material_index(), 20);
        assert_eq!(palette.selected().id, "stone");
    }

    #[test]
    fn test_material_palette_selection() {
        let mut palette = MaterialPalette::new();

        // Select by palette index
        palette.select(18);
        assert_eq!(palette.selected_index(), 18);
        assert_eq!(palette.selected().id, "dirt");

        // Select by material index
        palette.select_by_material(50);
        assert_eq!(palette.selected_index(), 50);
        assert_eq!(palette.selected().id, "gold");

        // Out of bounds selection should be ignored
        palette.select(200);
        assert_eq!(palette.selected_index(), 50); // Unchanged
    }

    #[test]
    fn test_material_palette_get() {
        let palette = MaterialPalette::new();

        // Get by index
        let stone = palette.get(20);
        assert_eq!(stone.id, "stone");
        assert_eq!(stone.index, 20);

        // Get by material index
        let glass = palette.get_by_material(2);
        assert!(glass.is_some());
        assert_eq!(glass.unwrap().id, "glass");

        // Invalid material index
        let invalid = palette.get_by_material(128);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_material_palette_find_by_id() {
        let palette = MaterialPalette::new();

        let diamond = palette.find_by_id("diamond");
        assert!(diamond.is_some());
        assert_eq!(diamond.unwrap().index, 119);

        let nonexistent = palette.find_by_id("unobtainium");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_material_palette_iter() {
        let palette = MaterialPalette::new();
        let materials: Vec<_> = palette.iter().collect();
        assert_eq!(materials.len(), 128);
        assert_eq!(materials[0].id, "empty");
        assert_eq!(materials[127].id, "reserved_127");
    }

    #[test]
    fn test_material_palette_categories() {
        let palette = MaterialPalette::new();

        // Test category retrieval
        let terrain = palette.get_category("terrain");
        assert!(!terrain.is_empty());
        assert!(terrain.iter().any(|m| m.id == "stone"));
        assert!(terrain.iter().any(|m| m.id == "dirt"));

        let metals = palette.get_category("metal");
        assert!(!metals.is_empty());
        assert!(metals.iter().any(|m| m.id == "gold"));
        assert!(metals.iter().any(|m| m.id == "iron"));

        // Test category for index
        assert_eq!(MaterialPalette::category_for_index(20), "terrain");
        assert_eq!(MaterialPalette::category_for_index(50), "metal");
        assert_eq!(MaterialPalette::category_for_index(119), "gems");
    }

    #[test]
    fn test_is_material() {
        assert!(MaterialPalette::is_material(0));
        assert!(MaterialPalette::is_material(127));
        assert!(!MaterialPalette::is_material(128));
        assert!(!MaterialPalette::is_material(255));
    }

    // ModelPalette tests

    #[test]
    fn test_model_palette_new() {
        let palette = ModelPalette::new();
        assert!(palette.is_empty());
        assert_eq!(palette.len(), 0);
        assert!(palette.selected().is_none());
        assert!(palette.selected_index().is_none());
    }

    #[test]
    fn test_palette_model_new() {
        let path = std::path::PathBuf::from("test_model.vox");
        let model = PaletteModel::new(42, "test_model", path.clone());

        assert_eq!(model.id, 42);
        assert_eq!(model.name, "test_model");
        assert_eq!(model.file_path, path);
        assert!(!model.is_loaded());
        assert_eq!(model.size(), IVec3::ZERO);
    }

    #[test]
    fn test_model_palette_with_cubebox() {
        let palette = ModelPalette::new();

        // We'll test the internal structure by checking basic operations
        assert!(palette.is_empty());

        // Test that a new palette has no selection
        assert!(palette.selected().is_none());
    }

    #[test]
    fn test_model_palette_selection() {
        let mut palette = ModelPalette::new();

        // Test selection on empty palette
        palette.select(0);
        assert!(palette.selected().is_none());

        // Test clear selection
        palette.clear_selection();
        assert!(palette.selected().is_none());
    }

    #[test]
    fn test_model_palette_find_by_name() {
        let palette = ModelPalette::new();

        // Test find on empty palette
        assert!(palette.find_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_model_palette_get_by_id() {
        let palette = ModelPalette::new();

        // Test get_by_id on empty palette
        assert!(palette.get_by_id(0).is_none());
        assert!(palette.get_by_id(999).is_none());
    }

    #[test]
    fn test_model_palette_remove_empty() {
        let mut palette = ModelPalette::new();

        // Test remove on empty palette
        assert!(palette.remove(0).is_none());
        assert!(palette.remove_by_id(0).is_none());
    }

    #[test]
    fn test_model_palette_clear() {
        let mut palette = ModelPalette::new();

        // Clear should work on empty palette
        palette.clear();
        assert!(palette.is_empty());
        assert!(palette.selected().is_none());
    }

    #[test]
    fn test_model_palette_iter_empty() {
        let palette = ModelPalette::new();

        let models: Vec<_> = palette.iter().collect();
        assert!(models.is_empty());
    }

    #[test]
    fn test_model_palette_sorted_empty() {
        let palette = ModelPalette::new();

        let sorted = palette.sorted_by_name();
        assert!(sorted.is_empty());
    }

    #[test]
    fn test_model_palette_max_models() {
        // Just verify the constant is reasonable
        assert!(ModelPalette::MAX_MODELS >= 256);
        assert!(ModelPalette::MAX_MODELS <= 4096);
    }
}
