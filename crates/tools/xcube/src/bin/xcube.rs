//! XCube CLI - Command-line interface for text-to-voxel generation

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use xcube::convert::xcube_to_csm;
use xcube::{GenerationRequest, XCubeClient};

/// XCube CLI - Text-to-voxel generation tool
#[derive(Parser)]
#[command(name = "xcube")]
#[command(about = "Generate voxel models from text prompts", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a voxel model from text prompt
    Generate {
        /// Text prompt describing the model to generate
        prompt: String,

        /// Output file path (.csm)
        #[arg(short, long)]
        output: PathBuf,

        /// DDIM sampling steps (default: 100, range: 1-1000)
        #[arg(long, default_value = "100")]
        steps: u32,

        /// Random seed for reproducibility
        #[arg(long)]
        seed: Option<i32>,

        /// XCube server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server: String,

        /// Request timeout in seconds
        #[arg(long, default_value = "300")]
        timeout: u64,
    },

    /// Check XCube server health
    Health {
        /// XCube server URL
        #[arg(long, default_value = "http://localhost:8000")]
        server: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            prompt,
            output,
            steps,
            seed,
            server,
            timeout,
        } => {
            generate_command(prompt, output, steps, seed, server, timeout).await?;
        }
        Commands::Health { server } => {
            health_command(server).await?;
        }
    }

    Ok(())
}

async fn generate_command(
    prompt: String,
    output: PathBuf,
    steps: u32,
    seed: Option<i32>,
    server: String,
    timeout_secs: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Determine output format from extension
    let extension = output.extension().and_then(|s| s.to_str()).unwrap_or("");
    if extension != "csm" {
        eprintln!("Error: Output file must have .csm extension");
        std::process::exit(1);
    }

    println!("XCube Text-to-Voxel Generator");
    println!("=============================");
    println!("Prompt: {}", prompt);
    println!("Server: {}", server);
    println!("Steps: {}", steps);
    if let Some(seed) = seed {
        println!("Seed: {}", seed);
    }
    println!("Output: {}", output.display());
    println!();

    // Create client
    let client = XCubeClient::new(server).with_generate_timeout(Duration::from_secs(timeout_secs));

    // Create generation request
    let mut request = GenerationRequest::new(&prompt).with_ddim_steps(steps);
    if let Some(seed) = seed {
        request = request.with_seed(seed);
    }

    // Create progress bar
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    progress.enable_steady_tick(Duration::from_millis(100));
    progress.set_message("Generating voxel model...");

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
        "Generated point cloud: {} coarse points",
        result.coarse_point_count()
    );
    if result.has_fine() {
        println!("  {} fine points", result.fine_point_count());
    }

    // Convert to CSM format
    progress.set_message("Converting to CSM format...");
    progress.enable_steady_tick(Duration::from_millis(100));

    let csm = xcube_to_csm(&result)?;
    let data = csm.into_bytes();

    progress.finish_with_message("✓ Conversion complete");

    // Write to file
    fs::write(&output, &data)?;

    println!("✓ Saved to {}", output.display());
    println!("  Size: {} bytes", data.len());

    Ok(())
}

async fn health_command(server: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Checking XCube server health...");
    println!("Server: {}", server);
    println!();

    let client = XCubeClient::new(server);

    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    progress.enable_steady_tick(Duration::from_millis(100));
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
    println!("XCube available: {}", status.xcube_available);
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
