//! Trellis CLI - Command-line interface for text-to-voxel generation

use clap::{Parser, Subcommand};

/// Trellis CLI - Text/Image-to-voxel generation tool
#[derive(Parser)]
#[command(name = "trellis")]
#[command(about = "Generate voxel models from text/image prompts", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
        Commands::Health { server } => {
            println!("Trellis health check not yet implemented");
            println!("Server: {}", server);
        }
    }

    Ok(())
}
