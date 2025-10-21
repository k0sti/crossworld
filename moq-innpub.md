# MoQ Voice Implementation Analysis

**Document Purpose:** Comprehensive analysis of voice communication in ref/innpub (reference implementation) and current project (packages/app), including connection initialization, broadcast paths, debugging, and migration roadmap.

## Executive Summary

### ref/innpub (Reference Implementation)
- **MoQ Relay:** `https://moq.justinmoon.com/anon` (hardcoded)
- **Protocol:** WebTransport over HTTPS (WebSocket disabled)
- **Namespace:** `innpub/rooms/v2/{room}/{npub}/{session-id}`
- **Features:** Multi-room audio, positional audio (distance-based volume), MoQ announcement discovery
- **Key Libraries:** `@kixelated/moq`, `moq-and-other-stuff/meet` (Hang)

### Current Project (packages/app)
- **MoQ Relay:** Fetched dynamically from Nostr live event (kind 30311)
- **Protocol:** WebTransport via `Moq.Connection.Reload` (auto-reconnect)
- **Namespace:** `crossworld/voice/{d-tag}/{npub}`
- **Features:** Single voice space, dual discovery (Nostr + MoQ), manual volume control
- **Missing:** Positional audio, room-based routing

### Critical Differences
1. **Relay Discovery:** ref/innpub hardcodes URL; current project fetches from Nostr
2. **Audio Routing:** ref/innpub has rooms; current project is global
3. **Volume Control:** ref/innpub auto-adjusts by distance; current project is manual
4. **Participant Discovery:** ref/innpub uses MoQ announcements; current project uses dual Nostr + MoQ

---

## Connection Initialization

### 1. **MoQ Relay Connection** (ref/innpub/src/multiplayer/moqConnection.ts:3-43)

The voice system connects to a **public MoQ relay**:

```typescript
// ref/innpub/src/multiplayer/moqConnection.ts:3-4
export const RELAY_URL = "https://moq.justinmoon.com/anon";
// export const RELAY_URL = "https://relay.cloudflare.mediaoverquic.com";
```

**Key Connection Details:**
- **Active URL:** `https://moq.justinmoon.com/anon`
- **Alternative (commented):** `https://relay.cloudflare.mediaoverquic.com`
- **Protocol:** HTTPS (WebTransport), **NOT WebSocket**

**Connection establishment** (ref/innpub/src/multiplayer/moqConnection.ts:42-43):

```typescript
async function establish(): Promise<MoqConnection> {
  const connection = await Moq.Connection.connect(
    new URL(RELAY_URL),
    { websocket: { enabled: false } }  // WebSocket explicitly DISABLED
  );
  // ...
}
```

**Critical:** WebSocket is disabled (`{ websocket: { enabled: false } }`), meaning the connection uses **WebTransport over HTTPS**.

---

## 2. **Voice Broadcast Path Structure**

### Room Audio Paths (ref/innpub/src/multiplayer/stream.ts:430-434)

Voice is broadcast per-room with this path structure:

```typescript
function buildAudioBroadcastPath(room: string, npub: string): Moq.Path.Valid {
  const normalizedRoom = room.trim();
  const normalizedNpub = npub.trim();
  return Moq.Path.from(
    "innpub",                    // Namespace
    "rooms",                     // Category
    ROOM_PROTOCOL_VERSION,       // "v2"
    normalizedRoom,              // Room ID (e.g., "main")
    normalizedNpub,              // User's Nostr public key
    AUDIO_SESSION_SUFFIX         // Random session ID
  );
}
```

**Example path:** `innpub/rooms/v2/main/npub1abc.../xyz123`

Where:
- `ROOM_PROTOCOL_VERSION = "v2"` (ref/innpub/src/game/state/types.ts:3)
- `AUDIO_SESSION_SUFFIX = Math.random().toString(36).slice(2, 8)` (ref/innpub/src/multiplayer/stream.ts:319)

---

## 3. **Microphone Publishing Flow**

### Step-by-step initialization:

