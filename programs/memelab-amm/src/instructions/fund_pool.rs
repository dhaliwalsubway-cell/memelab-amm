use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    constants::{POOL_SEED, VAULT_SEED},
    error::AmmError,
    state::Pool,
};

#[derive(Accounts)]
pub struct FundPool<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [POOL_SEED, mlab_mint.key().as_ref()],
        bump = pool.bump,
        has_one = authority,
        has_one = mlab_vault,
    )]
    pub pool: Account<'info, Pool>,

    pub mlab_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        seeds = [VAULT_SEED, pool.key().as_ref()],
        bump,
        token::mint = mlab_mint,
        token::authority = pool,
        token::token_program = token_2022::ID,
    )]
    pub mlab_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = mlab_mint,
        token::authority = authority,
        token::token_program = token_2022::ID,
    )]
    pub mlab_source: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, token_2022::Token2022>,

    pub system_program: Program<'info, System>,
}

pub fn handle_fund_pool(
    ctx: Context<FundPool>,
    mlab_amount: u64,
    sol_amount: u64,
) -> Result<()> {
    require!(mlab_amount > 0, AmmError::ZeroAmount);
    require!(sol_amount > 0, AmmError::ZeroAmount);

    let transfer_mlab = token_2022::transfer_checked(
        CpiContext::new(
            token_2022::ID,
            token_2022::TransferChecked {
                from: ctx.accounts.mlab_source.to_account_info(),
                mint: ctx.accounts.mlab_mint.to_account_info(),
                to: ctx.accounts.mlab_vault.to_account_info(),
                authority: ctx.accounts.authority.to_account_info(),
            },
        ),
        mlab_amount,
        ctx.accounts.mlab_mint.decimals,
    );

    transfer_mlab?;

    let transfer_sol = anchor_lang::system_program::transfer(
        CpiContext::new(
            anchor_lang::solana_program::system_program::ID,
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.pool.to_account_info(),
            },
        ),
        sol_amount,
    );

    transfer_sol?;

    ctx.accounts.pool.mlab_reserve = ctx
        .accounts
        .pool
        .mlab_reserve
        .checked_add(mlab_amount)
        .ok_or(AmmError::ReserveOverflow)?;

    ctx.accounts.pool.sol_reserve = ctx
        .accounts
        .pool
        .sol_reserve
        .checked_add(sol_amount)
        .ok_or(AmmError::ReserveOverflow)?;

    msg!(
        "Pool funded: {} MLAB base units + {} lamports",
        mlab_amount,
        sol_amount
    );

    Ok(())
}
