# Voice Chat (MoQ)

## Overview

Crossworld uses **MoQ (Media over QUIC)** for low-latency spatial voice chat. MoQ provides real-time audio streaming with sub-second latency using a publish/subscribe model over QUIC transport.

**Key Features**:
- Sub-second latency (typically 100-300ms)
- Spatial audio based on avatar positions
- Speaking detection and participant tracking
- Dual discovery (Nostr + MoQ announcements)
- Room isolation via broadcast paths

## Quick Start

### Testing with Public Relay (No Setup Required)

```bash
cd crates/worldtool
cargo run -- init-live
```

The default setup uses a public test relay. No additional configuration needed.

**Limitations**:
- Shared with other developers
- May have rate limits
- No availability guarantees
- Not suitable for production

### Local Development Server

Use worldtool to set up a local MoQ relay:

```bash
cd crates/worldtool

# Initialize server (clone and build)
cargo run -- server init

# Run server (generates self-signed cert automatically)
cargo run -- server run

# In another terminal, configure live event
cargo run -- init-live --streaming https://localhost:4443/anon
```

**Worldtool Commands**:
- `server init [--dir <path>]` - Clone and build MoQ relay
- `server run [options]` - Start the relay server

**Options for `server run`**:
- `--port <PORT>` - Port to bind (default: 4443)
- `--bind <ADDR>` - Bind address (default: 0.0.0.0)
- `--tls-cert <PATH>` - Custom TLS certificate
- `--tls-key <PATH>` - Custom TLS key
- `--verbose` - Enable verbose logging

**Examples**:
```bash
# Custom port
cargo run -- server run --port 8443

# With custom certificates
cargo run -- server run --tls-cert /path/to/cert.pem --tls-key /path/to/key.pem
```

## Architecture

### MoQ Connection Model

**Components**:
- **Connection**: Single persistent QUIC connection per client to relay server
- **Broadcasts**: Each participant publishes audio broadcast with unique path
- **Discovery**: Relay announces available broadcasts; clients subscribe to desired streams
- **Audio Pipeline**:
  - Publisher: `getUserMedia()` → Opus encoding → MoQ broadcast track
  - Subscriber: MoQ track → Opus decoding → `AudioContext` playback

### Broadcast Naming

Each participant broadcasts at path: `crossworld/voice/<d-tag>/<npub>`

Example: `crossworld/voice/crossworld-dev/npub1abc...xyz`

**Benefits**:
- Easy filtering of broadcasts by world
- Participant identification via npub
- Support for multiple worlds
- Room isolation through path namespacing

### Dual Discovery System

**1. Nostr-based Discovery**:
- Via ClientStatusService (kind 30315 events)
- Slower but reliable
- Works across network boundaries

**2. MoQ Announcement-based Discovery**:
- Via relay's native announcement system
- Fast local discovery
- Real-time participant updates

**Discovery Source Indicators**:
- `"nostr"` - Only found via Nostr
- `"moq"` - Only found via MoQ announcements
- `"both"` - Found via both methods (ideal!)

## Nostr Integration

### Live Event Discovery (Kind 30311)

Server configuration advertised in NIP-53 Live Event:

```json
{
  "kind": 30311,
  "tags": [
    ["d", "crossworld-dev"],
    ["title", "Crossworld Live Chat"],
    ["summary", "Live chat for Crossworld metaverse"],
    ["status", "live"],
    ["streaming", "https://moq.example.com/anon"],
    ["relay", "wss://strfry.atlantislabs.space/"],
    ["t", "crossworld"]
  ]
}
```

**Key Tags**:
- `d` - World identifier
- `streaming` - MoQ relay URL for voice/game data
- `relay` - Nostr relay(s) for chat (kind 1311 messages)
- `status` - `live` when active, `ended` when closed

**Client Startup Flow**:
1. Fetch kind 30311 with `d=crossworld-dev`
2. Parse `streaming` tag for MoQ relay URL
3. Parse `relay` tags for chat relay URLs
4. Connect to MoQ relay for voice
5. Use chat relays for kind 1311 messages

