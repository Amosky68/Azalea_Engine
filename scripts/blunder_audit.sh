#!/usr/bin/env bash
# Phase 0 — audit de blunders de course sur les parties d'Azalea.
#
#   scripts/blunder_audit.sh [options de race_bench]
#
# Variables : DEPTH (défaut 6), FILE (défaut export Azalea v2).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEPTH="${DEPTH:-6}"
FILE="${FILE:-$ROOT/Plays_dataset/traversal-arena-azalea-v2.json}"

cargo run -p azalea_core --release --bin race_bench -- \
  --file "$FILE" --depth "$DEPTH" "$@"
