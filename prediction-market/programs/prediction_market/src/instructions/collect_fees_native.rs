use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::state::{Config, Pool, PoolStatus};

#[derive(Accounts)]
pub struct CollectFeesNative<'info> {
    /// Anyone can trigger fee collection once the pool is resolved.
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

    /// CHECK: pure PDA, validated via seeds, signs the outgoing transfers.
    #[account(
        mut,
        seeds = [VAULT_SEED, pool.key().as_ref()],
        bump = pool.vault_bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    /// CHECK: must match config.platform_treasury.
    #[account(mut, address = config.platform_treasury @ PredictionMarketError::InvalidPlatformTreasury)]
    pub platform_treasury: UncheckedAccount<'info>,

    /// CHECK: must match pool.creator.
    #[account(mut, address = pool.creator @ PredictionMarketError::InvalidCreatorWallet)]
    pub creator_wallet: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<CollectFeesNative>) -> Result<()> {
    let pool = &mut ctx.accounts.pool;

    require!(pool.is_native_sol(), PredictionMarketError::ExpectedSplToken);
    require!(pool.status == PoolStatus::Resolved, PredictionMarketError::PoolNotResolved);
    require!(!pool.fees_collected, PredictionMarketError::AlreadyClaimed);

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
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_authority.to_account_info(),
                to: ctx.accounts.platform_treasury.to_account_info(),
            },
            signer_seeds,
        ),
        platform_fee,
    )?;

    transfer(
        CpiContext::new_with_signer(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_authority.to_account_info(),
                to: ctx.accounts.creator_wallet.to_account_info(),
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
