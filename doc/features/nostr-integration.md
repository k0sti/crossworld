# Nostr Integration

## Overview

Crossworld uses the Nostr protocol for decentralized identity, discovery, and state management. This provides user authentication without centralized servers, persistent avatar configuration, and real-time chat.

**Key Features**:
- Decentralized identity via Nostr keypairs
- Live event discovery for worlds and servers
- Avatar state persistence across sessions
- Real-time position updates
- In-world chat messaging

**Event Types**: See [../nostr.md](../nostr.md) for complete event specifications.

## Quick Start

### 1. User Login

Users authenticate via:
- **Browser extension** (nos2x, Alby, etc.)
- **Guest account** (ephemeral keypair in localStorage)
- **Amber** (Android Nostr signer)

```typescript
import { useNostrAccount } from '@applesauce-core/react'

function LoginButton() {
  const { login, logout, pubkey } = useNostrAccount()

  return pubkey ? (
    <button onClick={logout}>Logout {pubkey.slice(0,8)}...</button>
  ) : (
    <button onClick={login}>Login with Nostr</button>
  )
}
```

### 2. Fetch World Configuration

On app start, fetch the live event (kind 30311) to discover services:

```typescript
const liveEvent = await pool.get(relays, {
  kinds: [30311],
  authors: [serverPubkey],
  '#d': ['crossworld-dev']
})

// Extract service URLs
const moqRelay = liveEvent.tags.find(t => t[0] === 'streaming')?.[1]
const chatRelays = liveEvent.tags.filter(t => t[0] === 'relay').map(t => t[1])
```

### 3. Join World

Create or update avatar state (kind 30317):

```typescript
const avatarEvent = {
  kind: 30317,
  created_at: Math.floor(Date.now() / 1000),
  tags: [
    ['d', 'crossworld'],
    ['a', `30311:${serverPubkey}:crossworld-dev`],
    ['avatar_type', 'vox'],
    ['avatar_id', 'chr_army1'],
    ['position', JSON.stringify({ x: 4, y: 0, z: 4 })],
    ['status', 'active']
  ],
  content: ''
}

await signer.signEvent(avatarEvent)
await pool.publish(relays, avatarEvent)
```

### 4. Subscribe to Other Players

```typescript
const sub = pool.subscribeMany(relays, [{
  kinds: [30317],
  '#a': [`30311:${serverPubkey}:crossworld-dev`],
  '#status': ['active']
}], {
  onevent(event) {
    // Add remote avatar
    const pubkey = event.pubkey
    const avatarId = event.tags.find(t => t[0] === 'avatar_id')?.[1]
    const position = JSON.parse(event.tags.find(t => t[0] === 'position')?.[1] || '{}')

    createRemoteAvatar(pubkey, avatarId, position)
  }
})
```

### 5. Send Position Updates

```typescript
function sendPositionUpdate(position: Vector3) {
  const posEvent = {
    kind: 1317,
    created_at: Math.floor(Date.now() / 1000),
    tags: [
      ['a', `30317:${userPubkey}:crossworld`],
      ['a', `30311:${serverPubkey}:crossworld-dev`],
      ['update_type', 'position'],
      ['position', JSON.stringify(position)],
      ['move_style', 'walk'],
      ['expiration', Math.floor(Date.now() / 1000) + 60]
    ],
    content: ''
  }

  pool.publish(relays, posEvent)
}
```

## Worldtool CLI

The `worldtool` CLI manages live events and MoQ relay servers.

**Location**: `crates/worldtool/`

### Create Live Event

```bash
cd crates/worldtool

# Create with defaults
cargo run -- init-live

# Custom MoQ relay
cargo run -- init-live --streaming https://moq.example.com/anon

# Custom title
cargo run -- init-live --title "My World"

# Or via justfile
just start-live
```

### Manage MoQ Relay

```bash
# Initialize local relay
cargo run -- server init

# Run relay (default port 4443)
cargo run -- server run

# Custom port
cargo run -- server run --port 8443

# Verbose logging
cargo run -- server run --verbose
```

### List Active Worlds

```bash
cargo run -- list-live
```

## Event Reference

**Complete specifications**: [../nostr.md](../nostr.md)

| Kind | Name | Purpose | Format |
|------|------|---------|--------|
| 30311 | Live Event | World/server config | Replaceable |
| 1311 | Chat Message | In-world chat | Ephemeral |
| 30317 | Avatar State | Player configuration | Replaceable |
| 1317 | Position Update | Movement sync | Ephemeral, expiring |
| 30078 | World Model | Voxel world storage | Replaceable |

## Applesauce SDK

Crossworld uses Applesauce for Nostr connectivity.

**Packages**:
- `@applesauce-core/core` - Core Nostr client
- `@applesauce-core/react` - React hooks
- `@applesauce-core/relay` - Relay management
- `@applesauce-core/signers` - Event signing

**Example**:
```typescript
import { createNostrClient } from '@applesauce-core/core'
import { useNostrEvents } from '@applesauce-core/react'

const client = createNostrClient({
  relays: ['wss://strfry.atlantislabs.space/']
})

function ChatMessages() {
  const messages = useNostrEvents({
    kinds: [1311],
    '#a': ['30311:server:crossworld-dev']
  })

  return messages.map(msg => (
    <div key={msg.id}>{msg.content}</div>
  ))
}
```

## Privacy & Security

### Public Information
- Nostr public key (npub)
- Avatar configuration
- Position in world
- Voice connection status
- Chat messages

### Private Information
- Nostr private key (nsec) - **never shared**
- Browser localStorage data
- Local preferences

### Data Expiration
- Position updates (kind 1317) expire after 60 seconds
- Reduces tracking via expiration tags (NIP-40)
- Relays should not serve expired events

## Troubleshooting

### Relay Connection Issues

Test relay connectivity:
```bash
wscat -c wss://strfry.atlantislabs.space/
```

Check browser console for WebSocket errors.

### Live Event Not Found

- Verify server pubkey matches
- Check `d` tag is exact (case-sensitive)
- Try multiple relays
- Use `worldtool list-live` to verify event exists

### Event Publishing Fails

Common causes:
- No private key (user not logged in)
- Relay rate limiting
- Invalid event format (missing required tags)

Debug with:
```javascript
client.on('notice', (relay, message) => {
  console.log('Relay notice:', relay, message)
})
```

## Related Documentation

- [../nostr.md](../nostr.md) - Complete event specifications
- [voice-chat.md](voice-chat.md) - MoQ voice chat integration
- [avatar-system.md](avatar-system.md) - Avatar configuration
- [Nostr Protocol](https://github.com/nostr-protocol/nips)
