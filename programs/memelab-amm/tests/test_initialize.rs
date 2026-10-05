use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

#[test]
fn test_initialize() {
    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mlab_mint = Pubkey::new_unique();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[instruction],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    let res = svm.send_transaction(tx);

    assert!(res.is_ok());

    let pool_account = svm.get_account(&pool).unwrap();

    let mut data: &[u8] = &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(&mut data).unwrap();

    assert_eq!(pool_state.authority, payer.pubkey());
    assert_eq!(pool_state.mlab_mint, mlab_mint);
    assert_eq!(pool_state.sol_reserve, 0);
    assert_eq!(pool_state.mlab_reserve, 0);
    assert_eq!(pool_state.fee_bps, 30);
}

#[test]
fn test_initialize_rejects_excessive_fee() {
    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mlab_mint = Pubkey::new_unique();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();

    let instruction = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 31,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[instruction],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    let res = svm.send_transaction(tx);

    assert!(res.is_err());
}

#[test]
fn test_create_vault_token_2022() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use solana_transaction::versioned::VersionedTransaction;
    use anchor_spl::token_2022::spl_token_2022::instruction::initialize_mint2;

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 5_000_000_000).unwrap();

    // ------------------------------------------------------------
    // 1. Create a real Token-2022 mint account.
    // ------------------------------------------------------------
    let mint_space = 82usize;
    let mint_lamports = svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Confirm the mint really exists and is owned by Token-2022.
    let mint_account = svm.get_account(&mint.pubkey()).unwrap();
    assert_eq!(mint_account.owner, token_2022::ID);

    // ------------------------------------------------------------
    // 2. Initialize the AMM pool.
    // ------------------------------------------------------------
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Create the Token-2022 vault through our AMM program.
    // ------------------------------------------------------------
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "create_vault failed: {:?}",
        result.err()
    );

    // ------------------------------------------------------------
    // 4. Verify the pool and vault state.
    // ------------------------------------------------------------
    let pool_account = svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] = &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(&mut pool_data).unwrap();

    assert_eq!(pool_state.mlab_mint, mlab_mint);
    assert_eq!(pool_state.mlab_vault, vault);

    let vault_account = svm.get_account(&vault).unwrap();

    assert_eq!(vault_account.owner, token_2022::ID);
}

#[test]
fn test_create_vault_rejects_wrong_mint() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::initialize_mint2;

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();

    let mlab_mint = mint_a.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // Create two real Token-2022 mints.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_a = system_instruction::create_account(
        &payer.pubkey(),
        &mint_a.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_a = initialize_mint2(
        &token_2022::ID,
        &mint_a.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let create_b = system_instruction::create_account(
        &payer.pubkey(),
        &mint_b.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_b = initialize_mint2(
        &token_2022::ID,
        &mint_b.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[create_a, init_a, create_b, init_b],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint_a, &mint_b],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize the pool against Mint A.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_before = svm.get_account(&pool).unwrap();
    let pool_lamports_before = pool_before.lamports;
    let pool_data_before = pool_before.data.clone();

    // Attempt vault creation using Mint B against a pool bound to Mint A.
    let wrong_mint_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint: mint_b.pubkey(),
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[wrong_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "create_vault should reject a mint different from the pool's configured mint"
    );

    // The failed transaction must not modify the pool.
    let pool_after = svm.get_account(&pool).unwrap();

    assert_eq!(
        pool_after.lamports,
        pool_lamports_before,
        "Pool lamports changed after rejected vault creation"
    );

    assert_eq!(
        pool_after.data,
        pool_data_before,
        "Pool state changed after rejected vault creation"
    );

    // The vault PDA must not have been created.
    assert!(
        svm.get_account(&vault).is_none(),
        "Vault should not exist after rejected wrong-mint creation"
    );
}

#[test]
fn test_fund_pool() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::{
        instruction::{initialize_account3, initialize_mint2, mint_to_checked},
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // ------------------------------------------------------------
    // 1. Create a real Token-2022 MLAB mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create the user's MLAB source token account.
    // ------------------------------------------------------------

    let token_account_space = 165usize;

    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mlab_mint,
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint test MLAB into the source account.
    // ------------------------------------------------------------

    let mlab_amount: u64 = 1_000_000_000_000;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mlab_mint,
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        mlab_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize the AMM pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create the AMM Token-2022 vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Record pool SOL balance before funding.
    //
    // The pool already contains rent-exempt lamports, so we
    // compare the balance DELTA rather than the total account
    // balance.
    // ------------------------------------------------------------

    let pool_lamports_before =
        svm.get_account(&pool).unwrap().lamports;

    // ------------------------------------------------------------
    // 7. Fund the pool.
    // ------------------------------------------------------------

    let sol_amount: u64 = 1_000_000_000;

    let fund_pool_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount,
            sol_amount,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: anchor_lang::solana_program::system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_pool_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "fund_pool failed: {:?}",
        result.err()
    );

    // ------------------------------------------------------------
    // 8. Read the actual Token-2022 vault balance.
    // ------------------------------------------------------------

    let vault_account = svm.get_account(&vault).unwrap();

    let vault_token = &vault_account.data;

    // Token-2022 account amount is stored at bytes 64..72
    // in the standard token account layout.
    let vault_amount = u64::from_le_bytes(
        vault_token[64..72].try_into().unwrap()
    );

    // Mint and owner are stored in the standard token account layout.
    let vault_mint =
        Pubkey::try_from(&vault_token[0..32]).unwrap();

    let vault_owner =
        Pubkey::try_from(&vault_token[32..64]).unwrap();

    assert_eq!(vault_mint, mlab_mint);
    assert_eq!(vault_owner, pool);
    assert_eq!(vault_amount, mlab_amount);

    // ------------------------------------------------------------
    // 9. Read the actual SOL balance delta.
    // ------------------------------------------------------------

    let pool_lamports_after =
        svm.get_account(&pool).unwrap().lamports;

    let sol_delta =
        pool_lamports_after - pool_lamports_before;

    assert_eq!(sol_delta, sol_amount);

    // ------------------------------------------------------------
    // 10. Verify Pool accounting matches the real accounts.
    // ------------------------------------------------------------

    let pool_account = svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] = &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(pool_state.mlab_vault, vault);
    assert_eq!(pool_state.mlab_reserve, mlab_amount);
    assert_eq!(pool_state.sol_reserve, sol_amount);

    assert_eq!(pool_state.mlab_reserve, vault_amount);
    assert_eq!(pool_state.sol_reserve, sol_delta);
}


