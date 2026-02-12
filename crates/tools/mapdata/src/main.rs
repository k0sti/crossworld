use anyhow::{Context, Result};
use clap::Parser;
use image::{ImageBuffer, Rgb, RgbImage};
use jpeg2k::Image as Jp2Image;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read as IoRead, Write as IoWrite};
use std::path::PathBuf;
use tiff::decoder::{Decoder, DecodingResult};

/// Magic bytes for binary format identification
const MAGIC: &[u8; 4] = b"MAPG";
/// Binary format version
const VERSION: u32 = 1;

/// Grid cell containing height and color information
#[derive(Clone, Debug, Default)]
pub struct GridCell {
    /// Height in meters
    pub height: f32,
    /// RGB color from orthophoto
    pub color: [u8; 3],
}

/// Georeferenced bounds
#[derive(Clone, Debug, Default)]
pub struct Bounds {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// Grid of map data combining height and color
pub struct MapGrid {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<GridCell>,
    /// Minimum height in the dataset
    pub min_height: f32,
    /// Maximum height in the dataset
    pub max_height: f32,
    /// Resolution in meters per pixel
    pub resolution: f64,
    /// Georeferenced bounds (ETRS-TM35FIN coordinates)
    pub bounds: Bounds,
}

impl MapGrid {
    /// Get cell at (x, y) position
    pub fn get(&self, x: usize, y: usize) -> Option<&GridCell> {
        if x < self.width && y < self.height {
            Some(&self.cells[y * self.width + x])
        } else {
            None
        }
    }

