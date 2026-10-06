#!/usr/bin/env python3

from pathlib import Path
import json
import subprocess

def fail(message):
    print(f"FAIL: {message}")
    raise SystemExit(1)

print()
print("============================================================")
print(" MEMELAB — FINAL PRODUCTION WALLET GATE V1")
print("============================================================")
print("READ-ONLY PREFLIGHT")
print()

wallet = Path.home() / ".config/solana/memelab-production.json"

if not wallet.exists():
    fail("production wallet missing")

if wallet.name == "memelab-devnet.json":
    fail("development wallet rejected")

if oct(wallet.stat().st_mode & 0o777) != "0o600":
    fail("production wallet permissions must be 600")

pubkey = subprocess.check_output(
    ["solana-keygen", "pubkey", str(wallet)],
    text=True
).strip()

if pubkey != "6ozjmoWoUnfEdphpG92x7RXyEQnvpFYWzFxm6BDXSkVd":
    fail("production wallet public key mismatch")

print("PASS: production wallet verified")
print(f"Public key: {pubkey}")

print()
print("=== PRODUCTION ECONOMICS ===")

coins = [
    (
        "production/coins/rocket-rat.json",
        "Rocket Rat",
        "RKTROT",
        5,
        "100000000",
        "0.00000005",
    ),
    (
        "production/coins/degen-rat.json",
        "Degen Rat",
        "DGRAT",
        3,
        "100000000",
        "0.00000003",
    ),
]

for filename, name, symbol, expected_sol, expected_token, expected_price in coins:
    data = json.loads(Path(filename).read_text())

    if data["name"] != name:
        fail(f"{filename}: name mismatch")

    if data["symbol"] != symbol:
        fail(f"{filename}: symbol mismatch")

    if data["supply"] != "1000000000":
        fail(f"{filename}: supply mismatch")

    if data["decimals"] != 9:
        fail(f"{filename}: decimals mismatch")

    if data["token_standard"] != "Token-2022":
        fail(f"{filename}: token standard mismatch")

    if data["network"] != "mainnet-beta":
        fail(f"{filename}: network mismatch")

    liquidity = data["initial_liquidity"]

    if liquidity["sol"] != expected_sol:
        fail(f"{filename}: SOL liquidity mismatch")

    if liquidity["token"] != expected_token:
        fail(f"{filename}: token liquidity mismatch")

    if liquidity["initial_price_sol_per_token"] != expected_price:
        fail(f"{filename}: initial price mismatch")

    if data["raydium"]["pool_type"] != "CPMM":
        fail(f"{filename}: Raydium pool type mismatch")

    print(f"PASS: {name} economics locked")

print()
print("=== METADATA URL SAFETY ===")

metadata = [
    (
        "production/metadata/rocket-rat.json",
        "Rocket Rat",
        "RKTROT",
        "https://dhaliwalsubway-cell.github.io/memelab-amm/rocket-rat/rocket-rat.png",
    ),
    (
        "production/metadata/degen-rat.json",
        "Degen Rat",
        "DGRAT",
        "https://dhaliwalsubway-cell.github.io/memelab-amm/degen-rat/degen-rat.png",
    ),
]

for filename, name, symbol, expected_image in metadata:
    data = json.loads(Path(filename).read_text())

    if data["name"] != name:
        fail(f"{filename}: metadata name mismatch")

    if data["symbol"] != symbol:
        fail(f"{filename}: metadata symbol mismatch")

    if not data["image"].startswith("https://"):
        fail(f"{filename}: image must be HTTPS")

    if data["image"] != expected_image:
        fail(f"{filename}: image URL mismatch")

    if "[" in data["image"] or "](" in data["image"]:
        fail(f"{filename}: Markdown-wrapped image URL")

    print(f"PASS: {name} metadata verified")

print()
print("=== MINT PLANS ===")

for filename in [
    "production/mint-plans/rocket-rat-v2.json",
    "production/mint-plans/degen-rat-v2.json",
]:
    data = json.loads(Path(filename).read_text())

    if data["status"] != "PREPARED-NOT-EXECUTED":
        fail(f"{filename}: status not locked")

    if data["network"] != "mainnet-beta":
        fail(f"{filename}: network mismatch")

    if data["token_program"] != "Token-2022":
        fail(f"{filename}: token program mismatch")

    execution = data["execution"]

    required_false = [
        "signing",
        "broadcast",
        "mint_creation",
        "supply_minting",
        "authority_revocation",
        "liquidity_transfer",
        "raydium_pool_creation",
    ]

    if execution["prepare_only"] is not True:
        fail(f"{filename}: prepare_only must be true")

    for key in required_false:
        if execution[key] is not False:
            fail(f"{filename}: {key} must be false")

    print(f"PASS: {filename}")

print()
print("=== AUTHORITY POLICY ===")

policy = Path("production/AUTHORITY_POLICY.md").read_text()

required_policy = [
    "Mint authority: revoke after the initial 1,000,000,000 tokens are minted",
    "Freeze authority: none",
    "Metadata update authority: revoke after final metadata verification",
    "Future minting: disabled",
    "Freeze capability: disabled",
    "Metadata: immutable after final verification",
    "No production mint transaction may be signed until the final launch gate confirms",
]

for item in required_policy:
    if item not in policy:
        fail(f"missing authority policy requirement: {item}")

print("PASS: approved authority policy verified")

print()
print("=== EXECUTION LOCK ===")
print("TRANSACTION CREATION: FALSE")
print("SIGNING: FALSE")
print("BROADCAST: FALSE")
print("MINTING: FALSE")
print("LIQUIDITY: FALSE")
print("RAYDIUM: FALSE")

print()
print("============================================================")
print(" FINAL PRODUCTION WALLET GATE V1: GREEN")
print("============================================================")
print("NO TRANSACTION CREATED")
print("NO TRANSACTION SIGNED")
print("NO TRANSACTION BROADCAST")
print("============================================================")
