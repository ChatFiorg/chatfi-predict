use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::state::{Pool, PoolStatus, Stake};

#[derive(Accounts)]
#[instruction(outcome: u8, amount: u64)]
pub struct PlaceStakeSpl<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
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

    #[account(
        mut,
        address = pool.vault_token_account
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        space = 8 + Stake::MAX_SIZE,
        seeds = [STAKE_SEED, pool.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub stake: Account<'info, Stake>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<PlaceStakeSpl>, outcome: u8, amount: u64) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    require!(!pool.is_native_sol(), PredictionMarketError::ExpectedNativeSol);
    require!(pool.status == PoolStatus::Open, PredictionMarketError::PoolNotOpen);
    require!(amount > 0, PredictionMarketError::ZeroStakeAmount);
    require!(
        (outcome as usize) < NUM_OUTCOMES,
        PredictionMarketError::InvalidOutcomeIndex
    );
    require!(
        ctx.accounts.user_token_account.mint == pool.token_mint.unwrap(),
        PredictionMarketError::MintMismatch
    );

    let now = Clock::get()?.unix_timestamp;
    require!(now < pool.close_ts, PredictionMarketError::BettingClosed);

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_account.to_account_info(),
                to: ctx.accounts.vault_token_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        ),
        amount,
    )?;

    pool.total_staked = pool
        .total_staked
        .checked_add(amount)
        .ok_or(PredictionMarketError::MathOverflow)?;
    pool.stake_per_outcome[outcome as usize] = pool.stake_per_outcome[outcome as usize]
        .checked_add(amount)
        .ok_or(PredictionMarketError::MathOverflow)?;

    let stake = &mut ctx.accounts.stake;
    stake.pool = pool.key();
    stake.user = ctx.accounts.user.key();
    stake.outcome = outcome;
    stake.amount = amount;
    stake.claimed = false;
    stake.bump = ctx.bumps.stake;

    Ok(())
}