    /// Get mutable cell at (x, y) position
    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut GridCell> {
        if x < self.width && y < self.height {
            Some(&mut self.cells[y * self.width + x])
        } else {
            None
        }
    }
}

/// Load height data from a GeoTIFF file
fn load_height_map_tiff(path: &PathBuf) -> Result<MapGrid> {
    let file = File::open(path).context("Failed to open height map file")?;
    let mut decoder = Decoder::new(file).context("Failed to create TIFF decoder")?;

    let (width, height) = decoder.dimensions().context("Failed to get dimensions")?;
    let width = width as usize;
    let height = height as usize;

    println!("Loading TIFF height map: {}x{}", width, height);

    let result = decoder.read_image().context("Failed to read TIFF image")?;

    let heights: Vec<f32> = match result {
        DecodingResult::F32(data) => data,
        DecodingResult::F64(data) => data.into_iter().map(|v| v as f32).collect(),
        DecodingResult::U16(data) => data.into_iter().map(|v| v as f32).collect(),
        DecodingResult::U32(data) => data.into_iter().map(|v| v as f32).collect(),
        DecodingResult::U8(data) => data.into_iter().map(|v| v as f32).collect(),
        _ => anyhow::bail!("Unsupported TIFF format"),
    };

    // Find min/max heights
    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;
    for &h in &heights {
        if h.is_finite() {
            min_height = min_height.min(h);
            max_height = max_height.max(h);
        }
    }

    println!("Height range: {:.2}m - {:.2}m", min_height, max_height);

    // Create grid cells
    let cells: Vec<GridCell> = heights
        .into_iter()
        .map(|h| GridCell {
            height: h,
            color: [0, 0, 0],
        })
        .collect();

    Ok(MapGrid {
        width,
        height,
        cells,
        min_height,
        max_height,
        resolution: 2.0, // Default for TIFF (unknown)
        bounds: Bounds::default(),
    })
}

/// Load height data from a LAZ/LAS point cloud file and rasterize to grid
fn load_height_map_laz(path: &PathBuf, resolution: f64) -> Result<MapGrid> {
    let mut reader = las::Reader::from_path(path).context("Failed to open LAZ file")?;

    let header = reader.header();
    let bounds = header.bounds();
    let point_count = header.number_of_points();

    println!("Loading LAZ point cloud: {} points", point_count);
    println!(
        "Bounds: X [{:.2}, {:.2}], Y [{:.2}, {:.2}], Z [{:.2}, {:.2}]",
        bounds.min.x, bounds.max.x, bounds.min.y, bounds.max.y, bounds.min.z, bounds.max.z
    );

    // Calculate grid dimensions based on bounds and resolution
    let extent_x = bounds.max.x - bounds.min.x;
    let extent_y = bounds.max.y - bounds.min.y;
    let width = (extent_x / resolution).ceil() as usize;
    let height = (extent_y / resolution).ceil() as usize;

    println!(
        "Rasterizing to {}x{} grid at {:.2}m resolution",
        width, height, resolution
    );
    println!(
        "Coverage: {:.1}m x {:.1}m",
        width as f64 * resolution,
        height as f64 * resolution
    );

    // Create accumulator grid (sum of heights and count for averaging)
    let mut height_sum: Vec<f64> = vec![0.0; width * height];
    let mut height_count: Vec<u32> = vec![0; width * height];

    // Process all points
    let mut processed = 0u64;
    for point_result in reader.points() {
        let point = point_result.context("Failed to read point")?;

        // Calculate grid position
        let gx = ((point.x - bounds.min.x) / resolution) as usize;
        let gy = ((bounds.max.y - point.y) / resolution) as usize; // Flip Y for image coords

        if gx < width && gy < height {
            let idx = gy * width + gx;
            height_sum[idx] += point.z;
            height_count[idx] += 1;
        }

        processed += 1;
        if processed.is_multiple_of(1_000_000) {
            println!("  Processed {} / {} points...", processed, point_count);
        }
    }

    println!("Processed {} points total", processed);

    // Convert to average heights and find min/max
    let mut min_height = f32::MAX;
    let mut max_height = f32::MIN;
    let mut cells_with_data = 0usize;

    let cells: Vec<GridCell> = height_sum
        .iter()
        .zip(height_count.iter())
        .map(|(&sum, &count)| {
            let h = if count > 0 {
                cells_with_data += 1;
                let avg = (sum / count as f64) as f32;
                min_height = min_height.min(avg);
                max_height = max_height.max(avg);
                avg
            } else {
                f32::NAN // No data
            };
            GridCell {
                height: h,
                color: [0, 0, 0],
            }
        })
        .collect();

    println!(
        "Height range: {:.2}m - {:.2}m ({} cells with data, {} empty)",
        min_height,
        max_height,
        cells_with_data,
        width * height - cells_with_data
    );

    // Fill empty cells with interpolation from neighbors
    let mut grid = MapGrid {
        width,
        height,
        cells,
        min_height,
        max_height,
        resolution,
        bounds: Bounds {
            min_x: bounds.min.x,
            max_x: bounds.max.x,
            min_y: bounds.min.y,
            max_y: bounds.max.y,
        },
    };
    fill_empty_cells(&mut grid);

    Ok(grid)
}

/// Fill empty (NaN) cells by interpolating from nearest neighbors
fn fill_empty_cells(grid: &mut MapGrid) {
    let mut filled = 0usize;

    // Simple approach: iterate until no more cells can be filled
    loop {
        let mut changed = false;

        for y in 0..grid.height {
            for x in 0..grid.width {
                let idx = y * grid.width + x;
                if grid.cells[idx].height.is_nan() {
                    // Collect valid neighbors
                    let mut sum = 0.0f32;
                    let mut count = 0u32;

                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let nx = x as i32 + dx;
                            let ny = y as i32 + dy;
                            if nx >= 0
                                && nx < grid.width as i32
                                && ny >= 0
                                && ny < grid.height as i32
                            {
                                let nidx = ny as usize * grid.width + nx as usize;
                                let nh = grid.cells[nidx].height;
                                if nh.is_finite() {
                                    sum += nh;
                                    count += 1;
                                }
                            }
                        }
                    }

                    if count > 0 {
                        grid.cells[idx].height = sum / count as f32;
                        changed = true;
                        filled += 1;
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }

    if filled > 0 {
        println!("Filled {} empty cells by interpolation", filled);
    }
}

/// Load height data (auto-detects TIFF vs LAZ vs binary format)
fn load_height_map(path: &PathBuf, resolution: Option<f64>) -> Result<MapGrid> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("laz") | Some("las") => {
            let res = resolution.unwrap_or(0.5);
            load_height_map_laz(path, res)
        }
        Some("bin") => load_binary(path),
        _ => load_height_map_tiff(path),
    }
}

/// JP2 image bounds (from PGW world file for L4124D)
const JP2_ORIGIN_X: f64 = 338000.25;
const JP2_ORIGIN_Y: f64 = 6713999.75;
const JP2_PIXEL_SIZE: f64 = 0.5;

/// Load color data from a JPEG2000 image and apply to grid
fn load_color_data_jp2(grid: &mut MapGrid, path: &PathBuf) -> Result<()> {
    use jpeg2k::ImagePixelData;

    let jp2 = Jp2Image::from_file(path).context("Failed to open JP2 image")?;

    let img_width = jp2.width() as usize;
    let img_height = jp2.height() as usize;
    let num_components = jp2.num_components();

    println!(
        "Loading JP2 color data: {}x{} ({} components)",
        img_width, img_height, num_components
    );

    // Decode the image data
    let img_data = jp2
        .get_pixels(None)
        .context("Failed to decode JP2 pixels")?;

    // Check if grid has georeferenced bounds
    let use_georef = grid.bounds.min_x != 0.0 || grid.bounds.max_x != 0.0;

    // Extract grid parameters for closure
    let grid_width = grid.width;
    let grid_height = grid.height;
    let grid_resolution = grid.resolution;
    let grid_min_x = grid.bounds.min_x;
    let grid_max_y = grid.bounds.max_y;

    if use_georef {
        println!(
            "Using georeferenced sampling: grid bounds ({:.0}, {:.0}) - ({:.0}, {:.0})",
            grid.bounds.min_x, grid.bounds.min_y, grid.bounds.max_x, grid.bounds.max_y
        );
    }

    // Helper to convert grid cell to JP2 pixel coords
    let grid_to_jp2 = |gx: usize, gy: usize| -> (usize, usize) {
        if use_georef {
            // Convert grid cell to world coordinates
            let world_x = grid_min_x + (gx as f64 * grid_resolution);
            let world_y = grid_max_y - (gy as f64 * grid_resolution);

            // Convert world coordinates to JP2 pixel coordinates
            let jp2_x = ((world_x - JP2_ORIGIN_X) / JP2_PIXEL_SIZE) as usize;
            let jp2_y = ((JP2_ORIGIN_Y - world_y) / JP2_PIXEL_SIZE) as usize;

            (jp2_x.min(img_width - 1), jp2_y.min(img_height - 1))
        } else {
            // Simple scaling
            let scale_x = img_width as f64 / grid_width as f64;
            let scale_y = img_height as f64 / grid_height as f64;
            (
                ((gx as f64 * scale_x) as usize).min(img_width - 1),
                ((gy as f64 * scale_y) as usize).min(img_height - 1),
            )
        }
    };

    // Extract pixel data based on format
    match &img_data.data {
        ImagePixelData::Rgb8(pixels) => {
            for y in 0..grid_height {
                for x in 0..grid_width {
                    let (src_x, src_y) = grid_to_jp2(x, y);
                    let base = (src_y * img_width + src_x) * 3;

                    if let Some(cell) = grid.get_mut(x, y) {
                        cell.color = [pixels[base], pixels[base + 1], pixels[base + 2]];
                    }
                }
            }
        }
        ImagePixelData::Rgba8(pixels) => {
            for y in 0..grid_height {
                for x in 0..grid_width {
                    let (src_x, src_y) = grid_to_jp2(x, y);
                    let base = (src_y * img_width + src_x) * 4;

                    if let Some(cell) = grid.get_mut(x, y) {
                        cell.color = [pixels[base], pixels[base + 1], pixels[base + 2]];
                    }
                }
            }
        }
        ImagePixelData::Rgb16(pixels) => {
            for y in 0..grid_height {
                for x in 0..grid_width {
                    let (src_x, src_y) = grid_to_jp2(x, y);
                    let base = (src_y * img_width + src_x) * 3;

                    if let Some(cell) = grid.get_mut(x, y) {
                        cell.color = [
                            (pixels[base] >> 8) as u8,
                            (pixels[base + 1] >> 8) as u8,
                            (pixels[base + 2] >> 8) as u8,
                        ];
                    }
                }
            }
        }
        ImagePixelData::L8(pixels) => {
            for y in 0..grid_height {
                for x in 0..grid_width {
                    let (src_x, src_y) = grid_to_jp2(x, y);
                    let v = pixels[src_y * img_width + src_x];

                    if let Some(cell) = grid.get_mut(x, y) {
                        cell.color = [v, v, v];
                    }
                }
            }
        }
        _ => anyhow::bail!("Unsupported JP2 pixel format: {:?}", img_data.format),
    }

    Ok(())
}

/// Load color data from a standard image format and apply to grid
fn load_color_data_standard(grid: &mut MapGrid, path: &PathBuf) -> Result<()> {
    let img = image::open(path).context("Failed to open color image")?;
    let rgb_img = img.to_rgb8();

    let img_width = rgb_img.width() as usize;
    let img_height = rgb_img.height() as usize;

    println!("Loading color data: {}x{}", img_width, img_height);

    // Calculate scaling factors (color image may be higher resolution)
    let scale_x = img_width as f64 / grid.width as f64;
    let scale_y = img_height as f64 / grid.height as f64;

    println!("Scale factors: {:.2}x, {:.2}x", scale_x, scale_y);

    for y in 0..grid.height {
        for x in 0..grid.width {
            // Sample from corresponding position in color image
            let src_x = ((x as f64 * scale_x) as usize).min(img_width - 1);
            let src_y = ((y as f64 * scale_y) as usize).min(img_height - 1);

            let pixel = rgb_img.get_pixel(src_x as u32, src_y as u32);

            if let Some(cell) = grid.get_mut(x, y) {
                cell.color = [pixel[0], pixel[1], pixel[2]];
            }
        }
    }

    Ok(())
}

/// Load color data from an image (auto-detects JP2 vs standard formats)
fn load_color_data(grid: &mut MapGrid, path: &PathBuf) -> Result<()> {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        Some("jp2") | Some("j2k") | Some("jpx") | Some("j2c") => load_color_data_jp2(grid, path),
        _ => load_color_data_standard(grid, path),
    }
}

