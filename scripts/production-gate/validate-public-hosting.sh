#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

echo "=================================================="
echo " MEMELAB V1.0 — PUBLIC HOSTING VALIDATION"
echo "=================================================="

for coin in rocket-rat degen-rat; do
  test -f "$ROOT/public/$coin/metadata.json"
  test -f "$ROOT/public/$coin/index.html"

  grep -q '"name":' "$ROOT/public/$coin/metadata.json"
  grep -q '"symbol":' "$ROOT/public/$coin/metadata.json"
  grep -q '"image":' "$ROOT/public/$coin/metadata.json"

  echo "PASS: $coin public structure"
done

if grep -R "__IMAGE_URI__" "$ROOT/public" >/dev/null; then
  echo "PENDING: final image URLs"
fi

if grep -R "__WEBSITE_URI__" "$ROOT/public" >/dev/null; then
  echo "PENDING: final website URLs"
fi

echo ""
echo "No public production URL has been claimed."
echo "No mint created."
echo "No pool created."
echo "No liquidity deposited."
echo "No mainnet transaction submitted."
echo ""
echo "RESULT: V1.0 HOSTING FOUNDATION GREEN"
