use anchor_lang::prelude::*;

use crate::{constants::*, error::AmmError, state::Pool};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// CHECK: The MLAB mint is validated by the client during this local-lab stage.
    pub mlab_mint: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + Pool::INIT_SPACE,
        seeds = [POOL_SEED, mlab_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(
    ctx: Context<Initialize>,
    fee_bps: u16,
) -> Result<()> {
    require!(fee_bps <= MAX_FEE_BPS, AmmError::InvalidFee);

    let pool = &mut ctx.accounts.pool;

    pool.authority = ctx.accounts.payer.key();
    pool.mlab_mint = ctx.accounts.mlab_mint.key();
    pool.sol_reserve = 0;
    pool.mlab_reserve = 0;
    pool.fee_bps = fee_bps;
    pool.bump = ctx.bumps.pool;

    msg!("MemeLab AMM pool initialized");
    msg!("MLAB mint: {}", pool.mlab_mint);
    msg!("Fee bps: {}", pool.fee_bps);

    Ok(())
}