/// Save grid to binary format
/// Format:
///   Magic: 4 bytes "MAPG"
///   Version: u32
///   Width: u32
///   Height: u32
///   Resolution: f64
///   Min height: f32
///   Max height: f32
///   Bounds: min_x, max_x, min_y, max_y (4x f64)
///   Cells: [height: f32, r: u8, g: u8, b: u8] × (width × height)
fn save_binary(grid: &MapGrid, path: &PathBuf) -> Result<()> {
    let file = File::create(path).context("Failed to create output file")?;
    let mut writer = BufWriter::new(file);

    // Header
    writer.write_all(MAGIC)?;
    writer.write_all(&VERSION.to_le_bytes())?;
    writer.write_all(&(grid.width as u32).to_le_bytes())?;
    writer.write_all(&(grid.height as u32).to_le_bytes())?;
    writer.write_all(&grid.resolution.to_le_bytes())?;
    writer.write_all(&grid.min_height.to_le_bytes())?;
    writer.write_all(&grid.max_height.to_le_bytes())?;
    writer.write_all(&grid.bounds.min_x.to_le_bytes())?;
    writer.write_all(&grid.bounds.max_x.to_le_bytes())?;
    writer.write_all(&grid.bounds.min_y.to_le_bytes())?;
    writer.write_all(&grid.bounds.max_y.to_le_bytes())?;

    // Cell data
    for cell in &grid.cells {
        writer.write_all(&cell.height.to_le_bytes())?;
        writer.write_all(&cell.color)?;
    }

    writer.flush()?;

    let file_size = path.metadata()?.len();
    println!(
        "Saved binary to: {} ({:.2} MB)",
        path.display(),
        file_size as f64 / 1_000_000.0
    );

    Ok(())
}

