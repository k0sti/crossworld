# Crossworld Server

This crate implements the skeleton described in `docs/server.md`.  It provides:

- **Protocol definitions** (`protocol/`) shared between the native server and future WASM clients.
- **Authentication** (`auth/`) using Nostr public keys with optional signature verification (enable the `nostr` feature).
- **World management** (`world/`) that layers caching and persistence on top of the existing `cube` crate.
- **Networking glue** (`network/`) with a WebTransport-facing server façade.
- **Binary entry point** (`src/main.rs`) that wires configuration, storage, and the async runtime.

## Running

```bash
cargo run -p crossworld-server
```

Environment variables (all optional) control paths and auth behaviour:

| Variable | Description | Default |
| --- | --- | --- |
| `CROSSWORLD_BIND` | Bind address | `0.0.0.0:4443` |
| `CROSSWORLD_PUBLIC_URL` | URL used in handshake signatures | `https://localhost:4443` |
| `CROSSWORLD_DATA` | Base directory for assets | `./data` |
| `CROSSWORLD_WORLD_FILE` | Binary world snapshot | `$CROSSWORLD_DATA/world.bin` |
| `CROSSWORLD_EDIT_LOG` | Edit log file | `$CROSSWORLD_DATA/world.edits` |
| `CROSSWORLD_ADMIN_NPUBS` | Comma-separated admin npubs | _empty_ |
| `CROSSWORLD_USER_NPUBS` | Comma-separated allowed npubs | _all verified users_ |

Enable WebTransport integration (stub implementation) and signature verification via:

```
cargo run -p crossworld-server --features "webtransport nostr"
```
