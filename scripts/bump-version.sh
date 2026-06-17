#!/usr/bin/env bash
# Bump Crossworld product version across Rust workspace and npm packages.
# Usage: scripts/bump-version.sh <semver>
set -euo pipefail

NEW_VERSION="${1:?Usage: bump-version.sh <semver> (e.g. 0.2.0)}"

if ! [[ "$NEW_VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
    echo "Error: '$NEW_VERSION' is not valid semver (expected MAJOR.MINOR.PATCH)" >&2
    exit 1
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

CURRENT="$(tr -d '[:space:]' < VERSION)"
if [[ "$CURRENT" == "$NEW_VERSION" ]]; then
    echo "Version is already $NEW_VERSION"
    exit 0
fi

echo "$NEW_VERSION" > VERSION

if ! grep -q '^\[workspace\.package\]' Cargo.toml; then
    echo "Error: [workspace.package] not found in Cargo.toml" >&2
    exit 1
fi

sed -i "/^\[workspace\.package\]/,/^\[/{s/^version = .*/version = \"$NEW_VERSION\"/}" Cargo.toml

for pkg in package.json packages/app/package.json packages/common/package.json packages/editor/package.json packages/proto/package.json; do
    if [[ -f "$pkg" ]]; then
        if grep -q '"version"' "$pkg"; then
            sed -i "s/\"version\": \"[^\"]*\"/\"version\": \"$NEW_VERSION\"/" "$pkg"
        else
            sed -i "s/\"private\": true,/\"version\": \"$NEW_VERSION\",\n  \"private\": true,/" "$pkg"
        fi
    fi
done

echo "Bumped Crossworld version: $CURRENT -> $NEW_VERSION"
echo "  VERSION"
echo "  Cargo.toml [workspace.package]"
echo "  package.json (root + packages/*)"
