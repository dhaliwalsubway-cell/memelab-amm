use anchor_lang::prelude::*;

#[error_code]
#[derive(PartialEq)]
pub enum AmmError {
    #[msg("Trading fee exceeds the configured maximum")]
    InvalidFee,

    #[msg("The supplied MLAB mint does not match the pool")]
    InvalidMint,

    #[msg("Funding amounts must be greater than zero")]
    ZeroAmount,

    #[msg("Pool reserve would overflow")]
    ReserveOverflow,

    #[msg("Insufficient pool liquidity")]
    InsufficientLiquidity,

    #[msg("Mathematical calculation overflow")]
    MathOverflow,

    #[msg("Input amount is too small after fees")]
    AmountTooSmall,

    #[msg("Output amount is below the minimum requested amount")]
    SlippageExceeded,
}
