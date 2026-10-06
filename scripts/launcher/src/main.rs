use anchor_lang::prelude::Pubkey;
use anchor_lang::AccountDeserialize;
use anchor_lang::{InstructionData, ToAccountMetas};
use dirs::home_dir;
use memelab_amm::constants::{POOL_SEED, VAULT_SEED};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_keypair::{read_keypair_file, Keypair};
use solana_signer::Signer;
use solana_transaction::versioned::VersionedTransaction;
use solana_transaction::{Message, VersionedMessage};
use std::env;
use std::process::{Command, Stdio};
use std::str::FromStr;

const RPC_URL: &str = "http://localhost:8899";
const AMM_PROGRAM: &str = "DTRqHX3AvqMUnskqTpAz4ZZooSEw88S46NjbeQvN7zz";
const TOKEN_2022_PROGRAM: &str = "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb";

struct LaunchConfig {
    name: String,
    symbol: String,
    supply: String,
    decimals: u8,
    metadata_uri: String,
    sol_liquidity: String,
    token_liquidity: String,
    fee_bps: u16,
    execute: bool,
    self_test: bool,
}

fn usage() {
    println!("Usage:");
    println!("  meme-launcher <name> <symbol> <supply> <decimals> <metadata_uri> <sol_liquidity> <token_liquidity> <fee_bps>");
    println!("  meme-launcher --launch-local <name> <symbol> <supply> <decimals> <metadata_uri> <sol_liquidity> <token_liquidity> <fee_bps>
       --self-test-local <name> <symbol> <supply> <decimals> <metadata_uri> <sol_liquidity> <token_liquidity> <fee_bps>");
    println!();
    println!("Default mode is PLAN ONLY.");
    println!("--launch-local is required to submit local transactions.");
}

fn validate_metadata_uri(uri: &str) -> Result<(), String> {
    let uri = uri.trim();

    if uri.is_empty() {
        return Err("metadata URI cannot be empty".to_string());
    }

    if uri.contains("](") || uri.starts_with("[") || uri.ends_with(")") {
        return Err("metadata URI must be a plain URL, not Markdown-wrapped text".to_string());
    }

    let lower = uri.to_ascii_lowercase();

    if !lower.starts_with("https://") {
        return Err("metadata URI must use https://".to_string());
    }

    if uri.chars().any(|c| c.is_whitespace()) {
        return Err("metadata URI cannot contain whitespace".to_string());
    }

    if !lower.ends_with(".json") {
        return Err("metadata URI must point to a .json metadata document".to_string());
    }

    Ok(())
}

fn parse_ui_amount(value: &str, decimals: u8) -> Result<u64, String> {
    let value = value.trim();
    let mut parts = value.split('.');
    let whole = parts.next().unwrap_or("");
    let fraction = parts.next().unwrap_or("");

    if parts.next().is_some()
        || whole.is_empty()
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
        || fraction.len() > decimals as usize
    {
        return Err(format!("invalid amount: {value}"));
    }

    let scale = 10u64
        .checked_pow(decimals as u32)
        .ok_or("decimal scale overflow")?;

    let whole_units = whole
        .parse::<u64>()
        .map_err(|_| format!("amount overflow: {value}"))?
        .checked_mul(scale)
        .ok_or_else(|| format!("amount overflow: {value}"))?;

    let mut padded = fraction.to_owned();
    while padded.len() < decimals as usize {
        padded.push('0');
    }

    let fraction_units = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<u64>()
            .map_err(|_| format!("amount overflow: {value}"))?
    };

    whole_units
        .checked_add(fraction_units)
        .ok_or_else(|| format!("amount overflow: {value}"))
}

fn run_cli(args: &[String]) -> Result<String, String> {
    let output = Command::new("spl-token")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("failed to execute spl-token: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "spl-token failed\nstdout:\n{stdout}\nstderr:\n{stderr}"
        ));
    }

    Ok(stdout)
}