## Implementation

### TypeScript Components

**1. MoQ Connection Manager** (`src/voice/moq-connection.ts`):
```typescript
interface VoiceConnection {
  relay_url: string;
  connection: Moq.Connection;
  status: 'connecting' | 'connected' | 'disconnected';
}
```
- Establishes and maintains MoQ connection
- Handles reconnection with exponential backoff
- Exposes connection state via signals

**2. Audio Publisher** (`src/voice/publisher.ts`):
```typescript
interface AudioPublisher {
  broadcast: Hang.Publish.Broadcast;
  mic_enabled: Signal<boolean>;
  speaking: Signal<boolean>;
}
```
- Acquires microphone via `getUserMedia()`
- Encodes audio to Opus
- Publishes to MoQ broadcast track
- Detects speaking activity

**3. Audio Subscriber** (`src/voice/subscriber.ts`):
- Listens for MoQ announcements
- Creates watchers for participant broadcasts
- Decodes Opus audio
- Plays audio via `AudioContext`

**4. Voice Manager** (`src/voice/voice-manager.ts`):
- Coordinates publisher and subscriber
- Tracks participants
- Manages speaking states
- Handles spatial audio positioning

## Production Deployment

### Prerequisites

- Server with public IP
- Domain name (e.g., moq.yourdomain.com)
- HTTPS certificate (Let's Encrypt recommended)
- Open port 4443 (or custom port)

### Method 1: Native Installation

**Install Rust**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Clone and Build**:
```bash
git clone https://github.com/kixelated/moq.git
cd moq
cargo build --release --bin moq-relay
```

**Run with systemd** (`/etc/systemd/system/moq-relay.service`):
```ini
[Unit]
Description=MoQ Relay Server
After=network.target

[Service]
Type=simple
User=moq
WorkingDirectory=/opt/moq
ExecStart=/opt/moq/target/release/moq-relay \
  --bind 0.0.0.0:4443 \
  --tls-cert /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem \
  --tls-key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

**Enable and Start**:
```bash
sudo systemctl daemon-reload
sudo systemctl enable moq-relay
sudo systemctl start moq-relay
```

### Method 2: Docker

**Docker Run**:
```bash
docker run -d \
  --name moq-relay \
  -p 4443:4443 \
  -v /etc/letsencrypt:/etc/letsencrypt:ro \
  kixelated/moq-relay \
  --bind 0.0.0.0:4443 \
  --tls-cert /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem \
  --tls-key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem
```

**Docker Compose** (`docker-compose.yml`):
```yaml
version: '3.8'
services:
  moq-relay:
    image: kixelated/moq-relay:latest
    container_name: moq-relay
    restart: unless-stopped
    ports:
      - "4443:4443/tcp"
      - "4443:4443/udp"
    volumes:
      - /etc/letsencrypt:/etc/letsencrypt:ro
    command: >
      --bind 0.0.0.0:4443
      --tls-cert /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem
      --tls-key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem
```

### HTTPS Setup

**Option A: Caddy (Automatic HTTPS)**:
```caddyfile
moq.yourdomain.com {
    reverse_proxy localhost:4443 {
        transport http {
            versions h2
        }
    }
}
```

**Option B: Let's Encrypt + certbot**:
```bash
sudo apt install certbot
sudo certbot certonly --standalone -d moq.yourdomain.com
```

### Firewall Configuration

**UFW**:
```bash
sudo ufw allow 4443/tcp
sudo ufw allow 4443/udp
```

**firewalld**:
```bash
sudo firewall-cmd --permanent --add-port=4443/tcp
sudo firewall-cmd --permanent --add-port=4443/udp
sudo firewall-cmd --reload
```

### Update Live Event

```bash
cd crates/worldtool
cargo run -- init-live --streaming "https://moq.yourdomain.com/anon"
```

## Debugging

### Quick Diagnosis Checklist

**1. Check MoQ Connection**:
```
✅ Good: [MoQ Connection] Status: connected
❌ Bad: [MoQ Connection] Status: connecting (stuck)
```

**2. Check Broadcast Announcement**:
```
✅ Good: [MoQ Publisher] Creating broadcast: { path: "...", ... }
❌ Bad: (No "Creating broadcast" message)
```

**3. Check Discovery Systems**:
```
✅ Good (Nostr): [MoQ Subscriber] Nostr discovery active
✅ Good (MoQ): [MoQ Subscriber] Announcement received: { ... }
```

**4. Check Participant Detection**:
```
✅ Good: [MoQ Subscriber] Watcher created and active for: npub1...
❌ Bad: (No watcher creation messages)
```

### Common Issues

**Connection Stuck on "connecting"**:
- Check relay URL format (`https://relay.example.com/anon`)
- Verify relay is running
- Check browser console for CORS/SSL errors
- Try different relay

**No Participants Detected**:
- Single user (only one connected)
- Discovery mismatch (different d-tags/paths)
- Relay not forwarding announcements

**Participant Detected But No Audio**:
- Audio pipeline issue
- Volume settings (muted)
- Browser autoplay restrictions
- Check speaking state updates in console

### Testing Procedure

**Local Relay Test**:
```bash
# Terminal 1: Start relay
just moq-relay

# Terminal 2: Update live event
cd crates/worldtool
cargo run -- init-live --streaming http://localhost:4443/anon
```

Then in browser:
1. Open two windows (use incognito for different identity)
2. Login to both
3. Enable voice in both windows
4. Check console logs for announcements
5. Verify participants detected in both windows

### Console Helpers

```javascript
// Check voice manager state
voiceManager.status.peek()  // 'disconnected' | 'connecting' | 'connected'
voiceManager.getParticipants()  // Array of participants
voiceManager.getParticipantCount()  // Number

// Check connection state
moqConnection.isConnected()  // boolean
moqConnection.status.peek()  // connection status
```

### URL Format Reference

**✅ Correct**:
```
http://localhost:4443/anon
https://relay.moq.dev/anon
https://relay.cloudflare.mediaoverquic.com/anon
```

**❌ Incorrect**:
```
https://relay.moq.dev/crossworld-dev  (custom path won't work)
https://relay.moq.dev  (missing /anon suffix)
http://relay.moq.dev/anon  (should be https for public)
```

## Monitoring

### View Logs

**systemd**:
```bash
sudo journalctl -u moq-relay -f
```

**Docker**:
```bash
docker logs -f moq-relay
```

### Metrics

Monitor:
- Connection count: `ss -tln | grep 4443`
- Docker stats: `docker stats moq-relay`
- Bandwidth usage
- CPU/memory usage

## Security

### Best Practices

**1. Strong TLS Configuration**:
```nginx
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers HIGH:!aNULL:!MD5;
```

**2. Rate Limiting** (nginx):
```nginx
limit_req_zone $binary_remote_addr zone=relay:10m rate=10r/s;
limit_req zone=relay burst=20 nodelay;
```

**3. Regular Updates**:
```bash
cd /opt/moq
git pull
cargo build --release --bin moq-relay
sudo systemctl restart moq-relay
```

## Performance Tuning

### System Limits

Increase file descriptor limits (`/etc/security/limits.conf`):
```
* soft nofile 65536
* hard nofile 65536
```

### Relay Configuration

```bash
moq-relay \
  --bind 0.0.0.0:4443 \
  --tls-cert cert.pem \
  --tls-key key.pem \
  --max-connections 1000  # Adjust based on capacity
```

## Resources

- **MoQ Project**: https://github.com/kixelated/moq
- **WebTransport Spec**: https://w3c.github.io/webtransport/
- **QUIC Protocol**: https://quicwg.org/
- **Browser Support**: Chrome 97+, Edge 97+ (WebTransport required)

## Related Documentation

- [nostr-integration.md](nostr-integration.md) - Nostr protocol and live events
- `packages/app/src/voice/` - Voice chat implementation
- `crates/worldtool/` - Worldtool CLI for server management
