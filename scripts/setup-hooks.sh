#!/usr/bin/env bash
# setup-hooks.sh — Install git hooks for the A2X project
#
# Creates a pre-commit hook that runs:
#   1. cargo fmt --check   (formatting)
#   2. cargo clippy         (linting)
#   3. cargo test           (tests)
#
# Usage:
#   ./scripts/setup-hooks.sh
#
# Per PLAN Appendix C.

set -euo pipefail

HOOKS_DIR="$(git rev-parse --git-dir)/hooks"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "==> Installing A2X git hooks..."

# Create pre-commit hook
cat > "$HOOKS_DIR/pre-commit" <<'HOOK'
#!/usr/bin/env bash
set -euo pipefail

echo "==> Running cargo fmt --check..."
cargo fmt --check || {
    echo "ERROR: Code is not formatted. Run 'cargo fmt' to fix."
    exit 1
}

echo "==> Running cargo clippy..."
cargo clippy --workspace -- -D warnings || {
    echo "ERROR: Clippy found issues. Please fix them before committing."
    exit 1
}

echo "==> Running cargo test..."
cargo test --workspace || {
    echo "ERROR: Tests failed. Please fix them before committing."
    exit 1
}

echo "==> All checks passed!"
HOOK

chmod +x "$HOOKS_DIR/pre-commit"

echo "==> Git hooks installed successfully!"
echo "    Pre-commit hook: $HOOKS_DIR/pre-commit"
