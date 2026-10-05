use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022,
    token_interface::Mint,
};

use crate::{
    constants::{POOL_SEED, VAULT_SEED},
    error::AmmError,
    state::Pool,
};

#[derive(Accounts)]
pub struct CreateVault<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    pub mlab_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [POOL_SEED, mlab_mint.key().as_ref()],
        bump = pool.bump,
        has_one = authority
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: This PDA is created and owned by Token-2022 below.
    #[account(
        mut,
        seeds = [VAULT_SEED, pool.key().as_ref()],
        bump
    )]
    pub vault: UncheckedAccount<'info>,

    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,

    pub token_program: Program<'info, token_2022::Token2022>,
}

pub fn handle_create_vault(ctx: Context<CreateVault>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.pool.mlab_mint,
        ctx.accounts.mlab_mint.key(),
        AmmError::InvalidMint
    );

    let vault_bump = ctx.bumps.vault;
    let pool_key = ctx.accounts.pool.key();

    let vault_seeds: &[&[u8]] = &[
        VAULT_SEED,
        pool_key.as_ref(),
        &[vault_bump],
    ];

    let size = token_2022::get_account_data_size(
        CpiContext::new(
            token_2022::ID,
            token_2022::GetAccountDataSize {
                mint: ctx.accounts.mlab_mint.to_account_info(),
            },
        ),
        &[],
    )?;

    let rent = Rent::get()?.minimum_balance(size as usize);

    anchor_lang::system_program::create_account(
        CpiContext::new_with_signer(
            anchor_lang::system_program::ID,
            anchor_lang::system_program::CreateAccount {
                from: ctx.accounts.payer.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
            },
            &[vault_seeds],
        ),
        rent,
        size,
        &token_2022::ID,
    )?;

    token_2022::initialize_account3(
        CpiContext::new(
            token_2022::ID,
            token_2022::InitializeAccount3 {
                account: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mlab_mint.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
        ),
    )?;

    ctx.accounts.pool.mlab_vault = ctx.accounts.vault.key();

    Ok(())
}
