#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "=================================================="
echo " MEMELAB V0.8 — ASSET / METADATA GATE"
echo "=================================================="
echo ""

FAIL=0

check_dir() {
    if [ -d "$1" ]; then
        echo "PASS: directory $1"
    else
        echo "FAIL: missing directory $1"
        FAIL=1
    fi
}

check_dir production/assets/rocket-rat/master
check_dir production/assets/rocket-rat/avatar
check_dir production/assets/rocket-rat/mascot
check_dir production/assets/rocket-rat/wordmark
check_dir production/assets/rocket-rat/banner

check_dir production/assets/degen-rat/master
check_dir production/assets/degen-rat/avatar
check_dir production/assets/degen-rat/mascot
check_dir production/assets/degen-rat/wordmark
check_dir production/assets/degen-rat/banner

check_dir production/launch-manifests/rocket-rat
check_dir production/launch-manifests/degen-rat

echo ""
echo "--- METADATA IDENTITY ---"

python3 - <<'PY'
import json
from pathlib import Path

coins = [
    ("Rocket Rat", "RKTROT", Path("production/metadata/rocket-rat.json")),
    ("Degen Rat", "DGRAT", Path("production/metadata/degen-rat.json")),
]

for name, symbol, path in coins:
    if not path.exists():
        raise SystemExit(f"FAIL: missing {path}")

    data = json.loads(path.read_text())

    assert data["name"] == name
    assert data["symbol"] == symbol
    assert data["image"].startswith("https://")
    assert data["image"].endswith(".png")
    assert data["external_url"].startswith("https://")
    assert "__IMAGE_URI__" not in data["image"]
    assert "__WEBSITE_URI__" not in data["external_url"]

    print(f"PASS: {name} metadata identity")
    print(f"PASS: {name} production image URL")
    print(f"PASS: {name} hosted metadata URL")
PY

echo ""
echo "--- ASSET STATUS ---"

for coin in rocket-rat degen-rat; do
    echo ""
    echo "$coin"

    for type in master avatar mascot wordmark banner; do
        count=$(find "production/assets/$coin/$type" \
            -type f \
            ! -name ".gitkeep" \
            2>/dev/null | wc -l | tr -d ' ')

        if [ "$count" -gt 0 ]; then
            echo "PASS: $type contains $count asset(s)"
        else
            echo "PENDING: $type asset"
        fi
    done
done

echo ""
echo "--- PRODUCTION URL PROTECTION ---"

if grep -R "__IMAGE_URI__\|__WEBSITE_URI__" production/metadata >/dev/null 2>&1; then
    echo "FAIL: production metadata still contains placeholders"
    FAIL=1
else
    echo "PASS: no production metadata placeholders"
fi

if grep -R -E '\]\(https?://' production/metadata >/dev/null 2>&1; then
    echo "FAIL: Markdown-wrapped production URL detected"
    FAIL=1
else
    echo "PASS: no Markdown-wrapped production URLs"
fi

echo ""
echo "=================================================="

if [ "$FAIL" -eq 0 ]; then
    echo " RESULT: V0.8 ASSET STRUCTURE GREEN"
else
    echo " RESULT: V0.8 ASSET STRUCTURE RED"
fi

echo "=================================================="

exit "$FAIL"
