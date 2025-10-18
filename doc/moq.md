# MoQ Voice Connection and Streaming Implementation

This document provides a thorough technical explanation of how voice connection initialization and streaming works in the MoQ `hang` library (`ref/moq/js/hang`) and the demo application (`ref/moq/js/hang-demo`).

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Demo Application Entry Points](#demo-application-entry-points)
3. [Publisher Side: Voice Transmission](#publisher-side-voice-transmission)
4. [Subscriber Side: Voice Reception](#subscriber-side-voice-reception)
5. [Audio Pipeline Details](#audio-pipeline-details)
6. [Connection Management](#connection-management)
7. [Signal-based Reactive System](#signal-based-reactive-system)

## Architecture Overview

The MoQ voice system uses a **layered architecture**:

```
┌─────────────────────────────────────────────────────┐
│  Demo Application (hang-demo)                       │
│  - Web Components: <hang-publish>, <hang-watch>    │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  Hang Library (hang)                                │
│  - Broadcast management                             │
│  - Audio encoding/decoding                          │
│  - Connection handling                              │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  MoQ Core (moq)                                     │
│  - QUIC/WebTransport                                │
│  - Pub/Sub primitives                               │
└─────────────────────────────────────────────────────┘
```

The voice system is entirely **reactive** using signals for state management, enabling automatic cleanup and dependency tracking.

## Demo Application Entry Points

### Publisher Demo (`ref/moq/js/hang-demo/src/publish.ts`)

The publisher demo uses the `<hang-publish>` Web Component:

```typescript
// Creates a Web Component that handles the entire publish pipeline
const publish = document.querySelector("hang-publish") as HangPublish;

// Configuration via attributes
publish.setAttribute("path", "broadcast-name");  // MoQ broadcast path
publish.setAttribute("audio", "");               // Enable audio
publish.setAttribute("video", "");               // Enable video (optional)
```

### Watcher Demo (`ref/moq/js/hang-demo/src/index.ts`)

The watcher demo uses the `<hang-watch>` Web Component:

```typescript
// Creates a Web Component that handles the entire watch pipeline
const watch = document.querySelector("hang-watch") as HangWatch;

// Configuration via attributes
watch.setAttribute("path", "broadcast-name");    // MoQ broadcast path to watch
watch.setAttribute("url", "wss://relay-url");    // MoQ relay server URL
```

Both components are **declarative** - just insert them into the DOM with the right attributes, and they handle everything automatically.

## Publisher Side: Voice Transmission

### 1. Component Initialization (`ref/moq/js/hang/src/publish/element.ts`)

When the `<hang-publish>` element is connected to the DOM:

```typescript
connectedCallback() {
    this.active.set(new HangPublishInstance(this));
}
```

This creates a `HangPublishInstance` which orchestrates the entire publishing pipeline.

#### Key Steps in HangPublishInstance Constructor:

**Step 1: Connection Setup** (`element.ts:175-178`)

```typescript
this.connection = new Moq.Connection.Reload({
    enabled: true,
    url: this.parent.signals.url,
});
```

- Creates a **self-healing MoQ connection** that automatically reconnects on failure
- `Connection.Reload` wraps the base connection with retry logic
- The connection is reactive - when URL changes, it reconnects automatically

**Step 2: Broadcast Creation** (`element.ts:180-199`)

```typescript
this.broadcast = new Broadcast({
    connection: this.connection.established,
    enabled: true,
    path: this.parent.signals.path,
    audio: {
        enabled: this.parent.signals.audio,
        captions: {
            enabled: this.parent.signals.captions,
        },
        speaking: {
            enabled: this.parent.signals.captions,
        },
    },
    video: { /* ... */ },
});
```

- Creates a `Broadcast` object that manages the MoQ broadcast
- The broadcast is **signal-driven** - when `audio.enabled` changes, audio starts/stops automatically
- Speaking detection and captions are optional features

**Step 3: Source Acquisition** (`element.ts:230-283`)

The `#runSource` effect acquires the media source (microphone or screen):

```typescript
#runSource(effect: Effect) {
    const source = effect.get(this.parent.signals.source);

    if (source === "camera") {
        // Create microphone source
        const audio = new Source.Microphone({
            enabled: this.broadcast.audio.enabled
        });

        // Connect microphone output to broadcast input
        audio.signals.effect((effect) => {
            const source = effect.get(audio.source);
            effect.set(this.broadcast.audio.source, source);
        });
    }
}
```

### 2. Microphone Acquisition (`ref/moq/js/hang/src/publish/source/microphone.ts`)

The `Microphone` class handles acquiring the actual audio track:

```typescript
#run(effect: Effect): void {
    const enabled = effect.get(this.enabled);
    if (!enabled) return;

    const device = effect.get(this.device.requested);
    const constraints = effect.get(this.constraints) ?? {};

    const finalConstraints: MediaTrackConstraints = {
        ...constraints,
        deviceId: device !== undefined ? { exact: device } : undefined,
    };

    effect.spawn(async () => {
        const stream = await navigator.mediaDevices.getUserMedia({
            audio: finalConstraints
        });

        const track = stream.getAudioTracks()[0];
        effect.set(this.source, track);  // Expose the audio track
    });
}
```

**Key behaviors:**
- **Automatic cleanup**: When the effect is cancelled, tracks are automatically stopped
- **Device selection**: Supports selecting specific microphone devices
- **Permission handling**: Triggers browser permission dialog on first use

### 3. Audio Encoding (`ref/moq/js/hang/src/publish/audio/encoder.ts`)

The `Encoder` class converts raw audio into encoded frames:

#### Audio Context Setup (`encoder.ts:79-127`)

```typescript
#runSource(effect: Effect): void {
    const source = effect.get(this.source);  // MediaStreamTrack
    if (!source) return;

    const settings = source.getSettings();

    // Create AudioContext with optimal settings
    const context = new AudioContext({
        latencyHint: "interactive",
        sampleRate: settings.sampleRate,
    });

    // Create input node from MediaStreamTrack
    const root = new MediaStreamAudioSourceNode(context, {
        mediaStream: new MediaStream([source]),
    });

    // Add gain control for volume/muting
    const gain = new GainNode(context, {
        gain: this.volume.peek(),
    });
    root.connect(gain);

    // Load and register the capture worklet
    await context.audioWorklet.addModule(CaptureWorklet);

    // Create the capture worklet
    const worklet = new AudioWorkletNode(context, "capture", {
        numberOfInputs: 1,
        numberOfOutputs: 0,
        channelCount: settings.channelCount,
    });

    gain.connect(worklet);
}
```

**The AudioWorklet Pipeline:**

1. **MediaStreamTrack** → Raw audio from microphone
2. **MediaStreamAudioSourceNode** → Converts track to Web Audio API node
3. **GainNode** → Volume control and muting
4. **CaptureWorklet** → Captures audio samples and sends them to the main thread

#### Encoding Pipeline (`encoder.ts:161-239`)

```typescript
serve(track: Moq.Track, effect: Effect): void {
    const worklet = effect.get(this.#worklet);
    const config = effect.get(this.#config);

    let group: Moq.Group = track.appendGroup();
    let groupTimestamp: Time.Micro | undefined;

    // Create Opus encoder
    const encoder = new AudioEncoder({
        output: (frame) => {
            // Start new group every maxLatency milliseconds
            if (frame.timestamp - groupTimestamp >= this.maxLatency) {
                group.close();
                group = track.appendGroup();
                groupTimestamp = frame.timestamp;
            }

            // Encode frame and write to MoQ group
            const buffer = Frame.encode(frame, frame.timestamp);
            group.writeFrame(buffer);
        },
        error: (err) => {
            console.error("encoder error", err);
            group.close(err);
        },
    });

    // Configure with Opus codec
    encoder.configure({
        codec: "opus",
        sampleRate: config.sampleRate,
        numberOfChannels: config.numberOfChannels,
        bitrate: config.numberOfChannels * 32_000,
    });

    // Receive raw samples from worklet
    worklet.port.onmessage = ({ data }: { data: AudioFrame }) => {
        const frame = new AudioData({
            format: "f32-planar",
            sampleRate: worklet.context.sampleRate,
            numberOfFrames: data.channels[0].length,
            numberOfChannels: data.channels.length,
            timestamp: data.timestamp,
            data: joinedChannels,
        });

        encoder.encode(frame);
        frame.close();
    };
}
```

**Frame Grouping Strategy:**
- Frames are grouped into **MoQ Groups** for efficient transmission
- New group created every `maxLatency` milliseconds (default: 100ms)
- Groups allow subscribers to drop old data if they fall behind

### 4. Broadcast Publishing (`ref/moq/js/hang/src/publish/broadcast.ts`)

The `Broadcast` class manages the MoQ broadcast and handles track subscriptions:

```typescript
#run(effect: Effect) {
    const connection = effect.get(this.connection);
    const path = effect.get(this.path);

    // Create MoQ broadcast
    const broadcast = new Moq.Broadcast();

    // Publish broadcast to relay
    connection.publish(path, broadcast);

    // Handle incoming track requests
    effect.spawn(this.#runBroadcast.bind(this, broadcast, effect));
}

async #runBroadcast(broadcast: Moq.Broadcast, effect: Effect) {
    for (;;) {
        const request = await broadcast.requested();
        if (!request) break;

        // Route track requests to appropriate handlers
        switch (request.track.name) {
            case "catalog.json":
                this.#serveCatalog(request.track, effect);
                break;
            case "audio/data":
                this.audio.serve(request.track, effect);
                break;
            // ... other tracks
        }
    }
}
```

**Catalog Track (`catalog.json`):**
- Describes available tracks and their configurations
- Published as JSON containing audio/video rendition info
- Subscribers read this first to discover available tracks

## Subscriber Side: Voice Reception

### 1. Component Initialization (`ref/moq/js/hang/src/watch/element.ts`)

When `<hang-watch>` is connected to the DOM:

```typescript
connectedCallback() {
    this.active.set(new HangWatchInstance(this));
}
```

#### HangWatchInstance Constructor (`element.ts:214-256`)

**Step 1: Connection Setup** (`element.ts:216-219`)

```typescript
this.connection = new Moq.Connection.Reload({
    url: this.parent.signals.url,
    enabled: true,
});
```

**Step 2: Broadcast Subscription** (`element.ts:221-238`)

```typescript
this.broadcast = new Broadcast({
    connection: this.connection.established,
    path: this.parent.signals.path,
    enabled: true,
    reload: this.parent.signals.reload,
    audio: {
        captions: { enabled: this.parent.signals.captions },
        speaking: { enabled: this.parent.signals.captions },
        latency: this.parent.signals.latency,  // Jitter buffer size
    },
    video: {
        latency: this.parent.signals.latency,
    },
});
```

**Step 3: Audio Emitter** (`element.ts:251-255`)

```typescript
this.audio = new Audio.Emitter(this.broadcast.audio, {
    volume: this.parent.signals.volume,
    muted: this.parent.signals.muted,
    paused: this.parent.signals.paused,
});
```

- The `Emitter` connects the audio source to the speakers
- Provides volume control and mute functionality

### 2. Broadcast Consumption (`ref/moq/js/hang/src/watch/broadcast.ts`)

The watch-side `Broadcast` manages subscribing to a remote broadcast:

#### Announcement Watching (`broadcast.ts:77-111`)

```typescript
#runReload(effect: Effect): void {
    const conn = effect.get(this.connection);
    const path = effect.get(this.path);

    // Watch for broadcast announcements
    const announced = conn.announced(path);

    effect.spawn(async () => {
        for (;;) {
            const update = await announced.next();
            if (!update) break;

            // Activate broadcast when announced as live
            effect.set(this.#active, update.active, false);
        }
    });
}
```

**Announcement Mechanism:**
- The relay announces when broadcasts become available
- Clients can wait for broadcasts to go live before connecting
- Prevents wasting resources subscribing to offline broadcasts

#### Broadcast Consumption (`broadcast.ts:113-124`)

```typescript
#runBroadcast(effect: Effect): void {
    const conn = effect.get(this.connection);
    const path = effect.get(this.path);
    const active = effect.get(this.#active);

    if (!conn || !path || !active) return;

    // Consume the broadcast
    const broadcast = conn.consume(path);

    effect.set(this.#broadcast, broadcast);
}
```

#### Catalog Fetching (`broadcast.ts:126-157`)

```typescript
#runCatalog(effect: Effect): void {
    const broadcast = effect.get(this.#broadcast);
    if (!broadcast) return;

    this.status.set("loading");

    // Subscribe to catalog track
    const catalog = broadcast.subscribe("catalog.json", PRIORITY.catalog);

    effect.spawn(async () => {
        for (;;) {
            const update = await Catalog.fetch(catalog);
            if (!update) break;

            this.#catalog.set(update);
            this.status.set("live");
        }
    });
}
```

**Catalog Usage:**
- First track subscribed is always `catalog.json`
- Provides metadata about available audio/video tracks
- Updates dynamically if broadcaster changes configuration

### 3. Audio Source (`ref/moq/js/hang/src/watch/audio/source.ts`)

The `Source` class handles audio track subscription and decoding:

#### AudioWorklet Setup (`source.ts:94-139`)

```typescript
#runWorklet(effect: Effect): void {
    const config = effect.get(this.config);  // From catalog
    if (!config) return;

    // Create AudioContext
    const context = new AudioContext({
        latencyHint: "interactive",
        sampleRate: config.sampleRate,
    });

    // Register render worklet
    await context.audioWorklet.addModule(RenderWorklet);

    // Create worklet for audio playback
    const worklet = new AudioWorkletNode(context, "render", {
        channelCount: config.numberOfChannels,
        channelCountMode: "explicit",
    });

    // Initialize worklet with jitter buffer settings
    const init: Render.Init = {
        type: "init",
        rate: config.sampleRate,
        channels: config.numberOfChannels,
        latency: this.latency.peek(),  // Default: 100ms
    };
    worklet.port.postMessage(init);

    effect.set(this.#worklet, worklet);
}
```

**RenderWorklet Role:**
- Implements a **jitter buffer** for smooth playback
- Compensates for network timing variations
- Default latency: 100ms (configurable via `latency` signal)

#### Decoder Pipeline (`source.ts:153-207`)

```typescript
#runDecoder(effect: Effect): void {
    const catalog = effect.get(this.catalog);
    const broadcast = effect.get(this.broadcast);
    const config = effect.get(this.config);
    const active = effect.get(this.active);  // Track name

    // Subscribe to audio track
    const sub = broadcast.subscribe(active, catalog.priority);

    // Create frame consumer with jitter buffer
    const consumer = new Frame.Consumer(sub, {
        latency: Math.max(this.latency.peek() - 25, 0),  // Slightly less than render worklet
    });

    effect.spawn(async () => {
        // Create Opus decoder
        const decoder = new AudioDecoder({
            output: (data) => this.#emit(data),
            error: (error) => console.error(error),
        });

        // Configure decoder based on catalog
        decoder.configure({
            codec: config.codec,  // "opus"
            sampleRate: config.sampleRate,
            numberOfChannels: config.numberOfChannels,
        });

        // Decode frames as they arrive
        for (;;) {
            const frame = await consumer.decode();
            if (!frame) break;

            const chunk = new EncodedAudioChunk({
                type: frame.keyframe ? "key" : "delta",
                data: frame.data,
                timestamp: frame.timestamp,
            });

            decoder.decode(chunk);
        }
    });
}
```

**Frame Consumer:**
- Manages the **jitter buffer** and frame timing
- Drops old frames if playback falls behind
- Uses timestamp-based ordering to handle out-of-order delivery

#### Audio Emission (`source.ts:209-240`)

```typescript
#emit(sample: AudioData) {
    const worklet = this.#worklet.peek();
    if (!worklet) return;

    // Extract channel data
    const channelData: Float32Array[] = [];
    for (let channel = 0; channel < sample.numberOfChannels; channel++) {
        const data = new Float32Array(sample.numberOfFrames);
        sample.copyTo(data, {
            format: "f32-planar",
            planeIndex: channel
        });
        channelData.push(data);
    }

    // Send to render worklet for playback
    const msg: Render.Data = {
        type: "data",
        data: channelData,
        timestamp: sample.timestamp,
    };

    worklet.port.postMessage(
        msg,
        msg.data.map(data => data.buffer)  // Transfer ownership
    );

    sample.close();
}
```

### 4. Audio Emitter (`ref/moq/js/hang/src/watch/audio/emitter.ts`)

The `Emitter` class connects the audio source to the speakers:

```typescript
constructor(source: Source, props?: EmitterProps) {
    this.source = source;
    this.volume = Signal.from(props?.volume ?? 0.5);
    this.muted = Signal.from(props?.muted ?? false);
    this.paused = Signal.from(props?.paused ?? false);

    // Create gain node for volume control
    this.#signals.effect((effect) => {
        const root = effect.get(this.source.root);  // AudioWorkletNode
        if (!root) return;

        const gain = new GainNode(root.context, {
            gain: effect.get(this.volume)
        });
        root.connect(gain);

        effect.set(this.#gain, gain);

        // Only connect to speakers when enabled
        effect.effect(() => {
            const enabled = effect.get(this.source.enabled);
            if (!enabled) return;

            gain.connect(root.context.destination);  // speakers
            effect.cleanup(() => gain.disconnect());
        });
    });
}
```

**Audio Graph:**
```
RenderWorklet → GainNode → AudioContext.destination (speakers)
```

## Audio Pipeline Details

### Publisher Pipeline

```
Microphone
    ↓
getUserMedia()
    ↓
MediaStreamTrack
    ↓
MediaStreamAudioSourceNode
    ↓
GainNode (volume control)
    ↓
CaptureWorklet (audio processing thread)
    ↓ postMessage
AudioData (main thread)
    ↓
AudioEncoder (Opus)
    ↓
EncodedAudioChunk
    ↓
Frame.encode()
    ↓
MoQ.Group.writeFrame()
    ↓
MoQ.Track
    ↓
MoQ.Broadcast
    ↓
MoQ.Connection
    ↓
WebTransport/QUIC
    ↓
Network
```

### Subscriber Pipeline

```
Network
    ↓
WebTransport/QUIC
    ↓
MoQ.Connection
    ↓
MoQ.Broadcast.subscribe()
    ↓
MoQ.Track
    ↓
Frame.Consumer (jitter buffer)
    ↓
EncodedAudioChunk
    ↓
AudioDecoder (Opus)
    ↓
AudioData
    ↓ postMessage
RenderWorklet (audio processing thread, jitter buffer)
    ↓
GainNode (volume control)
    ↓
AudioContext.destination (speakers)
```

## Connection Management

### Connection Lifecycle

The `Moq.Connection.Reload` class provides automatic reconnection:

```typescript
// Simplified internal structure
class ConnectionReload {
    url: Signal<URL | undefined>;
    enabled: Signal<boolean>;
    established: Signal<Connection.Established | undefined>;
    status: Signal<"disconnected" | "connecting" | "connected">;

    #run(effect: Effect) {
        const url = effect.get(this.url);
        const enabled = effect.get(this.enabled);

        if (!url || !enabled) return;

        effect.spawn(async () => {
            for (;;) {
                this.status.set("connecting");

                try {
                    const conn = await Connection.connect(url);
                    this.status.set("connected");
                    this.established.set(conn);

                    // Wait for disconnection
                    await conn.closed();
                } catch (err) {
                    console.error("Connection failed:", err);
                }

                this.status.set("disconnected");
                this.established.set(undefined);

                // Exponential backoff before retry
                await delay(retryDelay);
            }
        });
    }
}
```

**Key Features:**
- **Automatic reconnection** with exponential backoff
- **Status signals** for UI updates
- **Reactive URL changes** - changing URL triggers reconnection

### WebTransport/QUIC

The underlying transport uses **WebTransport** (browser) or native **QUIC** (native apps):

- **Multiplexed streams**: Multiple tracks over single connection
- **Unreliable delivery**: Old frames can be dropped to reduce latency
- **Per-stream prioritization**: Audio can be prioritized over video
- **Built-in congestion control**: QUIC handles network congestion

## Signal-based Reactive System

The entire system uses **@kixelated/signals** for reactive state management:

### Effect System

```typescript
// Effects automatically track dependencies and cleanup
this.#signals.effect((effect) => {
    const enabled = effect.get(this.enabled);  // Track dependency
    if (!enabled) return;

    const source = effect.get(this.source);
    if (!source) return;

    // Do something with enabled source
    const context = new AudioContext();

    // Automatic cleanup when effect re-runs or is disposed
    effect.cleanup(() => context.close());
});
```

### Signal Updates

```typescript
// Setting a signal triggers dependent effects
this.enabled.set(true);

// Update based on previous value
this.volume.update((prev) => Math.min(prev + 0.1, 1.0));

// Peek without creating dependency
const currentVolume = this.volume.peek();
```

### Nested Effects

```typescript
this.#signals.effect((effect) => {
    const connection = effect.get(this.connection);
    if (!connection) return;

    // Nested effect - automatically cleaned up with parent
    effect.effect((effect) => {
        const path = effect.get(this.path);
        if (!path) return;

        const broadcast = connection.publish(path);
        effect.cleanup(() => broadcast.close());
    });
});
```

**Benefits:**
- **Automatic cleanup**: No manual resource management
- **Dependency tracking**: Effects re-run only when dependencies change
- **Composability**: Complex state machines built from simple effects
- **No memory leaks**: Cleanup guaranteed when components unmount

## Key Takeaways

1. **Declarative Components**: Web Components hide all complexity - just set attributes
2. **Reactive State**: Signals drive the entire system - changes propagate automatically
3. **Automatic Cleanup**: Effect system prevents resource leaks
4. **Low Latency**: Jitter buffers minimize latency while maintaining smooth playback
5. **Scalable**: Pub/sub model allows many-to-many voice chat
6. **Resilient**: Automatic reconnection and error handling
7. **Web Standards**: Uses WebCodecs, Web Audio API, and WebTransport

The architecture is **production-ready** and handles edge cases like network failures, device changes, and concurrent state updates gracefully through the reactive signal system.
