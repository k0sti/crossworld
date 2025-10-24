# MoQ Voice Implementation Summary

## What Changed

### Overview
Implemented **dual discovery system** for MoQ voice chat:
1. **Nostr-based discovery** (existing): Via ClientStatusService
2. **MoQ announcement discovery** (new): Via relay's native announcement system

Plus comprehensive debug logging and location broadcasting support.

---

## Key Improvements

### 1. Dual Discovery System

**Problem**: Relying solely on Nostr for participant discovery meant:
- If Nostr fails, no participants detected
- MoQ relay announcements were ignored
- Public relays might not work due to missing native discovery

**Solution**: Listen to BOTH sources:
```typescript
// Nostr discovery (existing)
clientStatusService.onChange((clients) => {
  // Create watchers for voice-connected clients
})

// MoQ discovery (new)
connection.announced(prefix).next()
  // Create watchers for announced broadcasts
```

**Result**: Participants discovered via either method, marked as `'nostr'`, `'moq'`, or `'both'`.

### 2. Comprehensive Debug Logging

All components now have tagged logging:
- `[MoQ Connection]` - Connection state changes
- `[MoQ Publisher]` - Broadcast creation, microphone acquisition
- `[MoQ Subscriber]` - Discovery, watcher creation, announcements
- `[Location Publisher]` - Position data broadcasting
- `[Location Subscriber]` - Position data receiving

**Example Console Output:**
```
[MoQ Connection] Initiating connection to: https://relay.moq.dev/anon
[MoQ Connection] Status: connecting
[MoQ Connection] Status: connected
[MoQ Publisher] Enabling microphone for: npub1...
[MoQ Publisher] Creating broadcast: { path: "...", npub: "...", dTag: "..." }
[MoQ Subscriber] Starting DUAL discovery (Nostr + MoQ announcements)...
[MoQ Subscriber] Announcement received: { path: "...", active: true, totalReceived: 1 }
```

### 3. Location Broadcasting

New components for real-time position sharing:
- `LocationPublisher`: Broadcasts position data via MoQ
- `LocationSubscriber`: Receives position data from others

**Path format**: `crossworld/location/${d-tag}/${npub}`

**Usage:**
```typescript
// Publishing
locationPublisher.enable(npub)
locationPublisher.updateLocation(x, y, z)

// Subscribing
locationSubscriber.startListening()
const locations = locationSubscriber.locations.peek()  // Map<npub, location>
```

### 4. URL Format Fixes

**Corrected relay URL format:**
```
✅ https://relay.moq.dev/anon
✅ https://relay.cloudflare.mediaoverquic.com/anon
❌ https://relay.cloudflare.mediaoverquic.com/crossworld-dev
```

The `/anon` suffix is standard for MoQ relays. Custom paths like `/crossworld-dev` won't work.

---

## File Changes

### Modified Files

1. **`packages/app/src/services/voice/connection.ts`**
   - Added debug logging for connection state changes
   - Added URL parsing validation

2. **`packages/app/src/services/voice/publisher.ts`**
   - Added debug logging for microphone acquisition
   - Added debug logging for broadcast creation
   - Added speaking state logging

3. **`packages/app/src/services/voice/subscriber.ts`** (MAJOR)
   - Added MoQ announcement listener (`startAnnouncementListener`)
   - Implemented dual discovery (Nostr + MoQ)
   - Added `discoverySource` tracking ('nostr' | 'moq' | 'both')
   - Added comprehensive debug logging
   - Added public debug stats (`announcementsReceived`, `activeAnnouncementCount`)

4. **`packages/app/src/App.tsx`**
   - Updated relay URL comments with correct format
   - Added clearer debug logging

### New Files

1. **`packages/app/src/services/voice/location-publisher.ts`**
   - Publishes position data via MoQ
   - Similar architecture to AudioPublisher
   - JSON-based data transmission

2. **`packages/app/src/services/voice/location-subscriber.ts`**
   - Receives position data from participants
   - MoQ announcement-based discovery
   - Tracks `discoverySource` like audio subscriber

3. **`doc/moq-debugging-guide.md`**
   - Comprehensive debugging guide
   - Common issues and solutions
   - Log pattern reference
   - Testing procedures

4. **`doc/moq-implementation-summary.md`** (this file)
   - Implementation overview
   - Design decisions
   - Integration guide

---

## Architecture

### Discovery Flow

```
┌─────────────────┐
│  Participant A  │
│  (publishing)   │
└────────┬────────┘
         │
         │ 1. Creates broadcast
         │    Path: crossworld/voice/crossworld-dev/npubA
         v
┌────────────────────┐
│    MoQ Relay       │
│                    │
│  - Stores broadcast│
│  - Announces to    │
│    subscribers     │
└────────┬───────────┘
         │
         │ 2a. Announces broadcast
         │     (MoQ discovery)
         │
         │ 2b. Nostr status event
         │     (Nostr discovery)
         │
         v
┌─────────────────┐
│  Participant B  │
│  (subscribing)  │
│                 │
│  Discoveries:   │
│  □ MoQ announce │ ← creates watcher (source: 'moq')
│  □ Nostr status │ ← marks as 'both' if already watching
└─────────────────┘
```

### Component Relationships

```
VoiceManager
├── MoqConnectionManager (shared)
│   └── Moq.Connection.Reload
│       └── Moq.Connection.Established
│
├── AudioPublisher
│   ├── Microphone acquisition
│   └── Hang.Publish.Broadcast
│       └── Announces to relay
│
└── AudioSubscriber
    ├── Nostr discovery (ClientStatusService)
    ├── MoQ discovery (connection.announced)
    └── ParticipantWatcher (per participant)
        └── Hang.Watch.Broadcast
            └── Hang.Watch.Audio.Emitter

LocationManager (similar structure)
├── MoqConnectionManager (same instance)
├── LocationPublisher
│   └── Moq.Broadcast
└── LocationSubscriber
    └── LocationWatcher (per participant)
        └── Moq.Track
```

