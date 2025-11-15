# Nostr Integration

## Overview

Crossworld uses the Nostr protocol for decentralized identity, discovery, and state management. The integration leverages Applesauce SDK for Nostr connectivity.

**Key Features**:
- Decentralized identity via Nostr public keys (npub)
- Live event discovery (NIP-53)
- Avatar state persistence (kind 30317)
- Position updates (kind 1317)
- Chat messaging (kind 1311)
- No centralized user database

## Nostr Basics

### Identity

**Public Key (npub)**:
- User's unique identifier
- Example: `npub1abc...xyz` (bech32 encoded)
- Also represented in hex format

**Private Key (nsec)**:
- Used to sign events
- Never shared or transmitted
- Stored in browser extension or app

### Events

Nostr uses JSON events for all communication:

```json
{
  "id": "<event-id>",
  "pubkey": "<author-pubkey>",
  "created_at": <unix-timestamp>,
  "kind": <event-kind>,
  "tags": [["tag-name", "value"], ...],
  "content": "<content>",
  "sig": "<signature>"
}
```

## Event Types Used

### Live Event (Kind 30311) - NIP-53

**Purpose**: Server configuration and discovery

**Tags**:
- `d` - World identifier (e.g., "crossworld-dev")
- `title` - World name
- `summary` - Description
- `status` - "live" or "ended"
- `streaming` - MoQ relay URL
- `relay` - Nostr relay URLs (multiple allowed)
- `t` - Topic tags

**Example**:
```json
{
  "kind": 30311,
  "tags": [
    ["d", "crossworld-dev"],
    ["title", "Crossworld Development World"],
    ["summary", "Test environment for Crossworld"],
    ["status", "live"],
    ["streaming", "https://moq.example.com/anon"],
    ["relay", "wss://strfry.atlantislabs.space/"],
    ["t", "crossworld"]
  ],
  "content": ""
}
```

**Client Usage**:
1. On startup, fetch kind 30311 with `d=crossworld-dev`
2. Extract `streaming` tag → Connect to MoQ relay
3. Extract `relay` tags → Connect to Nostr relays for chat
4. Monitor `status` tag → Know if world is active

### Avatar State (Kind 30317)

**Purpose**: Persistent avatar configuration

**Tags**:
- `d` - State identifier (e.g., "crossworld")
- `a` - Reference to world event (kind:pubkey:d-tag)
- `avatar_type` - "vox" or "glb"
- `avatar_id` - Model ID from models.json
- `avatar_url` - Custom model URL
- `position` - JSON position {x, y, z}
- `status` - "active" or "inactive"
- `voice` - "connected" or "disconnected"
- `mic` - "enabled" or "disabled"

**Example**:
```json
{
  "kind": 30317,
  "tags": [
    ["d", "crossworld"],
    ["a", "30311:pubkey:crossworld-dev"],
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

**State Management**:
- Created when user joins world
- Updated when avatar/position changes
- Queried by other clients to discover participants
- Deleted/updated to "inactive" when user leaves

### Position Updates (Kind 1317)

**Purpose**: Real-time movement synchronization

**Tags**:
- `a` - References to avatar state and world
- `update_type` - "position"
- `position` - JSON position {x, y, z}
- `move_style` - Movement type (walk/run/teleport)
- `expiration` - Unix timestamp for event expiration

**Example**:
```json
{
  "kind": 1317,
  "tags": [
    ["a", "30317:pubkey:crossworld"],
    ["a", "30311:pubkey:crossworld-dev"],
    ["update_type", "position"],
    ["position", "{\"x\":5.5,\"y\":0,\"z\":3.2}"],
    ["move_style", "walk"],
    ["expiration", "1234567890"]
  ],
  "content": ""
}
```

**Movement Styles**:
- `walk` - Normal walking
- `run` - Running (SHIFT+click)
- `teleport:fade` - Teleport with fade (CTRL+click)
- `teleport:scale` - Scale animation
- `teleport:spin` - Spin animation

**Update Strategy**:
- Send on significant position changes
- Include expiration to prevent stale events
- Remote clients interpolate between updates

### Chat Messages (Kind 1311) - NIP-53

**Purpose**: In-world chat

**Tags**:
- `a` - Reference to live event
- Other tags as per NIP-53

**Example**:
```json
{
  "kind": 1311,
  "tags": [
    ["a", "30311:pubkey:crossworld-dev"]
  ],
  "content": "Hello, world!"
}
```

## Worldtool CLI

**Location**: `crates/worldtool/`

### Initialize Live Event

```bash
cd crates/worldtool

# Create new live event
cargo run -- init-live

# With custom MoQ relay
cargo run -- init-live --streaming https://moq.yourdomain.com/anon

# With custom title
cargo run -- init-live --title "My World"
```

### Manage MoQ Relay Server

```bash
# Initialize local server
cargo run -- server init

