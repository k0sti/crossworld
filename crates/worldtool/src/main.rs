use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nostr_sdk::prelude::*;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::str::FromStr;

const LIVE_CHAT_D_TAG: &str = "crossworld-dev";
const DEFAULT_SERVER_DIR: &str = "./moq-server";
const MOQ_REPO: &str = "https://github.com/kixelated/moq.git";

#[derive(Subcommand)]
enum ServerCommands {
    /// Initialize MoQ relay server (clone and build)
    Init {
        /// Directory to install server (default: ./moq-server)
        #[arg(short, long, default_value = DEFAULT_SERVER_DIR)]
        dir: PathBuf,

        /// Skip building after clone
        #[arg(long)]
        no_build: bool,
    },

    /// Run the MoQ relay server
    Run {
        /// Server directory (default: ./moq-server)
        #[arg(short, long, default_value = DEFAULT_SERVER_DIR)]
        dir: PathBuf,

        /// Port to bind (default: 4443)
        #[arg(short, long, default_value = "4443")]
        port: u16,

        /// Bind address (default: 0.0.0.0)
        #[arg(short, long, default_value = "0.0.0.0")]
        bind: String,

        /// TLS certificate path (if not provided, generates self-signed cert)
        #[arg(long)]
        tls_cert: Option<PathBuf>,

        /// TLS key path (if not provided, generates self-signed cert)
        #[arg(long)]
        tls_key: Option<PathBuf>,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },
}

#[derive(Parser)]
#[command(name = "worldtool")]
#[command(about = "Crossworld Nostr management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// MoQ relay server management
    #[command(subcommand)]
    Server(ServerCommands),

    /// Initialize the live event (kind 30311)
    InitLive {
        /// Nostr private key (nsec or hex). If not provided, reads from NSEC env variable or nostr-private-key.txt
        #[arg(short, long)]
        nsec: Option<String>,

        /// Relay URLs to publish to (can specify multiple)
        #[arg(short, long, default_values_t = vec![
            "wss://strfry.atlantislabs.space/".to_string(),
            // "wss://relay.damus.io".to_string(),
            // "wss://nos.lol".to_string(),
            // "wss://relay.primal.net".to_string(),
        ])]
        relays: Vec<String>,

        /// Title of the live chat
        #[arg(short, long, default_value = "Crossworld")]
        title: String,

        /// Summary/description
        #[arg(short, long, default_value = "Crossworld Nostr Metaverse")]
        summary: String,

        /// Image URL
        #[arg(short, long)]
        image: Option<String>,

        /// MoQ relay URL for voice/game data streaming
        /// If not specified, uses public test relay (https://relay.moq.dev/anon)
        /// WARNING: Public relay is for testing only. Set up your own relay for production.
        /// See doc/voicechat.md for relay setup instructions.
        #[arg(long, default_missing_value = "https://relay.moq.dev/anon", num_args = 0..=1)]
        streaming: Option<String>,

        /// Status: planned, live, or ended
        #[arg(long, default_value = "live")]
        status: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server(cmd) => match cmd {
            ServerCommands::Init { dir, no_build } => {
                server_init(dir, no_build)?;
            }
            ServerCommands::Run {
                dir,
                port,
                bind,
                tls_cert,
                tls_key,
                verbose,
            } => {
                server_run(dir, port, bind, tls_cert, tls_key, verbose)?;
            }
        },
        Commands::InitLive {
            nsec,
            relays,
            title,
            summary,
            image,
            streaming,
            status,
        } => {
            init_live(nsec, relays, title, summary, image, streaming, status).await?;
        }
    }

    Ok(())
}