/// Load grid from binary format
fn load_binary(path: &PathBuf) -> Result<MapGrid> {
    let file = File::open(path).context("Failed to open binary file")?;
    let mut reader = BufReader::new(file);

    // Read and verify magic
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if &magic != MAGIC {
        anyhow::bail!("Invalid file format (bad magic bytes)");
    }

    // Read header
    let mut buf4 = [0u8; 4];
    let mut buf8 = [0u8; 8];

    reader.read_exact(&mut buf4)?;
    let version = u32::from_le_bytes(buf4);
    if version != VERSION {
        anyhow::bail!("Unsupported version: {}", version);
    }

    reader.read_exact(&mut buf4)?;
    let width = u32::from_le_bytes(buf4) as usize;

    reader.read_exact(&mut buf4)?;
    let height = u32::from_le_bytes(buf4) as usize;

    reader.read_exact(&mut buf8)?;
    let resolution = f64::from_le_bytes(buf8);

    reader.read_exact(&mut buf4)?;
    let min_height = f32::from_le_bytes(buf4);

    reader.read_exact(&mut buf4)?;
    let max_height = f32::from_le_bytes(buf4);

    reader.read_exact(&mut buf8)?;
    let min_x = f64::from_le_bytes(buf8);

    reader.read_exact(&mut buf8)?;
    let max_x = f64::from_le_bytes(buf8);

    reader.read_exact(&mut buf8)?;
    let min_y = f64::from_le_bytes(buf8);

    reader.read_exact(&mut buf8)?;
    let max_y = f64::from_le_bytes(buf8);

    println!(
        "Loading binary: {}x{} at {}m resolution",
        width, height, resolution
    );

    // Read cells
    let cell_count = width * height;
    let mut cells = Vec::with_capacity(cell_count);

    for _ in 0..cell_count {
        reader.read_exact(&mut buf4)?;
        let h = f32::from_le_bytes(buf4);

        let mut color = [0u8; 3];
        reader.read_exact(&mut color)?;

        cells.push(GridCell { height: h, color });
    }

    println!("Height range: {:.2}m - {:.2}m", min_height, max_height);

    Ok(MapGrid {
        width,
        height,
        cells,
        min_height,
        max_height,
        resolution,
        bounds: Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        },
    })
}

/// Generate test image with height as one color band (green), scaled to full range
fn generate_height_image(grid: &MapGrid, output_path: &PathBuf) -> Result<()> {
    let mut img: RgbImage = ImageBuffer::new(grid.width as u32, grid.height as u32);

    let height_range = grid.max_height - grid.min_height;

    for y in 0..grid.height {
        for x in 0..grid.width {
            if let Some(cell) = grid.get(x, y) {
                // Normalize height to 0-255 range
                let normalized = if height_range > 0.0 && cell.height.is_finite() {
                    ((cell.height - grid.min_height) / height_range * 255.0) as u8
                } else {
                    0
                };

                // R = original red, G = height, B = original blue
                img.put_pixel(
                    x as u32,
                    y as u32,
                    Rgb([cell.color[0], normalized, cell.color[2]]),
                );
            }
        }
    }

    img.save(output_path)
        .context("Failed to save output image")?;

    println!("Saved height image to: {}", output_path.display());

    Ok(())
}

