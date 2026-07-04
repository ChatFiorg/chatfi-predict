// Fee split: 1% total, divided evenly between platform and pool creator.
pub const PLATFORM_FEE_BPS: u64 = 50; // 0.5%
pub const CREATOR_FEE_BPS: u64 = 50; // 0.5%
pub const FEE_DENOMINATOR: u64 = 10_000;

// String size limits, used to size accounts.
pub const MAX_QUESTION_LEN: usize = 200;
pub const MAX_OUTCOME_LEN: usize = 32;
pub const NUM_OUTCOMES: usize = 2;

// PDA seeds.
pub const POOL_SEED: &[u8] = b"pool";
pub const VAULT_SEED: &[u8] = b"vault";
pub const VAULT_TOKEN_SEED: &[u8] = b"vault_token";
pub const STAKE_SEED: &[u8] = b"stake";

// Minimum gap enforced between close_ts and resolve_ts so there is always
// a window between betting closing and the admin being allowed to resolve.
pub const MIN_CLOSE_TO_RESOLVE_GAP: i64 = 60; // seconds