# Run server
cargo run -- server run

# Custom port
cargo run -- server run --port 8443

# With verbose logging
cargo run -- server run --verbose
```

### List Active Events

```bash
# Show all live events
cargo run -- list-live
```

## Applesauce Integration

**SDK**: Applesauce 4.0 (Nostr SDK)

**Packages Used**:
- `@applesauce-core/core` - Core Nostr functionality
- `@applesauce-core/react` - React hooks and components
- `@applesauce-core/relay` - Relay connection management
- `@applesauce-core/signers` - Event signing

### Connection Setup

```typescript
import { createNostrClient } from '@applesauce-core/core'

const client = createNostrClient({
  relays: [
    'wss://strfry.atlantislabs.space/',
    'wss://relay.damus.io/',
    'wss://relay.nostr.band/'
  ]
})
```

### Fetch Live Event

```typescript
const liveEvent = await client.fetchEvent({
  kinds: [30311],
  authors: [serverPubkey],
  '#d': ['crossworld-dev']
})

const moqRelay = liveEvent.tags.find(t => t[0] === 'streaming')?.[1]
const chatRelays = liveEvent.tags.filter(t => t[0] === 'relay').map(t => t[1])
```

### Publish Avatar State

```typescript
const avatarEvent = {
  kind: 30317,
  tags: [
    ['d', 'crossworld'],
    ['a', `30311:${serverPubkey}:crossworld-dev`],
    ['avatar_type', 'vox'],
    ['avatar_id', selectedAvatarId],
    ['position', JSON.stringify({ x: 4, y: 0, z: 4 })],
    ['status', 'active']
  ],
  content: ''
}

await client.publish(avatarEvent)
```

### Subscribe to Participants

```typescript
const subscription = client.subscribe({
  kinds: [30317],
  '#a': [`30311:${serverPubkey}:crossworld-dev`],
  '#status': ['active']
}, {
  onEvent: (event) => {
    // New participant or state update
    updateParticipant(event)
  }
})
```

## Discovery Flow

```
1. App Starts
   ↓
2. Fetch Live Event (30311)
   - Parse MoQ relay URL
   - Parse Nostr relays
   ↓
3. Connect to Services
   - MoQ relay (voice)
   - Nostr relays (chat/state)
   ↓
4. User Logs In
   - Load or create avatar state (30317)
   ↓
5. Subscribe to Participants
   - Listen for active avatars (30317)
   - Listen for position updates (1317)
   ↓
6. Render World
   - Show local and remote avatars
   - Update positions from events
```

## NIP References

**NIPs Used**:
- **NIP-01**: Basic protocol flow
- **NIP-19**: Bech32 encoding (npub, nsec, note, etc.)
- **NIP-33**: Parameterized replaceable events (kind 30000+)
- **NIP-53**: Live events (kind 30311)
- **NIP-40**: Expiration timestamp

**Relevant NIPs**:
- **NIP-05**: DNS-based verification (future)
- **NIP-65**: Relay list metadata (future)

## Privacy Considerations

### Public Information

**Visible to All**:
- Nostr public key (identity)
- Avatar configuration
- Position in world
- Voice connection status
- Chat messages

### Private Information

**Never Shared**:
- Nostr private key (nsec)
- Browser storage data
- Local preferences

### Temporary Data

**Ephemeral Events**:
- Position updates (kind 1317) include expiration tags
- Expired events should not be stored by relays
- Reduces long-term tracking

## Future Enhancements

### Planned Features

1. **NIP-05 Verification**
   - DNS-based identity verification
   - Display verified names

2. **Relay List (NIP-65)**
   - User-configurable relay preferences
   - Follow users across relays

3. **Private Worlds**
   - Encrypted events for private spaces
   - Access control via Nostr

4. **Achievements/NFTs**
   - Store achievements as Nostr events
   - Link to Bitcoin NFTs

5. **World Persistence**
   - Save world state to Nostr
   - Collaborative building with event history

## Troubleshooting

### Connection Issues

**Relay Not Connecting**:
```bash
# Test relay connectivity
wscat -c wss://strfry.atlantislabs.space/
```

**Live Event Not Found**:
- Check `d` tag matches exactly
- Verify server pubkey
- Try alternative relays

### Event Publishing Fails

**Common Causes**:
- Private key not set (user not logged in)
- Relay rejection (spam filters, rate limits)
- Invalid event format

**Debug**:
```javascript
// Check event before publishing
console.log('Publishing event:', event)

// Monitor relay responses
client.on('notice', (relay, message) => {
  console.log('Relay notice:', relay, message)
})
```

## Related Documentation

- [voice-chat.md](voice-chat.md) - MoQ integration with live events
- [avatar-system.md](avatar-system.md) - Avatar state events
- `crates/worldtool/` - CLI tool source code
- **External**: [Nostr Protocol](https://github.com/nostr-protocol/nips)