**A. User enables microphone** (ref/innpub/src/multiplayer/stream.ts:1931-1954):

```typescript
async function startMicrophoneCapture(): Promise<void> {
  if (!audioSupported) {
    throw new Error("Microphone capture is not supported in this browser");
  }

  const identity = localState?.npub ?? pendingLocalIdentity ?? localSession?.npub;
  if (!identity) {
    throw new Error("Login before enabling the microphone");
  }

  ensureBeforeUnloadHook();
  micRequested = true;

  await syncLocalAudioPublishState();
}
```

**B. Acquire microphone track** (ref/innpub/src/multiplayer/stream.ts:456-491):

```typescript
async function ensureMicrophoneTrack(): Promise<void> {
  // ...
  const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
  const [track] = stream.getAudioTracks();

  hangMicrophoneTrack = track;
  track.addEventListener("ended", handleLocalTrackEnded);

  // Set the track as audio source for publishing
  hangPublish.audio.source.set(track as Hang.Publish.Audio.Source);

  micEnabled = true;
  setAudioState({ micEnabled: true, micError: null });
}
```

**C. Publish to relay** (ref/innpub/src/multiplayer/stream.ts:517-547):

```typescript
async function syncLocalAudioPublishState(): Promise<void> {
  const identity = localState?.npub ?? pendingLocalIdentity ?? localSession?.npub;
  const room = currentAudioRoom;  // First room in localRooms array
  const shouldPublish = micRequested && !!identity && !!room && audioSupported;

  if (!shouldPublish) {
    hangPublishEnabled.set(false);
    hangBroadcastPath.set(undefined);
    releaseMicrophoneTrack();
    return;
  }

  // Build the broadcast path for this room
  const path = buildAudioBroadcastPath(room, identity);
  hangBroadcastPath.set(path);  // e.g., "innpub/rooms/v2/main/npub1.../abc123"

  await ensureMicrophoneTrack();
  hangPublishEnabled.set(true);  // Start publishing
}
```

**D. Hang.Publish.Broadcast setup** (ref/innpub/src/multiplayer/stream.ts:320-331):

```typescript
const hangPublish = new Hang.Publish.Broadcast({
  connection: hangConnectionSignal,    // MoQ connection to relay
  enabled: hangPublishEnabled,         // Signal controlling publish state
  path: hangBroadcastPath,             // Signal with the broadcast path
  audio: {
    enabled: hangPublishEnabled,
    speaking: { enabled: true },       // Voice activity detection
  },
});
```

---

## 4. **Receiving Remote Voice**

### Room subscription (ref/innpub/src/multiplayer/stream.ts:549-563):

```typescript
function ensureRoomSubscription(room: string) {
  if (roomAudioSubscriptions.has(room)) return;

  // Subscribe to all audio in this room
  const prefix = Moq.Path.from("innpub", "rooms", ROOM_PROTOCOL_VERSION, room);
  // e.g., "innpub/rooms/v2/main"

  const roomWatcher = new HangRoom({
    connection: hangConnectionSignal,
    path: prefix
  });

  roomWatcher.onRemote((path, broadcast) => {
    if (broadcast) {
      handleRemoteAudioAdded(room, path, broadcast);
    } else {
      handleRemoteAudioRemoved(path);
    }
  });

  roomAudioSubscriptions.set(room, { room, prefix, roomWatcher });
}
```

### Remote audio playback (ref/innpub/src/multiplayer/stream.ts:581-649):

```typescript
function handleRemoteAudioAdded(
  room: string,
  path: Moq.Path.Valid,
  broadcast: Hang.Watch.Broadcast
) {
  // Don't play our own audio
  const localPath = hangBroadcastPath.peek();
  if (localPath && localPath === path) return;

  // Parse the path to get npub
  const { npub } = parseAudioBroadcastPath(path);

  // Create audio emitter for playback
  const emitter = new Hang.Watch.Audio.Emitter(broadcast.audio, {
    muted: !speakerEnabled,
    paused: !speakerEnabled,
  });

  // Track speaking state
  const disposeSpeaking = broadcast.audio.speaking.active.watch(active => {
    if (npub && active !== undefined) {
      setSpeakingLevel(npub, active ? 1 : 0);
    }
  });

  broadcast.enabled.set(true);
  broadcast.audio.enabled.set(speakerEnabled && localRooms.includes(room));

  remoteAudioSessions.set(path, {
    path, room, npub, broadcast, emitter, disposeSpeaking
  });
}
```