/// Generate grayscale height map image
fn generate_grayscale_height_image(grid: &MapGrid, output_path: &PathBuf) -> Result<()> {
    let mut img: RgbImage = ImageBuffer::new(grid.width as u32, grid.height as u32);

    let height_range = grid.max_height - grid.min_height;

    for y in 0..grid.height {
        for x in 0..grid.width {
            if let Some(cell) = grid.get(x, y) {
                let normalized = if height_range > 0.0 && cell.height.is_finite() {
                    ((cell.height - grid.min_height) / height_range * 255.0) as u8
                } else {
                    0
                };

                img.put_pixel(
                    x as u32,
                    y as u32,
                    Rgb([normalized, normalized, normalized]),
                );
            }
        }
    }

    img.save(output_path)
        .context("Failed to save grayscale height image")?;

    println!("Saved grayscale height image to: {}", output_path.display());

    Ok(())
}

/// Generate color-only image (orthophoto)
fn generate_color_image(grid: &MapGrid, output_path: &PathBuf) -> Result<()> {
    let mut img: RgbImage = ImageBuffer::new(grid.width as u32, grid.height as u32);

    for y in 0..grid.height {
        for x in 0..grid.width {
            if let Some(cell) = grid.get(x, y) {
                img.put_pixel(x as u32, y as u32, Rgb(cell.color));
            }
        }
    }

    img.save(output_path)
        .context("Failed to save color image")?;

    println!("Saved color image to: {}", output_path.display());

    Ok(())
}

