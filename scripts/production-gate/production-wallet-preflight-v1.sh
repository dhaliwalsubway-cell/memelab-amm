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
echo " MEMELAB — PRODUCTION WALLET PREFLIGHT V1"
echo "============================================================"
echo "NO TRANSACTION CREATION"
echo "NO SIGNING"
echo "NO BROADCAST"
echo

command -v solana >/dev/null 2>&1 || fail "solana CLI unavailable"
command -v solana-keygen >/dev/null 2>&1 || fail "solana-keygen unavailable"

WALLET="${MEMELAB_PRODUCTION_WALLET:-}"

[ -n "$WALLET" ] || fail "MEMELAB_PRODUCTION_WALLET is not set"
[ -f "$WALLET" ] || fail "production wallet file does not exist"

case "$WALLET" in
    *memelab-devnet.json)
        fail "development wallet is permanently rejected"
        ;;
esac

PUBKEY="$(solana-keygen pubkey "$WALLET")"
[ -n "$PUBKEY" ] || fail "unable to derive production wallet public key"

echo "Production wallet: $WALLET"
echo "Public key: $PUBKEY"

echo
echo "=== RPC PREFLIGHT ==="

RPC="$(solana config get 2>/dev/null | awk -F': ' '/RPC URL/ {print $2}' | tr -d '[:space:]')"

echo "Configured RPC: ${RPC:-UNKNOWN}"

if [ "$RPC" = "http://127.0.0.1:8899" ] || [ "$RPC" = "http://localhost:8899" ]; then
    fail "local RPC detected; production wallet preflight requires mainnet-beta RPC"
fi

echo "PASS: non-local RPC configured"

echo
echo "=== WALLET BALANCE PREFLIGHT ==="

BALANCE="$(solana balance "$PUBKEY" 2>/dev/null || true)"

[ -n "$BALANCE" ] || fail "unable to query production wallet balance"

echo "Balance: $BALANCE"
echo "PASS: wallet is queryable"

echo
echo "=== PRIVATE KEY SAFETY ==="

case "$WALLET" in
    *.json)
        echo "PASS: wallet file uses expected Solana keypair format"
        ;;
    *)
        fail "production wallet path is not a JSON keypair"
        ;;
esac

echo
echo "=== EXECUTION LOCK ==="
echo "TRANSACTION CREATION: FALSE"
echo "SIGNING: FALSE"
echo "BROADCAST: FALSE"
echo "MINTING: FALSE"
echo "LIQUIDITY: FALSE"
echo "RAYDIUM: FALSE"

echo
echo "============================================================"
echo " PRODUCTION WALLET PREFLIGHT COMPLETE"
echo "============================================================"
echo "WALLET VERIFIED FOR PREFLIGHT ONLY"
echo "NO TRANSACTION CREATED"
echo "============================================================"
