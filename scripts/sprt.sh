#!/usr/bin/env bash
# Phase 0 — banc SPRT autour de l'arène locale.
#
#   scripts/sprt.sh [candidat.wasm] [baseline.wasm|1..5] [parties] [threads]
#
# Défauts :
#   candidat = target/wasm32-unknown-unknown/release/azalea_bot.wasm
#   baseline = target/wasm32-unknown-unknown/release/azalea_arena_1975.wasm
#   parties  = 400, threads = 8
#
# Codes de sortie : 0 accepté, 1 rejeté, 2 indécis, 3 erreur.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CAND="${1:-$ROOT/target/wasm32-unknown-unknown/release/azalea_bot.wasm}"
BASE="${2:-$ROOT/target/wasm32-unknown-unknown/release/azalea_arena_1975.wasm}"
GAMES="${3:-400}"
THREADS="${4:-8}"

if [[ ! -f "$CAND" ]]; then
  echo "candidat absent : $CAND" >&2
  echo "construis-le : cargo build -p azalea_bot --release --target wasm32-unknown-unknown" >&2
  exit 3
fi

if [[ "$BASE" =~ ^[1-5]$ ]]; then
  BASE_ARG="$BASE"
else
  if [[ ! -f "$BASE" ]]; then
    echo "baseline absente : $BASE" >&2
    exit 3
  fi
  BASE_ARG="/w/${BASE#"$ROOT"/}"
fi
CAND_ARG="/w/${CAND#"$ROOT"/}"

echo "candidat : $CAND_ARG"
echo "baseline : $BASE_ARG"
echo "parties  : $GAMES (threads $THREADS)"
echo

OUT="$(MSYS_NO_PATHCONV=1 docker run --rm -v "$ROOT:/w" traversal-arena duel \
  "$CAND_ARG" "$BASE_ARG" --games "$GAMES" --threads "$THREADS" 2>&1)"
printf '%s\n' "$OUT"

SUMMARY="$(printf '%s\n' "$OUT" | grep -E '^A : ' | tail -1 || true)"
if [[ -z "$SUMMARY" ]]; then
  echo "résumé illisible (l'arène n'a pas produit de ligne 'A : ...')" >&2
  exit 3
fi

SCORE="$(sed -E 's/.*score ([0-9.]+) %.*/\1/' <<<"$SUMMARY")"
ELO_CI="$(sed -E 's/.*≈ ([+-]?[0-9]+) ± ([0-9]+).*/\1 \2/' <<<"$SUMMARY")"
read -r ELO CI <<<"$ELO_CI"

if [[ ! "$ELO" =~ ^[+-]?[0-9]+$ || ! "$CI" =~ ^[0-9]+$ ]]; then
  echo "impossible de parser l'Elo : $SUMMARY" >&2
  exit 3
fi

FAULT_LINE="$(printf '%s\n' "$OUT" | grep -E 'fuel moyen' | tail -1 || true)"
FAULTS="$(sed -E 's/.*· ([0-9]+) partie\(s\) perdue.*/\1/' <<<"$FAULT_LINE")"
[[ "$FAULTS" =~ ^[0-9]+$ ]] || FAULTS=0

LOW=$((ELO - CI))
HIGH=$((ELO + CI))

echo
echo "score candidat : $SCORE %"
echo "Elo(A)-Elo(B)  : $ELO ± $CI  (IC [$LOW, $HIGH])"
echo "fautes         : $FAULTS"

if (( FAULTS > 0 )); then
  echo "REJETÉ : $FAULTS partie(s) perdue(s) sur faute (coup inexistant ou plantage)."
  exit 1
fi
if (( LOW > 0 )); then
  echo "ACCEPTÉ : le candidat est devant sur cet échantillon."
  exit 0
elif (( HIGH < 0 )); then
  echo "REJETÉ : le candidat est derrière."
  exit 1
else
  echo "INDÉCIS : augmenter le nombre de parties."
  exit 2
fi
