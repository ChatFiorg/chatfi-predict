use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};

use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::state::{Config, Pool, PoolStatus};

#[derive(Accounts)]
pub struct CollectFeesSpl<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

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

    #[account(mut, address = pool.vault_token_account)]
    pub vault_token_account: Account<'info, TokenAccount>,

    #[account(mut, constraint = platform_treasury_token.mint == pool.token_mint.unwrap() @ PredictionMarketError::MintMismatch)]
    pub platform_treasury_token: Account<'info, TokenAccount>,

    #[account(mut, constraint = creator_token_account.mint == pool.token_mint.unwrap() @ PredictionMarketError::MintMismatch)]
    pub creator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<CollectFeesSpl>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    require!(!pool.is_native_sol(), PredictionMarketError::ExpectedNativeSol);
    require!(pool.status == PoolStatus::Resolved, PredictionMarketError::PoolNotResolved);
    require!(!pool.fees_collected, PredictionMarketError::AlreadyClaimed);
    require!(
        ctx.accounts.platform_treasury_token.owner == ctx.accounts.config.platform_treasury,
        PredictionMarketError::InvalidPlatformTreasury
    );
    require!(
        ctx.accounts.creator_token_account.owner == pool.creator,
        PredictionMarketError::InvalidCreatorWallet
    );

    let platform_fee = pool
        .total_staked
        .checked_mul(PLATFORM_FEE_BPS)
        .and_then(|v| v.checked_div(FEE_DENOMINATOR))
        .ok_or(PredictionMarketError::MathOverflow)?;
    let creator_fee = pool
        .total_staked
        .checked_mul(CREATOR_FEE_BPS)
        .and_then(|v| v.checked_div(FEE_DENOMINATOR))
        .ok_or(PredictionMarketError::MathOverflow)?;

    let pool_key = pool.key();
    let seeds: &[&[u8]] = &[VAULT_SEED, pool_key.as_ref(), &[pool.vault_bump]];
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.platform_treasury_token.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        platform_fee,
    )?;

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_token_account.to_account_info(),
                to: ctx.accounts.creator_token_account.to_account_info(),
                authority: ctx.accounts.vault_authority.to_account_info(),
            },
            signer_seeds,
        ),
        creator_fee,
    )?;

    pool.fee_taken = platform_fee
        .checked_add(creator_fee)
        .ok_or(PredictionMarketError::MathOverflow)?;
    pool.fees_collected = true;

    Ok(())
}