#[test]
fn test_fund_pool_failure_rolls_back_mlab_transfer() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::{
        instruction::{initialize_account3, initialize_mint2, mint_to_checked},
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source token account.
    // ------------------------------------------------------------

    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mlab_mint,
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint valid MLAB to the source account.
    // ------------------------------------------------------------

    let mlab_amount = 1_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mlab_mint,
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        mlab_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Capture balances before the deliberately failing funding.
    // ------------------------------------------------------------

    let source_before = svm.get_account(&source.pubkey().into()).unwrap();
    let vault_before = svm.get_account(&vault).unwrap();
    let pool_before = svm.get_account(&pool).unwrap();

    let source_mlab_before =
        u64::from_le_bytes(source_before.data[64..72].try_into().unwrap());

    let vault_mlab_before =
        u64::from_le_bytes(vault_before.data[64..72].try_into().unwrap());

    let pool_sol_before = pool_before.lamports;

    // ------------------------------------------------------------
    // 7. Deliberately make SOL funding impossible.
    // ------------------------------------------------------------

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount,
            sol_amount: u64::MAX,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "Funding transaction should have failed"
    );

    // ------------------------------------------------------------
    // 8. Verify the failed transaction rolled back everything.
    // ------------------------------------------------------------

    let source_after = svm.get_account(&source.pubkey().into()).unwrap();
    let vault_after = svm.get_account(&vault).unwrap();
    let pool_after = svm.get_account(&pool).unwrap();

    let source_mlab_after =
        u64::from_le_bytes(source_after.data[64..72].try_into().unwrap());

    let vault_mlab_after =
        u64::from_le_bytes(vault_after.data[64..72].try_into().unwrap());

    assert_eq!(
        source_mlab_after,
        source_mlab_before,
        "Source MLAB changed after failed funding"
    );

    assert_eq!(
        vault_mlab_after,
        vault_mlab_before,
        "Vault MLAB changed after failed funding"
    );

    assert_eq!(
        pool_after.lamports,
        pool_sol_before,
        "Pool SOL balance changed after failed funding"
    );

    let mut pool_data: &[u8] = &pool_after.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(&mut pool_data)
            .unwrap();

    assert_eq!(
        pool_state.mlab_reserve,
        0,
        "MLAB reserve changed after failed funding"
    );

    assert_eq!(
        pool_state.sol_reserve,
        0,
        "SOL reserve changed after failed funding"
    );
}


#[test]
fn test_fund_pool_rejects_wrong_mint_source() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint_a = Keypair::new();
    let mint_b = Keypair::new();
    let source_b = Keypair::new();

    let mlab_mint = mint_a.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // Create Mint A and Mint B.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_a = system_instruction::create_account(
        &payer.pubkey(),
        &mint_a.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_a = initialize_mint2(
        &token_2022::ID,
        &mint_a.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let create_mint_b = system_instruction::create_account(
        &payer.pubkey(),
        &mint_b.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_b = initialize_mint2(
        &token_2022::ID,
        &mint_b.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[
            create_mint_a,
            init_mint_a,
            create_mint_b,
            init_mint_b,
        ],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint_a, &mint_b],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create a source token account belonging to Mint B.
    let source_space = 165usize;
    let source_lamports =
        svm.minimum_balance_for_rent_exemption(source_space);

    let create_source = system_instruction::create_account(
        &payer.pubkey(),
        &source_b.pubkey(),
        source_lamports,
        source_space as u64,
        &token_2022::ID,
    );

    let init_source = initialize_account3(
        &token_2022::ID,
        &source_b.pubkey(),
        &mint_b.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source, init_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source_b],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Give the wrong-mint source account some Mint B tokens.
    let mint_to = mint_to_checked(
        &token_2022::ID,
        &mint_b.pubkey(),
        &source_b.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize the pool against Mint A.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create the valid Mint-A vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_before = svm.get_account(&pool).unwrap();
    let vault_before = svm.get_account(&vault).unwrap();
    let source_before = svm.get_account(&source_b.pubkey()).unwrap();

    // Attempt to fund the Mint-A pool from a Mint-B source account.
    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 100_000_000,
            sol_amount: 1_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source_b.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    assert!(
        svm.send_transaction(tx).is_err(),
        "fund_pool should reject a source account belonging to the wrong mint"
    );

    // Nothing should change after rejection.
    let pool_after = svm.get_account(&pool).unwrap();
    let vault_after = svm.get_account(&vault).unwrap();
    let source_after = svm.get_account(&source_b.pubkey()).unwrap();

    assert_eq!(pool_before.lamports, pool_after.lamports);
    assert_eq!(pool_before.data, pool_after.data);
    assert_eq!(vault_before.lamports, vault_after.lamports);
    assert_eq!(vault_before.data, vault_after.data);
    assert_eq!(source_before.lamports, source_after.lamports);
    assert_eq!(source_before.data, source_after.data);
}

#[test]
fn test_fund_pool_rejects_unauthorized_authority() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let attacker = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 5_000_000_000).unwrap();

    // Create the MLAB mint.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint, init_mint],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create the legitimate authority's source token account.
    let source_space = 165usize;
    let source_lamports =
        svm.minimum_balance_for_rent_exemption(source_space);

    let create_source = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        source_lamports,
        source_space as u64,
        &token_2022::ID,
    );

    let init_source = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source, init_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Give the legitimate source account MLAB.
    let mint_to = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize pool with payer as the configured authority.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create the vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_before = svm.get_account(&pool).unwrap();
    let vault_before = svm.get_account(&vault).unwrap();
    let source_before = svm.get_account(&source.pubkey()).unwrap();

    // Attempt funding using an unauthorized signer.
    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 100_000_000,
            sol_amount: 1_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: attacker.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&attacker.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&attacker],
    )
    .unwrap();

    assert!(
        svm.send_transaction(tx).is_err(),
        "fund_pool should reject an unauthorized authority"
    );

    // Verify nothing changed.
    let pool_after = svm.get_account(&pool).unwrap();
    let vault_after = svm.get_account(&vault).unwrap();
    let source_after = svm.get_account(&source.pubkey()).unwrap();

    assert_eq!(pool_before.lamports, pool_after.lamports);
    assert_eq!(pool_before.data, pool_after.data);
    assert_eq!(vault_before.lamports, vault_after.lamports);
    assert_eq!(vault_before.data, vault_after.data);
    assert_eq!(source_before.lamports, source_after.lamports);
    assert_eq!(source_before.data, source_after.data);
}

