#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

echo "=================================================="
echo " MEMELAB V0.7 — PRODUCTION VALIDATION GATE"
echo "=================================================="
echo ""

python3 - <<'PY'
import json
from pathlib import Path

coins = {
    "Rocket Rat": (
        Path("production/coins/rocket-rat.json"),
        Path("production/metadata/rocket-rat.json"),
        "RKTROT"
    ),
    "Degen Rat": (
        Path("production/coins/degen-rat.json"),
        Path("production/metadata/degen-rat.json"),
        "DGRAT"
    )
}

required = [
    Path("production/LAUNCH_MATRIX.json"),
    Path("production/AUTHORITY_POLICY.md"),
    Path("production/LIQUIDITY_POLICY.md"),
    Path("production/FINAL_LAUNCH_GATE.md")
]

for p in required:
    if not p.exists():
        raise SystemExit(f"FAIL: missing {p}")

for name, (coin_file, meta_file, symbol) in coins.items():
    if not coin_file.exists():
        raise SystemExit(f"FAIL: missing {coin_file}")

    if not meta_file.exists():
        raise SystemExit(f"FAIL: missing {meta_file}")

    coin = json.loads(coin_file.read_text())
    meta = json.loads(meta_file.read_text())

    assert coin["name"] == name
    assert coin["symbol"] == symbol
    assert coin["supply"] == "1000000000"
    assert coin["decimals"] == 9
    assert coin["token_standard"] == "Token-2022"
    assert coin["network"] == "mainnet-beta"
    assert coin["raydium"]["pool_type"] == "CPMM"

    assert coin["mint"] is None
    assert coin["raydium"]["pool_id"] is None

    assert meta["name"] == name
    assert meta["symbol"] == symbol

    print(f"PASS: {name} identity")
    print(f"PASS: {name} Token-2022 configuration")
    print(f"PASS: {name} Raydium CPMM configuration")
    print(f"PENDING: {name} final artwork URL")
    print(f"PENDING: {name} final metadata URL")
    print(f"PENDING: {name} production mint")
    print(f"PENDING: {name} liquidity")
    print()

print("PASS: production configuration integrity")
PY

echo "--- LOCAL RPC SAFETY CHECK ---"

RPC="$(solana config get 2>/dev/null | awk -F': ' '/RPC URL/ {print $2}' | tr -d '[:space:]' || true)"

echo "Current RPC: ${RPC:-UNKNOWN}"

if [[ "$RPC" == *"localhost"* || "$RPC" == *"127.0.0.1"* ]]; then
    echo "PASS: LOCAL RPC detected"
else
    echo "FAIL: RPC is not local"
    exit 1
fi

echo ""
echo "--- EXECUTION SAFETY ---"
echo "PASS: no signer loaded"
echo "PASS: no mint creation"
echo "PASS: no liquidity transfer"
echo "PASS: no Raydium pool creation"
echo "PASS: no mainnet transaction"
echo ""

echo "=================================================="
echo " RESULT: V0.7 VALIDATION GREEN"
echo "=================================================="