/// Convert normalized height (0-1) to false color (blue -> green -> yellow -> red)
fn height_to_false_color(t: f32) -> [u8; 3] {
    // Blue (low) -> Cyan -> Green -> Yellow -> Red (high)
    let (r, g, b) = if t < 0.25 {
        // Blue to Cyan
        let s = t / 0.25;
        (0.0, s, 1.0)
    } else if t < 0.5 {
        // Cyan to Green
        let s = (t - 0.25) / 0.25;
        (0.0, 1.0, 1.0 - s)
    } else if t < 0.75 {
        // Green to Yellow
        let s = (t - 0.5) / 0.25;
        (s, 1.0, 0.0)
    } else {
        // Yellow to Red
        let s = (t - 0.75) / 0.25;
        (1.0, 1.0 - s, 0.0)
    };

    [(r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8]
}

/// Generate false-color height map image
fn generate_false_color_height_image(grid: &MapGrid, output_path: &PathBuf) -> Result<()> {
    let mut img: RgbImage = ImageBuffer::new(grid.width as u32, grid.height as u32);

    let height_range = grid.max_height - grid.min_height;

    for y in 0..grid.height {
        for x in 0..grid.width {
            if let Some(cell) = grid.get(x, y) {
                let normalized = if height_range > 0.0 && cell.height.is_finite() {
                    (cell.height - grid.min_height) / height_range
                } else {
                    0.0
                };

                let color = height_to_false_color(normalized);
                img.put_pixel(x as u32, y as u32, Rgb(color));
            }
        }
    }

    img.save(output_path)
        .context("Failed to save false-color height image")?;

    println!(
        "Saved false-color height image to: {}",
        output_path.display()
    );

    Ok(())
}

// ============================================================================
// BCF (Binary Cube Format) Output
// ============================================================================

/// BCF magic bytes
const BCF_MAGIC: &[u8; 4] = b"BCF1";
/// BCF version
const BCF_VERSION: u8 = 0x01;

/// Convert RGB color to a material index (0-255)
/// Uses simple quantization to 6x6x6 color cube + grayscale
fn rgb_to_material(r: u8, g: u8, b: u8) -> u8 {
    // Reserve 0 for empty/air
    // 1-216: 6x6x6 color cube
    // 217-255: grayscale ramp

    let r6 = (r as u32 * 6 / 256) as u8;
    let g6 = (g as u32 * 6 / 256) as u8;
    let b6 = (b as u32 * 6 / 256) as u8;

    // Color cube index: 1 + r*36 + g*6 + b
    1 + r6 * 36 + g6 * 6 + b6
}

/// Octree node for BCF generation
enum OctreeNode {
    /// Solid leaf with material value
    Solid(u8),
    /// Branch with 8 children
    Branch(Box<[OctreeNode; 8]>),
}

/// Sample terrain at a 3D position, returns material (0 = air, >0 = solid)
fn sample_terrain(grid: &MapGrid, x: usize, y: usize, z: usize, z_scale: f32) -> u8 {
    if x >= grid.width || y >= grid.height {
        return 0; // Air outside bounds
    }

    let cell = &grid.cells[y * grid.width + x];
    let terrain_z = ((cell.height - grid.min_height) * z_scale) as usize;

    if z <= terrain_z {
        // Below or at terrain surface - solid
        rgb_to_material(cell.color[0], cell.color[1], cell.color[2])
    } else {
        0 // Air above terrain
    }
}

/// Check if a 3D region is uniform (all same material)
fn check_region_uniform(
    grid: &MapGrid,
    x0: usize,
    y0: usize,
    z0: usize,
    size: usize,
    z_scale: f32,
) -> Option<u8> {
    let first = sample_terrain(grid, x0, y0, z0, z_scale);

    // Sample corners and center to check uniformity
    let samples = [
        (x0, y0, z0),
        (x0 + size - 1, y0, z0),
        (x0, y0 + size - 1, z0),
        (x0 + size - 1, y0 + size - 1, z0),
        (x0, y0, z0 + size - 1),
        (x0 + size - 1, y0, z0 + size - 1),
        (x0, y0 + size - 1, z0 + size - 1),
        (x0 + size - 1, y0 + size - 1, z0 + size - 1),
        (x0 + size / 2, y0 + size / 2, z0 + size / 2),
    ];

    for (x, y, z) in samples {
        if sample_terrain(
            grid,
            x.min(grid.width - 1),
            y.min(grid.height - 1),
            z,
            z_scale,
        ) != first
        {
            return None;
        }
    }

    Some(first)
}

/// Build octree recursively
#[allow(clippy::too_many_arguments)]
fn build_octree(
    grid: &MapGrid,
    x0: usize,
    y0: usize,
    z0: usize,
    size: usize,
    z_scale: f32,
    depth: u32,
    max_depth: u32,
) -> OctreeNode {
    // Check if region is uniform
    if let Some(material) = check_region_uniform(grid, x0, y0, z0, size, z_scale) {
        return OctreeNode::Solid(material);
    }

    // If at max depth, sample center
    if depth >= max_depth || size <= 1 {
        let material = sample_terrain(
            grid,
            (x0 + size / 2).min(grid.width - 1),
            (y0 + size / 2).min(grid.height - 1),
            z0 + size / 2,
            z_scale,
        );
        return OctreeNode::Solid(material);
    }

    // Subdivide into 8 children
    let half = size / 2;
    let children: [OctreeNode; 8] = [
        build_octree(grid, x0, y0, z0, half, z_scale, depth + 1, max_depth),
        build_octree(grid, x0 + half, y0, z0, half, z_scale, depth + 1, max_depth),
        build_octree(grid, x0, y0 + half, z0, half, z_scale, depth + 1, max_depth),
        build_octree(
            grid,
            x0 + half,
            y0 + half,
            z0,
            half,
            z_scale,
            depth + 1,
            max_depth,
        ),
        build_octree(grid, x0, y0, z0 + half, half, z_scale, depth + 1, max_depth),
        build_octree(
            grid,
            x0 + half,
            y0,
            z0 + half,
            half,
            z_scale,
            depth + 1,
            max_depth,
        ),
        build_octree(
            grid,
            x0,
            y0 + half,
            z0 + half,
            half,
            z_scale,
            depth + 1,
            max_depth,
        ),
        build_octree(
            grid,
            x0 + half,
            y0 + half,
            z0 + half,
            half,
            z_scale,
            depth + 1,
            max_depth,
        ),
    ];

    // Check if all children are identical solids
    if let OctreeNode::Solid(v) = &children[0] {
        let all_same = children
            .iter()
            .all(|c| matches!(c, OctreeNode::Solid(cv) if cv == v));
        if all_same {
            return OctreeNode::Solid(*v);
        }
    }

    OctreeNode::Branch(Box::new(children))
}

/// Serialize octree node to BCF bytes, returns (bytes, node_count)
fn serialize_octree(node: &OctreeNode, buffer: &mut Vec<u8>) -> usize {
    match node {
        OctreeNode::Solid(v) => {
            if *v <= 127 {
                // Inline leaf: 0VVVVVVV
                buffer.push(*v);
            } else {
                // Extended leaf: 0x80 + value
                buffer.push(0x80);
                buffer.push(*v);
            }
            1
        }
        OctreeNode::Branch(children) => {
            // Check if all children are solid (octa-leaves optimization)
            let all_solid = children.iter().all(|c| matches!(c, OctreeNode::Solid(_)));

            if all_solid {
                // Octa-leaves: 0x90 + 8 values
                buffer.push(0x90);
                for child in children.iter() {
                    if let OctreeNode::Solid(v) = child {
                        buffer.push(*v);
                    }
                }
                1
            } else {
                // Octa-pointers: serialize children first, then write pointers
                let _start_offset = buffer.len();

                // Placeholder for type byte and pointers
                let placeholder_pos = buffer.len();
                buffer.push(0xA2); // Type with 4-byte pointers
                for _ in 0..8 {
                    buffer.extend_from_slice(&[0u8; 4]);
                }

                // Serialize children and record offsets
                let mut child_offsets = [0u32; 8];
                let mut node_count = 1;

                for (i, child) in children.iter().enumerate() {
                    child_offsets[i] = buffer.len() as u32;
                    node_count += serialize_octree(child, buffer);
                }

                // Write child offsets back
                for (i, offset) in child_offsets.iter().enumerate() {
                    let pos = placeholder_pos + 1 + i * 4;
                    buffer[pos..pos + 4].copy_from_slice(&offset.to_le_bytes());
                }

                node_count
            }
        }
    }
}

/// Generate BCF file from terrain grid
fn generate_bcf(grid: &MapGrid, output_path: &PathBuf, depth: Option<u32>) -> Result<()> {
    // Calculate octree size (power of 2)
    let max_dim = grid.width.max(grid.height);
    let xy_bits = (max_dim as f64).log2().ceil() as u32;

    // Z dimension based on height range
    let height_range = grid.max_height - grid.min_height;
    let z_cells = (height_range / grid.resolution as f32).ceil() as usize;
    let z_bits = (z_cells as f64).log2().ceil() as u32;

    // Use max of xy and z for cubic octree
    let auto_depth = xy_bits.max(z_bits).clamp(4, 12);
    let max_depth = depth.unwrap_or(auto_depth);
    let octree_size = 1usize << max_depth;

    // Z scale: map height range to octree Z dimension
    let z_scale = (octree_size as f32) / height_range;

    println!(
        "Building octree: {}x{}x{} (depth {}), z_scale={:.2}",
        octree_size, octree_size, octree_size, max_depth, z_scale
    );

    // Build octree
    let root = build_octree(grid, 0, 0, 0, octree_size, z_scale, 0, max_depth);

    // Serialize to BCF
    let mut buffer = Vec::new();

    // Header placeholder (12 bytes)
    buffer.extend_from_slice(BCF_MAGIC);
    buffer.push(BCF_VERSION);
    buffer.extend_from_slice(&[0u8; 3]); // Reserved
    buffer.extend_from_slice(&[0u8; 4]); // Root offset placeholder

    // Serialize octree
    let root_offset = buffer.len() as u32;
    let node_count = serialize_octree(&root, &mut buffer);

    // Write root offset
    buffer[8..12].copy_from_slice(&root_offset.to_le_bytes());

    // Write file
    let mut file = File::create(output_path).context("Failed to create BCF file")?;
    file.write_all(&buffer)?;

    let file_size = buffer.len();
    println!(
        "Saved BCF to: {} ({:.2} MB, {} nodes)",
        output_path.display(),
        file_size as f64 / 1_000_000.0,
        node_count
    );

    Ok(())
}

// ============================================================================
// CSM (CubeScript) Output
// ============================================================================

/// Octant index to character (a-h)
const OCTANT_CHARS: [char; 8] = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];

