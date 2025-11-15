# Crossworld Documentation

Welcome to the Crossworld documentation! This guide will help you navigate the project's documentation and get started with development.

## Getting Started

New to Crossworld? Start here:

1. **[QUICKSTART.md](QUICKSTART.md)** - Voice chat setup and first-run guide
2. **[EDITOR_SETUP.md](EDITOR_SETUP.md)** - Configure your development environment
3. **[CONVENTIONS.md](CONVENTIONS.md)** - Coding standards and project conventions

## Architecture

Understand how the system works:

- **[overview.md](architecture/overview.md)** - High-level system architecture and components
- **[voxel-system.md](architecture/voxel-system.md)** - Voxel octree engine and CSM format
- **[physics.md](architecture/physics.md)** - Physics integration with Rapier3D
- **[raycast.md](architecture/raycast.md)** - Ray-octree intersection system
- **[rendering.md](architecture/rendering.md)** - Rendering pipeline and Three.js integration

## Features

Learn about specific features:

- **[avatar-system.md](features/avatar-system.md)** - Avatar design, physics, and animation
- **[voice-chat.md](features/voice-chat.md)** - MoQ-based spatial voice chat setup and debugging
- **[nostr-integration.md](features/nostr-integration.md)** - Nostr identity, discovery, and worldtool CLI

## Reference

Technical references and specifications:

- **[project-structure.md](reference/project-structure.md)** - Repository organization and crate layout
- **[build-system.md](reference/build-system.md)** - Build process, justfile commands, and WASM compilation
- **[materials.md](reference/materials.md)** - Material system and shader specifications

## Change Proposals

For architectural decisions and change proposals, see the [OpenSpec system](../openspec/):

- **[openspec/project.md](../openspec/project.md)** - Project context and tech stack
- **[openspec/AGENTS.md](../openspec/AGENTS.md)** - OpenSpec workflow for AI assistants
- **[openspec/changes/](../openspec/changes/)** - Active change proposals

## External Resources

- **[Nostr NIPs](https://github.com/nostr-protocol/nips)** - Nostr protocol specifications
- **[MoQ (Media over QUIC)](https://github.com/kixelated/moq)** - Voice chat protocol
- **[Rapier3D](https://rapier.rs/)** - Physics engine documentation
- **[Three.js](https://threejs.org/docs/)** - 3D rendering library
- **[Applesauce](https://github.com/coracle-social/applesauce)** - Nostr SDK

## Project Overview

Crossworld is a Nostr-based metaverse prototyping platform featuring:
- **Voxel World**: Octree-based 3D voxel engine compiled to WebAssembly
- **Physics**: Real-time physics simulation with character controllers
- **Voice Chat**: Spatial audio via Media over QUIC (MoQ)
- **Identity**: Decentralized identity using Nostr public keys
- **Rendering**: Hybrid CPU/GPU rendering with Three.js

The project uses a **Rust → WASM → TypeScript** architecture:
- **Rust crates** (`crates/`) - Core systems compiled to WebAssembly
- **TypeScript packages** (`packages/`) - UI and rendering
- **Bun** - Package manager and runtime (per project conventions)
