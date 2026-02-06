# worldtool

CLI tool for Nostr event management.

## Features

- Initialize NIP-53 live chat events for Crossworld

## Installation

```bash
cargo build --release -p worldtool
```

The binary will be available at `target/release/worldtool`.

## Usage

### Initialize Live Chat Event

Creates a NIP-53 kind 30311 live event for the Crossworld chat system.

**Private Key Priority:**
1. Command line `--nsec` argument
2. `NSEC` environment variable (from `.env` file)

```bash
# Using NSEC from .env file
echo 'NSEC=nsec1...' >> .env
worldtool init-live-chat

# Specify private key directly
worldtool init-live-chat --nsec <nsec or hex>

# Customize title and description
worldtool init-live-chat \
  --title "Crossworld Dev Chat" \
  --summary "Development chat for Crossworld metaverse" \
  --status live

# Specify custom relays
worldtool init-live-chat \
  --relays wss://relay1.com \
  --relays wss://relay2.com

# Add an image
worldtool init-live-chat \
  --image https://example.com/image.jpg
```

### Options

- `--nsec, -p <KEY>`: Nostr private key (nsec or hex format). If not provided, reads from `NSEC` env variable
- `--relays, -r <URL>`: Relay URLs to publish to (can specify multiple). Default relays:
  - wss://strfry.atlantislabs.space/
  - wss://relay.damus.io
  - wss://nos.lol
  - wss://relay.primal.net
- `--title, -t <TEXT>`: Title of the live chat (default: "Crossworld Live Chat")
- `--summary, -s <TEXT>`: Description (default: "Live chat for Crossworld metaverse")
- `--image, -i <URL>`: Preview image URL
- `--status <STATUS>`: Event status: planned, live, or ended (default: "live")

## Event Details

The tool creates a NIP-53 live event with:
- Kind: 30311 (Live Event)
- d-tag: `crossworld-dev`
- a-tag: `30311:<pubkey>:crossworld-dev`

Chat messages should use:
- Kind: 1311 (Live Chat Message)
- a-tag referencing the live event

## Example Output

```
Creating live chat event with pubkey: e9aeccc7e11ce384c2c6ad6e1e7cee9c889294ad1213da7e1f18636c0c8149ac
Added relay: wss://strfry.atlantislabs.space/
Added relay: wss://relay.damus.io
Connected to relays

Event created:
  ID: abc123...
  Kind: 30311
  Pubkey: e9aeccc7e11ce384c2c6ad6e1e7cee9c889294ad1213da7e1f18636c0c8149ac
  d-tag: crossworld-dev
  a-tag: 30311:e9aeccc7e11ce384c2c6ad6e1e7cee9c889294ad1213da7e1f18636c0c8149ac:crossworld-dev

Event JSON:
{
  "id": "abc123...",
  "pubkey": "e9aeccc7...",
  "created_at": 1234567890,
  "kind": 30311,
  "tags": [
    ["d", "crossworld-dev"],
    ["title", "Crossworld Live Chat"],
    ["summary", "Live chat for Crossworld metaverse"],
    ["status", "live"],
    ["t", "crossworld"],
    ["t", "metaverse"]
  ],
  "content": "",
  "sig": "..."
}

Publishing event...
Event published: ...

Live chat event initialized successfully!
Users can now send messages to this live chat using kind 1311 with a-tag: 30311:e9aeccc7e11ce384c2c6ad6e1e7cee9c889294ad1213da7e1f18636c0c8149ac:crossworld-dev
```

## References

- [NIP-53: Live Activities](https://github.com/nostr-protocol/nips/blob/master/53.md)
- [NIP-01: Basic protocol flow description](https://github.com/nostr-protocol/nips/blob/master/01.md)
