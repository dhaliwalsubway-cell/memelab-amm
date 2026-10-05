use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Pool {
    pub authority: Pubkey,
    pub mlab_mint: Pubkey,
    pub mlab_vault: Pubkey,
    pub sol_reserve: u64,
    pub mlab_reserve: u64,
    pub fee_bps: u16,
    pub bump: u8,
}