---

## 5. **Positional Audio** (Distance-based volume)

### Volume calculation (ref/innpub/src/multiplayer/stream.ts:755-793):

```typescript
function calculateDistance(x1: number, y1: number, x2: number, y2: number): number {
  const dx = x2 - x1;
  const dy = y2 - y1;
  return Math.sqrt(dx * dx + dy * dy);
}

function distanceToVolume(distance: number): number {
  const referenceDistance = 50;   // Full volume when closer than this
  const maxDistance = 250;         // Silent beyond this distance

  if (distance < referenceDistance) return 1.0;  // Full volume
  if (distance > maxDistance) return 0.0;        // Silent

  // Inverse square law falloff
  const normalizedDistance = (distance - referenceDistance) / (maxDistance - referenceDistance);
  const volume = Math.pow(1 - normalizedDistance, 2);

  return Math.max(0.0, Math.min(1.0, volume));
}
```

### Apply volume to sessions (ref/innpub/src/multiplayer/stream.ts:795-831):

```typescript
function updateAudioMix(): void {
  if (!localState) return;

  const localX = localState.x;
  const localY = localState.y;

  // Update volume for each remote audio session
  for (const session of remoteAudioSessions.values()) {
    if (!session.npub) continue;

    const remotePlayer = players.get(session.npub);
    if (!remotePlayer) continue;

    // Calculate distance
    const distance = calculateDistance(localX, localY, remotePlayer.x, remotePlayer.y);

    // Convert to volume and apply
    const volume = distanceToVolume(distance);
    session.emitter.volume.set(volume);  // Set actual playback volume

    volumeLevels.set(session.npub, volume);
  }

  syncPlayersToStore();
}
```

---

## 6. **Connection State Management**

### MoQ connection lifecycle (ref/innpub/src/multiplayer/stream.ts:1512-1529):

```typescript
function handleConnected(connection: Moq.Connection.Established) {
  gameStore.setConnection({
    status: "connected",
    relayUrl: RELAY_URL,  // "https://moq.justinmoon.com/anon"
    error: undefined,
    lastConnectedAt: Date.now(),
  });

  hangConnectionSignal.set(connection);  // Enables Hang.Publish/Watch

  // Subscribe to player announcements
  announcementAbort = new AbortController();
  void startAnnouncementLoop(connection, announcementAbort.signal);

  // Re-establish local session and rooms
  if (localState) {
    void ensureLocalSession(localState.npub);
  }
  updateRoomAudioSubscriptions();
  void syncLocalAudioPublishState().catch(() => undefined);
}
```

---

## **Key Differences: Localhost vs Public Relay**

### Why it works with localhost but not public relays:

1. **WebTransport Requirements:**
   - Public relays require **valid TLS certificates**
   - Localhost bypasses certificate validation
   - Check browser console for WebTransport errors

2. **Network/Firewall Issues:**
   - Public relays use **UDP (QUIC protocol)**
   - Some networks block UDP traffic
   - Localhost always allows UDP

3. **URL Validation:**
   ```typescript
   // moqConnection.ts:43
   new URL(RELAY_URL)  // Must be valid HTTPS URL
   ```
   - Ensure `https://moq.justinmoon.com/anon` is reachable
   - Test: `curl -I https://moq.justinmoon.com/anon`

4. **CORS/Origin Issues:**
   - Public relays may have origin restrictions
   - Localhost has no CORS enforcement

5. **Relay Configuration:**
   - The relay must support the MoQ protocol version
   - Check relay status/health endpoints

---

## **Debugging Checklist**

1. **Browser Console Errors:**
   - Look for `WebTransport` connection failures
   - Check for certificate errors

