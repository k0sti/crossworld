# Nostr Events Reference

Complete specification of all Nostr event types used in Crossworld.

## Standard NIPs

### NIP-01: Basic Protocol

All events follow the standard Nostr event format:

```json
{
  "id": "<32-byte hex event id>",
  "pubkey": "<32-byte hex pubkey>",
  "created_at": <unix timestamp>,
  "kind": <event kind>,
  "tags": [["tag", "value"], ...],
  "content": "<arbitrary string>",
  "sig": "<64-byte hex signature>"
}
```

### NIP-19: Bech32 Encoding

- `npub1...` - Public key
- `nsec1...` - Private key (never share)
- `note1...` - Event ID
- `nprofile1...` - Profile with relay hints
- `nevent1...` - Event with relay hints

### NIP-33: Parameterized Replaceable Events

Events with kind ≥30000 and a `d` tag. Only latest event per (kind, pubkey, d-tag) is kept.

### NIP-40: Expiration

Optional `expiration` tag with unix timestamp. Relays should not serve event after expiration.

### NIP-53: Live Activities

Defines kinds 30311 (live event) and 1311 (live chat message).

## Live Event (Kind 30311)

**Purpose**: Server/world configuration and discovery

**Format**: Parameterized replaceable (NIP-33)

**Tags**:
- `d` (required) - World identifier (e.g., "crossworld-dev")
- `title` - Human-readable world name
- `summary` - World description
- `status` - "live" or "ended"
- `streaming` - MoQ relay URL for voice chat
- `relay` - Nostr relay URLs (multiple allowed)
- `t` - Topic hashtags
- `starts` - Scheduled start time (unix timestamp)
- `ends` - Scheduled end time (unix timestamp)
- `participants` - Current participant count

**Content**: Optional additional information (typically empty)

**Example**:
```json
{
  "kind": 30311,
  "pubkey": "abc123...",
  "created_at": 1234567890,
  "tags": [
    ["d", "crossworld-dev"],
    ["title", "Crossworld Development World"],
    ["summary", "Test environment for Crossworld"],
    ["status", "live"],
    ["streaming", "https://moq.example.com/anon"],
    ["relay", "wss://strfry.atlantislabs.space/"],
    ["relay", "wss://relay.damus.io/"],
    ["t", "crossworld"],
    ["t", "metaverse"],
    ["participants", "5"]
  ],
  "content": ""
}
```

**Queries**:
```json
{"kinds": [30311], "authors": ["<server-pubkey>"], "#d": ["crossworld-dev"]}
```

## Live Chat Message (Kind 1311)

**Purpose**: In-world chat messages

**Format**: Regular ephemeral event (NIP-53)

**Tags**:
- `a` (required) - Reference to live event (kind:pubkey:d-tag)
- `p` - Mentioned user pubkeys
- `e` - Reply to event ID

**Content**: Chat message text

**Example**:
```json
{
  "kind": 1311,
  "pubkey": "def456...",
  "created_at": 1234567891,
  "tags": [
    ["a", "30311:abc123...:crossworld-dev"]
  ],
  "content": "Hello, world!"
}
```

**Queries**:
```json
{"kinds": [1311], "#a": ["30311:<pubkey>:<d-tag>"], "since": <timestamp>}
```

## Avatar State (Kind 30317)

**Purpose**: Persistent avatar configuration per world

**Format**: Parameterized replaceable (custom, follows NIP-33)

**Tags**:
- `d` (required) - Application identifier (e.g., "crossworld")
- `a` - Reference to world event (30311:pubkey:d-tag)
- `avatar_type` - "vox" or "glb"
- `avatar_id` - Model ID from models.json
- `avatar_url` - Custom model URL (if avatar_id not used)
- `position` - JSON position `{"x":0,"y":0,"z":0}`
- `status` - "active" or "inactive"
- `voice` - "connected" or "disconnected"
- `mic` - "enabled" or "disabled"

**Content**: Empty or additional avatar metadata

**Example**:
```json
{
  "kind": 30317,
  "pubkey": "def456...",
  "created_at": 1234567892,
  "tags": [
    ["d", "crossworld"],
    ["a", "30311:abc123...:crossworld-dev"],
    ["avatar_type", "vox"],
    ["avatar_id", "chr_army1"],
    ["position", "{\"x\":4,\"y\":0,\"z\":4}"],
    ["status", "active"],
    ["voice", "connected"],
    ["mic", "enabled"]
  ],
  "content": ""
}
```

