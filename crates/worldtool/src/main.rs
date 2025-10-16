use anyhow::Result;
use clap::{Parser, Subcommand};
use nostr_sdk::prelude::*;
use std::str::FromStr;

const APP_PUBKEY: &str = "e9aeccc7e11ce384c2c6ad6e1e7cee9c889294ad1213da7e1f18636c0c8149ac";
const LIVE_CHAT_D_TAG: &str = "crossworld-dev";

#[derive(Parser)]
#[command(name = "worldtool")]
#[command(about = "Crossworld Nostr management tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the live chat event (kind 30311)
    InitLiveChat {
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
        #[arg(short, long, default_value = "Crossworld Live Chat")]
        title: String,

        /// Summary/description
        #[arg(short, long, default_value = "Live chat for Crossworld metaverse")]
        summary: String,

        /// Image URL
        #[arg(short, long)]
        image: Option<String>,

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
        Commands::InitLiveChat {
            nsec,
            relays,
            title,
            summary,
            image,
            status,
        } => {
            init_live_chat(nsec, relays, title, summary, image, status).await?;
        }
    }

    Ok(())
}

async fn init_live_chat(
    nsec_arg: Option<String>,
    relay_urls: Vec<String>,
    title: String,
    summary: String,
    image: Option<String>,
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

    // Verify the public key matches
    let pubkey_hex = keys.public_key().to_hex();
    if pubkey_hex != APP_PUBKEY {
        eprintln!("Warning: Provided key pubkey ({}) does not match APP_PUBKEY ({})", pubkey_hex, APP_PUBKEY);
        eprintln!("The live chat event will be created with the provided key's pubkey.");
    }

    println!("Creating live chat event with pubkey: {}", pubkey_hex);

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
        Tag::custom(TagKind::Custom(std::borrow::Cow::Borrowed("d")), vec![LIVE_CHAT_D_TAG]),
        Tag::custom(TagKind::Custom(std::borrow::Cow::Borrowed("title")), vec![&title]),
        Tag::custom(TagKind::Custom(std::borrow::Cow::Borrowed("summary")), vec![&summary]),
        Tag::custom(TagKind::Custom(std::borrow::Cow::Borrowed("status")), vec![&status]),
    ];

    if let Some(img) = image {
        tags.push(Tag::custom(TagKind::Custom(std::borrow::Cow::Borrowed("image")), vec![&img]));
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
    println!("Users can now send messages to this live chat using kind 1311 with a-tag: 30311:{}:{}", pubkey_hex, LIVE_CHAT_D_TAG);

    Ok(())
}
