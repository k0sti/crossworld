# Voice Chat with MoQ Implementation Design

## MoQ Voice Architecture Overview

**MoQ (Media over QUIC)** provides low-latency real-time audio streaming using a pub/sub model over QUIC transport:

- **Connection Model**: Single persistent QUIC connection per client to a relay server
- **Broadcasts**: Each participant publishes an audio broadcast with a unique path/name
- **Discovery**: Relay announces available broadcasts; clients subscribe to streams they want to hear
- **Audio Pipeline**:
  - Publisher: `getUserMedia()` → Opus encoding → MoQ broadcast track
  - Subscriber: MoQ track → Opus decoding → `AudioContext` playback
- **Participant State**: Tracked via broadcast catalog metadata (speaking detection, mute status)
- **Room Isolation**: Achieved through broadcast path namespacing (e.g., `crossworld/voice/<room-id>/<npub>`)

Key characteristics:
- Sub-second latency (typically 100-300ms)
- Automatic jitter buffering
- Speaking detection built-in
- No centralized mixing (client-side audio mixing)

## Nostr Integration

### Live Event Discovery (kind:30311)

All game server configuration (chat relays, MoQ relay, etc.) is advertised in a single NIP-53 Live Event:

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

**Key tags**:
- `d`: World identifier (hardcoded to "crossworld-dev" for now)
- `streaming`: MoQ relay URL for voice/game data
- `relay`: Nostr relay(s) for chat (kind 1311 messages)
- `status`: `live` when active, `ended` when closed

**Client startup**:
1. Client starts and fetches kind:30311 with `d=crossworld-dev` using applesauce
2. Parses `streaming` tag for MoQ relay URL
3. Parses `relay` tags for chat relay URLs
4. Passes MoQ relay URL to game/voice systems (via wasm interface if needed)
5. Uses chat relays for kind 1311 messages (existing chat system)

**Architecture note**: Nostr communication happens on TypeScript side with applesauce. Live event data is sent through wasm interface to game code if needed.

### Participant Broadcast Naming

Each participant broadcasts at path: `crossworld/voice/<d-tag>/<npub>`

Example: `crossworld/voice/crossworld-dev/npub1abc...xyz`

This allows:
- Easy filtering of broadcasts by world
- Participant identification via npub
- Support for multiple worlds in the future

## Implementation Components

### 1. MoQ Connection Manager

**File**: `src/voice/moq-connection.ts`

```typescript
interface VoiceConnection {
  relay_url: string;
  connection: Moq.Connection;
  status: 'connecting' | 'connected' | 'disconnected';
}
```

Responsibilities:
- Establish and maintain MoQ connection to relay
- Handle reconnection with exponential backoff
- Expose connection state via signals

### 2. Audio Publisher

**File**: `src/voice/publisher.ts`

```typescript
interface AudioPublisher {
  broadcast: Hang.Publish.Broadcast;
  mic_enabled: Signal<boolean>;
  speaking: Signal<boolean>;
}
```

Responsibilities:
- Request mic permission via `getUserMedia({ audio: true })`
- Create `Hang.Publish.Broadcast` with path `crossworld/voice/<channel>/<local-npub>`
- Toggle publishing based on `mic_enabled` state
- Expose speaking detection for UI feedback

### 3. Audio Subscriber

**File**: `src/voice/subscriber.ts`

```typescript
interface VoiceParticipant {
  npub: string;
  watcher: Hang.Watch.Broadcast;
  speaking: Signal<boolean>;
  volume: Signal<number>;
}
```

Responsibilities:
- Discover broadcasts matching `crossworld/voice/<channel>/` prefix
- Create `Hang.Watch.Broadcast` per remote participant
- Route decoded audio to shared `AudioContext`
- Track participant speaking state
- Cleanup watchers when participants leave

### 4. Participant Tracker

**File**: `src/voice/participants.ts`

```typescript
interface ParticipantInfo {
  npub: string;
  speaking: boolean;
  connected_at_ms: number;
}

class ParticipantTracker {
  participants: Map<string, ParticipantInfo>;

  // Discovery via MoQ announced() broadcasts
  async trackAnnouncements(connection: Moq.Connection, channel: string);

  // Prune stale participants (no updates for 10s)
  pruneStale();
}
```

### 5. Nostr Event Manager