async fn init_live(
    nsec_arg: Option<String>,
    relay_urls: Vec<String>,
    title: String,
    summary: String,
    image: Option<String>,
    streaming: Option<String>,
    status: String,
) -> Result<()> {
    // Load .env file if it exists
    let _ = dotenvy::dotenv();

    // Load private key with priority: CLI arg > .env NSEC > nostr-private-key.txt
    let nsec_str = if let Some(key) = nsec_arg {
        key
    } else if let Ok(nsec) = std::env::var("NSEC") {
        nsec
    } else {
        eprintln!("No private key provided.");
        eprintln!("Please provide a private key via:");
        eprintln!("  1. --nsec argument");
        eprintln!("  2. NSEC environment variable in .env");
        std::process::exit(1);
    };

    // Parse the key
    let keys = if nsec_str.starts_with("nsec") {
        Keys::parse(&nsec_str)?
    } else {
        let secret_key = SecretKey::from_str(&nsec_str)?;
        Keys::new(secret_key)
    };

    // Get public key in both formats
    let pubkey_hex = keys.public_key().to_hex();
    let pubkey_bech32 = keys.public_key().to_bech32()?;

    println!("Creating live chat event with pubkey:");
    println!("  Hex:  {}", pubkey_hex);
    println!("  Npub: {}", pubkey_bech32);

    // Create the client
    let client = Client::new(keys.clone());

    // Add relays
    for relay_url in &relay_urls {
        client.add_relay(relay_url).await?;
        println!("Added relay: {}", relay_url);
    }

    // Connect to relays
    client.connect().await;
    println!("Connected to relays");

    // Build the event
    let mut tags = vec![
        Tag::custom(
            TagKind::Custom(std::borrow::Cow::Borrowed("d")),
            vec![LIVE_CHAT_D_TAG],
        ),
        Tag::custom(
            TagKind::Custom(std::borrow::Cow::Borrowed("title")),
            vec![&title],
        ),
        Tag::custom(
            TagKind::Custom(std::borrow::Cow::Borrowed("summary")),
            vec![&summary],
        ),
        Tag::custom(
            TagKind::Custom(std::borrow::Cow::Borrowed("status")),
            vec![&status],
        ),
    ];

    if let Some(img) = image {
        tags.push(Tag::custom(
            TagKind::Custom(std::borrow::Cow::Borrowed("image")),
            vec![&img],
        ));
    }

    // Add streaming URL (use default public relay if not specified)
    let stream_url = streaming.unwrap_or_else(|| {
        const DEFAULT_RELAY: &str = "https://relay.moq.dev/anon";
        eprintln!("\n⚠️  WARNING: Using public test relay for MoQ streaming");
        eprintln!("   URL: {}", DEFAULT_RELAY);
        eprintln!("   This relay is for TESTING ONLY and may have:");
        eprintln!("   - Rate limits or connection limits");
        eprintln!("   - Availability issues");
        eprintln!("   - No privacy guarantees");
        eprintln!("\n   For production use, set up your own MoQ relay server.");
        eprintln!("   See doc/voicechat.md for setup instructions.\n");
        DEFAULT_RELAY.to_string()
    });
    tags.push(Tag::custom(
        TagKind::Custom(std::borrow::Cow::Borrowed("streaming")),
        vec![&stream_url],
    ));

    // Add relay tags for chat relays
    for relay_url in &relay_urls {
        tags.push(Tag::custom(
            TagKind::Custom(std::borrow::Cow::Borrowed("relay")),
            vec![relay_url],
        ));
    }

    // Add hashtags
    tags.push(Tag::hashtag("crossworld"));
    tags.push(Tag::hashtag("metaverse"));

    // Create the event
    let event_builder = EventBuilder::new(Kind::from(30311), "").tags(tags);
    let event = event_builder.sign_with_keys(&keys)?;

    println!("\nEvent created:");
    println!("  ID: {}", event.id);
    println!("  Kind: {}", event.kind);
    println!("  Pubkey: {}", event.pubkey);
    println!("  d-tag: {}", LIVE_CHAT_D_TAG);
    println!("  a-tag: 30311:{}:{}", pubkey_hex, LIVE_CHAT_D_TAG);
    println!("\nEvent JSON:");
    println!("{}", serde_json::to_string_pretty(&event)?);

    // Publish the event
    println!("\nPublishing event...");
    let output = client.send_event(event).await?;
    println!("Event published: {:?}", output);

    // Wait a bit for confirmation
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("\nLive chat event initialized successfully!");
    println!(
        "Users can now send messages to this live chat using kind 1311 with a-tag: 30311:{}:{}",
        pubkey_hex, LIVE_CHAT_D_TAG
    );
    println!("\nVoice streaming configured:");
    println!("  MoQ relay: {}", stream_url);
    if stream_url == "https://relay.moq.dev/anon" {
        println!("\n  ⚠️  Remember to set up your own MoQ relay for production!");
        println!("     See doc/voicechat.md for instructions.");
    }

    Ok(())
}

