use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "assets")]
#[command(about = "Asset management tools for Crossworld", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create asset index files (models.json and avatars.json)
    Index,
    /// Analyze vox model color palettes for consistency
    VoxPalette {
        /// Calculate and display minimal unified palette across all models
        #[arg(long)]
        palette: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelEntry {
    name: String,
    path: String,
    #[serde(rename = "type")]
    model_type: String,
    size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModelsIndex {
    generated: String,
    count: usize,
    models: Vec<ModelEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AvatarsIndex {
    vox: Vec<[String; 2]>,
}

fn traverse_directory(
    dir: &Path,
    base_dir: &Path,
    models: &mut Vec<ModelEntry>,
) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            traverse_directory(&path, base_dir, models)?;
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "vox" || ext_str == "glb" {
                    let relative_path = path
                        .strip_prefix(base_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");

                    let name = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();

                    let metadata = fs::metadata(&path)?;

                    models.push(ModelEntry {
                        name,
                        path: relative_path,
                        model_type: ext_str,
                        size: metadata.len(),
                    });
                }
            }
        }
    }

    Ok(())
}

fn cmd_index() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = PathBuf::from("assets");
    let models_dir = assets_dir.join("models");

    if !models_dir.exists() {
        eprintln!("Error: assets/models directory not found");
        std::process::exit(1);
    }

    println!("Scanning assets/models directory...");

    let mut models = Vec::new();
    traverse_directory(&models_dir, &models_dir, &mut models)?;

    // Sort models by name
    models.sort_by(|a, b| a.name.cmp(&b.name));

    let vox_count = models.iter().filter(|m| m.model_type == "vox").count();
    let glb_count = models.iter().filter(|m| m.model_type == "glb").count();

    println!("Found {} models", models.len());
    println!("  - VOX: {}", vox_count);
    println!("  - GLB: {}", glb_count);

    // Generate models.json
    let models_index = ModelsIndex {
        generated: chrono::Utc::now().to_rfc3339(),
        count: models.len(),
        models: models.clone(),
    };

    let models_json = serde_json::to_string_pretty(&models_index)?;
    let models_output = assets_dir.join("models.json");
    fs::write(&models_output, models_json)?;
    println!("\nGenerated {}", models_output.display());

    // Generate avatars.json (chr_ vox models only)
    let avatars: Vec<[String; 2]> = models
        .iter()
        .filter(|m| m.model_type == "vox" && m.name.starts_with("chr_"))
        .map(|m| {
            [
                m.name.clone(),
                m.path
                    .strip_prefix("vox/")
                    .unwrap_or(&m.path)
                    .to_string(),
            ]
        })
        .collect();

    let avatars_index = AvatarsIndex { vox: avatars };
    let avatars_json = serde_json::to_string_pretty(&avatars_index)?;
    let avatars_output = assets_dir.join("avatars.json");
    fs::write(&avatars_output, avatars_json)?;
    println!("Generated {}", avatars_output.display());

    Ok(())
}

#[derive(Debug)]
struct ColorStats {
    rgb: [u8; 3],
    total_voxels: usize,
    model_count: usize,
}

