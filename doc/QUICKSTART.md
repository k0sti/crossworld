# Crossworld Voice Chat - Quick Start Guide

Get voice chat running in 5 minutes!

## Prerequisites

- Rust toolchain (for worldtool)
- Git
- OpenSSL (usually pre-installed)
- Bun (package manager and runtime)

## Step 1: Initialize MoQ Relay Server

```bash
cd crates/worldtool

# Clone and build MoQ relay server
cargo run -- server init

# This will:
# - Clone https://github.com/kixelated/moq to ./moq-server
# - Build the release binary (~2-3 minutes)
```

## Step 2: Start the Relay Server

```bash
# In the same terminal (or a new one)
cargo run -- server run

# This will:
# - Generate a self-signed certificate (for dev)
# - Start relay on https://localhost:4443
```

You should see:
```
🚀 Starting server...
   Bind address: 0.0.0.0:4443
   Certificate: ./moq-server/certs/cert.pem
   Key: ./moq-server/certs/key.pem

   Access URL: https://localhost:4443/anon

   Press Ctrl+C to stop
```

Keep this terminal running!

## Step 3: Configure Live Event

Open a **new terminal**:

```bash
cd crates/worldtool

# Set NSEC environment variable (or use --nsec flag)
export NSEC="your-nostr-private-key-here"

# Create live event with local relay
cargo run -- init-live --streaming https://localhost:4443/anon
```

You should see:
```
✅ Live chat event initialized successfully!

Voice streaming configured:
  MoQ relay: https://localhost:4443/anon
```

## Step 4: Run the App

```bash
cd packages/app

# Install dependencies (first time only)
bun install

# Start the app
bun run dev
```

Open browser to `http://localhost:5173` (or the URL shown)

## Step 5: Test Voice Chat

1. **Login** with Nostr extension (Alby, nos2x, etc.)
2. Click the **headphones icon (🎧)** in the left sidebar
3. You should see a toast: "Voice connected"
4. Click the **microphone icon (🎤)** to unmute
5. Grant microphone permission when prompted
6. Start talking - the mic button will pulse when speaking!

## Testing with Multiple Users

Open the app in **two browser tabs** (or two browsers):

**Tab 1**:
1. Login as User A
2. Connect voice
3. Enable mic

**Tab 2**:
1. Login as User B
2. Connect voice
3. Enable mic

Both users should:
- See participant count badge (🎧 with number)
- Hear each other's audio
- See speaking indicators when the other person talks

## Troubleshooting

### "Failed to connect to MoQ relay"

**Check relay is running**:
```bash
# You should see server logs in the terminal
```

**Test relay connectivity**:
```bash
curl -k https://localhost:4443/anon
# Should get: "WebTransport not supported" (this is expected)
```

### "Microphone access denied"

**In Chrome/Edge**:
- Click the lock icon in the address bar
- Reset permissions
- Reload page

**HTTPS required**: Voice chat requires HTTPS (or localhost)

### "No streaming URL found in live event"

**Re-run init-live**:
```bash
cd crates/worldtool
cargo run -- init-live --streaming https://localhost:4443/anon
```

**Restart the app** to fetch updated live event

### Certificate warnings in browser

**Expected for self-signed certs!**
- Click "Advanced"
- Click "Proceed to localhost (unsafe)"
- This is safe for local development

## Production Deployment

For production, see:
- **Voice Chat Guide**: [doc/features/voice-chat.md](features/voice-chat.md)
- **Server Design**: [doc/reference/server.md](reference/server.md)

Quick production checklist:
- ✅ Get a domain (e.g., moq.yourdomain.com)
- ✅ Set up Let's Encrypt certificate
- ✅ Use reverse proxy (Caddy/nginx)
- ✅ Configure firewall (port 4443)
- ✅ Run relay with systemd/docker
- ✅ Update live event with production URL

## Alternative: Use Public Relay

For quick testing without local server:

```bash
cd crates/worldtool

# Uses public relay (https://relay.moq.dev/anon)
cargo run -- init-live
```

⚠️ Public relay is shared and may have rate limits

## Next Steps

- **Customize UI**: Edit `packages/app/src/components/VoiceButton.tsx`
- **Add features**: Participant list, mute controls, etc.
- **Deploy**: Follow production setup guide
- **Monitor**: Check relay logs for connections

## Common Commands

```bash
# Server management
cargo run -- server init              # Initialize server
cargo run -- server init --dir ~/moq  # Custom directory
cargo run -- server run               # Run server
cargo run -- server run --port 8443   # Custom port
cargo run -- server run --verbose     # Verbose logs

# Live event management
cargo run -- init-live                           # Default (public relay)
cargo run -- init-live --streaming <URL>         # Custom relay
cargo run -- init-live --title "My World"        # Custom title

# Help
cargo run -- server init --help
cargo run -- server run --help
cargo run -- init-live --help
```

## Support

Issues? Check:
1. Server logs (terminal running `server run`)
2. Browser console (F12)
3. Network tab for WebTransport errors
4. Firewall/antivirus blocking port 4443

Still stuck? Open an issue with:
- Error messages
- Server logs
- Browser console output
