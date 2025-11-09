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
    /// Generate materials.json from doc/materials.md
    Materials,
    /// Generate seamless textures using Replicate API
    Textures {
        /// Model to use: seamless-texture, sdxl, flux-dev
        #[arg(long, default_value = "sdxl")]
        model: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Material {
    index: u8,
    id: String,
    color: String,
    description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct MaterialsData {
    generated: String,
    count: usize,
    materials: Vec<Material>,
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
                m.path.strip_prefix("vox/").unwrap_or(&m.path).to_string(),
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

        if path.is_file() && path.extension().is_some_and(|e| e == "vox") {
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
                            .or_default()
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

                                let stats =
                                    color_stats_map.entry(rgb).or_insert_with(|| ColorStats {
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
            b.model_count
                .cmp(&a.model_count)
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
            let hex_color = format!(
                "#{:02x}{:02x}{:02x}",
                stats.rgb[0], stats.rgb[1], stats.rgb[2]
            );

            println!(
                "| {} | {} | {} | {} |",
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

fn cmd_materials() -> Result<(), Box<dyn std::error::Error>> {
    let doc_path = PathBuf::from("doc/materials.md");
    let output_path = PathBuf::from("assets/materials.json");

    if !doc_path.exists() {
        eprintln!("Error: doc/materials.md not found");
        std::process::exit(1);
    }

    println!("Reading doc/materials.md...");

    let content = fs::read_to_string(&doc_path)?;
    let mut materials = Vec::new();
    let mut in_table = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Detect table start (header line with pipes)
        if line.starts_with("| Index") {
            in_table = true;
            continue;
        }

        // Skip separator line
        if line.starts_with("|---") {
            continue;
        }

        // Parse table rows
        if in_table && line.starts_with('|') {
            let parts: Vec<&str> = line
                .split('|')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            if parts.len() >= 4 {
                // Parse index
                if let Ok(index) = parts[0].parse::<u8>() {
                    materials.push(Material {
                        index,
                        id: parts[1].to_string(),
                        color: parts[2].to_string(),
                        description: parts[3].to_string(),
                    });
                }
            }
        }

        // Stop when we hit a non-table line after being in table
        if in_table && !line.starts_with('|') {
            break;
        }
    }

    println!("Found {} materials from markdown", materials.len());

    // Sort by index
    materials.sort_by_key(|m| m.index);

    // Auto-generate materials 128-255 (7-bit RGB: r:2, g:3, b:2)
    println!("Generating colored blocks 128-255...");
    for i in 128..=255 {
        let index = i as u8;

        // Extract RGB bits: r:2, g:3, b:2
        let r_bits = (index >> 5) & 0b11; // Top 2 bits
        let g_bits = (index >> 2) & 0b111; // Middle 3 bits
        let b_bits = index & 0b11; // Bottom 2 bits

        // Convert to 8-bit RGB values (standard R2G3B2 mapping)
        // 2 bits: 0->0x00, 1->0x55, 2->0xAA, 3->0xFF
        let r = match r_bits {
            0 => 0x00,
            1 => 0x55,
            2 => 0xAA,
            3 => 0xFF,
            _ => 0x00,
        };

        // 3 bits: 0->0x00, 1->0x24, 2->0x49, 3->0x6D, 4->0x92, 5->0xB6, 6->0xDB, 7->0xFF
        let g = match g_bits {
            0 => 0x00,
            1 => 0x24,
            2 => 0x49,
            3 => 0x6D,
            4 => 0x92,
            5 => 0xB6,
            6 => 0xDB,
            7 => 0xFF,
            _ => 0x00,
        };

        // 2 bits: 0->0x00, 1->0x55, 2->0xAA, 3->0xFF
        let b = match b_bits {
            0 => 0x00,
            1 => 0x55,
            2 => 0xAA,
            3 => 0xFF,
            _ => 0x00,
        };

        let color = format!("#FF{:02X}{:02X}{:02X}", r, g, b);
        let id = format!("color_{:03}", i);
        let description = format!(
            "Auto-generated color (r:{}, g:{}, b:{})",
            r_bits, g_bits, b_bits
        );

        materials.push(Material {
            index,
            id,
            color,
            description,
        });
    }

    println!("Total materials: {}", materials.len());

    // Generate materials.json
    let materials_data = MaterialsData {
        generated: chrono::Utc::now().to_rfc3339(),
        count: materials.len(),
        materials,
    };

    let json = serde_json::to_string_pretty(&materials_data)?;

    // Ensure assets directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&output_path, json)?;
    println!("\nGenerated {}", output_path.display());

    Ok(())
}

#[derive(Debug, Serialize)]
struct ReplicateInput {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    aspect_ratio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_quality: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    go_fast: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guidance_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_inference_steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_safety_checker: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ReplicateRequest {
    version: String,
    input: ReplicateInput,
}

#[derive(Debug, Deserialize)]
struct ReplicateResponse {
    id: String,
    status: String,
    output: Option<Vec<String>>,
}

async fn cmd_textures(model_name: String) -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file from project root
    let env_path = PathBuf::from(".env");
    if env_path.exists() {
        dotenv::from_path(&env_path).ok();
        println!("Loaded environment from .env");
    }

    let materials_path = PathBuf::from("assets/materials.json");
    let textures_dir = PathBuf::from("assets/textures");

    // Read materials.json
    if !materials_path.exists() {
        eprintln!("Error: assets/materials.json not found");
        eprintln!("Please run 'assets materials' first");
        std::process::exit(1);
    }

    println!("Loading materials.json...");
    let materials_content = fs::read_to_string(&materials_path)?;
    let materials_data: MaterialsData = serde_json::from_str(&materials_content)?;

    // Get API token from environment
    let api_token = std::env::var("REPLICATE_API_TOKEN")
        .map_err(|_| "REPLICATE_API_TOKEN not found in environment or .env file")?;

    // Create textures directory
    fs::create_dir_all(&textures_dir)?;
    println!(
        "Created/verified textures directory at {}",
        textures_dir.display()
    );

    // Create HTTP client
    let client = reqwest::Client::new();

    // Process materials 1-127
    let materials_to_process: Vec<_> = materials_data
        .materials
        .iter()
        .filter(|m| m.index >= 1 && m.index <= 127)
        .collect();

    // Select model version based on user choice
    let (version_id, model_type) = match model_name.as_str() {
        "seamless-texture" => (
            "9a59c0dce189bfe8a7fcb379c497713500ff959652c4e7874023f15983dec839",
            "seamless-texture",
        ),
        "sdxl" => (
            "7762fd07cf82c948538e41f63f77d685e02b063e37e496e96eefd46c929f9bdc",
            "sdxl",
        ),
        "flux-dev" => (
            "6e4a938f85952bdabcc15aa329178c4d681c52bf25a0342403287dc26944661d",
            "flux-dev",
        ),
        _ => {
            eprintln!(
                "Unknown model: {}. Use: seamless-texture, sdxl, or flux-dev",
                model_name
            );
            std::process::exit(1);
        }
    };

    println!("Using model: {} ({})", model_type, version_id);
    println!(
        "\nGenerating textures for {} materials (1-127) in webp format...\n",
        materials_to_process.len()
    );

    for material in materials_to_process {
        let texture_filename = format!("{}.webp", material.id);
        let texture_path = textures_dir.join(&texture_filename);

        // Skip if texture already exists
        if texture_path.exists() {
            println!(
                "[{}] {} - already exists, skipping",
                material.index, material.id
            );
            continue;
        }

        println!(
            "[{}] {} - generating texture...",
            material.index, material.id
        );

        // Create prompt optimized for albedo/diffuse texture maps
        // Use simple, concrete visual terms the AI model understands
        let prompt = format!(
            "flat top-down view of {} surface, seamless tileable pattern, \
             uniform overcast lighting, no shadows, no shine, no glare, matte finish, \
             covers 1 square meter area, ultra detailed, photographic quality",
            material.description
        );

        // Configure parameters based on model type
        let (width, height, aspect_ratio, model_param, guidance, steps) = match model_type {
            "seamless-texture" => (
                Some(256),
                Some(256),
                None,
                Some("dev".to_string()),
                Some(3.5),
                Some(50),
            ),
            "sdxl" => (Some(1024), Some(1024), None, None, Some(7.5), Some(50)),
            "flux-dev" => (
                None,
                None,
                Some("1:1".to_string()),
                None,
                Some(3.5),
                Some(50),
            ),
            _ => (Some(256), Some(256), None, None, Some(3.5), Some(50)),
        };

        // Create Replicate API request with quality settings
        let request = ReplicateRequest {
            version: version_id.to_string(),
            input: ReplicateInput {
                prompt: prompt.clone(),
                width,
                height,
                aspect_ratio,
                output_format: Some("webp".to_string()),
                output_quality: Some(100),
                go_fast: Some(true),
                model: model_param,
                guidance_scale: guidance,
                num_inference_steps: steps,
                disable_safety_checker: Some(true),
            },
        };

        // Submit prediction
        let response = client
            .post("https://api.replicate.com/v1/predictions")
            .header("Authorization", format!("Token {}", api_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            eprintln!("  Error: API returned status {}: {}", status, error_text);
            continue;
        }

        let mut prediction: ReplicateResponse = response.json().await?;
        println!("  Prediction ID: {}", prediction.id);

        // Poll for completion
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 60; // 5 minutes at 5 second intervals

        while prediction.status != "succeeded"
            && prediction.status != "failed"
            && attempts < MAX_ATTEMPTS
        {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            attempts += 1;

            let poll_response = client
                .get(format!(
                    "https://api.replicate.com/v1/predictions/{}",
                    prediction.id
                ))
                .header("Authorization", format!("Token {}", api_token))
                .send()
                .await?;

            prediction = poll_response.json().await?;
            println!(
                "  Status: {} (attempt {}/{})",
                prediction.status, attempts, MAX_ATTEMPTS
            );
        }

        if prediction.status == "succeeded" {
            if let Some(output_urls) = prediction.output {
                if let Some(url) = output_urls.first() {
                    println!("  Downloading texture from: {}", url);

                    // Download the image
                    let image_response = client.get(url).send().await?;
                    let image_bytes = image_response.bytes().await?;

                    // Save to file
                    fs::write(&texture_path, image_bytes)?;
                    println!("  Saved to: {}", texture_path.display());
                } else {
                    eprintln!("  Error: No output URL in response");
                }
            } else {
                eprintln!("  Error: No output in response");
            }
        } else {
            eprintln!(
                "  Error: Texture generation failed with status: {}",
                prediction.status
            );
        }

        println!();
    }

    println!("Texture generation complete!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index => cmd_index(),
        Commands::VoxPalette { palette } => cmd_vox_palette(palette),
        Commands::Materials => cmd_materials(),
        Commands::Textures { model } => cmd_textures(model).await,
    }
}