/// Serialize octree to CSM format
fn serialize_octree_csm(
    node: &OctreeNode,
    path: &mut String,
    output: &mut String,
    statement_count: &mut usize,
) {
    match node {
        OctreeNode::Solid(v) => {
            // Only output non-zero (non-air) solids, or root
            if *v != 0 || path.is_empty() {
                output.push_str(&format!(
                    ">{} {}\n",
                    if path.is_empty() { "@" } else { path.as_str() },
                    v
                ));
                *statement_count += 1;
            }
        }
        OctreeNode::Branch(children) => {
            // Check if all children are solid
            let all_solid = children.iter().all(|c| matches!(c, OctreeNode::Solid(_)));

            if all_solid {
                // Output as array: >[path] [v0 v1 v2 v3 v4 v5 v6 v7]
                let values: Vec<u8> = children
                    .iter()
                    .map(|c| match c {
                        OctreeNode::Solid(v) => *v,
                        _ => 0,
                    })
                    .collect();

                // Only output if not all zeros
                if values.iter().any(|&v| v != 0) {
                    output.push_str(&format!(
                        ">{} [{} {} {} {} {} {} {} {}]\n",
                        if path.is_empty() { "@" } else { path.as_str() },
                        values[0],
                        values[1],
                        values[2],
                        values[3],
                        values[4],
                        values[5],
                        values[6],
                        values[7]
                    ));
                    *statement_count += 1;
                }
            } else {
                // Recurse into children
                for (i, child) in children.iter().enumerate() {
                    path.push(OCTANT_CHARS[i]);
                    serialize_octree_csm(child, path, output, statement_count);
                    path.pop();
                }
            }
        }
    }
}