fn cmd_vox_palette(calculate_minimal_palette: bool) -> Result<(), Box<dyn std::error::Error>> {
    let models_dir = PathBuf::from("assets/models/vox");

    if !models_dir.exists() {
        eprintln!("Error: assets/models/vox directory not found");
        std::process::exit(1);
    }

    println!("Analyzing VOX model palettes...\n");

    let mut palette_map: HashMap<Vec<u8>, Vec<String>> = HashMap::new();
    let mut color_stats_map: HashMap<[u8; 3], ColorStats> = HashMap::new();
    let mut error_count = 0;
    let mut total_count = 0;

    // Traverse all .vox files
    for entry in fs::read_dir(&models_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |e| e == "vox") {
            total_count += 1;
            let name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // Load vox file
            match fs::read(&path) {
                Ok(bytes) => match dot_vox::load_bytes(&bytes) {
                    Ok(vox_data) => {
                        if vox_data.models.is_empty() {
                            eprintln!("Error: {} has no models", name);
                            error_count += 1;
                            continue;
                        }

                        // Convert palette to bytes for comparison
                        let palette_bytes: Vec<u8> = vox_data
                            .palette
                            .iter()
                            .flat_map(|color| vec![color.r, color.g, color.b])
                            .collect();

                        palette_map
                            .entry(palette_bytes.clone())
                            .or_insert_with(Vec::new)
                            .push(name.clone());

                        // Collect actual color usage from voxels if calculating minimal palette
                        if calculate_minimal_palette {
                            // Count voxels by color index
                            let mut index_counts: HashMap<u8, usize> = HashMap::new();
                            let model = &vox_data.models[0];
                            for voxel in &model.voxels {
                                // MagicaVoxel uses 1-based indexing, convert to 0-based
                                let color_index = if voxel.i > 0 { voxel.i - 1 } else { 0 };
                                *index_counts.entry(color_index).or_insert(0) += 1;
                            }

                            // Map each used index to its RGB color
                            for (color_index, voxel_count) in index_counts {
                                let rgb = [
                                    palette_bytes[color_index as usize * 3],
                                    palette_bytes[color_index as usize * 3 + 1],
                                    palette_bytes[color_index as usize * 3 + 2],
                                ];

                                let stats = color_stats_map.entry(rgb).or_insert_with(|| ColorStats {
                                    rgb,
                                    total_voxels: 0,
                                    model_count: 0,
                                });

                                stats.total_voxels += voxel_count;
                                stats.model_count += 1;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error loading {}: {}", name, e);
                        error_count += 1;
                    }
                },
                Err(e) => {
                    eprintln!("Error reading file {}: {}", name, e);
                    error_count += 1;
                }
            }
        }
    }

    if calculate_minimal_palette {
        // Sort colors by number of models using them (descending), then by total voxels
        let mut color_list: Vec<&ColorStats> = color_stats_map.values().collect();
        color_list.sort_by(|a, b| {
            b.model_count.cmp(&a.model_count)
                .then(b.total_voxels.cmp(&a.total_voxels))
        });

        println!("# Minimal Unified Palette (Actually Used Colors)");
        println!();
        println!("Total unique colors used: {}", color_list.len());
        println!();

        // Print markdown table header
        println!("| Index | Color | Models | Total Voxels |");
        println!("|-------|-------|--------|--------------|");

        // Print table rows
        for (idx, stats) in color_list.iter().enumerate() {
            // Convert RGB to hex
            let hex_color = format!("#{:02x}{:02x}{:02x}",
                stats.rgb[0], stats.rgb[1], stats.rgb[2]);

            println!("| {} | {} | {} | {} |",
                idx + 1,
                hex_color,
                stats.model_count,
                stats.total_voxels
            );
        }
    } else {
        // Report results
        println!("═══════════════════════════════════════════");
        println!("VOX Palette Analysis Results");
        println!("═══════════════════════════════════════════");
        println!("Total VOX files analyzed: {}", total_count);
        println!("Errors encountered: {}", error_count);
        println!("Unique palettes found: {}", palette_map.len());
        println!();

        if palette_map.len() == 1 {
            println!("✓ All VOX models share the SAME palette!");
            println!();
            if let Some((_, models)) = palette_map.iter().next() {
                println!("Models using this palette: {}", models.len());
            }
        } else {
            println!("✗ VOX models use DIFFERENT palettes");
            println!();

            // Sort by number of models using each palette
            let mut palette_list: Vec<_> = palette_map.iter().collect();
            palette_list.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

            for (idx, (palette_bytes, models)) in palette_list.iter().enumerate() {
                println!("Palette #{} ({} colors)", idx + 1, palette_bytes.len() / 3);
                println!("  Used by {} models", models.len());
                println!(
                    "  Examples: {}",
                    models
                        .iter()
                        .take(5)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                if models.len() > 5 {
                    println!("  ... and {} more", models.len() - 5);
                }
                println!();
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index => cmd_index(),
        Commands::VoxPalette { palette } => cmd_vox_palette(palette),
    }
}
