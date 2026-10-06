#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

fail() {
    echo "FAIL: $1"
    exit 1
}

echo
echo "============================================================"
echo " MEMELAB V1 — PRODUCTION TOKEN-2022 MINT PREFLIGHT"
echo "============================================================"
echo "SIGNING: DISABLED"
echo "BROADCAST: DISABLED"
echo "MINT CREATION: DISABLED"
echo "LIQUIDITY: DISABLED"
echo

command -v python3 >/dev/null 2>&1 || fail "python3 unavailable"
command -v solana >/dev/null 2>&1 || fail "solana CLI unavailable"

echo "PASS: toolchain"

python3 - <<'PY'
import json
from pathlib import Path
from decimal import Decimal

EXPECTED = {
    "rocket-rat": {
        "name": "Rocket Rat",
        "symbol": "RKTROT",
        "sol": Decimal("5"),
        "token": Decimal("100000000"),
        "price": Decimal("0.00000005"),
    },
    "degen-rat": {
        "name": "Degen Rat",
        "symbol": "DGRAT",
        "sol": Decimal("3"),
        "token": Decimal("100000000"),
        "price": Decimal("0.00000003"),
    },
}

for coin, e in EXPECTED.items():
    p = Path(f"production/coins/{coin}.json")
    assert p.exists()

    d = json.loads(p.read_text())

    assert d["name"] == e["name"]
    assert d["symbol"] == e["symbol"]
    assert d["supply"] == "1000000000"
    assert d["decimals"] == 9
    assert d["token_standard"] == "Token-2022"
    assert d["network"] == "mainnet-beta"

    liq = d["initial_liquidity"]

    assert Decimal(str(liq["sol"])) == e["sol"]
    assert Decimal(str(liq["token"])) == e["token"]
    assert Decimal(str(liq["initial_price_sol_per_token"])) == e["price"]

    auth = d["authority_policy"]

    assert auth["fixed_supply"] is True
    assert auth["future_minting"] is False
    assert auth["freeze_capability"] is False
    assert auth["mint_authority"] == "REVOKE_AFTER_INITIAL_MINT"
    assert auth["freeze_authority"] == "NONE"
    assert auth["metadata_update_authority"] == "REVOKE_AFTER_FINAL_VERIFICATION"
    assert auth["metadata_immutable"] is True

    supply_base = 1_000_000_000 * (10 ** 9)
    liquidity_base = 100_000_000 * (10 ** 9)

    assert supply_base == 1_000_000_000_000_000_000
    assert liquidity_base == 100_000_000_000_000_000

    print(f"PASS: {e['name']} configuration")
    print(f"  supply base units: {supply_base}")
    print(f"  liquidity base units: {liquidity_base}")
    print(f"  SOL liquidity: {e['sol']}")
    print(f"  opening price: {e['price']} SOL")
PY

echo
echo "=== METADATA ==="

python3 - <<'PY'
import json
from pathlib import Path

for coin in ("rocket-rat", "degen-rat"):
    d = json.loads(Path(f"production/metadata/{coin}.json").read_text())

    assert d["image"].startswith("https://")
    assert d["external_url"].startswith("https://")
    assert "](" not in d["image"]
    assert "](" not in d["external_url"]

    print(f"PASS: {coin} metadata")
PY

echo
echo "=== RPC SAFETY CHECK ==="

RPC="$(solana config get 2>/dev/null | awk -F': ' '/RPC URL/ {print $2}' | tr -d '[:space:]')"

echo "Current RPC: ${RPC:-UNKNOWN}"

if [ "$RPC" = "http://127.0.0.1:8899" ] || \
   [ "$RPC" = "http://localhost:8899" ]; then
    echo "PASS: local RPC safety mode"
else
    echo "PASS: non-local RPC detected — preflight remains execution-disabled"
fi

echo
echo "=== GENERATE DETERMINISTIC PLANS ==="

python3 - <<'PY'
import json
from pathlib import Path

BASE = "https://dhaliwalsubway-cell.github.io/memelab-amm"