/// Generate CSM file from terrain grid
fn generate_csm(grid: &MapGrid, output_path: &PathBuf, depth: Option<u32>) -> Result<()> {
    // Calculate octree size (power of 2)
    let max_dim = grid.width.max(grid.height);
    let xy_bits = (max_dim as f64).log2().ceil() as u32;

    // Z dimension based on height range
    let height_range = grid.max_height - grid.min_height;
    let z_cells = (height_range / grid.resolution as f32).ceil() as usize;
    let z_bits = (z_cells as f64).log2().ceil() as u32;

    // Use max of xy and z for cubic octree
    let auto_depth = xy_bits.max(z_bits).clamp(4, 10);
    let max_depth = depth.unwrap_or(auto_depth);
    let octree_size = 1usize << max_depth;

    // Z scale: map height range to octree Z dimension
    let z_scale = (octree_size as f32) / height_range;

    println!(
        "Building octree for CSM: {}x{}x{} (depth {}), z_scale={:.2}",
        octree_size, octree_size, octree_size, max_depth, z_scale
    );

    // Build octree
    let root = build_octree(grid, 0, 0, 0, octree_size, z_scale, 0, max_depth);

    // Serialize to CSM
    let mut output = String::new();
    output.push_str("# Terrain CSM generated by mapdata\n");
    output.push_str(&format!(
        "# Grid: {}x{}, Height: {:.1}m - {:.1}m\n",
        grid.width, grid.height, grid.min_height, grid.max_height
    ));
    output.push_str(&format!(
        "# Octree: {}x{}x{} (depth {})\n\n",
        octree_size, octree_size, octree_size, max_depth
    ));

    let mut path = String::new();
    let mut statement_count = 0;
    serialize_octree_csm(&root, &mut path, &mut output, &mut statement_count);

    // Write file
    let mut file = File::create(output_path).context("Failed to create CSM file")?;
    file.write_all(output.as_bytes())?;

    let file_size = output.len();
    println!(
        "Saved CSM to: {} ({:.2} MB, {} statements)",
        output_path.display(),
        file_size as f64 / 1_000_000.0,
        statement_count
    );

    Ok(())
}

#[derive(Parser, Debug)]
#[command(name = "mapdata")]
#[command(about = "Process map height and color data into a grid format")]
struct Args {
    /// Path to the height map (TIFF, LAZ/LAS, or .bin)
    #[arg(short = 'e', long)]
    height_map: PathBuf,

    /// Path to the color/orthophoto image (optional)
    #[arg(short, long)]
    color_image: Option<PathBuf>,

    /// Output path for the image or binary file
    #[arg(short, long, default_value = "output.png")]
    output: PathBuf,

    /// Generate grayscale height-only image
    #[arg(short, long)]
    grayscale: bool,

    /// Resolution in meters per pixel (for LAZ rasterization, default 0.5)
    #[arg(short, long)]
    resolution: Option<f64>,

    /// Save as binary format instead of image
    #[arg(short, long)]
    binary: bool,

    /// Output color-only PNG (orthophoto)
    #[arg(long)]
    color_png: Option<PathBuf>,

    /// Output false-color height PNG
    #[arg(long)]
    height_png: Option<PathBuf>,

    /// Output BCF (Binary Cube Format) terrain file
    #[arg(long)]
    bcf: Option<PathBuf>,

    /// Output CSM (CubeScript) terrain file
    #[arg(long)]
    csm: Option<PathBuf>,

    /// Octree depth for BCF/CSM output (default: auto-calculated)
    #[arg(long)]
    bcf_depth: Option<u32>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load height map
    let mut grid = load_height_map(&args.height_map, args.resolution)?;

    // Load color data if provided
    if let Some(color_path) = &args.color_image {
        load_color_data(&mut grid, color_path)?;
    }

    // Generate main output
    if args.binary {
        save_binary(&grid, &args.output)?;
    } else if args.grayscale {
        generate_grayscale_height_image(&grid, &args.output)?;
    } else {
        generate_height_image(&grid, &args.output)?;
    }

    // Generate separate color PNG if requested
    if let Some(color_path) = &args.color_png {
        generate_color_image(&grid, color_path)?;
    }

    // Generate false-color height PNG if requested
    if let Some(height_path) = &args.height_png {
        generate_false_color_height_image(&grid, height_path)?;
    }

    // Generate BCF if requested
    if let Some(bcf_path) = &args.bcf {
        generate_bcf(&grid, bcf_path, args.bcf_depth)?;
    }

    // Generate CSM if requested
    if let Some(csm_path) = &args.csm {
        generate_csm(&grid, csm_path, args.bcf_depth)?;
    }

    println!(
        "Grid size: {}x{} ({} cells)",
        grid.width,
        grid.height,
        grid.cells.len()
    );
    println!(
        "Resolution: {}m, Bounds: ({:.0}, {:.0}) - ({:.0}, {:.0})",
        grid.resolution, grid.bounds.min_x, grid.bounds.min_y, grid.bounds.max_x, grid.bounds.max_y
    );

    Ok(())
}
