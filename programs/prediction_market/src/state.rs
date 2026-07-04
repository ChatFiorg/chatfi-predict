use anchor_lang::prelude::*;

use crate::constants::{MAX_OUTCOME_LEN, MAX_QUESTION_LEN, NUM_OUTCOMES};

/// Singleton config holding the platform treasury wallet. Initialized once
/// via initialize_config. Only the upgrade authority can update it later.
#[account]
pub struct Config {
    pub authority: Pubkey,
    pub platform_treasury: Pubkey,
    pub bump: u8,
}

impl Config {
    pub const MAX_SIZE: usize = 32 + 32 + 1;
}

#[account]
pub struct Pool {
    /// Wallet that created the pool and receives the 0.5% creator fee.
    pub creator: Pubkey,

    /// Wallet authorized to call resolve_pool for this pool.
    pub admin: Pubkey,

    /// The question being predicted, e.g. "Will Tinubu address the nation today?".
    pub question: String,

    /// Human readable labels for each outcome, e.g. ["Yes", "No"].
    pub outcome_names: [String; NUM_OUTCOMES],

    /// None = pool is denominated in native SOL.
    /// Some(mint) = pool is denominated in the given SPL token (e.g. USDC).
    pub token_mint: Option<Pubkey>,

    /// PDA that holds custody of staked funds and signs outgoing transfers.
    pub vault_authority: Pubkey,

    /// For SPL pools only: the token account holding staked tokens.
    /// Unused (default pubkey) for native SOL pools.
    pub vault_token_account: Pubkey,

    /// Total amount staked across all outcomes.
    pub total_staked: u64,

    /// Amount staked per outcome index.
    pub stake_per_outcome: [u64; NUM_OUTCOMES],

    /// Unix timestamp after which no more stakes are accepted.
    pub close_ts: i64,

    /// Earliest unix timestamp at which the admin may resolve the pool.
    pub resolve_ts: i64,

    pub status: PoolStatus,

    /// Set once resolve_pool is called.
    pub winning_outcome: Option<u8>,

    /// Total fee amount (platform + creator) taken at resolution, kept for
    /// bookkeeping and off-chain indexing.
    pub fee_taken: u64,

    /// True once collect_fees has paid out the platform and creator cut.
    /// Prevents fees being taken more than once.
    pub fees_collected: bool,

    pub bump: u8,
    pub vault_bump: u8,
    pub vault_token_bump: u8,
}

impl Pool {
    /// Anchor account space, not counting the 8 byte discriminator.
    pub const MAX_SIZE: usize = 32 // creator
        + 32 // admin
        + 4 + MAX_QUESTION_LEN // question
        + (4 + MAX_OUTCOME_LEN) * NUM_OUTCOMES // outcome_names
        + 1 + 32 // token_mint Option<Pubkey>
        + 32 // vault_authority
        + 32 // vault_token_account
        + 8 // total_staked
        + 8 * NUM_OUTCOMES // stake_per_outcome
        + 8 // close_ts
        + 8 // resolve_ts
        + 1 // status
        + 1 + 1 // winning_outcome Option<u8>
        + 8 // fee_taken
        + 1 // fees_collected
        + 1 + 1 + 1; // bumps

    pub fn is_native_sol(&self) -> bool {
        self.token_mint.is_none()
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum PoolStatus {
    Open,
    Closed,
    Resolved,
    Cancelled,
}

#[account]
pub struct Stake {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub outcome: u8,
    pub amount: u64,
    pub claimed: bool,
    pub bump: u8,
}

impl Stake {
    pub const MAX_SIZE: usize = 32 // pool
        + 32 // user
        + 1 // outcome
        + 8 // amount
        + 1 // claimed
        + 1; // bump
}
