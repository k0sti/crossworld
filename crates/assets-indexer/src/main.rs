use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize)]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets_dir = PathBuf::from("assets");
    let models_dir = assets_dir.join("models");
    let output_path = assets_dir.join("models.json");

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

    let index = ModelsIndex {
        generated: chrono::Utc::now().to_rfc3339(),
        count: models.len(),
        models,
    };

    let json = serde_json::to_string_pretty(&index)?;
    fs::write(&output_path, json)?;

    println!("\nGenerated {}", output_path.display());

    Ok(())
}
