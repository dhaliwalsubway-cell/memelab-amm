use anchor_lang::prelude::*;
use anchor_spl::{
    token_2022,
    token_interface::{Mint, TokenAccount},
};

use crate::{
    constants::{POOL_SEED, VAULT_SEED},
    error::AmmError,
    math::amount_out,
    state::Pool,
};

#[derive(Accounts)]
pub struct SwapSolForMlab<'info> {
    #[account(mut)]
    pub trader: Signer<'info>,

    #[account(
        mut,
        seeds = [POOL_SEED, mlab_mint.key().as_ref()],
        bump = pool.bump,
        has_one = mlab_mint,
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
        token::authority = trader,
        token::token_program = token_2022::ID,
    )]
    pub trader_mlab_account: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Program<'info, token_2022::Token2022>,
    pub system_program: Program<'info, System>,
}

pub fn handle_swap_sol_for_mlab(
    ctx: Context<SwapSolForMlab>,
    sol_amount: u64,
    min_mlab_out: u64,
) -> Result<()> {
    require!(sol_amount > 0, AmmError::ZeroAmount);

    let mlab_out = amount_out(
        sol_amount,
        ctx.accounts.pool.sol_reserve,
        ctx.accounts.pool.mlab_reserve,
        ctx.accounts.pool.fee_bps,
    )
    .map_err(|err| {
        msg!("Swap calculation failed: {:?}", err);
        err
    })?;

    require!(
        mlab_out >= min_mlab_out,
        AmmError::SlippageExceeded
    );

    require!(
        mlab_out < ctx.accounts.pool.mlab_reserve,
        AmmError::InsufficientLiquidity
    );

    let transfer_sol = anchor_lang::system_program::transfer(
        CpiContext::new(
            anchor_lang::solana_program::system_program::ID,
            anchor_lang::system_program::Transfer {
                from: ctx.accounts.trader.to_account_info(),
                to: ctx.accounts.pool.to_account_info(),
            },
        ),
        sol_amount,
    );

    transfer_sol?;

    let mlab_mint_key = ctx.accounts.mlab_mint.key();

    let signer_seeds: &[&[u8]] = &[
        POOL_SEED,
        mlab_mint_key.as_ref(),
        &[ctx.accounts.pool.bump],
    ];

    let transfer_mlab = token_2022::transfer_checked(
        CpiContext::new_with_signer(
            token_2022::ID,
            token_2022::TransferChecked {
                from: ctx.accounts.mlab_vault.to_account_info(),
                mint: ctx.accounts.mlab_mint.to_account_info(),
                to: ctx.accounts.trader_mlab_account.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            &[signer_seeds],
        ),
        mlab_out,
        ctx.accounts.mlab_mint.decimals,
    );

    transfer_mlab?;

    ctx.accounts.pool.sol_reserve = ctx
        .accounts
        .pool
        .sol_reserve
        .checked_add(sol_amount)
        .ok_or(AmmError::ReserveOverflow)?;

    ctx.accounts.pool.mlab_reserve = ctx
        .accounts
        .pool
        .mlab_reserve
        .checked_sub(mlab_out)
        .ok_or(AmmError::InsufficientLiquidity)?;

    msg!(
        "SOL -> MLAB swap: {} lamports -> {} MLAB base units",
        sol_amount,
        mlab_out
    );

    Ok(())
}
