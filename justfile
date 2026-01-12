# Show available commands
default:
    @echo "Available commands:"
    @echo ""
    @echo "  just dev              - Start development server (builds WASM first)"
    @echo "  just build            - Build everything for production"
    @echo "  just build-wasm       - Build WASM module in release mode"
    @echo "  just build-wasm-dev   - Build WASM module in development mode"
    @echo "  just install          - Install dependencies"
    @echo "  just preview          - Preview production build"
    @echo "  just clean            - Clean build artifacts"
    @echo "  just test             - Run all tests (Rust + TypeScript)"
    @echo "  just check            - Check everything before deployment"
    @echo "  just start-live       - Initialize live event with default parameters"
    @echo "  just moq-relay        - Run local MoQ relay server"
    @echo "  just server           - Run game server (development mode)"
    @echo "  just server-prod      - Run game server (production mode)"
    @echo "  just gen-cert         - Generate self-signed certificate for development"
    @echo "  just test-client      - Run test client to connect to server"
    @echo "  just raycast-report   - Generate comprehensive raycast test report for all tracers"
    @echo "  just raytrace-report  - Test rendering output (color accuracy, tracer consistency)"
    @echo "  just test-raytracing  - Run complete raytracing test suite (raycast + raytrace)"
    @echo "  just editor           - Run native voxel editor (Bevy)"
    @echo "  just editor-release   - Run native voxel editor (optimized build)"
    @echo "  just proto            - Run physics prototype (Bevy)"
    @echo "  just game-run         - Run hot-reload app (Terminal 1)"
    @echo "  just game-watch       - Watch and auto-rebuild game (Terminal 2)"
    @echo "  just hot-reload       - Run hot-reload demo in tmux (auto-rebuild on save)"
    @echo "  just build-game       - Build game library once"
    @echo "  just xcube-setup      - Set up XCube server environment"
    @echo "  just xcube-server     - Start XCube inference server"
    @echo "  just xcube-generate   - Generate 3D object from text prompt"
    @echo "  just trellis-setup    - Set up Trellis.2 server environment"
    @echo "  just trellis-server   - Start Trellis.2 inference server"
    @echo ""

# Build WASM module in development mode
build-wasm-dev:
    cd crates/core && wasm-pack build --dev --target web --out-dir ../../packages/wasm-core --out-name core
    cd crates/world && wasm-pack build --dev --target web --out-dir ../../packages/wasm-world --out-name crossworld-world
    cd crates/cube && wasm-pack build --dev --target web --out-dir ../../packages/wasm-cube -- --features wasm
    cd crates/physics && wasm-pack build --dev --target web --out-dir ../../packages/wasm-physics --out-name crossworld_physics -- --features wasm

# Build WASM module in release mode
build-wasm:
    cd crates/core && wasm-pack build --target web --out-dir ../../packages/wasm-core --out-name core
    cd crates/world && wasm-pack build --target web --out-dir ../../packages/wasm-world --out-name crossworld-world
    cd crates/cube && wasm-pack build --target web --out-dir ../../packages/wasm-cube -- --features wasm
    cd crates/physics && wasm-pack build --target web --out-dir ../../packages/wasm-physics --out-name crossworld_physics -- --features wasm

# Start development server (builds WASM first)
dev: build-wasm-dev
    cd packages/app && bun run dev

# Build everything for production
build: build-wasm
    cd packages/app && bun run build

# Clean build artifacts
clean:
    rm -rf packages/wasm-core packages/wasm-world packages/wasm-cube packages/wasm-physics
    cd packages/app && rm -rf dist node_modules/.vite

# Install dependencies
install:
    cd packages/app && bun install

# Preview production build
preview:
    cd packages/app && bun run preview

# Initialize Crossworld Nostr live event
start-live:
    cd crates/worldtool && cargo run -- init-live --streaming https://moq.justinmoon.com/anon

# Run all tests
test:
    @echo "Running Rust tests..."
    cargo test --workspace
    @echo "\nRunning TypeScript type check..."
    cd packages/app && bun run build