2. **Network Tab:**
   - Verify HTTPS connection to `moq.justinmoon.com`
   - Check for failed WebTransport streams

3. **Test Alternative Relay:**
   ```typescript
   // Uncomment line 4 in moqConnection.ts
   export const RELAY_URL = "https://relay.cloudflare.mediaoverquic.com";
   ```

4. **Verify Relay Accessibility:**
   ```bash
   curl -v https://moq.justinmoon.com/anon
   nc -zv moq.justinmoon.com 443
   ```

5. **Check Browser Support:**
   - WebTransport requires Chrome/Edge 97+ or Safari 17.5+
   - Firefox doesn't support WebTransport yet

---

## **Current Project Voice Implementation**

The current project (packages/app) has a similar but distinct implementation:

### Architecture Overview

**Service Layer:**
- `packages/app/src/services/voice/connection.ts` - MoQ connection management
- `packages/app/src/services/voice/manager.ts` - High-level voice manager
- `packages/app/src/services/voice/publisher.ts` - Audio publishing
- `packages/app/src/services/voice/subscriber.ts` - Audio subscription
- `packages/app/src/hooks/useVoice.ts` - React hook for UI integration

### Key Differences from ref/innpub:

1. **Dynamic Relay URL Discovery:**
   - ref/innpub: Hardcoded `https://moq.justinmoon.com/anon`
   - Current project: Fetches from Nostr live event (NIP-53)

   ```typescript
   // packages/app/src/services/live-event.ts:86
   const streaming_url = getTag('streaming')  // From live event kind:30311

   // packages/app/src/App.tsx:66-70
   const liveEvent = await fetchLiveEvent()
   if (liveEvent?.streaming_url) {
     setStreamingUrl(liveEvent.streaming_url)
   }
   ```

2. **Broadcast Path Structure:**
   - ref/innpub: `innpub/rooms/v2/{room}/{npub}/{session-id}`
   - Current project: `crossworld/voice/{LIVE_CHAT_D_TAG}/{npub}`

   ```typescript
   // packages/app/src/services/voice/publisher.ts:145
   const path = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}/${npub}`)
   // Example: "crossworld/voice/crossworld-dev/npub1abc..."
   ```

3. **Connection Management:**
   - Uses `Moq.Connection.Reload` for automatic reconnection
   - No explicit WebSocket disable (relies on library defaults)

   ```typescript
   // packages/app/src/services/voice/connection.ts:26-29
   this.connection = new Moq.Connection.Reload({
     enabled: this.enabledSignal,
     url: this.urlSignal,
   })
   ```

4. **Audio Publishing:**
   - Uses `@kixelated/hang` library's `Hang.Publish.Broadcast`
   - Automatic voice activity detection (VAD) built-in
   - Reactive signal-based architecture

   ```typescript
   // packages/app/src/services/voice/publisher.ts:153-164
   const broadcast = new Hang.Publish.Broadcast({
     connection: conn,
     path,
     enabled: true,
     audio: {
       enabled: true,
       source: audioSource,
       speaking: {
         enabled: true,  // VAD enabled
       },
     },
   })
   ```

### How Voice is Initialized:

**Step 1: Fetch MoQ relay URL from Nostr**
```typescript
// App.tsx:66-70
const liveEvent = await fetchLiveEvent()
if (liveEvent?.streaming_url) {
  setStreamingUrl(liveEvent.streaming_url)
  // Expected URLs (from comments):
  // - https://relay.moq.dev/anon
  // - https://relay.cloudflare.mediaoverquic.com/anon
}
```

**Step 2: User connects to voice**
```typescript
// Via useVoice hook
const voice = useVoice()
await voice.connect(streamingUrl, npub)
```

**Step 3: Connection manager establishes link**
```typescript
// connection.ts:53-62
connect(url: string): void {
  const parsedUrl = new URL(url)
  this.urlSignal.set(parsedUrl)
  this.enabledSignal.set(true)
  // Connection.Reload automatically connects
}
```

**Step 4: User enables microphone**
```typescript
// Via UI or hook
await voice.toggleMic()

