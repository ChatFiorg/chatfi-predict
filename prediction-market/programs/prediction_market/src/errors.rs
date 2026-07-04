use anchor_lang::prelude::*;

#[error_code]
pub enum PredictionMarketError {
    #[msg("Question string exceeds max length")]
    QuestionTooLong,

    #[msg("Outcome label exceeds max length")]
    OutcomeLabelTooLong,

    #[msg("close_ts must be in the future")]
    CloseTimeInPast,

    #[msg("resolve_ts must be after close_ts plus the minimum gap")]
    ResolveTimeTooSoon,

    #[msg("Stake amount must be greater than zero")]
    ZeroStakeAmount,

    #[msg("Outcome index is out of range")]
    InvalidOutcomeIndex,

    #[msg("Pool is not open for staking")]
    PoolNotOpen,

    #[msg("Betting has already closed for this pool")]
    BettingClosed,

    #[msg("Pool cannot be resolved before resolve_ts")]
    TooEarlyToResolve,

    #[msg("Pool has already been resolved")]
    PoolAlreadyResolved,

    #[msg("Pool is not in a resolvable state")]
    PoolNotResolvable,

    #[msg("Only the designated admin can resolve this pool")]
    UnauthorizedResolver,

    #[msg("Pool has not been resolved yet")]
    PoolNotResolved,

    #[msg("This stake has already been claimed")]
    AlreadyClaimed,

    #[msg("This stake did not pick the winning outcome")]
    NotAWinningStake,

    #[msg("Token mint mismatch between pool and provided account")]
    MintMismatch,

    #[msg("Pool expected native SOL but an SPL token account was provided")]
    ExpectedNativeSol,

    #[msg("Pool expected an SPL token but native SOL was provided")]
    ExpectedSplToken,

    #[msg("No stakes were placed on the winning side, pool must be cancelled instead")]
    NoWinningStakes,

    #[msg("Arithmetic overflow")]
    MathOverflow,

    #[msg("Provided fee wallet does not match the platform treasury")]
    InvalidPlatformTreasury,

    #[msg("Provided creator wallet does not match the pool creator")]
    InvalidCreatorWallet,
}
