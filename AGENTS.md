# Repository Guidelines

## Project Structure & Module Organization
Rust gameplay, physics, renderer, and tooling crates live under `crates/`, each exposing a focused lib plus inline tests. The Bun/Vite client runs from `packages/app/src`, with shared UI utilities in `packages/common` and editor helpers in `packages/editor`. `packages/wasm-*` contains generated bundles—regenerate via `just build-wasm*` rather than editing.

## Build, Test, and Development Commands
- `just dev` — builds WASM in dev mode, then runs `bun run dev` in `packages/app` for hot reload.
- `just build` — release WASM plus `bun run build` to emit a production Vite bundle.
- `just test` — executes `cargo test --workspace` and a TypeScript type-check (`bun run build`).
- `just check` — runs `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, rebuilds WASM, and compiles the app.
- Targeted loops: `cargo test crates/world` for a single crate or `bun run preview` after `just build` to inspect the optimized bundle.

## Coding Style & Naming Conventions
Rust code targets stable toolchains: run `cargo fmt`, enforce `cargo clippy --workspace -- -D warnings`, and keep files snake_case with `mod tests` colocated at the bottom. TypeScript/React sticks to 2-space indentation, ES modules, PascalCase components (e.g., `AvatarPanel.tsx`), and camelCase hooks/utilities. Keep Vite path aliases in sync between `tsconfig.json` and `vite.config.ts`.

## Testing Guidelines
Place Rust unit and integration tests next to the code they cover (e.g., `crates/physics/src/lib.rs` → `mod tests` or `crates/physics/tests/`). When altering FFI or serialization, add fixture-based smoke tests to keep `wasm-pack` output stable. The frontend relies on TypeScript’s compiler; if a component gains stateful logic, add a Vitest case under `packages/app/src/__tests__` and wire it to `bun test`. Always finish with `just test` before opening a PR.

## Commit & Pull Request Guidelines
Commits use short, imperative subjects (`Remove excessive logging`, `Centralize world panel settings`) and focus on a single concern. Each PR should include a concise description, verification evidence (output from `just test` or `just check`), linked issue/epic, and screenshots or clips whenever `packages/app` visuals change. Draft PRs are welcome, but switch to “Ready” only after CI-equivalent commands pass locally.

## Configuration & Security Notes
Store relay URLs and experimental flags in `.env.local` files ignored by git. Commands like `just start-live` or `just moq-relay` talk to external services; keep certificates and configs under `moq-relay/rs` and never hard-code credentials. Generated assets such as `packages/app/dist` or `packages/wasm-*` stay out of commits—they are rebuilt in CI.