// publisher.ts:40-48
async enableMic(npub: string): Promise<void> {
  this.npubSignal.set(npub)
  this.enabledSignal.set(true)
  // Triggers reactive effects to:
  // 1. Acquire microphone
  // 2. Create broadcast
  // 3. Start publishing
}
```

**Step 5: Microphone acquisition**
```typescript
// publisher.ts:82-88
const stream = await navigator.mediaDevices.getUserMedia({
  audio: {
    echoCancellation: true,
    noiseSuppression: true,
    autoGainControl: true,
  },
})
const track = stream.getAudioTracks()[0]
this.microphoneSource.set(track)
```

**Step 6: Broadcast creation**
```typescript
// publisher.ts:134-164 (reactive effect)
#runBroadcast(effect: Effect): void {
  const conn = effect.get(this.connection.established)
  const npub = effect.get(this.npubSignal)
  const audioSource = effect.get(this.microphoneSource)

  const path = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}/${npub}`)

  const broadcast = new Hang.Publish.Broadcast({
    connection: conn,
    path,
    enabled: true,
    audio: { enabled: true, source: audioSource, speaking: { enabled: true } },
  })
}
```

### Current Project Configuration:

**Live Event Settings** (packages/app/src/config.ts):
```typescript
export const LIVE_CHAT_D_TAG = 'crossworld-dev'
export const APP_NPUB = 'npub1ga6mzn7ygwuxpytr264uw09huwef9ypzfda767088gv83ypgtjtsxf25vh'
export const DEFAULT_RELAYS = [
  'wss://strfry.atlantislabs.space/',
  'wss://relay.damus.io',
  'wss://nos.lol',
  'wss://relay.primal.net',
]
```

**Voice Namespace:**
- Prefix: `crossworld/voice/{d-tag}/{npub}`
- Example: `crossworld/voice/crossworld-dev/npub1abc...`

### Debugging Current Project:

1. **Check if streaming URL is fetched:**
   ```javascript
   // Browser console
   // Look for: "[App] MoQ streaming URL from live event: ..."
   ```

2. **Verify live event exists:**
   - Check kind 30311 event from `APP_NPUB`
   - Must have `["streaming", "https://..."]` tag
   - Query relays: `wss://relay.damus.io`, etc.

3. **Check connection logs:**
   ```
   [MoQ Connection] Initiating connection to: https://...
   [MoQ Connection] Status: connecting
   [MoQ Connection] Status: connected
   [MoQ Connection] Established: { url: "https://..." }
   ```

4. **Check publisher logs:**
   ```
   [MoQ Publisher] Enabling microphone for: npub1...
   [MoQ Publisher] Requesting microphone permission...
   [MoQ Publisher] Microphone acquired: { label: "...", enabled: true, readyState: "live" }
   [MoQ Publisher] Creating broadcast: { path: "crossworld/voice/...", npub: "...", dTag: "..." }
   [MoQ Publisher] Now publishing audio to relay
   ```

5. **Common Issues:**
   - **No streaming URL:** Live event missing or doesn't have `streaming` tag
   - **Connection timeout:** MoQ relay unreachable or wrong URL
   - **WebTransport blocked:** Network/firewall blocking UDP
   - **Browser incompatibility:** Use Chrome/Edge 97+ or Safari 17.5+

### Audio Subscription (Receiving Voice):

The current project uses **dual discovery** - both Nostr events AND MoQ announcements:

