pub mod constants;
pub mod error;
pub mod instructions;

pub mod math;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("DTRqHX3AvqMUnskqTpAz4ZZooSEw88S46NjbeQvN7zz8");

#[program]
pub mod memelab_amm {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        fee_bps: u16,
    ) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, fee_bps)
    }

    pub fn create_vault(
        ctx: Context<CreateVault>,
    ) -> Result<()> {
        crate::instructions::create_vault::handle_create_vault(ctx)
    }

    pub fn fund_pool(
        ctx: Context<FundPool>,
        mlab_amount: u64,
        sol_amount: u64,
    ) -> Result<()> {
        crate::instructions::fund_pool::handle_fund_pool(
            ctx,
            mlab_amount,
            sol_amount,
        )
    }

    pub fn swap_sol_for_mlab(
        ctx: Context<SwapSolForMlab>,
        sol_amount: u64,
        min_mlab_out: u64,
    ) -> Result<()> {
        crate::instructions::swap_sol_for_mlab::handle_swap_sol_for_mlab(
            ctx,
            sol_amount,
            min_mlab_out,
        )
    }

    pub fn swap_mlab_for_sol(
        ctx: Context<SwapMlabForSol>,
        mlab_amount: u64,
        min_sol_out: u64,
    ) -> Result<()> {
        crate::instructions::swap_mlab_for_sol::handle_swap_mlab_for_sol(
            ctx,
            mlab_amount,
            min_sol_out,
        )
    }
}