**Queries** (active participants):
```json
{
  "kinds": [30317],
  "#a": ["30311:<server-pubkey>:<d-tag>"],
  "#status": ["active"]
}
```

**Lifecycle**:
1. Created when user joins world
2. Updated when avatar/position changes
3. Set `status=inactive` when user leaves
4. Clients filter for `status=active`

## Position Update (Kind 1317)

**Purpose**: Real-time movement synchronization

**Format**: Regular ephemeral event with expiration (custom)

**Tags**:
- `a` - Reference to avatar state (30317:pubkey:d-tag)
- `a` - Reference to world (30311:pubkey:d-tag)
- `update_type` - "position"
- `position` - JSON position `{"x":0,"y":0,"z":0}`
- `rotation` - JSON quaternion `{"x":0,"y":0,"z":0,"w":1}` (optional)
- `move_style` - Movement animation hint
- `expiration` - Unix timestamp (NIP-40)

**Content**: Empty

**Move Styles**:
- `walk` - Normal walking
- `run` - Running (shift+move)
- `teleport:fade` - Teleport with fade effect
- `teleport:scale` - Teleport with scale effect
- `teleport:spin` - Teleport with spin effect

**Example**:
```json
{
  "kind": 1317,
  "pubkey": "def456...",
  "created_at": 1234567893,
  "tags": [
    ["a", "30317:def456...:crossworld"],
    ["a", "30311:abc123...:crossworld-dev"],
    ["update_type", "position"],
    ["position", "{\"x\":5.5,\"y\":0,\"z\":3.2}"],
    ["rotation", "{\"x\":0,\"y\":0.707,\"z\":0,\"w\":0.707}"],
    ["move_style", "walk"],
    ["expiration", "1234567953"]
  ],
  "content": ""
}
```

**Queries** (recent positions):
```json
{
  "kinds": [1317],
  "#a": ["30311:<server-pubkey>:<d-tag>"],
  "since": <timestamp-60s>
}
```

**Update Strategy**:
- Send on significant position changes (>0.1 units)
- Include 60s expiration to prevent stale data
- Clients interpolate between updates

## World Model (Kind 30078)

**Purpose**: Voxel world data storage

**Format**: Parameterized replaceable (NIP-78)

**d-tag Format**: `[octant]:[macro][:micro]`
- `octant` - Octant path (optional, e.g., "abc", "efg")
- `macro` - Macro depth (required, 1-10)
- `micro` - Micro depth (optional, 0-3, default 0)

**Examples**: `:3`, `:3:2`, `abc:4`, `abc:4:1`

**Tags**:
- `d` (required) - World identifier (format above)
- `a` - Reference to live event (optional)
- `title` - World name
- `description` - World description
- `macro` - Macro depth value (for filtering)
- `micro` - Micro depth value (for filtering)
- `thumbnail` - Preview image URL (optional)

**Content**: CSM (Cube Script Model) code

**Example**:
```json
{
  "kind": 30078,
  "pubkey": "def456...",
  "created_at": 1234567894,
  "tags": [
    ["d", ":3:2"],
    ["title", "My Castle"],
    ["description", "Medieval castle with gardens"],
    ["macro", "3"],
    ["micro", "2"]
  ],
  "content": ">d [100 100 100 100 100 100 100 100]\n>dd [150 150 150 150 150 150 150 150]\n..."
}
```

