//! Trellis CLI - Command-line interface for image-to-voxel generation

use base64::Engine;
use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use trellis::{GenerationRequest, Resolution, TrellisClient};

/// Trellis CLI - Image-to-voxel generation tool
#[derive(Parser)]
#[command(name = "trellis")]
#[command(about = "Generate voxel models from images", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a voxel model from an image
    Generate {
        /// Input image file (PNG, JPG, WebP)
        image_path: PathBuf,

        /// Output file path (.csm)
        #[arg(short, long)]
        output: PathBuf,

        /// Generation resolution (512, 1024, or 1536)
        #[arg(short, long, default_value = "512")]
        resolution: u32,

        /// Random seed for reproducibility
        #[arg(short, long)]
        seed: Option<i64>,

        /// Trellis server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server: String,

        /// Request timeout in seconds
        #[arg(long, default_value = "600")]
        timeout: u64,

        /// Voxel grid depth for conversion
        #[arg(long, default_value = "6")]
        depth: u8,
    },

    /// Check Trellis server health
    Health {
        /// Trellis server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            image_path,
            output,
            resolution,
            seed,
            server,
            timeout,
            depth,
        } => {
            generate_command(image_path, output, resolution, seed, server, timeout, depth).await?;
        }
        Commands::Health { server } => {
            health_command(server).await?;
        }
    }

    Ok(())
}

async fn generate_command(
    image_path: PathBuf,
    output: PathBuf,
    resolution: u32,
    seed: Option<i64>,
    server: String,
    timeout_secs: u64,
    depth: u8,
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate output extension
    let extension = output.extension().and_then(|s| s.to_str()).unwrap_or("");
    if extension != "csm" {
        eprintln!("Error: Output file must have .csm extension");
        std::process::exit(1);
    }

    // Validate image file exists
    if !image_path.exists() {
        eprintln!("Error: Image file not found: {}", image_path.display());
        std::process::exit(1);
    }

    // Parse resolution
    let resolution = match resolution {
        512 => Resolution::R512,
        1024 => Resolution::R1024,
        1536 => Resolution::R1536,
        _ => {
            eprintln!("Error: Resolution must be 512, 1024, or 1536");
            std::process::exit(1);
        }
    };

    // Validate depth
    if !(1..=10).contains(&depth) {
        eprintln!("Error: Depth must be between 1 and 10");
        std::process::exit(1);
    }

    println!("Trellis Image-to-Voxel Generator");
    println!("=================================");
    println!("Image: {}", image_path.display());
    println!("Server: {}", server);
    println!("Resolution: {:?}", resolution);
    if let Some(seed) = seed {
        println!("Seed: {}", seed);
    }
    println!("Voxel depth: {}", depth);
    println!("Output: {}", output.display());
    println!();

    // Read and encode image
    let progress = create_progress_bar();
    progress.set_message("Reading image file...");

    let image_data = fs::read(&image_path)?;
    let base64_image = base64::engine::general_purpose::STANDARD.encode(&image_data);

    progress.finish_with_message("✓ Image loaded");

    // Create client
    let client =
        TrellisClient::new(server).with_generate_timeout(Duration::from_secs(timeout_secs));

    // Create generation request
    let mut request = GenerationRequest::new(&base64_image).with_resolution(resolution);
    if let Some(seed) = seed {
        request = request.with_seed(seed);
    }

    // Create progress bar for generation
    let progress = create_progress_bar();
    progress.set_message("Generating 3D mesh...");

    // Send generation request
    let result = match client.generate(&request).await {
        Ok(result) => {
            progress.finish_with_message("✓ Generation complete");
            result
        }
        Err(e) => {
            progress.finish_with_message("✗ Generation failed");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    println!(
        "Generated mesh: {} vertices, {} faces",
        result.vertex_count(),
        result.face_count()
    );

    // Convert to CSM format
    let progress = create_progress_bar();
    progress.set_message("Converting mesh to voxel octree...");

    // TODO: Implement mesh-to-voxel conversion when T2-04 is complete
    // For now, save the raw mesh data as a placeholder
    let csm_data = format!(
        "// Trellis mesh-to-CSM conversion not yet implemented\n\
         // Vertices: {}\n\
         // Faces: {}\n\
         // Depth: {}\n\
         s0",
        result.vertex_count(),
        result.face_count(),
        depth
    );

    progress.finish_with_message("✓ Placeholder conversion complete");

    // Write to file
    fs::write(&output, csm_data.as_bytes())?;

    println!("✓ Saved placeholder to {}", output.display());
    println!("  Size: {} bytes", csm_data.len());
    println!();
    println!("Note: Mesh-to-voxel conversion not yet implemented (requires T2-04).");

    Ok(())
}

async fn health_command(server: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking Trellis server health...");
    println!("Server: {}", server);
    println!();

    let client = TrellisClient::new(server);

    let progress = create_progress_bar();
    progress.set_message("Connecting to server...");

    let status = match client.health_check().await {
        Ok(status) => {
            progress.finish_with_message("✓ Server is reachable");
            status
        }
        Err(e) => {
            progress.finish_with_message("✗ Server is unreachable");
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    println!();
    println!("Server Status");
    println!("=============");
    println!("Status: {}", status.status);
    println!("Trellis available: {}", status.trellis_available);
    println!("GPU available: {}", status.gpu_available);

    if let Some(ref gpu_name) = status.gpu_name {
        println!("GPU: {}", gpu_name);
    }

    println!("Model loaded: {}", status.model_loaded);

    if let Some(ref error) = status.error {
        println!("Error: {}", error);
    }

    // Exit with non-zero if server is not ready
    if !status.is_ready() {
        std::process::exit(1);
    }

    Ok(())
}

fn create_progress_bar() -> ProgressBar {
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    progress.enable_steady_tick(Duration::from_millis(100));
    progress
}
