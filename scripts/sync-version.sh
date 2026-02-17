#!/bin/bash
# sync-version.sh - Update all Arcium version references across examples
# Usage: ./scripts/sync-version.sh <version>
# Example: ./scripts/sync-version.sh 0.6.6

set -eo pipefail

VERSION=${1:-0.6.6}
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Validate version format
if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Version must be in format X.Y.Z (e.g., 0.6.6)"
  exit 1
fi

# Check for jq
if ! command -v jq &> /dev/null; then
  echo "Error: jq is required but not installed"
  exit 1
fi

# Check arcium CLI version
ARCIUM_VERSION=$(arcium --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
if [ -z "$ARCIUM_VERSION" ]; then
  echo "Error: arcium CLI not found"
  exit 1
fi

if [ "$ARCIUM_VERSION" != "$VERSION" ]; then
  echo "Error: arcium CLI version ($ARCIUM_VERSION) doesn't match target ($VERSION)"
  echo "Install matching version: cargo install arcium-cli --version $VERSION"
  exit 1
fi

echo "Syncing all examples to Arcium version: $VERSION"
echo ""

cd "$REPO_ROOT"

# Find all example directories (those with package.json, excluding node_modules)
EXAMPLES=$(find "$REPO_ROOT" -name "package.json" -not -path "*/node_modules/*" | xargs -I{} dirname {} | sort)

for EXAMPLE in $EXAMPLES; do
  EXAMPLE_NAME=$(echo "$EXAMPLE" | sed "s|$REPO_ROOT/||")
  echo "Updating $EXAMPLE_NAME..."

  # 1. Update package.json - @arcium-hq/client
  if [ -f "$EXAMPLE/package.json" ]; then
    jq ".dependencies[\"@arcium-hq/client\"] = \"$VERSION\"" "$EXAMPLE/package.json" > "$EXAMPLE/package.json.tmp"
    mv "$EXAMPLE/package.json.tmp" "$EXAMPLE/package.json"
  fi

  # 2. Update programs/*/Cargo.toml
  PROGRAM_CARGO=$(find "$EXAMPLE/programs" -name "Cargo.toml" 2>/dev/null | head -1)
  if [ -n "$PROGRAM_CARGO" ] && [ -f "$PROGRAM_CARGO" ]; then
    # Handle arcium-client with both key orderings:
    # - { version = "X.Y.Z", default-features = false }
    # - { default-features = false, version = "X.Y.Z" }
    sed -i '' -E "s/(arcium-client = \{[^}]*version = \")[0-9]+\.[0-9]+\.[0-9]+/\1$VERSION/" "$PROGRAM_CARGO"
    sed -i '' "s/arcium-macros = \"[^\"]*\"/arcium-macros = \"$VERSION\"/" "$PROGRAM_CARGO"
    sed -i '' "s/arcium-anchor = \"[^\"]*\"/arcium-anchor = \"$VERSION\"/" "$PROGRAM_CARGO"
  fi

  # 3. Update encrypted-ixs/Cargo.toml - arcis
  if [ -f "$EXAMPLE/encrypted-ixs/Cargo.toml" ]; then
    sed -i '' "s/arcis = \"[^\"]*\"/arcis = \"$VERSION\"/" "$EXAMPLE/encrypted-ixs/Cargo.toml"
  fi
done

echo ""
echo "Version updates complete. Now regenerating yarn.lock files..."
echo ""

# Regenerate yarn.lock files
for EXAMPLE in $EXAMPLES; do
  EXAMPLE_NAME=$(echo "$EXAMPLE" | sed "s|$REPO_ROOT/||")
  echo "Regenerating yarn.lock for $EXAMPLE_NAME..."
  rm -f "$EXAMPLE/yarn.lock"
  (cd "$EXAMPLE" && yarn install --silent 2>/dev/null || yarn install)
done

echo ""
echo "Running arcium build in each example..."
echo ""

for EXAMPLE in $EXAMPLES; do
  EXAMPLE_NAME=$(echo "$EXAMPLE" | sed "s|$REPO_ROOT/||")
  echo "Building $EXAMPLE_NAME..."
  (cd "$EXAMPLE" && arcium build)
done

echo ""
echo "Done. Updated all examples to version $VERSION"
echo ""
echo "Changes:"
git diff --stat 2>/dev/null || echo "(not in git repo or no changes)"
