use anchor_lang::prelude::*;

use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::state::{Pool, PoolStatus};

#[derive(Accounts)]
pub struct ResolvePool<'info> {
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [POOL_SEED, pool.creator.as_ref(), pool.question.as_bytes()],
        bump = pool.bump,
        has_one = admin @ PredictionMarketError::UnauthorizedResolver
    )]
    pub pool: Account<'info, Pool>,
}

pub fn handler(ctx: Context<ResolvePool>, winning_outcome: u8) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    require!(
        pool.status == PoolStatus::Open || pool.status == PoolStatus::Closed,
        PredictionMarketError::PoolNotResolvable
    );
    require!(
        pool.winning_outcome.is_none(),
        PredictionMarketError::PoolAlreadyResolved
    );
    require!(
        (winning_outcome as usize) < NUM_OUTCOMES,
        PredictionMarketError::InvalidOutcomeIndex
    );

    let now = Clock::get()?.unix_timestamp;
    require!(now >= pool.resolve_ts, PredictionMarketError::TooEarlyToResolve);

    // Winning side must have at least one stake, otherwise there is nobody
    // to pay out to and the pool should be cancelled and refunded instead
    // (a future cancel_pool instruction can be added for that path).
    require!(
        pool.stake_per_outcome[winning_outcome as usize] > 0,
        PredictionMarketError::NoWinningStakes
    );

    pool.winning_outcome = Some(winning_outcome);
    pool.status = PoolStatus::Resolved;

    Ok(())
}
