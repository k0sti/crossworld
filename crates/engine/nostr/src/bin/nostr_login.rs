//! NIP-46 QR code login test binary
//!
//! Usage:
//!   cargo run -p crossworld-nostr --features nip46 --bin nostr-login
//!   cargo run -p crossworld-nostr --features nip46 --bin nostr-login -- --relay wss://relay.nsec.app
//!   cargo run -p crossworld-nostr --features nip46 --bin nostr-login -- --help

use nostr::{Keys, NostrSigner, ToBech32};
use nostr_connect::prelude::{NostrConnect, NostrConnectURI};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

/// Default relay for NIP-46 connections
const DEFAULT_RELAY: &str = "wss://relay.nsec.app";

/// Application name for NIP-46 metadata
const APP_NAME: &str = "Crossworld";

/// Default timeout for connection attempts (2 minutes)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// Stored connection state
#[derive(Debug, Serialize, Deserialize)]
struct ConnectionState {
    /// Relay URL used for connection
    relay: String,
    /// Connected user's npub
    npub: Option<String>,
    /// Last successful connection timestamp
    last_connected: Option<u64>,
}

impl Default for ConnectionState {
    fn default() -> Self {
        Self {
            relay: DEFAULT_RELAY.to_string(),
            npub: None,
            last_connected: None,
        }
    }
}

/// Get cache directory for nostr state
fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("crossworld")
}

/// Load connection state from cache
fn load_state() -> ConnectionState {
    let path = cache_dir().join("nostr.toml");
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(state) => return state,
                Err(e) => eprintln!("Warning: Failed to parse {}: {}", path.display(), e),
            },
            Err(e) => eprintln!("Warning: Failed to read {}: {}", path.display(), e),
        }
    }
    ConnectionState::default()
}

/// Save connection state to cache
fn save_state(state: &ConnectionState) -> io::Result<()> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("nostr.toml");
    let content = toml::to_string_pretty(state).map_err(io::Error::other)?;
    std::fs::write(&path, content)?;
    println!("State saved to: {}", path.display());
    Ok(())
}

/// Parse command line arguments
fn parse_args() -> (String, bool) {
    let args: Vec<String> = std::env::args().collect();
    let mut relay = String::new();
    let mut show_help = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--relay" | "-r" => {
                if i + 1 < args.len() {
                    relay = args[i + 1].clone();
                    i += 1;
                }
            }
            "--help" | "-h" => {
                show_help = true;
            }
            _ => {}
        }
        i += 1;
    }

    (relay, show_help)
}

/// Print usage information
fn print_help() {
    println!(
        r#"nostr-login - NIP-46 QR code login for Crossworld

USAGE:
    nostr-login [OPTIONS]

OPTIONS:
    -r, --relay <URL>    Relay URL for NIP-46 connection
                         Default: {}
    -h, --help           Show this help message

INTERACTIVE MODE:
    If no relay is specified, you'll be prompted to choose one.

STATE FILE:
    Connection state is saved to: ~/.cache/crossworld/nostr.toml
"#,
        DEFAULT_RELAY
    );
}

