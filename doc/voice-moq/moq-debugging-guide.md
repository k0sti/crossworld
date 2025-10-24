# MoQ Voice & Location Debugging Guide

## Overview

The MoQ implementation now uses **dual discovery** for maximum reliability:
- **Nostr-based discovery**: Via ClientStatusService (kind 30315 events)
- **MoQ announcement-based discovery**: Via relay's native announcement system

This guide helps you debug connectivity issues and understand what's happening.

## Quick Diagnosis Checklist

### 1. Check MoQ Connection Status

Look for these log messages in the browser console (filter by `[MoQ Connection]`):

```
✅ Good:
[MoQ Connection] Status: connected
[MoQ Connection] Established: { url: "...", sessionId: "..." }

❌ Bad:
[MoQ Connection] Status: connecting (stuck here)
[MoQ Connection] Status: disconnected
```

**If stuck on "connecting":**
- Check the relay URL format (must be `https://relay.example.com/anon`, NOT `/crossworld-dev`)
- Verify the relay is actually running (for local: `just moq-relay`)
- Check browser console for CORS or SSL errors
- Try a different relay (local vs public)

### 2. Check Broadcast Announcement

When you enable your microphone, look for:

```
✅ Good:
[MoQ Publisher] Creating broadcast: { path: "crossworld/voice/crossworld-dev/npub1...", ... }
[MoQ Publisher] Broadcast created, waiting for announcement...
[MoQ Publisher] Now publishing audio to relay

❌ Bad:
[MoQ Publisher] Failed to acquire microphone: ...
(No "Creating broadcast" message appears)
```

**If broadcast not created:**
- Check microphone permissions
- Verify MoQ connection is established first
- Check for JavaScript errors in console

### 3. Check Discovery Systems

Both discovery methods should be active:

```
✅ Good (Nostr):
[MoQ Subscriber] Starting Nostr-based discovery...
[MoQ Subscriber] Nostr discovery active

✅ Good (MoQ):
[MoQ Subscriber] Starting MoQ announcement-based discovery...
[MoQ Subscriber] Listening for announcements with prefix: crossworld/voice/crossworld-dev
[MoQ Subscriber] Announcement received: { path: "...", active: true, totalReceived: 1 }

❌ Bad:
ClientStatusService not set - Nostr discovery disabled
(No announcement messages appearing)
```

### 4. Check Participant Detection

When another participant joins:

```
✅ Good:
[MoQ Subscriber] Participant announced via MoQ: npub1...
[MoQ Subscriber] Creating watcher for: npub1... via moq
[MoQ Subscriber] Watcher created and active for: npub1...

OR (if also using Nostr):
[MoQ Subscriber] Client joined voice (Nostr): npub1...
[MoQ Subscriber] Participant now discovered via both sources: npub1...

❌ Bad:
(No watcher creation messages)
Skipping own broadcast or invalid npub
```

## Common Issues & Solutions

### Issue: Local Relay Works, Public Relay Doesn't

**Symptoms:**
- Voice works with `http://localhost:4443/anon`
- Voice doesn't work with `https://relay.moq.dev/anon`

**Likely Causes:**
1. **URL Format**: Ensure public relay URL uses `/anon` suffix, not custom paths
2. **Firewall/Network**: Some networks block WebTransport/QUIC
3. **Relay Capacity**: Public relays may have connection limits

**Debug Steps:**
```javascript
// In browser console:
// Check if announcements are being received
// Look for: [MoQ Subscriber] Announcement received: ...

// If NO announcements:
// - The relay may not be forwarding announcements
// - Your broadcast may not be reaching the relay

// If YES announcements but no audio:
// - Check watcher creation logs
// - Check for audio decoding errors
```

### Issue: No Participants Detected

**Symptoms:**
- MoQ connection shows "connected"
- Your broadcast is created
- But no other participants appear

**Likely Causes:**
1. **Single User**: You might be the only one connected
2. **Discovery Mismatch**: Participants on different d-tags or paths
3. **Announcement Not Working**: Relay not forwarding announcements

**Debug Steps:**
1. Check announcement listener is active:
   ```
   [MoQ Subscriber] Listening for announcements with prefix: ...
   ```

2. Test with two browser windows:
   - Open two windows with different Nostr identities
   - Enable voice in both
   - Watch console logs in both windows

3. Verify path matching:
   - Publisher path: `crossworld/voice/crossworld-dev/npub1abc...`
   - Subscriber prefix: `crossworld/voice/crossworld-dev`
   - These must match!

### Issue: Participant Detected But No Audio

**Symptoms:**
- Watcher created successfully
- Speaking state updates appear
- But no audio plays

**Likely Causes:**
1. **Audio Pipeline Issue**: Decoding or playback problem
2. **Volume Settings**: Participant muted or volume at 0
3. **Browser Audio Policy**: Autoplay restrictions

**Debug Steps:**
```
✅ Check speaking detection:
[MoQ Subscriber] Speaking state for npub1...: true

✅ Check watcher created with audio enabled:
Look for Hang.Watch.Broadcast creation in logs

❌ If no speaking state updates:
- Audio stream may not be reaching the subscriber
- Check for MoQ track errors in console
```

## Discovery Source Indicators

The `discoverySource` field on participants shows how they were discovered:

- **`"nostr"`**: Only found via Nostr ClientStatusService
- **`"moq"`**: Only found via MoQ announcements
- **`"both"`**: Found via both methods (most reliable!)

