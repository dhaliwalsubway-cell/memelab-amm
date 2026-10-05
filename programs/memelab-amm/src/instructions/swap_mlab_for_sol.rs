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
pub struct SwapMlabForSol<'info> {
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

pub fn handle_swap_mlab_for_sol(
    ctx: Context<SwapMlabForSol>,
    mlab_amount: u64,
    min_sol_out: u64,
) -> Result<()> {
    require!(mlab_amount > 0, AmmError::ZeroAmount);
    require!(
        mlab_amount <= ctx.accounts.pool.mlab_reserve,
        AmmError::InsufficientLiquidity
    );


    let sol_out = amount_out(
        mlab_amount,
        ctx.accounts.pool.mlab_reserve,
        ctx.accounts.pool.sol_reserve,
        ctx.accounts.pool.fee_bps,
    )
    .map_err(|err| {
        msg!("Swap calculation failed: {:?}", err);
        err
    })?;

    require!(
        sol_out >= min_sol_out,
        AmmError::SlippageExceeded
    );

    require!(
        sol_out < ctx.accounts.pool.sol_reserve,
        AmmError::InsufficientLiquidity
    );

    // Trader sends MLAB into the AMM vault.
    let transfer_mlab = token_2022::transfer_checked(
        CpiContext::new(
            token_2022::ID,
            token_2022::TransferChecked {
                from: ctx.accounts.trader_mlab_account.to_account_info(),
                mint: ctx.accounts.mlab_mint.to_account_info(),
                to: ctx.accounts.mlab_vault.to_account_info(),
                authority: ctx.accounts.trader.to_account_info(),
            },
        ),
        mlab_amount,
        ctx.accounts.mlab_mint.decimals,
    );

    transfer_mlab?;

    // AMM sends SOL from the pool PDA to the trader.
    //
    // The pool is a data-bearing Anchor account, so the System Program's
    // normal `transfer` instruction cannot debit it. Move the lamports
    // directly between the program-owned accounts instead.
    let pool_info = ctx.accounts.pool.to_account_info();
    let trader_info = ctx.accounts.trader.to_account_info();

    let pool_lamports = pool_info.lamports();
    require!(
        pool_lamports >= sol_out,
        AmmError::InsufficientLiquidity
    );

    **pool_info.try_borrow_mut_lamports()? -= sol_out;
    **trader_info.try_borrow_mut_lamports()? += sol_out;

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
        .checked_sub(sol_out)
        .ok_or(AmmError::InsufficientLiquidity)?;

    msg!(
        "MLAB -> SOL swap: {} MLAB base units -> {} lamports",
        mlab_amount,
        sol_out
    );

    Ok(())
}
