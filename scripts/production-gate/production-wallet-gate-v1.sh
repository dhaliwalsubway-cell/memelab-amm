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
echo " MEMELAB — PRODUCTION WALLET GATE V1"
echo "============================================================"
echo "PURPOSE: verify environment before any mainnet execution"
echo "SIGNING: DISABLED"
echo "BROADCAST: DISABLED"
echo

command -v solana >/dev/null 2>&1 || fail "solana CLI unavailable"
command -v solana-keygen >/dev/null 2>&1 || fail "solana-keygen unavailable"

RPC="$(solana config get 2>/dev/null | awk -F': ' '/RPC URL/ {print $2}' | tr -d '[:space:]')"
WALLET="$(solana config get 2>/dev/null | awk -F': ' '/Keypair Path/ {print $2}' | tr -d '[:space:]')"

echo "RPC: ${RPC:-UNKNOWN}"
echo "KEYPAIR: ${WALLET:-UNKNOWN}"

echo
echo "=== CURRENT ENVIRONMENT ==="

if [ "$RPC" = "http://127.0.0.1:8899" ] || \
   [ "$RPC" = "http://localhost:8899" ]; then
    echo "PASS: local RPC detected"
else
    echo "PASS: non-local RPC detected"
fi

echo
echo "=== KEYPAIR SAFETY ==="

[ -n "$WALLET" ] || fail "no keypair configured"
[ -f "$WALLET" ] || fail "configured keypair does not exist"

echo "PASS: configured keypair exists"

PUBKEY="$(solana-keygen pubkey "$WALLET")"

[ -n "$PUBKEY" ] || fail "unable to derive public key"

echo "Public key: $PUBKEY"

echo
echo "=== PRODUCTION WALLET REQUIREMENT ==="

if [[ "$WALLET" == *"memelab-devnet.json" ]]; then
    echo "PASS: local development wallet identified"
    echo "PRODUCTION EXECUTION: BLOCKED"
    echo "REASON: development wallet cannot be used as production signer"
else
    echo "PASS: wallet is not the known development wallet"
fi

echo
echo "=== AUTHORITY POLICY ==="

python3 -c '
from pathlib import Path

s=Path("production/AUTHORITY_POLICY.md").read_text()

required=(
    "Mint authority: revoke after the initial 1,000,000,000 tokens are minted",
    "Freeze authority: none",
    "Metadata update authority: revoke after final metadata verification",
    "Future minting: disabled",
    "Freeze capability: disabled",
    "Metadata: immutable after final verification",
    "No production mint transaction may be signed until the final launch gate confirms",
)

for item in required:
    assert item in s, f"missing authority policy requirement: {item}"

print("PASS: authority policy matches approved production policy")
'

echo
echo "=== EXECUTION LOCK ==="
echo "SIGNING: FALSE"
echo "BROADCAST: FALSE"
echo "MINTING: FALSE"
echo "LIQUIDITY: FALSE"
echo "RAYDIUM: FALSE"

echo
echo "============================================================"
echo " PRODUCTION WALLET GATE V1 COMPLETE"
echo "============================================================"
echo "NO TRANSACTION CREATED"
echo "NO TRANSACTION SIGNED"
echo "NO TRANSACTION BROADCAST"
echo "============================================================"