/// Prompt user to select or enter a relay
fn prompt_relay(current: &str) -> String {
    let relays = [
        "wss://relay.nsec.app",
        "wss://relay.damus.io",
        "wss://nos.lol",
        "wss://relay.nostr.band",
    ];

    println!("\nSelect a relay or enter custom URL:");
    for (i, r) in relays.iter().enumerate() {
        let marker = if *r == current { " (current)" } else { "" };
        println!("  {}. {}{}", i + 1, r, marker);
    }
    println!("  c. Enter custom URL");
    println!("  q. Quit");
    print!("\nChoice [1]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    if input.is_empty() || input == "1" {
        relays[0].to_string()
    } else if input == "q" || input == "Q" {
        std::process::exit(0);
    } else if input == "c" || input == "C" {
        print!("Enter relay URL: ");
        io::stdout().flush().unwrap();
        let mut url = String::new();
        io::stdin().read_line(&mut url).unwrap();
        url.trim().to_string()
    } else if let Ok(n) = input.parse::<usize>() {
        if n > 0 && n <= relays.len() {
            relays[n - 1].to_string()
        } else {
            relays[0].to_string()
        }
    } else {
        relays[0].to_string()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("nostr_connect=debug".parse().unwrap())
                .add_directive("nostr_relay_pool=info".parse().unwrap()),
        )
        .init();

    // Parse CLI args
    let (cli_relay, show_help) = parse_args();

    if show_help {
        print_help();
        return Ok(());
    }

    // Load saved state
    let mut state = load_state();

    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           Crossworld NIP-46 Login Test                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    // Show current state if available
    if let Some(ref npub) = state.npub {
        println!("\nPreviously connected as: {}", npub);
        println!("Last relay: {}", state.relay);
    }

    // Determine relay to use
    let relay_url = if !cli_relay.is_empty() {
        cli_relay
    } else {
        prompt_relay(&state.relay)
    };

    println!("\nUsing relay: {}", relay_url);
    state.relay = relay_url.clone();

    // Generate client keys for this session
    let client_keys = Keys::generate();
    println!("Client pubkey: {}", client_keys.public_key().to_hex());

    // Parse relay URL
    let relay = nostr::RelayUrl::parse(&relay_url)?;

    // Create nostrconnect URI (client-side URI that signer will scan)
    let uri = NostrConnectURI::client(client_keys.public_key(), vec![relay], APP_NAME);
    let uri_string = uri.to_string();

    println!("\n────────────────────────────────────────────────────────────────");
    println!("Scan this QR code with your Nostr signer app (e.g., Amber):");
    println!("────────────────────────────────────────────────────────────────\n");

    // Generate and print QR code to terminal
    qr2term::print_qr(&uri_string)?;

    println!("\n────────────────────────────────────────────────────────────────");
    println!("Connection URI (for manual entry):");
    println!("{}", uri_string);
    println!("────────────────────────────────────────────────────────────────");

    println!(
        "\nWaiting for connection... (timeout: {} seconds)",
        DEFAULT_TIMEOUT.as_secs()
    );
    println!("Press Ctrl+C to cancel.\n");

    // Create NostrConnect client with our client URI
    // This will wait for the signer (Amber) to connect when we call get_public_key
    let connect = NostrConnect::new(uri, client_keys, DEFAULT_TIMEOUT, None)?;

    println!("NostrConnect created, waiting for signer to approve...");

    // The first call to get_public_key will:
    // 1. Connect to the relay
    // 2. Subscribe to NostrConnect events
    // 3. Wait for the signer to send an ACK response (happens when user approves in Amber)
    // 4. Return the signer's public key
    match NostrSigner::get_public_key(&connect).await {
        Ok(user_pubkey) => {
            let npub = user_pubkey.to_bech32()?;
            println!("\n✅ Connected successfully!");
            println!("────────────────────────────────────────────────────────────────");
            println!("User public key: {}", npub);
            println!("Hex: {}", user_pubkey.to_hex());
            println!("────────────────────────────────────────────────────────────────");

            // Update and save state
            state.npub = Some(npub);
            state.last_connected = Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            );
            if let Err(e) = save_state(&state) {
                eprintln!("Warning: Failed to save state: {}", e);
            }

            // Test signing
            println!("\nTesting event signing...");
            let event_builder = nostr::EventBuilder::text_note("Test note from Crossworld login");
            let unsigned = event_builder.build(user_pubkey);
            match connect.sign_event(unsigned).await {
                Ok(event) => {
                    println!("✅ Signing works! Event ID: {}", event.id.to_hex());
                }
                Err(e) => {
                    println!("⚠️  Signing test failed: {}", e);
                }
            }

            // Get the bunker URI for future reconnection
            match connect.bunker_uri().await {
                Ok(bunker_uri) => {
                    println!("\nBunker URI for future connections:");
                    println!("{}", bunker_uri);
                }
                Err(e) => {
                    println!("\n⚠️  Could not get bunker URI: {}", e);
                }
            }

            connect.shutdown().await;
            println!("\n🎉 Login test complete!");
        }
        Err(e) => {
            println!("\n❌ Connection failed: {}", e);
            connect.shutdown().await;
        }
    }

    Ok(())
}