# Check everything before deployment
check:
    @echo "=== Checking Rust code ==="
    cargo check --workspace
    cargo clippy --workspace -- -D warnings
    cargo fmt --check
    @echo "\n=== Building WASM ==="
    just build-wasm
    @echo "\n=== Building TypeScript app ==="
    cd packages/app && bun run build
    @echo "\n✅ All checks passed! Ready for deployment."

# Run local MoQ relay server
moq-relay:
    @if [ ! -f moq-relay/rs/Cargo.toml ]; then \
        echo "MoQ relay not found. Cloning from GitHub..."; \
        rm -rf moq-relay; \
        git clone https://github.com/kixelated/moq.git moq-relay; \
        echo "MoQ relay cloned successfully"; \
    fi
    @echo "Starting MoQ relay server on localhost:4443..."
    @echo "Certificate will be auto-generated for localhost"
    @echo "Press Ctrl+C to stop"
    @echo ""
    cd moq-relay/rs && cargo run --release --bin moq-relay -- moq-relay/cfg/dev.toml

# Generate self-signed certificate for development
gen-cert:
    @echo "Generating self-signed certificate for localhost..."
    openssl req -x509 -newkey rsa:4096 -keyout localhost-key.pem -out localhost.pem -days 365 -nodes -subj '/CN=localhost'
    @echo "✅ Certificate generated: localhost.pem"
    @echo "✅ Private key generated: localhost-key.pem"

# Run game server in development mode
server:
    @if [ ! -f localhost.pem ] || [ ! -f localhost-key.pem ]; then \
        echo "Certificate not found. Generating..."; \
        just gen-cert; \
    fi
    cargo run --bin server -- --log-level debug

# Run game server in production mode
server-prod:
    cargo run --release --bin server -- \
        --bind 0.0.0.0:4433 \
        --max-players 100 \
        --interest-radius 200 \
        --validate-positions \
        --max-move-speed 20.0 \
        --enable-discovery \
        --relays wss://relay.damus.io,wss://nos.lol

# Run test client
test-client:
    cargo run --bin test-client

# Test core raycast algorithm (octree traversal logic)
raycast-report:
    @echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    @echo "║                    RAYCAST ALGORITHM TESTS (cube package)                    ║"
    @echo "║  Tests: Octree traversal, hit detection, normals, positions, edge cases     ║"
    @echo "╚══════════════════════════════════════════════════════════════════════════════╝"
    @echo ""
    cargo test --test raycast_test_report -- --nocapture --test-threads=1

# Test rendering output across all tracers (visual correctness)
raytrace-report:
    @echo "╔══════════════════════════════════════════════════════════════════════════════╗"
    @echo "║                 RAYTRACE RENDERING TESTS (renderer package)                  ║"
    @echo "║  Tests: Color accuracy, tracer consistency, visual output validation         ║"
    @echo "╚══════════════════════════════════════════════════════════════════════════════╝"
    @echo ""
    @echo "Running color accuracy tests..."
    cargo test --package renderer --test color_accuracy_tests -- --nocapture
    @echo ""
    @echo "Running tracer consistency tests..."
    cargo test --package renderer --test tracer_consistency_tests -- --nocapture
    @echo ""
    @echo "Running combined render tests..."
    cargo test --package renderer --test combined_render_test -- --nocapture
    @echo ""
    @echo "✓ All raytrace rendering tests completed"

# Run all raycast and raytrace tests
test-raytracing:
    @echo "Running complete raytracing test suite..."
    @echo ""
    just raycast-report
    @echo ""
    just raytrace-report

# Run native voxel editor (Bevy) in development mode
editor:
    cargo run --bin editor

# Run native voxel editor (Bevy) in release mode
editor-release:
    cargo run --release --bin editor

# Run physics prototype (Bevy)
proto:
    cargo run --bin proto

# Run proto-gl physics viewer (OpenGL, lightweight)
proto-gl:
    cargo run --bin proto-gl

# Build game library for hot-reload
build-game:
    cargo build --package game

# Run the hot-reload app (Terminal 1)
game-run:
    @echo "🎮 Starting hot-reload app..."
    @echo "Open another terminal and run: just game-watch"
    @echo ""
    cargo build --package game
    cargo run --bin game

