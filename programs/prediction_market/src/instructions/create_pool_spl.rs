use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::errors::PredictionMarketError;
use crate::state::{Pool, PoolStatus};

#[derive(Accounts)]
#[instruction(question: String, outcome_names: [String; NUM_OUTCOMES])]
pub struct CreatePoolSpl<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    /// Wallet authorized to resolve this pool once it closes.
    /// CHECK: stored as pool.admin, only used as a pubkey reference.
    pub admin: UncheckedAccount<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = creator,
        space = 8 + Pool::MAX_SIZE,
        seeds = [POOL_SEED, creator.key().as_ref(), question.as_bytes()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    /// PDA authority over the vault token account.
    /// CHECK: pure PDA, no data, validated via seeds.
    #[account(
        seeds = [VAULT_SEED, pool.key().as_ref()],
        bump
    )]
    pub vault_authority: UncheckedAccount<'info>,

    #[account(
        init,
        payer = creator,
        seeds = [VAULT_TOKEN_SEED, pool.key().as_ref()],
        bump,
        token::mint = token_mint,
        token::authority = vault_authority
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<CreatePoolSpl>,
    question: String,
    outcome_names: [String; NUM_OUTCOMES],
    close_ts: i64,
    resolve_ts: i64,
) -> Result<()> {
    require!(
        question.len() <= MAX_QUESTION_LEN,
        PredictionMarketError::QuestionTooLong
    );
    for name in outcome_names.iter() {
        require!(
            name.len() <= MAX_OUTCOME_LEN,
            PredictionMarketError::OutcomeLabelTooLong
        );
    }

    let now = Clock::get()?.unix_timestamp;
    require!(close_ts > now, PredictionMarketError::CloseTimeInPast);
    require!(
        resolve_ts >= close_ts + MIN_CLOSE_TO_RESOLVE_GAP,
        PredictionMarketError::ResolveTimeTooSoon
    );

    let pool = &mut ctx.accounts.pool;
    pool.creator = ctx.accounts.creator.key();
    pool.admin = ctx.accounts.admin.key();
    pool.question = question;
    pool.outcome_names = outcome_names;
    pool.token_mint = Some(ctx.accounts.token_mint.key());
    pool.vault_authority = ctx.accounts.vault_authority.key();
    pool.vault_token_account = ctx.accounts.vault_token_account.key();
    pool.total_staked = 0;
    pool.stake_per_outcome = [0; NUM_OUTCOMES];
    pool.close_ts = close_ts;
    pool.resolve_ts = resolve_ts;
    pool.status = PoolStatus::Open;
    pool.winning_outcome = None;
    pool.fee_taken = 0;
    pool.fees_collected = false;
    pool.bump = ctx.bumps.pool;
    pool.vault_bump = ctx.bumps.vault_authority;
    pool.vault_token_bump = ctx.bumps.vault_token_account;

    Ok(())
}