fn server_init(dir: PathBuf, no_build: bool) -> Result<()> {
    println!("Initializing MoQ relay server...");
    println!("Directory: {}", dir.display());

    // Check if directory already exists
    if dir.exists() {
        eprintln!("❌ Directory already exists: {}", dir.display());
        eprintln!("   Remove it or choose a different directory.");
        std::process::exit(1);
    }

    // Check if git is available
    let git_check = ProcessCommand::new("git")
        .arg("--version")
        .output()
        .context("Git is not installed. Please install git first.")?;

    if !git_check.status.success() {
        eprintln!("❌ Git is not available");
        std::process::exit(1);
    }

    println!("\n📥 Cloning MoQ repository...");
    println!("   Source: {}", MOQ_REPO);

    let clone_status = ProcessCommand::new("git")
        .args(["clone", MOQ_REPO, dir.to_str().unwrap()])
        .status()
        .context("Failed to clone repository")?;

    if !clone_status.success() {
        eprintln!("❌ Failed to clone repository");
        std::process::exit(1);
    }

    println!("✅ Repository cloned successfully");

    if no_build {
        println!("\n⏭️  Skipping build (--no-build flag set)");
        println!("\nTo build manually:");
        println!("  cd {}", dir.display());
        println!("  cargo build --release --bin moq-relay");
    } else {
        // Check if cargo is available
        let cargo_check = ProcessCommand::new("cargo")
            .arg("--version")
            .output()
            .context("Cargo is not installed. Please install Rust toolchain.")?;

        if !cargo_check.status.success() {
            eprintln!("❌ Cargo is not available");
            eprintln!(
                "   Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            );
            std::process::exit(1);
        }

        println!("\n🔨 Building MoQ relay server...");
        println!("   This may take a few minutes...");

        let build_status = ProcessCommand::new("cargo")
            .args(["build", "--release", "--bin", "moq-relay"])
            .current_dir(&dir)
            .status()
            .context("Failed to build server")?;

        if !build_status.success() {
            eprintln!("❌ Build failed");
            std::process::exit(1);
        }

        println!("✅ Build completed successfully");
    }

    println!("\n🎉 MoQ relay server initialized!");
    println!("\nNext steps:");
    println!("  1. Run the server:");
    println!("     worldtool server run");
    println!("\n  2. Update live event with your server URL:");
    println!("     worldtool init-live --streaming https://localhost:4443/anon");
    println!("\n  For production deployment, see doc/moq-relay-setup.md");

    Ok(())
}

fn server_run(
    dir: PathBuf,
    port: u16,
    bind: String,
    tls_cert: Option<PathBuf>,
    tls_key: Option<PathBuf>,
    verbose: bool,
) -> Result<()> {
    println!("Starting MoQ relay server...");

    // Check if server directory exists
    if !dir.exists() {
        eprintln!("❌ Server directory not found: {}", dir.display());
        eprintln!("   Run 'worldtool server init' first");
        std::process::exit(1);
    }

    // Check if binary exists
    let binary_path = dir.join("target/release/moq-relay");
    if !binary_path.exists() {
        eprintln!("❌ Server binary not found: {}", binary_path.display());
        eprintln!("   Run 'worldtool server init' or build manually:");
        eprintln!(
            "     cd {} && cargo build --release --bin moq-relay",
            dir.display()
        );
        std::process::exit(1);
    }

    // Handle TLS certificates
    let (cert_path, key_path) = match (tls_cert, tls_key) {
        (Some(cert), Some(key)) => {
            // Use provided certificates
            if !cert.exists() {
                eprintln!("❌ Certificate file not found: {}", cert.display());
                std::process::exit(1);
            }
            if !key.exists() {
                eprintln!("❌ Key file not found: {}", key.display());
                std::process::exit(1);
            }
            println!("🔒 Using provided TLS certificate");
            (cert, key)
        }
        (None, None) => {
            // Generate self-signed certificate
            let cert_dir = dir.join("certs");
            std::fs::create_dir_all(&cert_dir).context("Failed to create certs directory")?;

            let cert_file = cert_dir.join("cert.pem");
            let key_file = cert_dir.join("key.pem");

            if !cert_file.exists() || !key_file.exists() {
                println!("🔐 Generating self-signed certificate...");

                let openssl_status = ProcessCommand::new("openssl")
                    .args([
                        "req",
                        "-x509",
                        "-newkey",
                        "rsa:4096",
                        "-keyout",
                        key_file.to_str().unwrap(),
                        "-out",
                        cert_file.to_str().unwrap(),
                        "-days",
                        "365",
                        "-nodes",
                        "-subj",
                        "/CN=localhost",
                    ])
                    .status()
                    .context("Failed to generate certificate. Is openssl installed?")?;

                if !openssl_status.success() {
                    eprintln!("❌ Failed to generate certificate");
                    std::process::exit(1);
                }

                println!("✅ Self-signed certificate generated");
                println!("   ⚠️  WARNING: Self-signed certificates are for development only!");
                println!("   For production, use Let's Encrypt or your own certificates.");
            } else {
                println!("🔒 Using existing self-signed certificate");
            }

            (cert_file, key_file)
        }
        _ => {
            eprintln!("❌ Both --tls-cert and --tls-key must be provided together");
            std::process::exit(1);
        }
    };

    // Build command arguments
    let bind_addr = format!("{}:{}", bind, port);
    let mut args = vec![
        "--bind".to_string(),
        bind_addr.clone(),
        "--tls-cert".to_string(),
        cert_path.to_str().unwrap().to_string(),
        "--tls-key".to_string(),
        key_path.to_str().unwrap().to_string(),
    ];

    if verbose {
        args.push("-v".to_string());
    }

    println!("\n🚀 Starting server...");
    println!("   Bind address: {}", bind_addr);
    println!("   Certificate: {}", cert_path.display());
    println!("   Key: {}", key_path.display());
    println!(
        "\n   Access URL: https://{}:{}/anon",
        if bind == "0.0.0.0" {
            "localhost"
        } else {
            &bind
        },
        port
    );
    println!("\n   Press Ctrl+C to stop\n");
    println!("{}", "─".repeat(60));

    // Run the server
    let status = ProcessCommand::new(&binary_path)
        .args(&args)
        .status()
        .context("Failed to start server")?;

    if !status.success() {
        eprintln!("\n❌ Server exited with error");
        std::process::exit(1);
    }

    Ok(())
}
