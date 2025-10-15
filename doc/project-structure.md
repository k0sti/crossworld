# Project Structure

This document describes the current organization of the CrossWorld project.

## Directory Layout

```
crossworld/
├── ref/                   # Reference material for implementation aid
│                          # Not part of the project and not stored in github
├── crates/
│   ├── server/            # Game server
│   └── world/             # Main Rust client crate compiled into WASM
├── packages/
│   ├── app/               # Main web application
│   └── world/             # Generated Rust WASM output
└── doc/                   # Project documentation
```

## Component Details

### `ref/`
Reference materials, documentation, and implementation aids that support development but are not part of the shipped project. This directory is excluded from version control.

### `crates/world/`
The core Rust implementation that handles:
- WebTransport networking
- Binary protocol serialization
- WASM exports for JavaScript interop
- Game state management

Compiled to WebAssembly using `wasm-pack --target web`.

### `packages/app/`
The main web application that provides:
- User interface
- Nostr identity integration (via Applesauce)
- Game rendering and client logic
- Discovery and connection management

### `packages/world/`
Auto-generated output from `wasm-pack` build of `crates/world`. Contains:
- WASM binary
- JavaScript bindings
- TypeScript definitions

This directory should not be edited manually as it's regenerated on each build.

## Build Flow

1. `crates/world/` is compiled to WASM → outputs to `packages/world/`
2. `packages/app/` imports from `packages/world/`
3. Final application bundles everything together

See [design-master.md](./design-master.md) for detailed technical design.