#[test]
fn test_fund_pool_rejects_zero_amount() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::{
        instruction::{initialize_account3, initialize_mint2, mint_to_checked},
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();
    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();

    // Create real Token-2022 mint.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create Token-2022 source account.
    let source_space = 165usize;
    let source_lamports =
        svm.minimum_balance_for_rent_exemption(source_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        source_lamports,
        source_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Mint MLAB into the source account.
    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize pool.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize { fee_bps: 30 }.data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: anchor_lang::solana_program::system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let source_before = svm.get_account(&source.pubkey()).unwrap();
    let vault_before = svm.get_account(&vault).unwrap();
    let pool_before = svm.get_account(&pool).unwrap();

    // Zero MLAB must fail.
    let zero_mlab_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 0,
            sol_amount: 1_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[zero_mlab_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    assert!(svm.send_transaction(tx).is_err());

    // Zero SOL must fail.
    let zero_sol_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 1_000_000,
            sol_amount: 0,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(
        &[zero_sol_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    assert!(svm.send_transaction(tx).is_err());

    // Both failed transactions must leave state unchanged.
    let source_after = svm.get_account(&source.pubkey()).unwrap();
    let vault_after = svm.get_account(&vault).unwrap();
    let pool_after = svm.get_account(&pool).unwrap();

    assert_eq!(source_before.lamports, source_after.lamports);
    assert_eq!(source_before.data, source_after.data);
    assert_eq!(vault_before.lamports, vault_after.lamports);
    assert_eq!(vault_before.data, vault_after.data);
    assert_eq!(pool_before.lamports, pool_after.lamports);
    assert_eq!(pool_before.data, pool_after.data);
}

#[test]
fn test_swap_sol_for_mlab() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();

    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;

    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source Token-2022 account for pool funding.
    // ------------------------------------------------------------

    let token_account_space = 165usize;

    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint MLAB to the funding source.
    // ------------------------------------------------------------

    let funding_amount = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        funding_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize AMM pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create AMM Token-2022 vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund the pool with 5 SOL + MLAB.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;
    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader's Token-2022 MLAB account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. Calculate the expected BUY output.
    // ------------------------------------------------------------

    let sol_in = 1_000_000_000u64;

    let expected_mlab = memelab_amm::math::amount_out(
        sol_in,
        pool_sol,
        pool_mlab,
        30,
    )
    .unwrap();

    assert!(expected_mlab > 0);
    assert!(expected_mlab < pool_mlab);

    // ------------------------------------------------------------
    // 9. Record balances before the swap.
    // ------------------------------------------------------------

    let pool_sol_before =
        svm.get_account(&pool).unwrap().lamports;

    let vault_before =
        svm.get_account(&vault).unwrap();

    let vault_mlab_before =
        u64::from_le_bytes(vault_before.data[64..72].try_into().unwrap());

    assert_eq!(vault_mlab_before, pool_mlab);

    // ------------------------------------------------------------
    // 10. Execute SOL -> MLAB BUY.
    // ------------------------------------------------------------

    let swap_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: sol_in,
            min_mlab_out: expected_mlab,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[swap_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "SOL -> MLAB swap failed: {:?}",
        result.err()
    );

    // ------------------------------------------------------------
    // 11. Verify SOL entered the pool.
    // ------------------------------------------------------------

    let pool_sol_after =
        svm.get_account(&pool).unwrap().lamports;

    assert_eq!(
        pool_sol_after - pool_sol_before,
        sol_in
    );

    // ------------------------------------------------------------
    // 12. Verify MLAB left the vault.
    // ------------------------------------------------------------

    let vault_after =
        svm.get_account(&vault).unwrap();

    let vault_mlab_after =
        u64::from_le_bytes(vault_after.data[64..72].try_into().unwrap());

    assert_eq!(
        vault_mlab_before - vault_mlab_after,
        expected_mlab
    );

    // ------------------------------------------------------------
    // 13. Verify MLAB arrived in the trader account.
    // ------------------------------------------------------------

    let trader_token_after =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_after =
        u64::from_le_bytes(
            trader_token_after.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_after, expected_mlab);

    // ------------------------------------------------------------
    // 14. Verify stored AMM reserves.
    // ------------------------------------------------------------

    let pool_account =
        svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] =
        &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(
        pool_state.sol_reserve,
        pool_sol + sol_in
    );

    assert_eq!(
        pool_state.mlab_reserve,
        pool_mlab - expected_mlab
    );
}



#[test]
fn test_swap_sol_for_mlab_pool_accounting_invariant() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::{
        instruction::{
            initialize_account3,
            initialize_mint2,
            mint_to_checked,
        },
        state::Account as SplTokenAccount,
    };
    use anchor_lang::solana_program::program_pack::Pack;

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let trader_mlab_account = Keypair::new();

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&trader.pubkey(), 5_000_000_000).unwrap();

    // Create MLAB mint.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint, init_mint],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create pool source account.
    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source, init_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create trader destination account.
    let create_trader_account = system_instruction::create_account(
        &payer.pubkey(),
        &trader_mlab_account.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_account = initialize_account3(
        &token_2022::ID,
        &trader_mlab_account.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_account, init_trader_account],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &trader_mlab_account],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Mint MLAB to the pool funding source.
    let mint_to = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize pool.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Seed the pool.
    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 500_000_000,
            sol_amount: 1_000_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_before_account = svm.get_account(&pool).unwrap();
    let vault_before_account = svm.get_account(&vault).unwrap();

    let mut pool_before_data = pool_before_account.data.as_slice();
    let pool_before_state =
        memelab_amm::state::Pool::try_deserialize(&mut pool_before_data)
            .unwrap();

    let vault_before_state =
        SplTokenAccount::unpack(&vault_before_account.data).unwrap();

    let buy_amount = 10_000_000u64;

    // Perform BUY.
    let swap_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: buy_amount,
            min_mlab_out: 1,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab_account.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[swap_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_after_account = svm.get_account(&pool).unwrap();
    let vault_after_account = svm.get_account(&vault).unwrap();

    let mut pool_after_data = pool_after_account.data.as_slice();
    let pool_after_state =
        memelab_amm::state::Pool::try_deserialize(&mut pool_after_data)
            .unwrap();

    let vault_after_state =
        SplTokenAccount::unpack(&vault_after_account.data).unwrap();

    let actual_sol_delta =
        pool_after_account.lamports - pool_before_account.lamports;

    let recorded_sol_delta =
        pool_after_state.sol_reserve - pool_before_state.sol_reserve;

    let actual_mlab_delta =
        vault_before_state.amount - vault_after_state.amount;

    let recorded_mlab_delta =
        pool_before_state.mlab_reserve - pool_after_state.mlab_reserve;

    let k_before =
        pool_before_state.sol_reserve as u128 * pool_before_state.mlab_reserve as u128;

    let k_after =
        pool_after_state.sol_reserve as u128 * pool_after_state.mlab_reserve as u128;

    assert!(
        k_after >= k_before,
        "BUY constant-product invariant violated: before={}, after={}",
        k_before,
        k_after
    );

    assert_eq!(
        actual_sol_delta,
        buy_amount,
        "actual pool SOL balance must increase by the BUY input"
    );

    assert_eq!(
        recorded_sol_delta,
        buy_amount,
        "recorded SOL reserve must increase by the BUY input"
    );

    assert_eq!(
        actual_sol_delta,
        recorded_sol_delta,
        "recorded SOL reserve must match actual SOL movement"
    );

    assert_eq!(
        actual_mlab_delta,
        recorded_mlab_delta,
        "recorded MLAB reserve must match actual vault movement"
    );

    assert!(
        actual_mlab_delta > 0,
        "BUY must transfer a positive amount of MLAB to the trader"
    );
}

#[test]
fn test_swap_mlab_for_sol_pool_accounting_invariant() {
    use anchor_lang::solana_program::system_instruction;

    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };
    use anchor_spl::token_2022::spl_token_2022::state::Account as SplTokenAccount;
    use anchor_lang::solana_program::program_pack::Pack;

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // 1. Create Token-2022 mint.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint, init_mint],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 2. Create pool source account.
    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source, init_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 3. Create trader MLAB account.
    let create_trader_account = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_account = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_account, init_trader_account],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 4. Mint MLAB for pool funding.
    let mint_to = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 5. Initialize pool.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 6. Create vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 7. Seed the pool.
    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 500_000_000,
            sol_amount: 1_000_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 8. BUY first so the trader owns MLAB to sell.
    let buy_amount = 10_000_000u64;

    let buy_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: buy_amount,
            min_mlab_out: 1,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[buy_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let trader_account = svm.get_account(&trader_mlab.pubkey()).unwrap();
    let trader_mlab_balance =
        u64::from_le_bytes(
            trader_account.data[64..72]
                .try_into()
                .unwrap(),
        );

    assert!(trader_mlab_balance > 0);

    // 9. Capture pool/vault state immediately before SELL.
    let pool_before_account = svm.get_account(&pool).unwrap();
    let vault_before_account = svm.get_account(&vault).unwrap();

    let mut pool_before_data = pool_before_account.data.as_slice();

    let pool_before_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_before_data,
        )
        .unwrap();

    let vault_before_state =
        SplTokenAccount::unpack(&vault_before_account.data).unwrap();

    let sell_amount = trader_mlab_balance;

    let expected_sol_out = memelab_amm::math::amount_out(
        sell_amount,
        pool_before_state.mlab_reserve,
        pool_before_state.sol_reserve,
        30,
    )
    .unwrap();

    assert!(expected_sol_out > 0);

    // 10. SELL MLAB for SOL.
    let sell_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: sell_amount,
            min_sol_out: expected_sol_out,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[sell_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 11. Capture pool/vault state immediately after SELL.
    let pool_after_account = svm.get_account(&pool).unwrap();
    let vault_after_account = svm.get_account(&vault).unwrap();

    let mut pool_after_data = pool_after_account.data.as_slice();

    let pool_after_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_after_data,
        )
        .unwrap();

    let vault_after_state =
        SplTokenAccount::unpack(&vault_after_account.data).unwrap();

    let actual_sol_delta =
        pool_before_account.lamports - pool_after_account.lamports;

    let recorded_sol_delta =
        pool_before_state.sol_reserve - pool_after_state.sol_reserve;

    let actual_mlab_delta =
        vault_after_state.amount - vault_before_state.amount;

    let recorded_mlab_delta =
        pool_after_state.mlab_reserve - pool_before_state.mlab_reserve;

    let k_before =
        pool_before_state.sol_reserve as u128 * pool_before_state.mlab_reserve as u128;

    let k_after =
        pool_after_state.sol_reserve as u128 * pool_after_state.mlab_reserve as u128;

    assert!(
        k_after >= k_before,
        "SELL constant-product invariant violated: before={}, after={}",
        k_before,
        k_after
    );

    assert_eq!(
        actual_sol_delta,
        expected_sol_out,
        "actual pool SOL balance must decrease by the SELL output"
    );

    assert_eq!(
        recorded_sol_delta,
        expected_sol_out,
        "recorded SOL reserve must decrease by the SELL output"
    );

    assert_eq!(
        actual_sol_delta,
        recorded_sol_delta,
        "recorded SOL reserve must match actual SOL movement"
    );

    assert_eq!(
        actual_mlab_delta,
        recorded_mlab_delta,
        "recorded MLAB reserve must match actual vault movement"
    );

    assert_eq!(
        actual_mlab_delta,
        sell_amount,
        "actual vault MLAB balance must increase by the SELL input"
    );
}