**Ideal state:** All participants should show `"both"` after a few seconds.

**If only "nostr":**
- MoQ announcements may not be working
- Check announcement listener logs

**If only "moq":**
- Nostr discovery may not be working
- Check ClientStatusService logs

## Testing Procedure

### Step 1: Test with Local Relay

```bash
# Terminal 1: Start local relay
just moq-relay

# Terminal 2: Update live event (if needed)
cd crates/worldtool
cargo run -- init-live --streaming http://localhost:4443/anon
```

Then in browser:
1. Open two windows (use incognito for different Nostr identity)
2. Login to both
3. Enable voice in Window 1
4. Check console logs for broadcast creation
5. Enable voice in Window 2
6. Window 1 should detect Window 2 via announcements
7. Window 2 should detect Window 1 via announcements

### Step 2: Test with Public Relay

Update streaming URL to `https://relay.moq.dev/anon` and repeat above steps.

**Expected difference:** Public relay may have higher latency but should work the same way.

## Debug Log Patterns

### Successful Voice Connection Flow

```
# Window 1 (First user):
[MoQ Connection] Status: connecting
[MoQ Connection] Status: connected
[MoQ Connection] Established: { url: "...", ... }
[MoQ Publisher] Enabling microphone for: npub1...
[MoQ Publisher] Microphone acquired: { label: "...", ... }
[MoQ Publisher] Creating broadcast: { path: "...", ... }
[MoQ Publisher] Now publishing audio to relay
[MoQ Subscriber] Starting DUAL discovery (Nostr + MoQ announcements)...
[MoQ Subscriber] Listening for announcements with prefix: crossworld/voice/crossworld-dev

# Window 2 (Second user):
[MoQ Connection] Status: connected
[MoQ Publisher] Creating broadcast: { path: "...", ... }
[MoQ Subscriber] Announcement received: { path: "...", active: true }
[MoQ Subscriber] Participant announced via MoQ: npub1...
[MoQ Subscriber] Creating watcher for: npub1... via moq
[MoQ Subscriber] Watcher created and active for: npub1...

# Window 1 (sees Window 2):
[MoQ Subscriber] Announcement received: { path: "...", active: true }
[MoQ Subscriber] Participant announced via MoQ: npub1...
[MoQ Subscriber] Creating watcher for: npub1... via moq
[MoQ Subscriber] Watcher created and active for: npub1...
```

## Console Helpers

You can inspect the current state in the browser console:

```javascript
// Check voice manager state
voiceManager.status.peek()  // 'disconnected' | 'connecting' | 'connected'
voiceManager.getParticipants()  // Array of participants
voiceManager.getParticipantCount()  // Number

// Check connection state
moqConnection.isConnected()  // boolean
moqConnection.status.peek()  // connection status
```

## URL Format Reference

**✅ Correct formats:**
```
http://localhost:4443/anon
https://relay.moq.dev/anon
https://relay.cloudflare.mediaoverquic.com/anon
```

**❌ Incorrect formats:**
```
https://relay.moq.dev/crossworld-dev  (custom path - won't work)
https://relay.moq.dev  (missing /anon suffix)
http://relay.moq.dev/anon  (should be https for public relays)
```

## Advanced Debugging

### Enable Verbose MoQ Logs

The MoQ library may have its own debug flags. Check the library documentation or enable verbose mode if available.

### Network Inspection

1. Open browser DevTools → Network tab
2. Filter by "WS" (WebSocket) or look for QUIC connections
3. Check if connection is established
4. Look for any error responses

### Common Error Messages

**"Failed to create watcher"**:
- The broadcast path may not exist
- Connection may have dropped
- Check MoQ connection status

**"Announcement loop failed"**:
- Connection was lost
- Relay stopped sending announcements
- Check network connectivity

**"ClientStatusService not set"**:
- This is OK - means Nostr discovery is disabled
- MoQ discovery should still work

## Location Broadcasting

Location broadcasting works the same way as voice:

```
✅ Good:
[Location Publisher] Creating broadcast: { path: "crossworld/location/...", ... }
[Location Subscriber] Announcement received: { path: "...", active: true }
[Location Subscriber] Received location for npub1...: { x: 10, y: 20, ... }

❌ Bad:
(No location messages appear)
```

**Note:** Location data is sent via JSON over MoQ tracks, which is simpler than audio but follows the same discovery pattern.

## Getting Help

When reporting issues, include:

1. **Browser console logs** (filter by `[MoQ` for relevant messages)
2. **Relay URL** you're trying to connect to
3. **Steps to reproduce** the issue
4. **Discovery source** of participants (nostr/moq/both)
5. **Whether local relay works** vs public relay

Example issue report:
```
Issue: No audio from participants

Environment:
- Relay: https://relay.moq.dev/anon
- Browser: Chrome 121
- Local relay: Works fine
- Public relay: Connection established but no participants detected

Logs:
[MoQ Connection] Status: connected
[MoQ Subscriber] Listening for announcements with prefix: crossworld/voice/crossworld-dev
(No "Announcement received" messages appearing)

Participants:
- Self: broadcast created successfully
- Others: Not detected (0 announcements received)
```

This helps identify if the issue is with:
- Connection establishment ❌
- Broadcast announcement ✅
- Discovery system ❌
- Audio pipeline ❓