**File**: `src/nostr/live-event.ts`

Responsibilities:
- Fetch kind:30311 event with `d=crossworld-dev` on client startup (using applesauce)
- Parse `streaming` tag for MoQ relay URL
- Parse `relay` tags for chat relay URLs
- Pass configuration to voice/game systems
- Monitor `status` for world availability

### 6. UI Components

#### Left Sidebar Voice Button

**Component**: `VoiceButton.tsx`

States:
- **Disconnected**: Grey headphone icon
- **Connected (muted)**: Green headphone icon
- **Connected (mic on)**: Green headphone icon + mic indicator

Click behavior: Toggle voice chat connection

#### Mic Control Button

**Component**: `MicButton.tsx` (shown when connected)

Position: Below voice button in left sidebar

States:
- **Muted**: Red mic-off icon
- **Active**: Green mic icon
- **Speaking**: Green mic icon + animated ring/glow

Click behavior: Toggle microphone mute

#### Participant List Overlay

**Component**: `VoiceParticipants.tsx` (optional)

Displays when voice connected (can be toggled):
- List of participant npubs (shortened)
- Speaking indicator per participant
- Visual feedback (avatar + speaking ring)

## Configuration

All configuration comes from kind:30311 live event. Hardcoded values:
- Live event d-tag: `"crossworld-dev"`
- Voice broadcast path prefix: `"crossworld/voice/crossworld-dev"`

MoQ/Hang audio settings use library defaults (no custom configuration needed).

## Implementation Flow

### Joining Voice Chat

1. User clicks voice button in left sidebar
2. Use MoQ relay URL from kind:30311 (already fetched on startup)
3. Establish MoQ connection to relay
4. Subscribe to broadcasts matching `crossworld/voice/crossworld-dev/`
5. Create watchers for each discovered broadcast
6. Show mic button (default: muted)
7. Update UI state to "connected"

### Publishing Audio

1. User clicks mic button (unmute)
2. Request `getUserMedia({ audio: true })`
3. Create `Hang.Publish.Broadcast` with path including local npub
4. Enable audio publishing
5. Stream speaking detection to UI
6. Update mic button to show "active" state

### Participant Discovery

1. Listen for `connection.announced()` broadcasts
2. Filter by path prefix `crossworld/voice/crossworld-dev/`
3. Extract npub from path (last segment)
4. Create `Hang.Watch.Broadcast` for new participant
5. Add to participant list
6. Subscribe to speaking state changes

### Leaving Voice Chat

1. User clicks voice button again
2. Disable local broadcast (if publishing)
3. Close all remote watchers
4. Disconnect from MoQ relay
5. Update UI to "disconnected"
6. Hide mic button

## Participant Tracking

Participants are visible through:

1. **MoQ Catalog Discovery**: The relay's `announced()` API reveals all active broadcasts
2. **Path-based Filtering**: Only broadcasts under our channel prefix are tracked
3. **Speaking State**: Each watcher exposes `watch.audio.speaking` signal
4. **Timeout Detection**: Participants without updates for 10s are removed

**Participant State Updates**:
- Real-time via MoQ broadcast metadata
- No separate signaling channel needed
- Speaking state available immediately from Hang's detector
- Npub extracted from broadcast path

## MoQ Relay Server Setup

### Option 1: Quick Start (Public Test Relay)

The default configuration uses a **public test relay** - no setup required:

```bash
# Creates live event with default public relay (https://relay.moq.dev/anon)
worldtool init-live
```

**⚠️ WARNING**: The public test relay is for testing only:
- May have rate limits or connection limits
- No availability guarantees
- No privacy guarantees
- Shared with other developers

### Option 2: Local Development Server (Recommended)

Set up a local MoQ relay in minutes using worldtool:

```bash
cd crates/worldtool

# 1. Initialize server (clone and build)
cargo run -- server init

# 2. Run server
cargo run -- server run

# 3. In another terminal, configure live event
cargo run -- init-live --streaming https://localhost:4443/anon
```

The server automatically:
- Clones MoQ relay from GitHub
- Builds the release binary
- Generates a self-signed certificate
- Starts the relay on port 4443

**Server commands**:
```bash
# Initialize server to custom directory
cargo run -- server init --dir ~/my-moq-server

# Run with custom port
cargo run -- server run --port 8443

# Run with verbose logging
cargo run -- server run --verbose

# Run with custom certificates
cargo run -- server run --tls-cert cert.pem --tls-key key.pem
```