```typescript
// subscriber.ts:152-177
async startListening(): Promise<void> {
  // 1. Nostr-based discovery (via AvatarStateService)
  //    Watches for users with voiceConnected=true in their avatar state
  this.unsubscribeAvatarState = this.avatarStateService.onChange((states) => {
    this.handleClientListUpdate(conn, states)
  })

  // 2. MoQ announcement-based discovery
  //    Listens for broadcasts at: crossworld/voice/{d-tag}/*
  const prefix = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}`)
  const announced = connection.announced(prefix)
  // Loop and watch for announcements...
}
```

**Discovery Sources:**
- `'nostr'` - User found via avatar state events (kind 30317/1317)
- `'moq'` - User found via MoQ relay announcements
- `'both'` - User discovered through both methods (most reliable)

**Participant Watcher** (subscriber.ts:20-109):
```typescript
class ParticipantWatcher {
  constructor(connection, npub, source) {
    const path = Moq.Path.from(`crossworld/voice/${LIVE_CHAT_D_TAG}/${npub}`)

    // Create broadcast watcher
    this.watcher = new Hang.Watch.Broadcast({
      connection,
      path,
      enabled: true,
      audio: {
        enabled: true,
        latency: 100,  // 100ms jitter buffer
        speaking: { enabled: true },
      },
    })

    // Create audio emitter for playback
    this.emitter = new Hang.Watch.Audio.Emitter(this.watcher.audio, {
      volume: this.volume,
      muted: this.muted,
      paused: new Signal(false),
    })
  }
}
```

### Key Implementation Differences:

| Feature | ref/innpub | Current Project |
|---------|-----------|-----------------|
| **Relay URL** | Hardcoded `https://moq.justinmoon.com/anon` | Dynamic from Nostr live event |
| **Namespace** | `innpub/rooms/v2/{room}/{npub}/{session}` | `crossworld/voice/{d-tag}/{npub}` |
| **Room System** | Multi-room with room-based audio | Single global voice space |
| **Discovery** | MoQ announcements only | Dual: Nostr + MoQ announcements |
| **Positional Audio** | Distance-based volume (50-250px) | Manual volume control per participant |
| **Connection** | `Moq.Connection.connect()` | `Moq.Connection.Reload` (auto-reconnect) |

### Migration Checklist (ref/innpub → Current Project):

- [x] MoQ connection manager with auto-reconnect
- [x] Dynamic relay URL from Nostr live events
- [x] Audio publisher with VAD
- [x] Audio subscriber with dual discovery (Nostr + MoQ)
- [x] Participant volume/mute controls
- [ ] Positional audio (distance-based volume) - **MISSING**
- [ ] Room-based audio routing - **MISSING**
- [ ] Location publisher/subscriber for positional audio - **NEEDED**

### What's Missing for Full Parity:

1. **Positional Audio System:**
   - ref/innpub calculates volume based on player distance
   - Current project has fixed volume per participant
   - **Solution:** Integrate avatar positions from `AvatarStateService` with participant volume

2. **Room-based Audio:**
   - ref/innpub supports multiple rooms with isolated audio
   - Current project has single global voice space
   - **Solution:** Add room parameter to broadcast path

3. **Audio Mix Updates:**
   - ref/innpub calls `updateAudioMix()` on player movement
   - Current project has no automatic distance-based volume adjustment
   - **Solution:** Subscribe to position changes and call `setParticipantVolume()`

### Implementation Roadmap for Positional Audio:

```typescript
// Proposed integration in subscriber.ts or new service

class PositionalAudioController {
  constructor(
    private subscriber: AudioSubscriber,
    private avatarState: AvatarStateService
  ) {}

  start() {
    // Watch for position changes
    this.avatarState.onChange((states) => {
      const localState = states.get(this.ownNpub)
      if (!localState) return

      // Update volume for each remote participant
      states.forEach((remoteState) => {
        if (remoteState.npub === this.ownNpub) return

        const distance = calculateDistance(
          localState.position.x,
          localState.position.y,
          remoteState.position.x,
          remoteState.position.y
        )

        const volume = distanceToVolume(distance)
        this.subscriber.setParticipantVolume(remoteState.npub, volume)
      })
    })
  }

  // From ref/innpub/src/multiplayer/stream.ts:769-793
  private distanceToVolume(distance: number): number {
    const referenceDistance = 50
    const maxDistance = 250
    if (distance < referenceDistance) return 1.0
    if (distance > maxDistance) return 0.0
    const normalized = (distance - referenceDistance) / (maxDistance - referenceDistance)
    return Math.pow(1 - normalized, 2)
  }
}
```
