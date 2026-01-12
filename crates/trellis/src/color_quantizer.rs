/// Color palette for quantization
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// Palette colors in RGB [0, 1] range
    pub colors: Vec<[f32; 3]>,
}

impl ColorPalette {
    /// Create a new empty palette
    pub fn new() -> Self {
        Self { colors: Vec::new() }
    }

    /// Create a palette with a single color
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            colors: Vec::with_capacity(capacity),
        }
    }

    /// Add a color to the palette
    pub fn add_color(&mut self, color: [f32; 3]) {
        self.colors.push(color);
    }

    /// Find the nearest palette index for a given color
    pub fn nearest_index(&self, color: &[f32; 3]) -> u8 {
        if self.colors.is_empty() {
            return 0;
        }

        let mut best_index = 0;
        let mut best_distance = f32::MAX;

        for (i, palette_color) in self.colors.iter().enumerate() {
            let distance = color_distance(color, palette_color);
            if distance < best_distance {
                best_distance = distance;
                best_index = i;
            }
        }

        best_index.min(255) as u8
    }

    /// Get the number of colors in the palette
    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Check if the palette is empty
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate Euclidean distance between two colors in RGB space
fn color_distance(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];
    dr * dr + dg * dg + db * db
}

/// Color bucket for median cut algorithm
#[derive(Debug, Clone)]
struct ColorBucket {
    colors: Vec<[f32; 3]>,
}

impl ColorBucket {
    fn new() -> Self {
        Self { colors: Vec::new() }
    }

    fn add(&mut self, color: [f32; 3]) {
        self.colors.push(color);
    }

    fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    fn len(&self) -> usize {
        self.colors.len()
    }

    /// Calculate the average color of the bucket
    fn average_color(&self) -> [f32; 3] {
        if self.colors.is_empty() {
            return [0.0, 0.0, 0.0];
        }

        let mut sum = [0.0, 0.0, 0.0];
        for color in &self.colors {
            sum[0] += color[0];
            sum[1] += color[1];
            sum[2] += color[2];
        }

        let count = self.colors.len() as f32;
        [sum[0] / count, sum[1] / count, sum[2] / count]
    }

    /// Find the color channel with the largest range
    fn largest_range_channel(&self) -> usize {
        if self.colors.is_empty() {
            return 0;
        }

        let mut mins = [f32::MAX, f32::MAX, f32::MAX];
        let mut maxs = [f32::MIN, f32::MIN, f32::MIN];

        for color in &self.colors {
            for i in 0..3 {
                mins[i] = mins[i].min(color[i]);
                maxs[i] = maxs[i].max(color[i]);
            }
        }

        let ranges = [maxs[0] - mins[0], maxs[1] - mins[1], maxs[2] - mins[2]];

        if ranges[0] >= ranges[1] && ranges[0] >= ranges[2] {
            0
        } else if ranges[1] >= ranges[2] {
            1
        } else {
            2
        }
    }