#[test]
fn test_swap_sol_for_mlab_rejects_slippage() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source Token-2022 account.
    // ------------------------------------------------------------

    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint MLAB to source.
    // ------------------------------------------------------------

    let funding_amount = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        funding_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund pool.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;
    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader Token-2022 account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. Calculate the real expected BUY output.
    // ------------------------------------------------------------

    let sol_in = 1_000_000_000u64;

    let expected_mlab = memelab_amm::math::amount_out(
        sol_in,
        pool_sol,
        pool_mlab,
        30,
    )
    .unwrap();

    assert!(expected_mlab > 0);

    // ------------------------------------------------------------
    // 9. Demand MORE than the pool can actually provide.
    // ------------------------------------------------------------

    let impossible_minimum = expected_mlab
        .checked_add(1)
        .unwrap();

    let swap_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: sol_in,
            min_mlab_out: impossible_minimum,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[swap_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    // ------------------------------------------------------------
    // 10. Swap MUST be rejected.
    // ------------------------------------------------------------

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "swap unexpectedly succeeded despite impossible slippage minimum"
    );

    // ------------------------------------------------------------
    // 11. Verify trader received NO MLAB.
    // ------------------------------------------------------------

    let trader_token_after =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_after =
        u64::from_le_bytes(
            trader_token_after.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_after, 0);

    // ------------------------------------------------------------
    // 12. Verify pool reserves did NOT change.
    // ------------------------------------------------------------

    let pool_account =
        svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] =
        &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(pool_state.sol_reserve, pool_sol);
    assert_eq!(pool_state.mlab_reserve, pool_mlab);
}