fn extract_labeled_pubkey(output: &str, label: &str) -> Result<Pubkey, String> {
    for line in output.lines() {
        if let Some(pos) = line.find(label) {
            for token in line[pos + label.len()..].split_whitespace() {
                if let Ok(key) = Pubkey::from_str(token.trim_matches(':')) {
                    return Ok(key);
                }
            }
        }
    }

    Err(format!("could not find pubkey after '{label}'"))
}

fn send_instruction(
    client: &RpcClient,
    payer: &Keypair,
    instruction: Instruction,
) -> Result<String, String> {
    let blockhash = client
        .get_latest_blockhash()
        .map_err(|e| format!("latest blockhash failed: {e}"))?;

    let message = Message::new_with_blockhash(&[instruction], Some(&payer.pubkey()), &blockhash);

    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(message), &[payer])
        .map_err(|e| format!("transaction signing failed: {e}"))?;

    client
        .send_and_confirm_transaction(&tx)
        .map(|sig| sig.to_string())
        .map_err(|e| format!("transaction failed: {e}"))
}

fn build_initialize_instruction(
    payer: Pubkey,
    mint: Pubkey,
    pool: Pubkey,
    fee_bps: u16,
) -> Instruction {
    Instruction::new_with_bytes(
        memelab_amm::ID,
        &memelab_amm::instruction::Initialize { fee_bps }.data(),
        memelab_amm::accounts::Initialize {
            payer,
            mlab_mint: mint,
            pool,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn build_create_vault_instruction(
    payer: Pubkey,
    mint: Pubkey,
    pool: Pubkey,
    vault: Pubkey,
) -> Instruction {
    Instruction::new_with_bytes(
        memelab_amm::ID,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer,
            mlab_mint: mint,
            pool,
            vault,
            authority: payer,
            system_program: anchor_lang::solana_program::system_program::ID,
            token_program: anchor_spl::token_interface::spl_token_2022::ID,
        }
        .to_account_metas(None),
    )
}

fn build_fund_pool_instruction(
    authority: Pubkey,
    pool: Pubkey,
    mint: Pubkey,
    vault: Pubkey,
    source: Pubkey,
    token_amount: u64,
    sol_amount: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        memelab_amm::ID,
        &memelab_amm::instruction::FundPool {
            mlab_amount: token_amount,
            sol_amount,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority,
            pool,
            mlab_mint: mint,
            mlab_vault: vault,
            mlab_source: source,
            token_program: anchor_spl::token_interface::spl_token_2022::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn verify_pool(
    client: &RpcClient,
    mint: Pubkey,
    pool: Pubkey,
    vault: Pubkey,
) -> Result<(), String> {
    let mint_account = client
        .get_account(&mint)
        .map_err(|e| format!("mint verification failed: {e}"))?;

    if mint_account.owner != anchor_spl::token_interface::spl_token_2022::ID {
        return Err("mint is not Token-2022 owned".into());
    }

    let pool_account = client
        .get_account(&pool)
        .map_err(|e| format!("pool verification failed: {e}"))?;

    if pool_account.owner != memelab_amm::ID {
        return Err("pool is not AMM-owned".into());
    }

    let vault_account = client
        .get_account(&vault)
        .map_err(|e| format!("vault verification failed: {e}"))?;

    if vault_account.owner != anchor_spl::token_interface::spl_token_2022::ID {
        return Err("vault is not Token-2022 owned".into());
    }

    println!("Verification: MINT GREEN");
    println!("Verification: POOL GREEN");
    println!("Verification: VAULT GREEN");

    Ok(())
}

fn validate_config(config: &LaunchConfig) -> Result<(u64, u64, u64), String> {
    if config.name.trim().is_empty() {
        return Err("name cannot be empty".into());
    }

    if config.symbol.trim().is_empty() {
        return Err("symbol cannot be empty".into());
    }

    if config.name.len() > 32 {
        return Err("name exceeds 32 characters".into());
    }

    if config.symbol.len() > 10 {
        return Err("symbol exceeds 10 characters".into());
    }

    if config.decimals > 18 {
        return Err("decimals must be 0-18".into());
    }

    if config.fee_bps > 30 {
        return Err("fee_bps exceeds AMM maximum of 30".into());
    }

    let supply = parse_ui_amount(&config.supply, config.decimals)?;
    let token_liquidity = parse_ui_amount(&config.token_liquidity, config.decimals)?;
    let sol_liquidity = parse_ui_amount(&config.sol_liquidity, 9)?;

    if supply == 0 {
        return Err("supply must be greater than zero".into());
    }

    if token_liquidity == 0 {
        return Err("token liquidity must be greater than zero".into());
    }

    if sol_liquidity == 0 {
        return Err("SOL liquidity must be greater than zero".into());
    }

    if token_liquidity > supply {
        return Err("token liquidity cannot exceed total supply".into());
    }

    Ok((supply, token_liquidity, sol_liquidity))
}

#[derive(Debug, Clone)]
struct LaunchArtifacts {
    mint: Pubkey,
    source: Pubkey,
    pool: Pubkey,
    vault: Pubkey,
}

#[derive(Debug, Clone)]
struct PoolSnapshot {
    mlab_mint: Pubkey,
    mlab_vault: Pubkey,
    sol_reserve: u64,
    mlab_reserve: u64,
    fee_bps: u16,
}

fn read_pool_snapshot(client: &RpcClient, pool: Pubkey) -> Result<PoolSnapshot, String> {
    let account = client
        .get_account(&pool)
        .map_err(|e| format!("pool read failed: {e}"))?;

    if account.owner != memelab_amm::ID {
        return Err("pool is not owned by MemeLab AMM".into());
    }

    let mut data: &[u8] = &account.data;

    let state = memelab_amm::Pool::try_deserialize(&mut data)
        .map_err(|e| format!("pool deserialization failed: {e}"))?;

    Ok(PoolSnapshot {
        mlab_mint: state.mlab_mint,
        mlab_vault: state.mlab_vault,
        sol_reserve: state.sol_reserve,
        mlab_reserve: state.mlab_reserve,
        fee_bps: state.fee_bps,
    })
}

fn token_balance(client: &RpcClient, token_account: Pubkey) -> Result<u64, String> {
    let balance = client
        .get_token_account_balance(&token_account)
        .map_err(|e| format!("token balance read failed: {e}"))?;

    balance
        .amount
        .parse::<u64>()
        .map_err(|e| format!("token balance parse failed: {e}"))
}

fn calculate_local_amount_out(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_bps: u16,
) -> Result<u64, String> {
    memelab_amm::math::amount_out(amount_in, reserve_in, reserve_out, fee_bps)
        .map_err(|e| format!("AMM quote failed: {e:?}"))
}

fn build_buy_instruction(
    trader: Pubkey,
    pool: Pubkey,
    mint: Pubkey,
    vault: Pubkey,
    trader_mlab_account: Pubkey,
    sol_amount: u64,
    min_mlab_out: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        memelab_amm::ID,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount,
            min_mlab_out,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader,
            pool,
            mlab_mint: mint,
            mlab_vault: vault,
            trader_mlab_account,
            token_program: anchor_spl::token_interface::spl_token_2022::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn build_sell_instruction(
    trader: Pubkey,
    pool: Pubkey,
    mint: Pubkey,
    vault: Pubkey,
    trader_mlab_account: Pubkey,
    mlab_amount: u64,
    min_sol_out: u64,
) -> Instruction {
    Instruction::new_with_bytes(
        memelab_amm::ID,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount,
            min_sol_out,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader,
            pool,
            mlab_mint: mint,
            mlab_vault: vault,
            trader_mlab_account,
            token_program: anchor_spl::token_interface::spl_token_2022::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn run_buy_sell_self_test(
    client: &RpcClient,
    payer: &Keypair,
    artifacts: &LaunchArtifacts,
) -> Result<(), String> {
    println!();
    println!("==================================================");
    println!(" MEME LAB v0.3 BUY -> SELL SELF-TEST");
    println!("==================================================");
    println!("Network: LOCALHOST ONLY");
    println!("Mint:    {}", artifacts.mint);
    println!("Pool:    {}", artifacts.pool);
    println!("Vault:   {}", artifacts.vault);
    println!("Trader:  {}", payer.pubkey());
    println!("Token:   {}", artifacts.source);
    println!();

    // --------------------------------------------------------------
    // Initial verification
    // --------------------------------------------------------------
    verify_pool(client, artifacts.mint, artifacts.pool, artifacts.vault)?;

    let initial_pool = read_pool_snapshot(client, artifacts.pool)?;
    let initial_token_balance = token_balance(client, artifacts.source)?;
    let initial_vault_balance = token_balance(client, artifacts.vault)?;

    if initial_pool.mlab_mint != artifacts.mint {
        return Err("SELF-TEST RED: pool mint mismatch".into());
    }

    if initial_pool.mlab_vault != artifacts.vault {
        return Err("SELF-TEST RED: pool vault mismatch".into());
    }

    if initial_pool.sol_reserve == 0 {
        return Err("SELF-TEST RED: SOL reserve is zero".into());
    }

    if initial_pool.mlab_reserve == 0 {
        return Err("SELF-TEST RED: MLAB reserve is zero".into());
    }

    if initial_vault_balance != initial_pool.mlab_reserve {
        return Err(format!(
            "SELF-TEST RED: initial vault/reserve mismatch: vault={} reserve={}",
            initial_vault_balance, initial_pool.mlab_reserve
        ));
    }

    println!("Initial pool:       GREEN");
    println!("Initial vault:      GREEN");
    println!("Initial liquidity:  GREEN");
    println!("Initial token bal:  {initial_token_balance}");

    // --------------------------------------------------------------
    // BUY
    // --------------------------------------------------------------
    //
    // 0.001 SOL = 1,000,000 lamports.
    //
    // Quote against the exact current pool state and submit that
    // quote as the minimum acceptable output.
    // --------------------------------------------------------------
    let buy_sol = 1_000_000u64;

    let quoted_buy = calculate_local_amount_out(
        buy_sol,
        initial_pool.sol_reserve,
        initial_pool.mlab_reserve,
        initial_pool.fee_bps,
    )?;

    if quoted_buy == 0 {
        return Err("SELF-TEST RED: BUY quote returned zero".into());
    }

    println!();
    println!("[BUY]");
    println!("Input:      {} lamports", buy_sol);
    println!("Quote:      {} token units", quoted_buy);
    println!("Min output: {} token units", quoted_buy);

    let buy_ix = build_buy_instruction(
        payer.pubkey(),
        artifacts.pool,
        artifacts.mint,
        artifacts.vault,
        artifacts.source,
        buy_sol,
        quoted_buy,
    );

    let buy_sig = send_instruction(client, payer, buy_ix)?;

    println!("BUY sig:    {buy_sig}");

    let after_buy_pool = read_pool_snapshot(client, artifacts.pool)?;
    let after_buy_token_balance = token_balance(client, artifacts.source)?;
    let after_buy_vault_balance = token_balance(client, artifacts.vault)?;

    let actual_bought = after_buy_token_balance
        .checked_sub(initial_token_balance)
        .ok_or_else(|| "SELF-TEST RED: BUY token balance decreased".to_string())?;

    if actual_bought != quoted_buy {
        return Err(format!(
            "SELF-TEST RED: BUY output mismatch: quoted={} actual={}",
            quoted_buy, actual_bought
        ));
    }

    if after_buy_pool.sol_reserve != initial_pool.sol_reserve + buy_sol {
        return Err(format!(
            "SELF-TEST RED: BUY SOL reserve mismatch: expected={} actual={}",
            initial_pool.sol_reserve + buy_sol,
            after_buy_pool.sol_reserve
        ));
    }

    if after_buy_pool.mlab_reserve != initial_pool.mlab_reserve - quoted_buy {
        return Err(format!(
            "SELF-TEST RED: BUY MLAB reserve mismatch: expected={} actual={}",
            initial_pool.mlab_reserve - quoted_buy,
            after_buy_pool.mlab_reserve
        ));
    }

    if after_buy_vault_balance != after_buy_pool.mlab_reserve {
        return Err(format!(
            "SELF-TEST RED: BUY vault/reserve mismatch: vault={} reserve={}",
            after_buy_vault_balance, after_buy_pool.mlab_reserve
        ));
    }

    println!("BUY execution:      GREEN");
    println!("BUY output:         {actual_bought}");
    println!("BUY accounting:     GREEN");
    println!("BUY vault match:    GREEN");

    // --------------------------------------------------------------
    // SELL
    // --------------------------------------------------------------
    //
    // Sell the exact amount purchased above.
    // Quote against the post-BUY pool state.
    // --------------------------------------------------------------
    let sell_amount = actual_bought;

    let quoted_sell = calculate_local_amount_out(
        sell_amount,
        after_buy_pool.mlab_reserve,
        after_buy_pool.sol_reserve,
        after_buy_pool.fee_bps,
    )?;

    if quoted_sell == 0 {
        return Err("SELF-TEST RED: SELL quote returned zero".into());
    }

    println!();
    println!("[SELL]");
    println!("Input:      {} token units", sell_amount);
    println!("Quote:      {} lamports", quoted_sell);
    println!("Min output: {} lamports", quoted_sell);

    let sell_ix = build_sell_instruction(
        payer.pubkey(),
        artifacts.pool,
        artifacts.mint,
        artifacts.vault,
        artifacts.source,
        sell_amount,
        quoted_sell,
    );

    let sell_sig = send_instruction(client, payer, sell_ix)?;

    println!("SELL sig:   {sell_sig}");

    let final_pool = read_pool_snapshot(client, artifacts.pool)?;
    let final_token_balance = token_balance(client, artifacts.source)?;
    let final_vault_balance = token_balance(client, artifacts.vault)?;

    if final_token_balance != initial_token_balance {
        return Err(format!(
            "SELF-TEST RED: token round-trip mismatch: initial={} final={}",
            initial_token_balance, final_token_balance
        ));
    }

    if final_pool.mlab_reserve != initial_pool.mlab_reserve {
        return Err(format!(
            "SELF-TEST RED: final MLAB reserve mismatch: initial={} final={}",
            initial_pool.mlab_reserve, final_pool.mlab_reserve
        ));
    }

    if final_pool.sol_reserve != after_buy_pool.sol_reserve - quoted_sell {
        return Err(format!(
            "SELF-TEST RED: SELL SOL reserve mismatch: expected={} actual={}",
            after_buy_pool.sol_reserve - quoted_sell,
            final_pool.sol_reserve
        ));
    }

    if final_vault_balance != final_pool.mlab_reserve {
        return Err(format!(
            "SELF-TEST RED: final vault/reserve mismatch: vault={} reserve={}",
            final_vault_balance, final_pool.mlab_reserve
        ));
    }

    println!("SELL execution:     GREEN");
    println!("SELL output:        {quoted_sell} lamports");
    println!("SELL accounting:    GREEN");
    println!("SELL vault match:   GREEN");

    // --------------------------------------------------------------
    // Final state
    // --------------------------------------------------------------
    println!();
    println!("==================================================");
    println!(" MEME LAB v0.3 SELF-TEST GREEN");
    println!("==================================================");
    println!("Launch path:        GREEN");
    println!("Token-2022 mint:    GREEN");
    println!("AMM pool:           GREEN");
    println!("Token vault:        GREEN");
    println!("BUY:                GREEN");
    println!("SELL:               GREEN");
    println!("Round trip:         GREEN");
    println!("Final token bal:    {final_token_balance}");
    println!("Final MLAB reserve: {}", final_pool.mlab_reserve);
    println!("Final SOL reserve:  {}", final_pool.sol_reserve);
    println!("Network:            LOCALHOST ONLY");
    println!("==================================================");
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let self_test = args
        .get(1)
        .map(|v| v == "--self-test-local")
        .unwrap_or(false);

    let execute = args.get(1).map(|v| v == "--launch-local").unwrap_or(false);

    let offset = if self_test || execute { 2 } else { 1 };

    if args.len() != offset + 8 {
        usage();
        std::process::exit(1);
    }

    let config = LaunchConfig {
        name: args[offset].clone(),
        symbol: args[offset + 1].clone(),
        supply: args[offset + 2].clone(),
        decimals: args[offset + 3].parse().unwrap_or(255),
        metadata_uri: args[offset + 4].clone(),
        sol_liquidity: args[offset + 5].clone(),
        token_liquidity: args[offset + 6].clone(),
        fee_bps: args[offset + 7].parse().unwrap_or(u16::MAX),
        execute: execute || self_test,
        self_test,
    };

    if let Err(error) = validate_metadata_uri(&config.metadata_uri) {
        eprintln!("ERROR: invalid metadata URI: {error}");
        std::process::exit(1);
    }

    let (supply_units, token_liquidity_units, sol_liquidity_lamports) =
        match validate_config(&config) {
            Ok(values) => values,
            Err(error) => {
                eprintln!("CONFIG ERROR: {error}");
                std::process::exit(1);
            }
        };

    println!("=== MEME LAB LAUNCHER v0.3 ===");
    println!("Network:         LOCALHOST ONLY");
    println!("AMM:             {AMM_PROGRAM}");
    println!("Token program:   {TOKEN_2022_PROGRAM}");
    println!("Name:            {}", config.name);
    println!("Symbol:          {}", config.symbol);
    println!("Supply:          {}", config.supply);
    println!("Decimals:        {}", config.decimals);
    println!("Metadata URI:    {}", config.metadata_uri);
    println!("SOL liquidity:   {}", config.sol_liquidity);
    println!("Token liquidity: {}", config.token_liquidity);
    println!("Fee:             {} bps", config.fee_bps);
    println!("Supply units:    {supply_units}");
    println!("Liquidity units: {token_liquidity_units}");
    println!("SOL lamports:    {sol_liquidity_lamports}");

    if !config.execute {
        println!();
        println!("Status: PLAN GREEN");
        println!("No transactions submitted.");
        println!("Use --launch-local to execute locally.");
        println!("Use --self-test-local to launch + BUY + SELL + verify.");
        return;
    }

    println!();
    println!("=== LOCAL LAUNCH EXECUTION ===");

    if config.self_test {
        println!("Mode: FULL LAUNCH + BUY/SELL SELF-TEST");
    } else {
        println!("Mode: FULL LAUNCH");
    }

    let payer_path = home_dir()
        .expect("Could not determine home directory")
        .join(".config/solana/memelab-devnet.json");

    let payer = read_keypair_file(&payer_path)
        .unwrap_or_else(|e| panic!("Could not read local payer {}: {e}", payer_path.display()));

    let client = RpcClient::new(RPC_URL.to_string());

    let balance = client
        .get_balance(&payer.pubkey())
        .expect("Could not read local payer balance");

    assert!(
        balance > sol_liquidity_lamports + 100_000_000,
        "payer does not have enough local SOL for requested liquidity + safety buffer"
    );

    println!("Payer:           {}", payer.pubkey());

    let fee_payer = payer_path.to_string_lossy().to_string();

    println!("[1/7] Creating Token-2022 mint...");

    let mint_output = run_cli(&[
        "--url".into(),
        RPC_URL.into(),
        "--program-2022".into(),
        "--fee-payer".into(),
        fee_payer.clone(),
        "create-token".into(),
        "--decimals".into(),
        config.decimals.to_string(),
        "--enable-metadata".into(),
    ])
    .unwrap_or_else(|e| panic!("{e}"));

    let mint = extract_labeled_pubkey(&mint_output, "Address:")
        .or_else(|_| extract_labeled_pubkey(&mint_output, "Creating token"))
        .unwrap_or_else(|e| panic!("{e}"));

    println!("Mint:             {mint}");

    println!("[2/7] Initializing metadata...");

    run_cli(&[
        "--url".into(),
        RPC_URL.into(),
        "--program-2022".into(),
        "--fee-payer".into(),
        fee_payer.clone(),
        "initialize-metadata".into(),
        mint.to_string(),
        config.name.clone(),
        config.symbol.clone(),
        config.metadata_uri.clone(),
    ])
    .unwrap_or_else(|e| panic!("{e}"));

    println!("[3/7] Creating source token account...");

    let account_output = run_cli(&[
        "--url".into(),
        RPC_URL.into(),
        "--program-2022".into(),
        "--fee-payer".into(),
        fee_payer.clone(),
        "create-account".into(),
        mint.to_string(),
    ])
    .unwrap_or_else(|e| panic!("{e}"));

    let source = extract_labeled_pubkey(&account_output, "Creating account")
        .unwrap_or_else(|e| panic!("{e}"));

    println!("Source account:   {source}");

    println!("[4/7] Minting total supply...");

    run_cli(&[
        "--url".into(),
        RPC_URL.into(),
        "--program-2022".into(),
        "--fee-payer".into(),
        fee_payer.clone(),
        "mint".into(),
        mint.to_string(),
        config.supply.clone(),
        source.to_string(),
    ])
    .unwrap_or_else(|e| panic!("{e}"));

    let (pool, _) = Pubkey::find_program_address(&[POOL_SEED, mint.as_ref()], &memelab_amm::ID);

    let (vault, _) = Pubkey::find_program_address(&[VAULT_SEED, pool.as_ref()], &memelab_amm::ID);

    println!("Pool:             {pool}");
    println!("Vault:            {vault}");

    println!("[5/7] Initializing AMM pool...");

    let initialize_ix = build_initialize_instruction(payer.pubkey(), mint, pool, config.fee_bps);

    let initialize_sig =
        send_instruction(&client, &payer, initialize_ix).unwrap_or_else(|e| panic!("{e}"));

    println!("Initialize sig:   {initialize_sig}");

    println!("[6/7] Creating Token-2022 vault...");

    let vault_ix = build_create_vault_instruction(payer.pubkey(), mint, pool, vault);

    let vault_sig = send_instruction(&client, &payer, vault_ix).unwrap_or_else(|e| panic!("{e}"));

    println!("Vault sig:        {vault_sig}");

    println!("[7/7] Funding pool...");

    let fund_ix = build_fund_pool_instruction(
        payer.pubkey(),
        pool,
        mint,
        vault,
        source,
        token_liquidity_units,
        sol_liquidity_lamports,
    );

    let fund_sig = send_instruction(&client, &payer, fund_ix).unwrap_or_else(|e| panic!("{e}"));

    println!("Funding sig:      {fund_sig}");

    verify_pool(&client, mint, pool, vault)
        .unwrap_or_else(|e| panic!("FINAL VERIFICATION FAILED: {e}"));

    let artifacts = LaunchArtifacts {
        mint,
        source,
        pool,
        vault,
    };

    if config.self_test {
        run_buy_sell_self_test(&client, &payer, &artifacts).unwrap_or_else(|e| panic!("{e}"));

        return;
    }

    println!();
    println!("=== MEME LAB LOCAL LAUNCH GREEN ===");
    println!("Mint:             {mint}");
    println!("Pool:             {pool}");
    println!("Vault:            {vault}");
    println!("Trading state:    OPEN");
    println!("Network:          LOCALHOST ONLY");
}

#[cfg(test)]
mod metadata_uri_tests {
    use super::validate_metadata_uri;

    #[test]
    fn metadata_uri_accepts_valid_https_json() {
        assert!(validate_metadata_uri("https://example.com/metadata.json").is_ok());
    }

    #[test]
    fn metadata_uri_validation_rejects_markdown() {
        assert!(validate_metadata_uri(
            "[https://example.com/metadata.json](https://example.com/metadata.json)"
        )
        .is_err());
    }

    #[test]
    fn metadata_uri_validation_rejects_http() {
        assert!(validate_metadata_uri("http://example.com/metadata.json").is_err());
    }

    #[test]
    fn metadata_uri_validation_rejects_non_json() {
        assert!(validate_metadata_uri("https://example.com/metadata.txt").is_err());
    }

    #[test]
    fn metadata_uri_validation_rejects_whitespace() {
        assert!(validate_metadata_uri("https://example.com/my metadata.json").is_err());
    }

    #[test]
    fn metadata_uri_validation_rejects_empty() {
        assert!(validate_metadata_uri("").is_err());
    }
}
