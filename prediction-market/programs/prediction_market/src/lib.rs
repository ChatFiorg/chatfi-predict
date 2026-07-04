use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod instructions;
pub mod state;

use constants::NUM_OUTCOMES;
use instructions::*;

declare_id!("Predict11111111111111111111111111111111111");

#[program]
pub mod prediction_market {
    use super::*;

    /// One-time setup, sets the platform treasury wallet that receives the
    /// 0.5% platform fee on every resolved pool.
    pub fn initialize_config(ctx: Context<InitializeConfig>, platform_treasury: Pubkey) -> Result<()> {
        instructions::initialize_config::handler(ctx, platform_treasury)
    }

    /// Creates a new prediction pool denominated in native SOL.
    pub fn create_pool_native(
        ctx: Context<CreatePoolNative>,
        question: String,
        outcome_names: [String; NUM_OUTCOMES],
        close_ts: i64,
        resolve_ts: i64,
    ) -> Result<()> {
        instructions::create_pool_native::handler(ctx, question, outcome_names, close_ts, resolve_ts)
    }

    /// Creates a new prediction pool denominated in an SPL token (e.g. USDC).
    pub fn create_pool_spl(
        ctx: Context<CreatePoolSpl>,
        question: String,
        outcome_names: [String; NUM_OUTCOMES],
        close_ts: i64,
        resolve_ts: i64,
    ) -> Result<()> {
        instructions::create_pool_spl::handler(ctx, question, outcome_names, close_ts, resolve_ts)
    }

    /// Stakes native SOL on an outcome for an open pool.
    pub fn place_stake_native(ctx: Context<PlaceStakeNative>, outcome: u8, amount: u64) -> Result<()> {
        instructions::place_stake_native::handler(ctx, outcome, amount)
    }

    /// Stakes an SPL token on an outcome for an open pool.
    pub fn place_stake_spl(ctx: Context<PlaceStakeSpl>, outcome: u8, amount: u64) -> Result<()> {
        instructions::place_stake_spl::handler(ctx, outcome, amount)
    }

    /// Admin-only: sets the winning outcome once resolve_ts has passed.
    /// Moves no funds, only updates pool state.
    pub fn resolve_pool(ctx: Context<ResolvePool>, winning_outcome: u8) -> Result<()> {
        instructions::resolve_pool::handler(ctx, winning_outcome)
    }

    /// Permissionless: pays the platform and creator fee out of a resolved
    /// native SOL pool. Can only run once per pool.
    pub fn collect_fees_native(ctx: Context<CollectFeesNative>) -> Result<()> {
        instructions::collect_fees_native::handler(ctx)
    }

    /// Permissionless: pays the platform and creator fee out of a resolved
    /// SPL token pool. Can only run once per pool.
    pub fn collect_fees_spl(ctx: Context<CollectFeesSpl>) -> Result<()> {
        instructions::collect_fees_spl::handler(ctx)
    }

    /// Winner claims their proportional share of a resolved native SOL pool.
    pub fn claim_payout_native(ctx: Context<ClaimPayoutNative>) -> Result<()> {
        instructions::claim_payout_native::handler(ctx)
    }

    /// Winner claims their proportional share of a resolved SPL token pool.
    pub fn claim_payout_spl(ctx: Context<ClaimPayoutSpl>) -> Result<()> {
        instructions::claim_payout_spl::handler(ctx)
    }
}