#[test]
fn test_swap_sol_for_mlab_rejects_unauthorized_destination() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let attacker = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let attacker_destination = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 5_000_000_000).unwrap();

    // Create MLAB mint.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint, init_mint],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create legitimate trader source account.
    let source_space = 165usize;
    let source_lamports =
        svm.minimum_balance_for_rent_exemption(source_space);

    let create_source = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        source_lamports,
        source_space as u64,
        &token_2022::ID,
    );

    let init_source = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source, init_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create attacker-controlled destination account.
    let destination_lamports =
        svm.minimum_balance_for_rent_exemption(source_space);

    let create_destination = system_instruction::create_account(
        &payer.pubkey(),
        &attacker_destination.pubkey(),
        destination_lamports,
        source_space as u64,
        &token_2022::ID,
    );

    let init_destination = initialize_account3(
        &token_2022::ID,
        &attacker_destination.pubkey(),
        &mint.pubkey(),
        &attacker.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_destination, init_destination],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &attacker_destination],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Give the legitimate trader MLAB so the destination/source accounts
    // are both valid Token-2022 accounts for the same mint.
    let mint_to = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize pool.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Fund the pool so a BUY has liquidity.
    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 500_000_000,
            sol_amount: 1_000_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_before = svm.get_account(&pool).unwrap();
    let destination_before =
        svm.get_account(&attacker_destination.pubkey()).unwrap();

    // Attempt a BUY while directing the MLAB output into an account
    // controlled by the attacker instead of the trader.
    let swap_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: 10_000_000,
            min_mlab_out: 1,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: attacker_destination.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[swap_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    assert!(
        svm.send_transaction(tx).is_err(),
        "swap_sol_for_mlab should reject an unauthorized destination account"
    );

    // Verify the rejected swap changed nothing.
    let pool_after = svm.get_account(&pool).unwrap();
    let destination_after =
        svm.get_account(&attacker_destination.pubkey()).unwrap();

    assert_eq!(pool_before.lamports, pool_after.lamports);
    assert_eq!(pool_before.data, pool_after.data);
    assert_eq!(
        destination_before.lamports,
        destination_after.lamports
    );
    assert_eq!(destination_before.data, destination_after.data);
}

#[test]
fn test_swap_sol_for_mlab_rejects_zero_amount() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source Token-2022 account.
    // ------------------------------------------------------------

    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint MLAB to source.
    // ------------------------------------------------------------

    let funding_amount = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        funding_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund pool.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;
    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader Token-2022 account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. Attempt zero-value BUY.
    // ------------------------------------------------------------

    let swap_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: 0,
            min_mlab_out: 0,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[swap_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    // ------------------------------------------------------------
    // 9. Zero-value swap MUST be rejected.
    // ------------------------------------------------------------

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "zero-value SOL -> MLAB swap unexpectedly succeeded"
    );

    // ------------------------------------------------------------
    // 10. Verify trader received no MLAB.
    // ------------------------------------------------------------

    let trader_token_after =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_after =
        u64::from_le_bytes(
            trader_token_after.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_after, 0);

    // ------------------------------------------------------------
    // 11. Verify pool reserves remain unchanged.
    // ------------------------------------------------------------

    let pool_account =
        svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] =
        &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(pool_state.sol_reserve, pool_sol);
    assert_eq!(pool_state.mlab_reserve, pool_mlab);
}