---

## Integration Guide

### Adding Location Broadcasting to Voice Manager

You can optionally integrate location broadcasting:

```typescript
// In VoiceManager constructor:
private locationPublisher: LocationPublisher
private locationSubscriber: LocationSubscriber

constructor() {
  // ... existing code ...
  this.locationPublisher = new LocationPublisher(this.connection)
  this.locationSubscriber = new LocationSubscriber(this.connection)
}

// In connect():
async connect(streamingUrl: string, npub: string) {
  // ... existing voice connection ...

  // Start location broadcasting
  await this.locationPublisher.enable(npub)
  await this.locationSubscriber.startListening()
  this.locationSubscriber.setOwnNpub(npub)
}

// Add method to update location:
updateLocation(x: number, y: number, z?: number) {
  this.locationPublisher.updateLocation(x, y, z)
}

// In disconnect():
async disconnect() {
  // ... existing voice disconnection ...

  await this.locationPublisher.disable()
  this.locationSubscriber.stopListening()
}
```

### Using Location Data in UI

```typescript
// In your component:
const locations = locationSubscriber.locations.peek()

locations.forEach((location, npub) => {
  console.log(`${npub} is at position (${location.x}, ${location.y})`)
  // Update avatar position, show on minimap, etc.
})

// Subscribe to changes:
locationSubscriber.locations.subscribe((locations) => {
  // React to location updates
})
```

---

## Design Decisions

### Why Dual Discovery?

**Reliability**: If one system fails, the other still works.
- Nostr relays down? MoQ announcements still work.
- MoQ announcements broken? Nostr discovery still works.

**Compatibility**:
- Works with any MoQ relay (using native announcements)
- Works with existing Nostr infrastructure

**Debugging**:
- `discoverySource` field helps identify issues
- If everyone shows 'both', both systems are working
- If only 'nostr' or 'moq', you know which system to debug

### Why Location via MoQ Instead of Nostr?

**Lower Latency**:
- MoQ designed for real-time streams
- Nostr events have relay propagation delay

**Less Overhead**:
- No signing required for every update
- No event storage/querying

**Consistent Architecture**:
- Same discovery pattern as voice
- Reuses MoQ connection

**Future Extensibility**:
- Easy to add more data streams (health, inventory, etc.)
- All use same MoQ infrastructure

### Why JSON for Location Data?

**Simplicity**:
- Easy to serialize/deserialize
- Human-readable for debugging

**Flexibility**:
- Can add fields without breaking protocol
- Compatible with future extensions

**Performance**:
- Location updates are infrequent compared to audio
- JSON overhead is negligible

---

## Testing Strategy

### Manual Testing

1. **Single User (Local Relay)**:
   ```bash
   just moq-relay
   # Open browser, login, enable voice
   # Check console for broadcast creation
   ```

2. **Two Users (Local Relay)**:
   ```bash
   # Window 1: Login as User A, enable voice
   # Window 2: Login as User B (incognito), enable voice
   # Both should detect each other via MoQ announcements
   ```

3. **Public Relay Test**:
   ```bash
   # Update live event to use https://relay.moq.dev/anon
   # Repeat two-user test
   # Verify announcements work over public internet
   ```

### Automated Testing (Future)

Consider adding:
- Unit tests for discovery logic
- Integration tests with mock MoQ relay
- E2E tests with real relay instances

---

## Known Limitations

1. **No Spatial Audio**: Location data exists but not yet integrated with audio volume/panning
2. **No Reconnection UI**: Connection status shown in console only
3. **No Rate Limiting**: Location updates not throttled
4. **No Persistence**: Location data lost on page refresh

---

## Future Enhancements

### Short Term
- [ ] Integrate location data with audio (positional audio)
- [ ] Add visual debug panel (UI component)
- [ ] Throttle location updates (e.g., max 10/second)
- [ ] Add connection status indicator in UI

### Medium Term
- [ ] Add more data streams (health, inventory, actions)
- [ ] Implement spatial audio based on location
- [ ] Add relay quality metrics
- [ ] Implement automatic relay fallback

### Long Term
- [ ] Multi-relay support (connect to multiple relays)
- [ ] Peer-to-peer mode (bypass relay for nearby users)
- [ ] Voice quality adaptation based on bandwidth
- [ ] Recording/playback support

---

## Debugging Quick Reference

**Check connection:**
```javascript
moqConnection.isConnected()  // true/false
```

**Check participants:**
```javascript
voiceManager.getParticipants()
// Look for discoverySource: 'both' (ideal)
```

**Check announcements:**
```javascript
// In console, filter by: [MoQ Subscriber]
// Look for: "Announcement received"
```

**Common issues:**
- Stuck on "connecting" → Check URL format
- No participants → Check announcement listener logs
- No audio → Check watcher creation logs

**See full guide:** `doc/moq-debugging-guide.md`

---

## Summary

This implementation provides:
✅ Robust dual discovery (Nostr + MoQ)
✅ Comprehensive debug logging
✅ Location broadcasting infrastructure
✅ Correct relay URL handling
✅ Detailed debugging documentation

**Status**: Ready for testing with both local and public relays.

**Next steps**:
1. Test with local relay (`just moq-relay`)
2. Test with public relay (https://relay.moq.dev/anon)
3. Integrate location data into game world
4. Add UI debug panel (optional)
