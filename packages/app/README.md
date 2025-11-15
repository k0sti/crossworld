# Crossworld Main Web App

## Overview

The main React application for Crossworld, providing the 3D voxel world interface with Nostr integration and voice chat.

## Key Features

- **3D Voxel World** - WebGL rendering via Three.js
- **Nostr Integration** - Decentralized identity and chat via Applesauce
- **Voice Chat** - Spatial audio via MoQ (Media over QUIC)
- **Physics** - Real-time physics simulation via Rapier3D (WASM)
- **Avatar System** - Voxel and GLB avatar support

## Architecture

### Dependencies

**WASM Modules** (generated from Rust crates):
- `wasm-world` - World simulation and state management
- `wasm-cube` - Voxel octree engine
- `wasm-physics` - Physics simulation with Rapier3D

**Nostr**:
- Applesauce SDK for Nostr connectivity
- Event-based state synchronization

**Voice Chat**:
- MoQ (Media over QUIC) via @kixelated packages
- WebTransport API for network connectivity

**Rendering**:
- Three.js for 3D graphics
- Custom voxel mesh generation

## Development

### Quick Start

```bash
# Install dependencies (from project root)
bun install

# Build WASM modules (required first time)
just build-wasm-dev

# Start dev server
cd packages/app
bun run dev
```

### Build Commands

```bash
# Development server
bun run dev

# Production build
bun run build

# Preview production build
bun run preview

# Type checking
bun run typecheck
```

### Project Structure

```
packages/app/
├── src/
│   ├── components/     # React components
│   ├── renderer/       # Three.js rendering
│   ├── services/       # Business logic
│   ├── voice/          # MoQ voice chat
│   ├── utils/          # Utilities
│   └── App.tsx         # Main app component
├── public/             # Static assets
│   └── assets/         # Models, worlds, etc.
└── vite.config.ts      # Vite configuration
```

## Documentation

For complete documentation, see [../../doc/README.md](../../doc/README.md)

**Key Documentation**:
- [Architecture Overview](../../doc/architecture/overview.md)
- [Build System](../../doc/reference/build-system.md)
- [Avatar System](../../doc/features/avatar-system.md)
- [Voice Chat](../../doc/features/voice-chat.md)

## Requirements

- **Browser**: Chrome 97+ or Edge 97+ (WebTransport support required)
- **HTTPS**: Required for SharedArrayBuffer and WebTransport
- **COOP/COEP Headers**: Set automatically by Vite config
