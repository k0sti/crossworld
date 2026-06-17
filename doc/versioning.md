# Versioning

Crossworld uses [Semantic Versioning](https://semver.org/) (`MAJOR.MINOR.PATCH`) for the **product** release version.

## Single source of truth

The canonical version lives in two synced places (updated together by `just bump-version`):

| Location | Purpose |
|----------|---------|
| [`VERSION`](../VERSION) | Human-readable file for scripts and Vite |
| [`Cargo.toml`](../Cargo.toml) `[workspace.package].version` | Rust workspace crates via `version.workspace = true` |

npm `package.json` files under `packages/` are kept in sync for consistency.

## Workspace policy

- **One product version** for the whole monorepo (Rust + web client).
- All workspace crates use `version.workspace = true` in their `Cargo.toml`.
- **Pre-1.0**: treat `0.MINOR.PATCH` as unstable; breaking changes may land in minor releases.
- **Patch** (`0.1.0` → `0.1.1`): bug fixes, no API breaks.
- **Minor** (`0.1.0` → `0.2.0`): new features, backward-compatible when possible.
- **Major** (`0.x` → `1.0.0` or `1.x` → `2.0.0`): breaking changes to public APIs or wire protocols.

## Not the same as format versions

Binary and protocol formats have their own version fields (e.g. BCF header byte `0x01`, network message versioning). Those are independent of the product SEMVER.

## Accessing the version in code

### Rust

```rust
use core::version::CROSSWORLD_VERSION;
// or, in any workspace crate:
env!("CARGO_PKG_VERSION")
```

CLI binaries with clap expose `--version` / `-V` automatically.

### TypeScript

```typescript
import { CROSSWORLD_VERSION } from '@crossworld/common'
```

Vite injects `__CROSSWORLD_VERSION__` from the root `VERSION` file at build time.

### WASM

```javascript
import init, { crossworldVersion } from 'core'
await init()
crossworldVersion() // "0.1.0"
```

## Bumping the version

```bash
just bump-version 0.2.0
```

This updates `VERSION`, `[workspace.package].version`, and all `package.json` version fields.

## Where the version is shown

- **Server**: startup log and Nostr discovery `version` field
- **CLI**: `--version` on clap-based binaries; `game --version`
- **Web**: About / Info panel
- **Nostr presence**: `client_version` tag on avatar state events (web client)
