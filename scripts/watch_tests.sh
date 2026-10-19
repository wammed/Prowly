#!/usr/bin/env bash
# Watch src/ and tests/ directories and automatically execute cargo test on every file change.
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

echo "👀 Watching for code changes in src/, tests/, Cargo.toml..."
echo "Press Ctrl+C to stop."

# Run tests once on startup
cargo test --all-targets --all-features

while true; do
    inotifywait -q -r -e modify,create,delete,move \
        --exclude '(target|\.git|\.system_generated)' \
        src tests Cargo.toml Cargo.lock > /dev/null 2>&1 || true

    # Debounce rapid saves
    sleep 0.2

    echo ""
    echo "🔄 Change detected at $(date +%T). Running tests..."
    if cargo test --all-targets --all-features; then
        echo "✅ Tests passed!"
    else
        echo "❌ Tests failed!"
    fi
done
