# ChatFI Prediction Market (Anchor / Solana)

Fully onchain prediction pools. Users create a pool with a question and two
outcomes, others stake native SOL or an SPL token (e.g. USDC) on an outcome,
an admin wallet resolves the outcome after close, and winners claim their
proportional share. 1% total fee (0.5% platform, 0.5% pool creator) is paid
out via a permissionless `collect_fees_*` instruction after resolution.

## Structure

```
programs/prediction_market/src/
  lib.rs                    program entrypoint, instruction list
  state.rs                  Config, Pool, Stake account structs
  errors.rs                 custom error codes
  constants.rs               fee bps, seeds, size limits
  instructions/
    initialize_config.rs     one-time: set platform treasury wallet
    create_pool_native.rs    create a SOL-denominated pool
    create_pool_spl.rs       create an SPL token-denominated pool
    place_stake_native.rs    stake SOL on an outcome
    place_stake_spl.rs       stake SPL tokens on an outcome
    resolve_pool.rs          admin sets winning outcome (no funds move)
    collect_fees_native.rs   permissionless, pays platform + creator (SOL)
    collect_fees_spl.rs      permissionless, pays platform + creator (SPL)
    claim_payout_native.rs   winner claims SOL payout
    claim_payout_spl.rs      winner claims SPL token payout
```

## Why two instructions per action (native vs spl)

Native SOL and SPL tokens use different transfer CPIs (`system_program::transfer`
vs `token::transfer`) and different vault account types (a bare PDA vs an
actual token account). Splitting them keeps each instruction simple and
avoids a pile of `if is_native { ... } else { ... }` branching inside a
single handler.

## Flow

1. `initialize_config` once, with your platform treasury wallet.
2. Creator calls `create_pool_native` or `create_pool_spl` with the question,
   `["Yes","No"]` outcome labels, `close_ts` (betting closes) and
   `resolve_ts` (earliest resolution time, must be >= close_ts + 60s).
3. Users call `place_stake_native` or `place_stake_spl` with an outcome index
   (0 or 1) and amount, before `close_ts`.
4. After `resolve_ts`, the admin wallet stored on the pool calls
   `resolve_pool` with the winning outcome index.
5. Anyone calls `collect_fees_native` / `collect_fees_spl` once to pay the
   platform and creator their 0.5% cut each.
6. Each winning staker calls `claim_payout_native` / `claim_payout_spl` to
   pull `(their_stake / winning_side_total) * (total_staked * 99%)`.

## Known MVP limitations, next steps

- One stake per wallet per pool (the `stake` PDA uses `[pool, user]` as
  seeds). To allow adding to a position, this instruction would need to
  become `add_stake` with a mutable existing account instead of `init`.
- If the losing side has zero stakes, `resolve_pool` will reject that
  outcome as the winner (`NoWinningStakes`) since there is nothing to
  distribute from. A `cancel_pool` + refund instruction is the natural
  follow-up for that edge case.
- Resolution is a single admin key for now. This is the piece to replace
  with an optimistic-oracle (propose/dispute/vote) flow later without
  needing to touch the staking or payout logic.
- No tests included yet. Recommend writing Anchor/TS tests for: happy path
  stake + resolve + claim, double-claim rejection, staking after close_ts
  rejection, resolving before resolve_ts rejection, and wrong-admin
  rejection.

## Setup (Termux)

```
cd ~
git clone https://github.com/sadekunle215-cmd/chatfi-prediction-market.git
cd chatfi-prediction-market
# copy these files in, then:
avm use 0.30.1
anchor build
anchor keys list
# replace the placeholder program id in lib.rs and Anchor.toml with the real one, then:
anchor build
anchor deploy --provider.cluster devnet
```