#[test]
fn test_swap_mlab_for_sol_rejects_insufficient_liquidity() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source Token-2022 account.
    // ------------------------------------------------------------

    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint enough MLAB for the trader to exceed the pool.
    // ------------------------------------------------------------

    let trader_amount = 2_000_000_000_000u64;
    let mint_amount = 3_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        mint_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund pool with a smaller MLAB reserve.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;
    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader Token-2022 account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. Move 2x the pool's MLAB reserve to the trader.
    // ------------------------------------------------------------

    let transfer_ix = anchor_spl::token_2022::spl_token_2022::instruction::transfer_checked(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &trader_mlab.pubkey(),
        &payer.pubkey(),
        &[],
        trader_amount,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[transfer_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 9. Attempt to sell more MLAB than the pool contains.
    // ------------------------------------------------------------

    let mlab_in = trader_amount;

    let expected_sol = memelab_amm::math::amount_out(
        mlab_in,
        pool_mlab,
        pool_sol,
        30,
    )
    .unwrap();

    assert!(expected_sol > 0);

    let swap_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: mlab_in,
            min_sol_out: 0,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[swap_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    // ------------------------------------------------------------
    // 10. Swap MUST be rejected.
    // ------------------------------------------------------------

    let result = svm.send_transaction(tx);

    assert!(
        result.is_err(),
        "SELL unexpectedly succeeded despite insufficient pool liquidity"
    );

    // ------------------------------------------------------------
    // 11. Verify pool reserves did NOT change.
    // ------------------------------------------------------------

    let pool_account =
        svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] =
        &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(pool_state.sol_reserve, pool_sol);
    assert_eq!(pool_state.mlab_reserve, pool_mlab);

    // ------------------------------------------------------------
    // 12. Verify trader still owns all of the MLAB.
    // ------------------------------------------------------------

    let trader_token_after =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_after =
        u64::from_le_bytes(
            trader_token_after.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_after, trader_amount);
}


#[test]
fn test_swap_mlab_for_sol() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();

    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source Token-2022 account.
    // ------------------------------------------------------------

    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint MLAB for pool funding.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        pool_mlab,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create AMM vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund pool.
    // ------------------------------------------------------------

    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader's Token-2022 account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. BUY MLAB first.
    // ------------------------------------------------------------

    let sol_in = 1_000_000_000u64;

    let mlab_bought = memelab_amm::math::amount_out(
        sol_in,
        pool_sol,
        pool_mlab,
        30,
    )
    .unwrap();

    let buy_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: sol_in,
            min_mlab_out: mlab_bought,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[buy_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let trader_token_after_buy =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_balance =
        u64::from_le_bytes(
            trader_token_after_buy.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_balance, mlab_bought);

    // ------------------------------------------------------------
    // 9. Calculate expected SELL output.
    // ------------------------------------------------------------

    let sell_amount = mlab_bought;

    let expected_sol_out = memelab_amm::math::amount_out(
        sell_amount,
        pool_mlab - mlab_bought,
        pool_sol + sol_in,
        30,
    )
    .unwrap();

    assert!(expected_sol_out > 0);
    assert!(expected_sol_out < pool_sol + sol_in);

    // ------------------------------------------------------------
    // 10. Execute MLAB -> SOL SELL.
    // ------------------------------------------------------------

    let trader_sol_before =
        svm.get_account(&trader.pubkey()).unwrap().lamports;

    let sell_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: sell_amount,
            min_sol_out: expected_sol_out,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[sell_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    assert!(
        result.is_ok(),
        "MLAB -> SOL swap failed: {:?}",
        result.err()
    );

    // ------------------------------------------------------------
    // 11. Verify trader received SOL.
    // ------------------------------------------------------------

    let trader_sol_after =
        svm.get_account(&trader.pubkey()).unwrap().lamports;

    // Transaction fee is also deducted, so the balance increase
    // must be slightly less than the raw swap output.
    assert!(
        trader_sol_after > trader_sol_before,
        "Trader did not receive SOL"
    );

    assert!(
        trader_sol_after - trader_sol_before <= expected_sol_out,
        "Trader received more SOL than the calculated swap output"
    );

    // ------------------------------------------------------------
    // 12. Verify trader MLAB was consumed.
    // ------------------------------------------------------------

    let trader_token_after_sell =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_after_sell =
        u64::from_le_bytes(
            trader_token_after_sell.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_after_sell, 0);

    // ------------------------------------------------------------
    // 13. Verify final pool reserves.
    // ------------------------------------------------------------

    let pool_account =
        svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] =
        &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(
        pool_state.mlab_reserve,
        pool_mlab
    );

    assert_eq!(
        pool_state.sol_reserve,
        pool_sol + sol_in - expected_sol_out
    );
}



#[test]
fn test_swap_mlab_for_sol_rejects_unauthorized_source() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let attacker = Keypair::new();
    let mint = Keypair::new();
    let legitimate_source = Keypair::new();
    let attacker_source = Keypair::new();
    let trader_destination = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 5_000_000_000).unwrap();

    // Create MLAB mint.
    let mint_space = 82usize;
    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint, init_mint],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create legitimate source account owned by the payer.
    let token_account_space = 165usize;
    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source = system_instruction::create_account(
        &payer.pubkey(),
        &legitimate_source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source = initialize_account3(
        &token_2022::ID,
        &legitimate_source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source, init_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &legitimate_source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create attacker-controlled MLAB source account.
    let create_attacker_source = system_instruction::create_account(
        &payer.pubkey(),
        &attacker_source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_attacker_source = initialize_account3(
        &token_2022::ID,
        &attacker_source.pubkey(),
        &mint.pubkey(),
        &attacker.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_attacker_source, init_attacker_source],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &attacker_source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create the trader's SOL destination account.
    let destination_lamports =
        svm.minimum_balance_for_rent_exemption(0);

    let create_destination = system_instruction::create_account(
        &payer.pubkey(),
        &trader_destination.pubkey(),
        destination_lamports,
        0,
        &system_program::ID,
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_destination],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &trader_destination],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Give the legitimate source MLAB.
    let mint_to = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &legitimate_source.pubkey(),
        &payer.pubkey(),
        &[],
        1_000_000_000,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Initialize pool.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Create vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // Fund pool so the SELL has SOL liquidity.
    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: 500_000_000,
            sol_amount: 1_000_000_000,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: legitimate_source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let pool_before = svm.get_account(&pool).unwrap();
    let attacker_source_before =
        svm.get_account(&attacker_source.pubkey()).unwrap();

    // Attempt SELL using an attacker-controlled source account while
    // signing as the legitimate trader.
    let sell_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: 10_000_000,
            min_sol_out: 1,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: attacker_source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[sell_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    assert!(
        svm.send_transaction(tx).is_err(),
        "swap_mlab_for_sol should reject an unauthorized source account"
    );

    // Verify the rejected SELL changed nothing.
    let pool_after = svm.get_account(&pool).unwrap();
    let attacker_source_after =
        svm.get_account(&attacker_source.pubkey()).unwrap();

    assert_eq!(pool_before.lamports, pool_after.lamports);
    assert_eq!(pool_before.data, pool_after.data);
    assert_eq!(
        attacker_source_before.lamports,
        attacker_source_after.lamports
    );
    assert_eq!(
        attacker_source_before.data,
        attacker_source_after.data
    );
}

#[test]
fn test_swap_mlab_for_sol_rejects_slippage() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();
    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();
    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    // ------------------------------------------------------------
    // 1. Create mint.
    // ------------------------------------------------------------

    let mint_space =
        82usize;

    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create source account.
    // ------------------------------------------------------------

    let token_account_space =
        165usize;

    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint MLAB for pool funding.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        pool_mlab,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create AMM vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund pool.
    // ------------------------------------------------------------

    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader Token-2022 account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. BUY MLAB first.
    // ------------------------------------------------------------

    let sol_in = 1_000_000_000u64;

    let mlab_bought = memelab_amm::math::amount_out(
        sol_in,
        pool_sol,
        pool_mlab,
        30,
    )
    .unwrap();

    let buy_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: sol_in,
            min_mlab_out: mlab_bought,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[buy_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 9. Calculate legitimate SELL output.
    // ------------------------------------------------------------

    let sell_amount = mlab_bought;

    let expected_sol_out = memelab_amm::math::amount_out(
        sell_amount,
        pool_mlab - mlab_bought,
        pool_sol + sol_in,
        30,
    )
    .unwrap();

    assert!(expected_sol_out > 0);

    // ------------------------------------------------------------
    // 10. Demand one lamport more than possible.
    // ------------------------------------------------------------

    let impossible_min_sol_out = expected_sol_out
        .checked_add(1)
        .unwrap();

    let sell_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: sell_amount,
            min_sol_out: impossible_min_sol_out,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[sell_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    let result = svm.send_transaction(tx);

    // ------------------------------------------------------------
    // 11. Slippage protection must reject the SELL.
    // ------------------------------------------------------------

    assert!(
        result.is_err(),
        "SELL should have been rejected by slippage protection"
    );

    let error = format!("{:?}", result.err().unwrap());

    assert!(
        error.contains("SlippageExceeded"),
        "Expected SlippageExceeded, got: {}",
        error
    );
}


#[test]
fn test_repeated_buy_sell_stress() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();

    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 100_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 100_000_000_000)
        .unwrap();

    let mint_space = 82usize;
    let token_account_space = 165usize;

    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    // 1. Create Token-2022 mint.
    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 2. Create pool funding token account.
    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 3. Mint MLAB for pool funding.
    let pool_mlab = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        pool_mlab,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 4. Initialize pool.
    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 5. Create vault.
    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 6. Fund pool.
    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 7. Create trader MLAB account.
    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // 8. Repeated BUY -> SELL cycles.
    let mut expected_sol_reserve = pool_sol;

    for cycle in 1..=10 {
        let sol_in = 100_000_000u64;

        let mlab_bought = memelab_amm::math::amount_out(
            sol_in,
            expected_sol_reserve,
            pool_mlab,
            30,
        )
        .unwrap();

        let buy_ix = Instruction::new_with_bytes(
            program_id,
            &memelab_amm::instruction::SwapSolForMlab {
                sol_amount: sol_in,
                min_mlab_out: mlab_bought,
            }
            .data(),
            memelab_amm::accounts::SwapSolForMlab {
                trader: trader.pubkey(),
                pool,
                mlab_mint,
                mlab_vault: vault,
                trader_mlab_account: trader_mlab.pubkey(),
                token_program: token_2022::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        );

        let blockhash = svm.latest_blockhash();

        let msg = Message::new_with_blockhash(
            &[buy_ix],
            Some(&trader.pubkey()),
            &blockhash,
        );

        let tx = VersionedTransaction::try_new(
            VersionedMessage::Legacy(msg),
            &[&trader],
        )
        .unwrap();

        svm.send_transaction(tx).unwrap();

        let trader_token_after_buy =
            svm.get_account(&trader_mlab.pubkey()).unwrap();

        let trader_mlab_after_buy =
            u64::from_le_bytes(
                trader_token_after_buy.data[64..72]
                    .try_into()
                    .unwrap()
            );

        assert_eq!(
            trader_mlab_after_buy,
            mlab_bought,
            "cycle {cycle}: BUY output mismatch"
        );

        let expected_sol_out = memelab_amm::math::amount_out(
            mlab_bought,
            pool_mlab - mlab_bought,
            expected_sol_reserve + sol_in,
            30,
        )
        .unwrap();

        let sell_ix = Instruction::new_with_bytes(
            program_id,
            &memelab_amm::instruction::SwapMlabForSol {
                mlab_amount: mlab_bought,
                min_sol_out: expected_sol_out,
            }
            .data(),
            memelab_amm::accounts::SwapMlabForSol {
                trader: trader.pubkey(),
                pool,
                mlab_mint,
                mlab_vault: vault,
                trader_mlab_account: trader_mlab.pubkey(),
                token_program: token_2022::ID,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        );

        let blockhash = svm.latest_blockhash();

        let msg = Message::new_with_blockhash(
            &[sell_ix],
            Some(&trader.pubkey()),
            &blockhash,
        );

        let tx = VersionedTransaction::try_new(
            VersionedMessage::Legacy(msg),
            &[&trader],
        )
        .unwrap();

        svm.send_transaction(tx).unwrap();

        let trader_token_after_sell =
            svm.get_account(&trader_mlab.pubkey()).unwrap();

        let trader_mlab_after_sell =
            u64::from_le_bytes(
                trader_token_after_sell.data[64..72]
                    .try_into()
                    .unwrap()
            );

        assert_eq!(
            trader_mlab_after_sell,
            0,
            "cycle {cycle}: trader retained MLAB"
        );

        expected_sol_reserve =
            expected_sol_reserve
                .checked_add(sol_in)
                .unwrap()
                .checked_sub(expected_sol_out)
                .unwrap();

        let pool_account =
            svm.get_account(&pool).unwrap();

        let mut pool_data: &[u8] =
            &pool_account.data;

        let pool_state =
            memelab_amm::state::Pool::try_deserialize(
                &mut pool_data
            )
            .unwrap();

        assert_eq!(
            pool_state.mlab_reserve,
            pool_mlab,
            "cycle {cycle}: MLAB reserve changed"
        );

        assert_eq!(
            pool_state.sol_reserve,
            expected_sol_reserve,
            "cycle {cycle}: SOL reserve mismatch"
        );

        let vault_account =
            svm.get_account(&vault).unwrap();

        let vault_balance =
            u64::from_le_bytes(
                vault_account.data[64..72]
                    .try_into()
                    .unwrap()
            );

        assert_eq!(
            vault_balance,
            pool_state.mlab_reserve,
            "cycle {cycle}: vault/pool mismatch"
        );
    }
}


#[test]
fn test_full_swap_round_trip() {
    use anchor_lang::solana_program::system_instruction;
    use anchor_spl::token_2022;
    use anchor_spl::token_2022::spl_token_2022::instruction::{
        initialize_account3,
        initialize_mint2,
        mint_to_checked,
    };

    let program_id = memelab_amm::id();

    let payer = Keypair::new();
    let mint = Keypair::new();
    let source = Keypair::new();
    let trader = Keypair::new();
    let trader_mlab = Keypair::new();

    let mlab_mint = mint.pubkey();

    let (pool, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::POOL_SEED,
            mlab_mint.as_ref(),
        ],
        &program_id,
    );

    let (vault, _) = Pubkey::find_program_address(
        &[
            memelab_amm::constants::VAULT_SEED,
            pool.as_ref(),
        ],
        &program_id,
    );

    let mut svm = LiteSVM::new();

    let amm_bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/memelab_amm.so"
    ));

    svm.add_program(program_id, amm_bytes).unwrap();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&trader.pubkey(), 5_000_000_000)
        .unwrap();

    let mint_space = 82usize;
    let token_account_space = 165usize;

    let mint_lamports =
        svm.minimum_balance_for_rent_exemption(mint_space);

    let token_account_lamports =
        svm.minimum_balance_for_rent_exemption(token_account_space);

    // ------------------------------------------------------------
    // 1. Create Token-2022 mint.
    // ------------------------------------------------------------

    let create_mint_ix = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        mint_lamports,
        mint_space as u64,
        &token_2022::ID,
    );

    let init_mint_ix = initialize_mint2(
        &token_2022::ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_mint_ix, init_mint_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &mint],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 2. Create pool funding token account.
    // ------------------------------------------------------------

    let create_source_ix = system_instruction::create_account(
        &payer.pubkey(),
        &source.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_source_ix = initialize_account3(
        &token_2022::ID,
        &source.pubkey(),
        &mint.pubkey(),
        &payer.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_source_ix, init_source_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer, &source],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 3. Mint MLAB for pool funding.
    // ------------------------------------------------------------

    let pool_mlab = 1_000_000_000_000u64;

    let mint_to_ix = mint_to_checked(
        &token_2022::ID,
        &mint.pubkey(),
        &source.pubkey(),
        &payer.pubkey(),
        &[],
        pool_mlab,
        9,
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[mint_to_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 4. Initialize pool.
    // ------------------------------------------------------------

    let initialize_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::Initialize {
            fee_bps: 30,
        }
        .data(),
        memelab_amm::accounts::Initialize {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[initialize_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 5. Create vault.
    // ------------------------------------------------------------

    let create_vault_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::CreateVault {}.data(),
        memelab_amm::accounts::CreateVault {
            payer: payer.pubkey(),
            mlab_mint,
            pool,
            vault,
            authority: payer.pubkey(),
            system_program: system_program::ID,
            token_program: token_2022::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_vault_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 6. Fund pool.
    // ------------------------------------------------------------

    let pool_sol = 5_000_000_000u64;

    let fund_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::FundPool {
            mlab_amount: pool_mlab,
            sol_amount: pool_sol,
        }
        .data(),
        memelab_amm::accounts::FundPool {
            authority: payer.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            mlab_source: source.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[fund_ix],
        Some(&payer.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&payer],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 7. Create trader MLAB account.
    // ------------------------------------------------------------

    let create_trader_token_ix = system_instruction::create_account(
        &trader.pubkey(),
        &trader_mlab.pubkey(),
        token_account_lamports,
        token_account_space as u64,
        &token_2022::ID,
    );

    let init_trader_token_ix = initialize_account3(
        &token_2022::ID,
        &trader_mlab.pubkey(),
        &mint.pubkey(),
        &trader.pubkey(),
    )
    .unwrap();

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[create_trader_token_ix, init_trader_token_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader, &trader_mlab],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 8. Record initial trader SOL.
    // ------------------------------------------------------------

    let trader_sol_initial =
        svm.get_account(&trader.pubkey()).unwrap().lamports;

    // ------------------------------------------------------------
    // 9. BUY: SOL -> MLAB.
    // ------------------------------------------------------------

    let sol_in = 1_000_000_000u64;

    let mlab_bought = memelab_amm::math::amount_out(
        sol_in,
        pool_sol,
        pool_mlab,
        30,
    )
    .unwrap();

    let buy_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapSolForMlab {
            sol_amount: sol_in,
            min_mlab_out: mlab_bought,
        }
        .data(),
        memelab_amm::accounts::SwapSolForMlab {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[buy_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    let trader_token_after_buy =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_after_buy =
        u64::from_le_bytes(
            trader_token_after_buy.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(
        trader_mlab_after_buy,
        mlab_bought
    );

    // ------------------------------------------------------------
    // 10. SELL: all purchased MLAB -> SOL.
    // ------------------------------------------------------------

    let expected_sol_out = memelab_amm::math::amount_out(
        mlab_bought,
        pool_mlab - mlab_bought,
        pool_sol + sol_in,
        30,
    )
    .unwrap();

    let sell_ix = Instruction::new_with_bytes(
        program_id,
        &memelab_amm::instruction::SwapMlabForSol {
            mlab_amount: mlab_bought,
            min_sol_out: expected_sol_out,
        }
        .data(),
        memelab_amm::accounts::SwapMlabForSol {
            trader: trader.pubkey(),
            pool,
            mlab_mint,
            mlab_vault: vault,
            trader_mlab_account: trader_mlab.pubkey(),
            token_program: token_2022::ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    );

    let blockhash = svm.latest_blockhash();

    let msg = Message::new_with_blockhash(
        &[sell_ix],
        Some(&trader.pubkey()),
        &blockhash,
    );

    let tx = VersionedTransaction::try_new(
        VersionedMessage::Legacy(msg),
        &[&trader],
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();

    // ------------------------------------------------------------
    // 11. Trader should now hold zero MLAB.
    // ------------------------------------------------------------

    let trader_token_final =
        svm.get_account(&trader_mlab.pubkey()).unwrap();

    let trader_mlab_final =
        u64::from_le_bytes(
            trader_token_final.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(trader_mlab_final, 0);

    // ------------------------------------------------------------
    // 12. Trader received SOL, after transaction fees.
    // ------------------------------------------------------------

    let trader_sol_final =
        svm.get_account(&trader.pubkey()).unwrap().lamports;

    assert!(trader_sol_final > trader_sol_initial - sol_in);

    // The round trip cannot return more SOL than was put in.
    assert!(
        trader_sol_final < trader_sol_initial
    );

    // ------------------------------------------------------------
    // 13. Verify final pool reserves.
    // ------------------------------------------------------------

    let pool_account =
        svm.get_account(&pool).unwrap();

    let mut pool_data: &[u8] =
        &pool_account.data;

    let pool_state =
        memelab_amm::state::Pool::try_deserialize(
            &mut pool_data
        )
        .unwrap();

    assert_eq!(
        pool_state.mlab_reserve,
        pool_mlab
    );

    assert_eq!(
        pool_state.sol_reserve,
        pool_sol + sol_in - expected_sol_out
    );

    // ------------------------------------------------------------
    // 14. Verify the vault contains the recorded MLAB reserve.
    // ------------------------------------------------------------

    let vault_account =
        svm.get_account(&vault).unwrap();

    let vault_balance =
        u64::from_le_bytes(
            vault_account.data[64..72]
                .try_into()
                .unwrap()
        );

    assert_eq!(
        vault_balance,
        pool_state.mlab_reserve
    );
}
