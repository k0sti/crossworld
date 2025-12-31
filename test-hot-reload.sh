#!/bin/bash

echo "=== Hot Reload Test ==="
echo ""
echo "Step 1: Building game library..."
cargo build --package game 2>&1 | tail -3

echo ""
echo "Step 2: Starting app in background..."
cargo run --bin app > /tmp/app-output.log 2>&1 &
APP_PID=$!
echo "App running with PID: $APP_PID"

sleep 2

echo ""
echo "Step 3: Checking initial output..."
grep -E "\[Game\]|\[AppRuntime\]" /tmp/app-output.log | tail -5

echo ""
echo "Step 4: Modifying source file (changing rotation speed)..."
sed -i 's/45\.0_f32\.to_radians()/90.0_f32.to_radians()/' crates/game/src/lib.rs
echo "Modified rotation speed from 45° to 90° per second"

echo ""
echo "Step 5: Rebuilding game library..."
cargo build --package game 2>&1 | tail -3

sleep 1

echo ""
echo "Step 6: Checking for hot-reload detection..."
if grep -q "Hot-reload triggered" /tmp/app-output.log; then
    echo "✓ Hot-reload was detected!"
else
    echo "✗ Hot-reload NOT detected"
    echo "Recent log output:"
    tail -10 /tmp/app-output.log
fi

echo ""
echo "Step 7: Reverting source change..."
sed -i 's/90\.0_f32\.to_radians()/45.0_f32.to_radians()/' crates/game/src/lib.rs
echo "Reverted rotation speed back to 45°"

echo ""
echo "Step 8: Stopping app..."
kill $APP_PID 2>/dev/null
wait $APP_PID 2>/dev/null

echo ""
echo "=== Test Complete ==="