    /// Split the bucket into two along the median of the largest range channel
    fn split(&mut self) -> Option<ColorBucket> {
        if self.colors.len() < 2 {
            return None;
        }

        let channel = self.largest_range_channel();

        // Sort by the selected channel
        self.colors.sort_by(|a, b| {
            a[channel]
                .partial_cmp(&b[channel])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Split at median
        let mid = self.colors.len() / 2;
        let right_colors = self.colors.split_off(mid);

        let mut right_bucket = ColorBucket::new();
        for color in right_colors {
            right_bucket.add(color);
        }

        Some(right_bucket)
    }
}

/// Quantize colors to a palette using the median cut algorithm
///
/// This algorithm recursively divides the color space into buckets,
/// splitting along the axis with the largest range until the desired
/// number of colors is reached.
pub fn quantize_colors(colors: &[[f32; 3]], max_colors: usize) -> ColorPalette {
    if colors.is_empty() {
        return ColorPalette::new();
    }

    // Clamp max_colors to valid range
    let max_colors = max_colors.clamp(1, 256);

    // If we have fewer unique colors than requested, just use them all
    // Note: We use a simple deduplication approach instead of HashSet
    // since f32 doesn't implement Hash/Eq
    let mut unique_colors: Vec<[f32; 3]> = Vec::new();
    for color in colors {
        let mut is_duplicate = false;
        for unique in &unique_colors {
            if (color[0] - unique[0]).abs() < 1e-6
                && (color[1] - unique[1]).abs() < 1e-6
                && (color[2] - unique[2]).abs() < 1e-6
            {
                is_duplicate = true;
                break;
            }
        }
        if !is_duplicate {
            unique_colors.push(*color);
        }
    }

    if unique_colors.len() <= max_colors {
        let mut palette = ColorPalette::with_capacity(unique_colors.len());
        for color in unique_colors {
            palette.add_color(color);
        }
        return palette;
    }

    // Initialize with all colors in one bucket
    let mut buckets = vec![ColorBucket::new()];
    for color in colors {
        buckets[0].add(*color);
    }

    // Iteratively split buckets until we have enough colors
    while buckets.len() < max_colors {
        // Find the largest bucket
        let largest_idx = buckets
            .iter()
            .enumerate()
            .max_by_key(|(_, b)| b.len())
            .map(|(i, _)| i)
            .unwrap();

        // Split the largest bucket
        if let Some(new_bucket) = buckets[largest_idx].split() {
            buckets.push(new_bucket);
        } else {
            // Can't split anymore, we're done
            break;
        }
    }

    // Create palette from bucket averages
    let mut palette = ColorPalette::with_capacity(buckets.len());
    for bucket in buckets {
        if !bucket.is_empty() {
            palette.add_color(bucket.average_color());
        }
    }

    palette
}

/// Fast approximate quantization using R2G3B2 encoding
///
/// This is a simpler, faster alternative to median cut that uses a fixed
/// 8-bit color space with 2 bits for red, 3 bits for green, and 2 bits for blue.
pub fn quantize_r2g3b2(color: &[f32; 3]) -> u8 {
    let r = (color[0].clamp(0.0, 1.0) * 3.0) as u8;
    let g = (color[1].clamp(0.0, 1.0) * 7.0) as u8;
    let b = (color[2].clamp(0.0, 1.0) * 3.0) as u8;

    // Material indices 128-255 are reserved for RGB encoding
    128 + ((r << 5) | (g << 2) | b)
}

/// Create a palette from R2G3B2 encoding
pub fn create_r2g3b2_palette() -> ColorPalette {
    let mut palette = ColorPalette::with_capacity(128);

    for i in 0..128 {
        let r = ((i >> 5) & 0b11) as f32 / 3.0;
        let g = ((i >> 2) & 0b111) as f32 / 7.0;
        let b = (i & 0b11) as f32 / 3.0;
        palette.add_color([r, g, b]);
    }

    palette
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_distance() {
        let a = [1.0, 0.0, 0.0];
        let b = [0.0, 0.0, 0.0];
        assert_eq!(color_distance(&a, &b), 1.0);

        let c = [0.5, 0.5, 0.5];
        assert!((color_distance(&a, &c) - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_palette_nearest() {
        let mut palette = ColorPalette::new();
        palette.add_color([1.0, 0.0, 0.0]); // Red
        palette.add_color([0.0, 1.0, 0.0]); // Green
        palette.add_color([0.0, 0.0, 1.0]); // Blue

        assert_eq!(palette.nearest_index(&[0.9, 0.1, 0.1]), 0); // Close to red
        assert_eq!(palette.nearest_index(&[0.1, 0.9, 0.1]), 1); // Close to green
        assert_eq!(palette.nearest_index(&[0.1, 0.1, 0.9]), 2); // Close to blue
    }

    #[test]
    fn test_bucket_average() {
        let mut bucket = ColorBucket::new();
        bucket.add([1.0, 0.0, 0.0]);
        bucket.add([0.0, 1.0, 0.0]);
        bucket.add([0.0, 0.0, 1.0]);

        let avg = bucket.average_color();
        assert!((avg[0] - 0.333).abs() < 0.01);
        assert!((avg[1] - 0.333).abs() < 0.01);
        assert!((avg[2] - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_quantize_colors() {
        let colors = vec![
            [1.0, 0.0, 0.0],
            [0.9, 0.1, 0.1],
            [0.0, 1.0, 0.0],
            [0.1, 0.9, 0.1],
            [0.0, 0.0, 1.0],
            [0.1, 0.1, 0.9],
        ];

        let palette = quantize_colors(&colors, 3);
        assert_eq!(palette.len(), 3);

        // Each color should map to one of the 3 palette colors
        for color in &colors {
            let idx = palette.nearest_index(color);
            assert!(idx < 3);
        }
    }

    #[test]
    fn test_quantize_r2g3b2() {
        // Pure red
        assert_eq!(quantize_r2g3b2(&[1.0, 0.0, 0.0]), 128 + (3 << 5));

        // Pure green
        assert_eq!(quantize_r2g3b2(&[0.0, 1.0, 0.0]), 128 + (7 << 2));

        // Pure blue
        assert_eq!(quantize_r2g3b2(&[0.0, 0.0, 1.0]), 128 + 3);

        // Black
        assert_eq!(quantize_r2g3b2(&[0.0, 0.0, 0.0]), 128);

        // White
        assert_eq!(
            quantize_r2g3b2(&[1.0, 1.0, 1.0]),
            128 + (3 << 5) + (7 << 2) + 3
        );
    }

    #[test]
    fn test_r2g3b2_palette() {
        let palette = create_r2g3b2_palette();
        assert_eq!(palette.len(), 128);

        // Check that black is at index 0
        assert_eq!(palette.colors[0], [0.0, 0.0, 0.0]);

        // Check that white is at the last index
        assert_eq!(palette.colors[127], [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_quantize_empty() {
        let palette = quantize_colors(&[], 256);
        assert_eq!(palette.len(), 0);
    }

    #[test]
    fn test_quantize_single_color() {
        let colors = vec![[0.5, 0.5, 0.5]];
        let palette = quantize_colors(&colors, 256);
        assert_eq!(palette.len(), 1);
        assert_eq!(palette.colors[0], [0.5, 0.5, 0.5]);
    }
}
