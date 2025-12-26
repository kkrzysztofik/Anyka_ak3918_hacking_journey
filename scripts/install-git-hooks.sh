#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

HOOKS_DIR=".githooks"

mkdir -p "$HOOKS_DIR"
chmod +x "$HOOKS_DIR" || true

# Ensure our pre-commit is executable
if [[ -f "$HOOKS_DIR/pre-commit" ]]; then
  chmod +x "$HOOKS_DIR/pre-commit" || true
fi

git config core.hooksPath "$HOOKS_DIR"

echo "Installed git hooks path to $HOOKS_DIR. To revert, run: git config --unset core.hooksPath"