**Queries** (user's worlds):
```json
{"kinds": [30078], "authors": ["<user-pubkey>"]}
```

**Queries** (specific world):
```json
{"kinds": [30078], "authors": ["<user-pubkey>"], "#d": [":3:2"]}
```

**CSM Format**: See `doc/architecture/voxel-system.md` for complete specification

**Size Considerations**:
- Large CSM files may exceed relay limits (~100KB)
- Consider compression or content-addressed storage (IPFS/Blossom)

## Server Discovery (Kind 30078)

**Purpose**: Game server metadata

**Format**: Parameterized replaceable (NIP-78)

**Tags**:
- `d` (required) - "crossworld-server"
- `endpoint` - Server endpoint (IP:port)
- `region` - Server region (e.g., "us-west", "eu-central")
- `name` - Server display name
- `max_players` - Maximum player capacity
- `players` - Current player count
- `version` - Server version
- `features` - Supported features (comma-separated)
- `world` - Associated world d-tag

**Content**: JSON server configuration (optional)

**Example**:
```json
{
  "kind": 30078,
  "pubkey": "abc123...",
  "created_at": 1234567895,
  "tags": [
    ["d", "crossworld-server"],
    ["endpoint", "game.example.com:4433"],
    ["region", "us-west"],
    ["name", "Crossworld Official Server"],
    ["max_players", "100"],
    ["players", "42"],
    ["version", "0.1.0"],
    ["features", "webtransport,nostr-auth,voice"],
    ["world", ":8:3"]
  ],
  "content": ""
}
```

## Event Flow

### World Join Flow

```
1. Fetch Live Event (30311)
   → Extract MoQ relay URL
   → Extract Nostr relay URLs

2. Connect to Services
   → WebTransport to MoQ relay
   → WebSocket to Nostr relays

3. User Login
   → Create/update Avatar State (30317)
   → Set status=active

4. Subscribe to Participants
   → Query active avatars (30317 with status=active)
   → Subscribe to position updates (1317)

5. Render World
   → Show local avatar
   → Show remote avatars
   → Update from position events
```

### Movement Flow

```
1. Local Movement
   → Update physics simulation
   → Update render position

2. Position Changed (>0.1 units)
   → Publish Position Update (1317)
   → Include 60s expiration

3. Remote Position Event
   → Receive via subscription
   → Interpolate to new position
   → Update remote avatar
```

### World Save/Load Flow

```
Save:
1. Generate CSM from voxel data
2. Build d-tag from current config
3. Publish World Model (30078)

Load:
1. Query World Model (30078) with d-tag
2. Parse CSM content
3. Load into voxel engine
```

## Privacy

### Public Data
- Nostr public key (identity)
- Avatar configuration and position
- Voice connection status
- Chat messages
- World models

### Private Data
- Nostr private key (never shared)
- Browser storage
- Local preferences

### Temporary Data
- Position updates expire after 60s
- Reduces long-term tracking

## Implementation

### Publishing Events

```typescript
import { SimplePool, finishEvent } from 'nostr-tools'

const pool = new SimplePool()
const relays = ['wss://relay.example.com']

// Build unsigned event
const unsignedEvent = {
  kind: 30317,
  created_at: Math.floor(Date.now() / 1000),
  tags: [
    ['d', 'crossworld'],
    ['avatar_type', 'vox'],
    ['avatar_id', 'chr_army1'],
    ['status', 'active']
  ],
  content: ''
}

// Sign with nostr-tools
const signedEvent = finishEvent(unsignedEvent, privateKeyHex)

// Or sign with extension
const signedEvent = await window.nostr.signEvent(unsignedEvent)

// Publish
await pool.publish(relays, signedEvent)
```

### Subscribing to Events

```typescript
const subscription = pool.subscribeMany(relays, [
  {
    kinds: [30317],
    '#a': ['30311:server-pubkey:crossworld-dev'],
    '#status': ['active']
  }
], {
  onevent(event) {
    console.log('Avatar state:', event)
  },
  oneose() {
    console.log('End of stored events')
  }
})

// Cleanup
subscription.close()
```

### Querying Events

```typescript
// Get single event (replaceable)
const liveEvent = await pool.get(relays, {
  kinds: [30311],
  authors: ['server-pubkey'],
  '#d': ['crossworld-dev']
})

// Get multiple events
const avatars = await pool.querySync(relays, {
  kinds: [30317],
  '#a': ['30311:server-pubkey:crossworld-dev'],
  '#status': ['active']
})
```

## NIPs Summary

| NIP | Purpose | Events |
|-----|---------|--------|
| NIP-01 | Basic protocol | All events |
| NIP-19 | Bech32 encoding | npub, nsec, note, etc. |
| NIP-33 | Parameterized replaceable | 30311, 30317, 30078 |
| NIP-40 | Expiration | 1317 (position updates) |
| NIP-53 | Live activities | 30311, 1311 |
| NIP-78 | Application data | 30078 (worlds, servers) |

## References

- [Nostr Protocol](https://github.com/nostr-protocol/nips)
- [nostr-tools](https://github.com/nbd-wtf/nostr-tools)
- [Applesauce SDK](https://github.com/coracle-social/applesauce)
- [doc/features/nostr-integration.md](features/nostr-integration.md) - Integration guide
- [doc/architecture/voxel-system.md](architecture/voxel-system.md) - CSM format