# Watch and auto-rebuild game on changes (Terminal 2)
game-watch:
    @echo "👀 Watching crates/game/ for changes..."
    @echo "Any changes will trigger automatic rebuild"
    @echo "Press Ctrl+C to stop"
    @echo ""
    cargo watch -x 'build --package game' -w crates/game

# Run hot-reload demo (rotating cube) - requires tmux
hot-reload:
    #!/usr/bin/env bash
    set -euo pipefail

    # Check if tmux is installed
    if ! command -v tmux &> /dev/null; then
        echo "❌ tmux is not installed. Install it or use manual method:"
        echo ""
        echo "Terminal 1: cargo run --bin app"
        echo "Terminal 2: cargo watch -x 'build --package game' -w crates/game"
        exit 1
    fi

    # Check if cargo-watch is installed
    if ! command -v cargo-watch &> /dev/null; then
        echo "📦 Installing cargo-watch..."
        cargo install cargo-watch
    fi

    # Build game first
    echo "🔨 Building game library..."
    cargo build --package game

    # Kill existing session if it exists
    tmux kill-session -t hot-reload 2>/dev/null || true

    # Create new tmux session
    echo "🚀 Starting hot-reload in tmux session 'hot-reload'"
    echo ""
    echo "Controls:"
    echo "  - Ctrl+B, then arrow keys to switch panes"
    echo "  - Ctrl+B, then D to detach (keeps running)"
    echo "  - Edit crates/game/src/lib.rs to see hot-reload!"
    echo "  - Type 'exit' in both panes to quit"
    echo ""
    sleep 2

    # Create tmux session with two panes
    tmux new-session -d -s hot-reload -n hot-reload
    tmux split-window -h -t hot-reload:0

    # Left pane: run app
    tmux send-keys -t hot-reload:0.0 'cargo run --bin app' C-m

    # Right pane: watch and rebuild
    tmux send-keys -t hot-reload:0.1 'echo "Watching for changes in crates/game/..."' C-m
    tmux send-keys -t hot-reload:0.1 'cargo watch -x "build --package game" -w crates/game' C-m

    # Attach to session
    tmux attach-session -t hot-reload

# Set up XCube server environment (clone repos, install deps)
xcube-setup *ARGS:
    @echo "Setting up XCube server environment..."
    crates/xcube/server/setup.sh {{ARGS}}

# Start XCube inference server
xcube-server:
    @echo "Starting XCube inference server on http://0.0.0.0:8000..."
    @echo "API docs: http://localhost:8000/docs"
    @echo ""
    cd crates/xcube/server && uv run server.py

# Generate 3D object from text prompt using XCube
xcube-generate PROMPT:
    @echo "Generating 3D object: '{{PROMPT}}'"
    @curl -s -X POST http://localhost:8000/generate \
        -H "Content-Type: application/json" \
        -d '{"prompt": "{{PROMPT}}", "ddim_steps": 50, "guidance_scale": 7.5, "use_fine": false}' \
        | uv run python -c "import sys, json; r = json.load(sys.stdin); print(f'Generated {len(r[\"coarse_xyz\"])} coarse points')"

# Set up Trellis.2 inference server environment
trellis-setup *ARGS:
    @echo "Setting up Trellis.2 server environment..."
    crates/trellis/server/setup.sh {{ARGS}}

# Start Trellis.2 inference server
trellis-server:
    @echo "Starting Trellis.2 inference server on http://0.0.0.0:8001..."
    @echo "API docs: http://localhost:8001/docs"
    @echo ""
    conda run -n trellis --no-capture-output python crates/trellis/server/server.py

# Run hot-reload demo (manual two-terminal method)
hot-reload-manual:
    @echo "🔧 Manual Hot-Reload Setup"
    @echo ""
    @echo "Terminal 1 (this one): Run the app"
    @echo "Terminal 2 (open another): Auto-rebuild on changes"
    @echo ""
    @echo "After both are running, edit crates/game/src/lib.rs"
    @echo "and save - you'll see hot-reload happen!"
    @echo ""
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo ""
    @echo "Terminal 2 command (copy and run in another terminal):"
    @echo "  cargo watch -x 'build --package game' -w crates/game"
    @echo ""
    @echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    @echo ""
    cargo build --package game
    @echo "Press Enter to start the app..."
    @read -p ""
    cargo run --bin app
