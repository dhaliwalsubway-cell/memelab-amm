#!/bin/bash
set -euo pipefail

NAME="${1:-}"
SYMBOL="${2:-}"
SUPPLY="${3:-}"
DECIMALS="${4:-}"
URI="${5:-}"
SOL_LIQUIDITY="${6:-}"
TOKEN_LIQUIDITY="${7:-}"
FEE_BPS="${8:-}"

if [[ -z "$NAME" || -z "$SYMBOL" || -z "$SUPPLY" || -z "$DECIMALS" || -z "$URI" || -z "$SOL_LIQUIDITY" || -z "$TOKEN_LIQUIDITY" || -z "$FEE_BPS" ]]; then
  echo "Usage:"
  echo "  $0 NAME SYMBOL SUPPLY DECIMALS METADATA_URI SOL_LIQUIDITY TOKEN_LIQUIDITY FEE_BPS"
  exit 1
fi

case "$URI" in
  https://*.json) ;;
  *)
    echo "ERROR: metadata URI must be a plain HTTPS .json URL."
    echo "Received: $URI"
    exit 1
    ;;
esac

case "$URI" in
  *"]("*|"]"*|*"["*|*")"*)
    echo "ERROR: Markdown-wrapped metadata URL detected."
    exit 1
    ;;
esac

mkdir -p launch-manifests

STAMP="$(date +"%Y%m%d-%H%M%S")"
FILE="launch-manifests/${SYMBOL}-${STAMP}.txt"

cat > "$FILE" <<EOF
MEME LAB PRODUCTION LAUNCH MANIFEST
===================================

Name:              $NAME
Symbol:            $SYMBOL
Supply:            $SUPPLY
Decimals:          $DECIMALS
Metadata URI:      $URI
SOL Liquidity:     $SOL_LIQUIDITY
Token Liquidity:   $TOKEN_LIQUIDITY
Fee BPS:           $FEE_BPS

Network:           NOT SELECTED
Mint:              PENDING
AMM Program:       PENDING
Pool:              PENDING
Vault:             PENDING

STATUS:            PRE-LAUNCH
MAINNET EXECUTION: BLOCKED

Required before execution:
- Real metadata JSON hosted and reachable
- Correct image URL
- Mainnet RPC explicitly selected
- Mainnet payer explicitly selected
- AMM program deployed and verified
- Upgrade-authority decision completed
- Liquidity amount confirmed
- Token supply confirmed
- Final launch parameters independently reviewed
EOF

echo "Manifest created:"
echo "$FILE"