coins = {
    "rocket-rat": {
        "name": "Rocket Rat",
        "symbol": "RKTROT",
        "sol": "5",
        "price": "0.00000005",
    },
    "degen-rat": {
        "name": "Degen Rat",
        "symbol": "DGRAT",
        "sol": "3",
        "price": "0.00000003",
    },
}

for coin, c in coins.items():
    plan = {
        "plan_version": "1.0",
        "status": "PREFLIGHT-ONLY-NOT-EXECUTED",
        "network": "mainnet-beta",
        "token_program": "Token-2022",
        "coin": c["name"],
        "symbol": c["symbol"],
        "decimals": 9,
        "total_supply_ui": "1000000000",
        "total_supply_base_units": "1000000000000000000",
        "metadata_uri": f"{BASE}/{coin}/metadata.json",
        "image_uri": f"{BASE}/{coin}/{coin}.png",
        "authority_policy": {
            "mint_authority": "REVOKE_AFTER_INITIAL_MINT",
            "freeze_authority": "NONE",
            "metadata_update_authority": "REVOKE_AFTER_FINAL_VERIFICATION"
        },
        "initial_liquidity": {
            "sol": c["sol"],
            "token_ui": "100000000",
            "token_base_units": "100000000000000000",
            "initial_price_sol_per_token": c["price"]
        },
        "execution": {
            "signing": False,
            "broadcast": False,
            "mint_creation": False,
            "liquidity_transfer": False,
            "raydium_pool_creation": False
        }
    }

    Path(f"production/mint-plans/{coin}.json").write_text(
        json.dumps(plan, indent=2) + "\n"
    )

    print(f"PASS: {coin} plan generated")
PY

echo
echo "=== PLAN SAFETY ==="

python3 - <<'PY'
import json
from pathlib import Path

for coin in ("rocket-rat", "degen-rat"):
    d = json.loads(Path(f"production/mint-plans/{coin}.json").read_text())

    assert d["status"] == "PREFLIGHT-ONLY-NOT-EXECUTED"

    for key, value in d["execution"].items():
        assert value is False, f"{coin}: execution flag {key} is not false"

    print(f"PASS: {coin} execution lock")
PY

echo
echo "=== EXISTING GATES ==="

./scripts/production-gate/validate-production.sh
./scripts/production-gate/validate-assets.sh

echo
echo "=== STATIC SAFETY AUDIT ==="

python3 - <<'PY'
from pathlib import Path

p = Path("scripts/production-gate/mint-preflight-v1.sh")
s = p.read_text()

# The audit examines the script text but excludes the audit itself.
audit_start = s.index("echo\n echo \"=== STATIC SAFETY AUDIT ===\"") if "echo\n echo \"=== STATIC SAFETY AUDIT ===\"" in s else -1

if audit_start == -1:
    # Find the final audit marker robustly.
    marker = 'echo "=== STATIC SAFETY AUDIT ==="'
    audit_start = s.index(marker)

body = s[:audit_start]

for forbidden in (
    "sendAndConfirm",
    "sendTransaction",
    "signTransaction",
    "signAllTransactions",
):
    assert forbidden not in body, f"forbidden execution API found: {forbidden}"

assert "solana transfer" not in body
assert "spl-token mint" not in body
assert "spl-token transfer" not in body
assert "spl-token create-token" not in body

print("PASS: no transaction-signing API")
print("PASS: no token mint command")
print("PASS: no token transfer command")
print("PASS: no SOL transfer command")
PY

echo
echo "=== FINAL PREFLIGHT STATUS ==="
echo "PASS: configuration"
echo "PASS: metadata"
echo "PASS: deterministic plans"
echo "PASS: execution flags hard-disabled"
echo "PASS: existing production gates"
echo "PASS: static execution audit"

echo
echo "============================================================"
echo " V1 PRODUCTION MINT PREFLIGHT GREEN"
echo "============================================================"
echo "MAINNET MINT: NOT CREATED"
echo "SIGNER: NOT LOADED"
echo "TRANSACTION: NOT SIGNED"
echo "TRANSACTION: NOT BROADCAST"
echo "LIQUIDITY: NOT TRANSFERRED"
echo "RAYDIUM POOL: NOT CREATED"
echo "============================================================"
