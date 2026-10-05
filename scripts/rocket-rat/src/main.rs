use anchor_lang::{
    prelude::Pubkey,
    AccountDeserialize,
};
use memelab_amm::constants::{POOL_SEED, VAULT_SEED};
use solana_client::rpc_client::RpcClient;
use std::env;

fn quote_buy(
    pool_state: &memelab_amm::state::Pool,
    sol_lamports: u64,
) {
    let mlab_out = memelab_amm::math::amount_out(
        sol_lamports,
        pool_state.sol_reserve,
        pool_state.mlab_reserve,
        pool_state.fee_bps,
    )
    .expect("Could not calculate BUY quote");

    println!("=== ROCKET RAT BUY QUOTE ===");
    println!(
        "Input SOL:       {}",
        sol_lamports as f64 / 1_000_000_000.0
    );
    println!("Input lamports:  {}", sol_lamports);
    println!(
        "Expected RKTROT: {}",
        mlab_out as f64 / 1_000_000_000.0
    );
    println!("Output units:    {}", mlab_out);
    println!("Pool fee:        {} bps", pool_state.fee_bps);
    println!("Quote only:      YES");
    println!("No transaction submitted.");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let rpc_url = "http://localhost:8899";
    let client = RpcClient::new(rpc_url.to_string());

    let rocket_rat_mint =
        Pubkey::try_from("Gu5HZ2r2CagGRFtttqeHRmY2cwpzJ3xXanYgcrS3G1jk")
            .expect("Invalid Rocket Rat mint");

    let (pool, _) = Pubkey::find_program_address(
        &[POOL_SEED, rocket_rat_mint.as_ref()],
        &memelab_amm::ID,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[VAULT_SEED, pool.as_ref()],
        &memelab_amm::ID,
    );

    println!("=== ROCKET RAT LOCAL DEPLOYMENT CHECK ===");
    println!("Mint:  {}", rocket_rat_mint);
    println!("Pool:  {}", pool);
    println!("Vault: {}", vault);

    // ------------------------------------------------------------
    // 1. Verify mint exists.
    // ------------------------------------------------------------

    let mint_account = client
        .get_account(&rocket_rat_mint)
        .expect("Rocket Rat mint account not found");

    println!("Mint account: FOUND");
    println!("Mint owner:   {}", mint_account.owner);
    println!("Mint data:    {} bytes", mint_account.data.len());

    assert_eq!(
        mint_account.owner,
        anchor_spl::token_interface::spl_token_2022::ID,
        "Rocket Rat mint is not owned by Token-2022"
    );

    println!("Mint Token-2022: PASS");

    // ------------------------------------------------------------
    // 2. Verify pool exists and is owned by AMM.
    // ------------------------------------------------------------

    let pool_account = client
        .get_account(&pool)
        .expect("Rocket Rat pool account not found");

    println!("Pool account: FOUND");
    println!("Pool owner:   {}", pool_account.owner);
    println!("Pool data:    {} bytes", pool_account.data.len());

    assert_eq!(
        pool_account.owner,
        memelab_amm::ID,
        "Rocket Rat pool is not owned by the AMM program"
    );

    println!("Pool AMM ownership: PASS");

    // ------------------------------------------------------------
    // 3. Decode pool state.
    // ------------------------------------------------------------

    let mut pool_data: &[u8] = &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(&mut pool_data)
            .expect("Could not decode Rocket Rat pool state");

    println!("Pool authority:     {}", pool_state.authority);
    println!("Pool MLAB mint:     {}", pool_state.mlab_mint);
    println!("Pool MLAB vault:    {}", pool_state.mlab_vault);
    println!("Pool SOL reserve:   {} lamports", pool_state.sol_reserve);
    println!("Pool MLAB reserve:  {}", pool_state.mlab_reserve);
    println!("Pool fee:           {} bps", pool_state.fee_bps);

    assert_eq!(
        pool_state.mlab_mint,
        rocket_rat_mint,
        "Pool references the wrong MLAB mint"
    );

    assert_eq!(
        pool_state.mlab_vault,
        vault,
        "Pool references the wrong MLAB vault"
    );

    println!("Pool state references: PASS");

    // ------------------------------------------------------------
    // QUOTE-ONLY BUY MODE
    // ------------------------------------------------------------

    if args.len() == 3 && args[1] == "--quote-buy" {
        let sol: f64 = args[2]
            .parse()
            .expect("BUY amount must be a number of SOL");

        assert!(
            sol > 0.0,
            "BUY amount must be greater than zero"
        );

        let sol_lamports = (sol * 1_000_000_000.0) as u64;

        quote_buy(&pool_state, sol_lamports);
        return;
    }

    // ------------------------------------------------------------
    // 4. Verify vault exists and is Token-2022.
    // ------------------------------------------------------------

    let vault_account = client
        .get_account(&vault)
        .expect("Rocket Rat vault account not found");

    println!("Vault account: FOUND");
    println!("Vault owner:   {}", vault_account.owner);
    println!("Vault data:    {} bytes", vault_account.data.len());

    assert_eq!(
        vault_account.owner,
        anchor_spl::token_interface::spl_token_2022::ID,
        "Rocket Rat vault is not owned by Token-2022"
    );

    println!("Vault Token-2022 ownership: PASS");

    // ------------------------------------------------------------
    // 5. Read vault balance.
    // ------------------------------------------------------------

    let vault_balance = client
        .get_token_account_balance(&vault)
        .expect("Could not read Rocket Rat vault token balance");

    println!("Vault actual RKTROT: {}", vault_balance.amount);

    let recorded_vault_balance = vault_balance
        .amount
        .parse::<u64>()
        .expect("Vault balance was not a valid u64");

    assert_eq!(
        recorded_vault_balance,
        pool_state.mlab_reserve,
        "Vault balance does not match recorded pool MLAB reserve"
    );

    println!("Vault/pool reserve reconciliation: PASS");

    // ------------------------------------------------------------
    // 6. Compare pool SOL reserve to actual account balance.
    // ------------------------------------------------------------

    let actual_pool_lamports = pool_account.lamports;

    println!(
        "Pool account SOL:       {} lamports",
        actual_pool_lamports
    );

    println!(
        "Recorded SOL reserve:   {} lamports",
        pool_state.sol_reserve
    );

    assert!(
        actual_pool_lamports >= pool_state.sol_reserve,
        "Pool account balance is below recorded SOL reserve"
    );

    let rent_buffer = actual_pool_lamports - pool_state.sol_reserve;

    println!(
        "Pool rent/extra balance: {} lamports",
        rent_buffer
    );

    println!("Pool SOL reconciliation: PASS");

    println!();
    println!("=== ROCKET RAT DEPLOYMENT CHECK GREEN ===");
    println!("No transactions were submitted.");
}
