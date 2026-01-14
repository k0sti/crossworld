//! Robocube CLI - Text-to-voxel generation using Roblox Cube3D
//!
//! Command-line interface for generating CSM voxel models from text prompts.

use clap::{Parser, Subcommand};
use indicatif::{ProgressBar, ProgressStyle};
use robocube::{
    convert::{occupancy_to_csm, occupancy_to_cube},
    OccupancyRequest, RobocubeClient, DEFAULT_SERVER_URL,
};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "robocube")]
#[command(
    author,
    version,
    about = "Generate CSM voxel models from text using Roblox Cube3D"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a voxel model from a text prompt
    Generate {
        /// Text description of the 3D model to generate
        prompt: String,

        /// Output file path (must end in .csm)
        #[arg(short, long)]
        output: PathBuf,

        /// Grid resolution (power of 2: 32, 64, 128)
        #[arg(short = 'r', long, default_value = "64")]
        resolution: u32,

        /// Random seed for reproducibility
        #[arg(short, long)]
        seed: Option<i64>,

        /// Occupancy threshold (default: 0.0)
        #[arg(short, long, default_value = "0.0")]
        threshold: f32,

        /// Guidance scale for generation
        #[arg(short, long, default_value = "3.0")]
        guidance: f32,

        /// Server URL
        #[arg(long, default_value = DEFAULT_SERVER_URL)]
        server: String,

        /// Request timeout in seconds
        #[arg(long, default_value = "600")]
        timeout: u64,

        /// Material index for occupied voxels (128-255)
        #[arg(short, long)]
        material: Option<u8>,

        /// Include raw logits in output (for debugging)
        #[arg(long)]
        include_logits: bool,
    },

    /// Check server health status
    Health {
        /// Server URL
        #[arg(long, default_value = DEFAULT_SERVER_URL)]
        server: String,
    },

    /// Show information about a generated result
    Info {
        /// Server URL
        #[arg(long, default_value = DEFAULT_SERVER_URL)]
        server: String,

        /// Text prompt to analyze
        prompt: String,

        /// Grid resolution
        #[arg(short = 'r', long, default_value = "64")]
        resolution: u32,

        /// Random seed
        #[arg(short, long)]
        seed: Option<i64>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Generate {
            prompt,
            output,
            resolution,
            seed,
            threshold,
            guidance,
            server,
            timeout,
            material,
            include_logits: _,
        } => {
            // Validate output path
            if output.extension().is_none_or(|ext| ext != "csm") {
                eprintln!("Error: Output file must have .csm extension");
                std::process::exit(1);
            }

            // Validate resolution
            if !resolution.is_power_of_two() || !(8..=256).contains(&resolution) {
                eprintln!(
                    "Error: Resolution must be a power of 2 between 8 and 256 (got {})",
                    resolution
                );
                std::process::exit(1);
            }

            // Create client
            let client =
                RobocubeClient::new(&server).with_generate_timeout(Duration::from_secs(timeout));

            // Create progress spinner
            let progress = ProgressBar::new_spinner();
            progress.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );

            // Check server health first
            progress.set_message("Checking server status...");
            progress.enable_steady_tick(Duration::from_millis(100));

            match client.health_check().await {
                Ok(status) => {
                    if !status.is_ready() {
                        if status.is_loading() {
                            progress
                                .finish_with_message("Server is loading models, please wait...");
                            // Could add retry logic here
                        } else {
                            progress.finish_with_message(format!(
                                "Server not ready: {}",
                                status.error.unwrap_or_else(|| "Unknown error".to_string())
                            ));
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    progress.finish_with_message(format!("Failed to connect to server: {}", e));
                    std::process::exit(1);
                }
            }

            // Build request
            let mut request = OccupancyRequest::new(&prompt)
                .with_grid_resolution(resolution)
                .with_threshold(threshold)
                .with_guidance_scale(guidance);

            if let Some(s) = seed {
                request = request.with_seed(s);
            }

            // Generate
            progress.set_message(format!(
                "Generating \"{}\" at {}³ resolution...",
                truncate_prompt(&prompt, 30),
                resolution
            ));

            let result = match client.generate_occupancy(&request).await {
                Ok(r) => r,
                Err(e) => {
                    progress.finish_with_message(format!("Generation failed: {}", e));
                    std::process::exit(1);
                }
            };

            progress.set_message("Converting to CSM format...");

            // Convert to CSM
            let csm = match occupancy_to_csm(&result, material) {
                Ok(s) => s,
                Err(e) => {
                    progress.finish_with_message(format!("Conversion failed: {}", e));
                    std::process::exit(1);
                }
            };

            // Write output
            progress.set_message("Writing output file...");
            if let Err(e) = std::fs::write(&output, &csm) {
                progress.finish_with_message(format!("Failed to write file: {}", e));
                std::process::exit(1);
            }

            progress.finish_with_message(format!(
                "Generated {} voxels → {}",
                result.occupied_count(),
                output.display()
            ));

            // Print statistics
            println!();
            println!("Statistics:");
            println!("  Resolution:  {}³", result.resolution);
            println!("  Occupied:    {} voxels", result.occupied_count());
            println!("  Occupancy:   {:.2}%", result.occupancy_ratio() * 100.0);
            println!("  File size:   {} bytes", csm.len());

            if let Some(meta) = &result.metadata {
                if let Some(time) = meta.generation_time_secs {
                    println!("  Gen time:    {:.2}s", time);
                }
                if let Some(seed) = meta.seed_used {
                    println!("  Seed used:   {}", seed);
                }
            }
        }

        Commands::Health { server } => {
            let client = RobocubeClient::new(&server);

            println!("Checking server at {}...", server);
            println!();

            match client.health_check().await {
                Ok(status) => {
                    println!("Status:      {}", status.status);
                    println!(
                        "GPU:         {}",
                        if status.gpu_available {
                            "Available"
                        } else {
                            "Not available"
                        }
                    );
                    if let Some(name) = &status.gpu_name {
                        println!("GPU Name:    {}", name);
                    }
                    println!(
                        "Model:       {}",
                        if status.model_loaded {
                            "Loaded"
                        } else {
                            "Not loaded"
                        }
                    );
                    if let Some(version) = &status.model_version {
                        println!("Version:     {}", version);
                    }
                    if let Some(uptime) = status.uptime_secs {
                        println!("Uptime:      {:.0}s", uptime);
                    }
                    if let Some(error) = &status.error {
                        println!("Error:       {}", error);
                    }

                    if !status.is_ready() {
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect: {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::Info {
            server,
            prompt,
            resolution,
            seed,
        } => {
            let client = RobocubeClient::new(&server);

            let progress = ProgressBar::new_spinner();
            progress.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.green} {msg}")
                    .unwrap()
                    .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
            );
            progress.set_message("Generating for analysis...");
            progress.enable_steady_tick(Duration::from_millis(100));

            let mut request = OccupancyRequest::new(&prompt)
                .with_grid_resolution(resolution)
                .with_threshold(-1000.0); // Include all voxels

            if let Some(s) = seed {
                request = request.with_seed(s);
            }

            let result = match client.generate_occupancy(&request).await {
                Ok(r) => r,
                Err(e) => {
                    progress.finish_with_message(format!("Failed: {}", e));
                    std::process::exit(1);
                }
            };

            progress.finish_and_clear();

            println!(
                "Occupancy Analysis for \"{}\"",
                truncate_prompt(&prompt, 40)
            );
            println!();
            println!("Grid:");
            println!(
                "  Resolution:    {}³ ({} cells)",
                result.resolution,
                result.total_cells()
            );
            println!(
                "  BBox Min:      [{:.2}, {:.2}, {:.2}]",
                result.bbox_min[0], result.bbox_min[1], result.bbox_min[2]
            );
            println!(
                "  BBox Max:      [{:.2}, {:.2}, {:.2}]",
                result.bbox_max[0], result.bbox_max[1], result.bbox_max[2]
            );
            println!();
            println!("Occupancy:");
            println!("  Occupied:      {} voxels", result.occupied_count());
            println!("  Ratio:         {:.2}%", result.occupancy_ratio() * 100.0);

            // Compute logit statistics if available
            if result.has_logits() {
                if let Ok(stats) = robocube::convert::compute_logit_statistics(&result) {
                    println!();
                    println!("Logit Distribution:");
                    println!("  Min:           {:.4}", stats.min);
                    println!("  25th %ile:     {:.4}", stats.p25);
                    println!("  Median:        {:.4}", stats.median);
                    println!("  75th %ile:     {:.4}", stats.p75);
                    println!("  90th %ile:     {:.4}", stats.p90);
                    println!("  Max:           {:.4}", stats.max);
                    println!("  Mean:          {:.4}", stats.mean);
                    println!();
                    println!("Suggested Thresholds:");
                    println!("  10% occupancy: {:.4}", stats.suggest_threshold(0.10));
                    println!("  25% occupancy: {:.4}", stats.suggest_threshold(0.25));
                    println!("  50% occupancy: {:.4}", stats.suggest_threshold(0.50));
                }
            }

            // Try generating cube to get depth info
            if let Ok(cubebox) = occupancy_to_cube(&result, None) {
                println!();
                println!("Octree:");
                println!("  Depth:         {}", cubebox.depth);
                println!(
                    "  Size:          {}x{}x{}",
                    cubebox.size.x, cubebox.size.y, cubebox.size.z
                );
            }
        }
    }

    Ok(())
}

/// Truncate a prompt for display
fn truncate_prompt(prompt: &str, max_len: usize) -> String {
    if prompt.len() <= max_len {
        prompt.to_string()
    } else {
        format!("{}...", &prompt[..max_len - 3])
    }
}