### Option 3: Production Deployment

For production, you **must** set up your own MoQ relay server with proper HTTPS.

#### Option 1: Run MoQ Relay Server

**Prerequisites**:
- Rust toolchain (1.70+)
- HTTPS certificate (required for WebTransport)

**Installation**:

```bash
# Clone MoQ relay
git clone https://github.com/kixelated/moq.git
cd moq

# Build and run
cargo run --release --bin moq-relay -- --bind 0.0.0.0:4443
```

**With HTTPS (Development - Self-signed cert)**:

```bash
# Generate self-signed certificate
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes \
  -subj "/CN=localhost"

# Run with TLS
cargo run --release --bin moq-relay -- \
  --bind 0.0.0.0:4443 \
  --tls-cert cert.pem \
  --tls-key key.pem
```

**With HTTPS (Production - Let's Encrypt)**:

Use a reverse proxy (nginx/caddy) with Let's Encrypt:

```nginx
# /etc/nginx/sites-available/moq
server {
    listen 443 ssl http2;
    server_name moq.yourdomain.com;

    ssl_certificate /etc/letsencrypt/live/moq.yourdomain.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/moq.yourdomain.com/privkey.pem;

    location / {
        proxy_pass https://localhost:4443;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

Or use Caddy (automatic HTTPS):

```caddyfile
# Caddyfile
moq.yourdomain.com {
    reverse_proxy localhost:4443
}
```

#### Option 2: Docker Deployment

```bash
# Run with Docker
docker run -d \
  -p 4443:4443 \
  --name moq-relay \
  kixelated/moq-relay

# Or with docker-compose
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  moq-relay:
    image: kixelated/moq-relay
    ports:
      - "4443:4443"
    volumes:
      - ./certs:/certs
    command: >
      --bind 0.0.0.0:4443
      --tls-cert /certs/cert.pem
      --tls-key /certs/key.pem
    restart: unless-stopped
```

#### Configure Live Event

Once your relay is running, update the live event:

```bash
# With your own relay
worldtool init-live --streaming "https://moq.yourdomain.com/anon"

# Or specify custom port
worldtool init-live --streaming "https://moq.yourdomain.com:4443/anon"
```

### Firewall Configuration

Ensure port 4443 (or your chosen port) is open:

```bash
# ufw (Ubuntu)
sudo ufw allow 4443/tcp

# firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=4443/tcp
sudo firewall-cmd --reload
```

### Health Check

Test your relay is accessible:

```bash
# Check if relay responds (should get WebTransport error, which is expected)
curl -k https://moq.yourdomain.com:4443/anon
```

### Monitoring

Monitor relay logs for connections:

```bash
# If running directly
cargo run --release --bin moq-relay -- --bind 0.0.0.0:4443 -v

# If using docker
docker logs -f moq-relay
```

## worldtool Configuration Examples

**Default (public test relay)**:
```bash
worldtool init-live
# Uses https://relay.moq.dev/anon with warning
```

**Production (custom relay)**:
```bash
worldtool init-live \
  --title "Crossworld Live Chat" \
  --summary "Live chat for Crossworld metaverse" \
  --streaming "https://moq.yourdomain.com/anon"
```

**With all options**:
```bash
worldtool init-live \
  --title "Crossworld Production" \
  --summary "Production metaverse instance" \
  --streaming "https://moq.yourdomain.com/anon" \
  --image "https://example.com/world.png" \
  --status "live"
```

## Dependencies

```json
{
  "@kixelated/moq": "^0.9.1",
  "@kixelated/hang": "^0.7.0",
  "@kixelated/signals": "^0.7.0",
  "applesauce-*": "^3.1.0"
}
```

## Notes

- Single voice channel per world instance (d-tag: "crossworld-dev" hardcoded for now)
- All configuration from kind:30311 live event (no env vars)
- Nostr operations use applesauce on TypeScript side
- MoQ relay URL passed to voice system (via wasm interface if needed)
- No video support initially (audio-only)
- Client-side audio mixing (no server-side MCU)
- MoQ/Hang use sensible defaults (no custom audio config)
- Browser permissions required for microphone access
- COOP/COEP headers required for SharedArrayBuffer (Hang requirement)
