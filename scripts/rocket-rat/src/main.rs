use anchor_lang::{
    prelude::Pubkey,
    InstructionData,
    ToAccountMetas,
};
use memelab_amm::constants::{POOL_SEED, VAULT_SEED};
use solana_client::rpc_client::RpcClient;
use solana_instruction::Instruction;
use solana_signer::Signer;
use solana_transaction::Transaction;

fn build_transaction(
    client: &RpcClient,
    payer: &solana_keypair::Keypair,
    instruction: Instruction,
) -> Transaction {
    let recent_blockhash = client
        .get_latest_blockhash()
        .expect("Could not get recent blockhash");

    Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[payer],
        recent_blockhash,
    )
}

fn main() {
    let rpc_url = "http://localhost:8899";
    let client = RpcClient::new(rpc_url.to_string());

    let keypair_path = dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".config/solana/memelab-devnet.json");

    let payer = solana_keypair::read_keypair_file(&keypair_path)
        .expect("Could not read memelab-devnet keypair");

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

    let trader_mlab_account =
        Pubkey::try_from("DkDYWAe6mMfUTt5iUqFBKUnYJN91GyNhDPXSiXNZ93ou")
            .expect("Invalid Rocket Rat trader token account");

    // -------------------------
    // TEST 1: ZERO SOL BUY
    // -------------------------

    let buy_accounts = memelab_amm::accounts::SwapSolForMlab {
        trader: payer.pubkey(),
        pool,
        mlab_mint: rocket_rat_mint,
        mlab_vault: vault,
        trader_mlab_account,
        token_program: anchor_spl::token_interface::spl_token_2022::ID,
        system_program: solana_system_interface::program::ID,
    }
    .to_account_metas(None);

    let buy_instruction = Instruction {
        program_id: memelab_amm::ID,
        accounts: buy_accounts,
        data: memelab_amm::instruction::SwapSolForMlab {
            sol_amount: 0,
            min_mlab_out: 0,
        }
        .data(),
    };

    let buy_transaction =
        build_transaction(&client, &payer, buy_instruction);

    println!("ZERO-AMOUNT TEST 1: BUY 0 SOL");

    match client.send_and_confirm_transaction(&buy_transaction) {
        Ok(signature) => {
            panic!(
                "UNEXPECTED SUCCESS — zero SOL BUY was accepted: {}",
                signature
            );
        }
        Err(error) => {
            let message = error.to_string();

            println!("Expected BUY failure received.");
            println!("Error: {}", message);

            if message.contains("ZeroAmount") {
                println!("BUY ZeroAmount protection confirmed.");
            } else {
                panic!("BUY failed for an unexpected reason.");
            }
        }
    }

    // -------------------------
    // TEST 2: ZERO RKTROT SELL
    // -------------------------

    let sell_accounts = memelab_amm::accounts::SwapMlabForSol {
        trader: payer.pubkey(),
        pool,
        mlab_mint: rocket_rat_mint,
        mlab_vault: vault,
        trader_mlab_account,
        token_program: anchor_spl::token_interface::spl_token_2022::ID,
        system_program: solana_system_interface::program::ID,
    }
    .to_account_metas(None);

    let sell_instruction = Instruction {
        program_id: memelab_amm::ID,
        accounts: sell_accounts,
        data: memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: 0,
            min_sol_out: 0,
        }
        .data(),
    };

    let sell_transaction =
        build_transaction(&client, &payer, sell_instruction);

    println!("ZERO-AMOUNT TEST 2: SELL 0 RKTROT");

    match client.send_and_confirm_transaction(&sell_transaction) {
        Ok(signature) => {
            panic!(
                "UNEXPECTED SUCCESS — zero RKTROT SELL was accepted: {}",
                signature
            );
        }
        Err(error) => {
            let message = error.to_string();

            println!("Expected SELL failure received.");
            println!("Error: {}", message);

            if message.contains("ZeroAmount") {
                println!("SELL ZeroAmount protection confirmed.");
            } else {
                panic!("SELL failed for an unexpected reason.");
            }
        }
    }

    println!("ZERO-AMOUNT FAILURE TESTS COMPLETE");
}
