use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::state::{Pool, PoolStatus, Stake};

#[derive(Accounts)]
pub struct ClaimPayoutSpl<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        seeds = [POOL_SEED, pool.creator.as_ref(), pool.question.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Account<'info, Pool>,

    /// CHECK: pure PDA authority, validated via seeds.
    #[account(
        seeds = [VAULT_SEED, pool.key().as_ref()],
        bump = pool.vault_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(mut, address = pool.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_account.mint == pool.token_mint.unwrap() @ PredictionMarketError::MintMismatch)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [STAKE_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump = stake.bump,
        has_one = user
    )]
    pub stake: Account<'info, Stake>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ClaimPayoutSpl>) -> Result<()> {
    let pool = &ctx.accounts.pool;
    let stake = &mut ctx.accounts.stake;

    require!(!pool.is_native_sol(), PredictionMarketError::ExpectedNativeSol);
    require!(pool.status == PoolStatus::Resolved, PredictionMarketError::PoolNotResolved);
    require!(!stake.claimed, PredictionMarketError::AlreadyClaimed);

    let winning_outcome = pool
        .winning_outcome
        .ok_or(PredictionMarketError::PoolNotResolved)?;
    require!(stake.outcome == winning_outcome, PredictionMarketError::NotAWinningStake);

    let winning_side_total = pool.stake_per_outcome[winning_outcome as usize];
    let total_fee_bps = PLATFORM_FEE_BPS + CREATOR_FEE_BPS;
    let distributable = pool
        .total_staked
        .checked_mul(FEE_DENOMINATOR - total_fee_bps)
        .and_then(|v| v.checked_div(FEE_DENOMINATOR))
        .ok_or(PredictionMarketError::MathOverflow)?;

    let payout = (stake.amount as u128)
        .checked_mul(distributable as u128)
        .and_then(|v| v.checked_div(winning_side_total as u128))
        .ok_or(PredictionMarketError::MathOverflow)? as u64;

    let pool_key = pool.key();
    let seeds: &[&[u8]] = &[VAULT_SEED, pool_key.as_ref(), &[pool.vault_bump]];
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.user_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        payout,
    )?;

    stake.claimed = true;

    Ok(())
}
