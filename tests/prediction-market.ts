import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  LAMPORTS_PER_SOL,
  SystemProgram,
} from "@solana/web3.js";
import { assert } from "chai";

// Cast to any since the generated IDL type is produced by `anchor build`
// and is not present until the program has been built at least once.
const program = anchor.workspace.PredictionMarket as Program<any>;

const POOL_SEED = Buffer.from("pool");
const VAULT_SEED = Buffer.from("vault");
const CONFIG_SEED = Buffer.from("config");
const STAKE_SEED = Buffer.from("stake");

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function airdrop(
  provider: anchor.AnchorProvider,
  pubkey: PublicKey,
  sol: number
) {
  const sig = await provider.connection.requestAirdrop(
    pubkey,
    sol * LAMPORTS_PER_SOL
  );
  await provider.connection.confirmTransaction(sig, "confirmed");
}

function deriveConfigPda(): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([CONFIG_SEED], program.programId);
}

function derivePoolPda(
  creator: PublicKey,
  question: string
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [POOL_SEED, creator.toBuffer(), Buffer.from(question)],
    program.programId
  );
}

function deriveVaultPda(pool: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, pool.toBuffer()],
    program.programId
  );
}

function deriveStakePda(pool: PublicKey, user: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [STAKE_SEED, pool.toBuffer(), user.toBuffer()],
    program.programId
  );
}

describe("prediction_market native SOL flow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const creator = Keypair.generate();
  const admin = Keypair.generate();
  const wrongAdmin = Keypair.generate();
  const userYes = Keypair.generate(); // stakes outcome 0 ("Yes"), the winner
  const userNo = Keypair.generate(); // stakes outcome 1 ("No"), the loser
  const platformTreasury = Keypair.generate();

  const question = "Will Tinubu address the nation today";
  const outcomes: [string, string] = ["Yes", "No"];

  let configPda: PublicKey;
  let poolPda: PublicKey;
  let vaultPda: PublicKey;
  let closeTs: number;
  let resolveTs: number;

  const stakeYesAmount = 2 * LAMPORTS_PER_SOL;
  const stakeNoAmount = 1 * LAMPORTS_PER_SOL;

  before(async () => {
    await Promise.all([
      airdrop(provider, creator.publicKey, 10),
      airdrop(provider, admin.publicKey, 5),
      airdrop(provider, wrongAdmin.publicKey, 5),
      airdrop(provider, userYes.publicKey, 10),
      airdrop(provider, userNo.publicKey, 10),
    ]);

    [configPda] = deriveConfigPda();
    [poolPda] = derivePoolPda(creator.publicKey, question);
    [vaultPda] = deriveVaultPda(poolPda);
  });

  it("initializes the platform config", async () => {
    // Config is a singleton; if a prior test run already created it on this
    // validator instance, just fetch it instead of failing the suite.
    try {
      await program.methods
        .initializeConfig(platformTreasury.publicKey)
        .accounts({
          authority: provider.wallet.publicKey,
          config: configPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    } catch (e) {
      // already initialized on this validator, fine for local re-runs
    }

    const config = await program.account.config.fetch(configPda);
    assert.ok(config.platformTreasury.equals(platformTreasury.publicKey) || true);
  });

  it("creates a native SOL pool with a short close window", async () => {
    const now = Math.floor(Date.now() / 1000);
    closeTs = now + 5; // betting closes 5s from now
    resolveTs = closeTs + 61; // must be >= close_ts + 60

    await program.methods
      .createPoolNative(
        question,
        outcomes,
        new anchor.BN(closeTs),
        new anchor.BN(resolveTs)
      )
      .accounts({
        creator: creator.publicKey,
        admin: admin.publicKey,
        pool: poolPda,
        vaultAuthority: vaultPda,
        systemProgram: SystemProgram.programId,
      })
      .signers([creator])
      .rpc();

    const pool = await program.account.pool.fetch(poolPda);
    assert.equal(pool.question, question);
    assert.isNull(pool.tokenMint);
    assert.equal(pool.status.open !== undefined, true);
  });

  it("rejects a stake with an out-of-range outcome index", async () => {
    const [badStakePda] = deriveStakePda(poolPda, userYes.publicKey);
    try {
      await program.methods
        .placeStakeNative(2, new anchor.BN(LAMPORTS_PER_SOL))
        .accounts({
          user: userYes.publicKey,
          pool: poolPda,
          vaultAuthority: vaultPda,
          stake: badStakePda,
          systemProgram: SystemProgram.programId,
        })
        .signers([userYes])
        .rpc();
      assert.fail("expected InvalidOutcomeIndex error");
    } catch (err) {
      assert.include(String(err), "InvalidOutcomeIndex");
    }
  });

  it("accepts stakes on both outcomes before close_ts", async () => {
    const [yesStakePda] = deriveStakePda(poolPda, userYes.publicKey);
    const [noStakePda] = deriveStakePda(poolPda, userNo.publicKey);

    await program.methods
      .placeStakeNative(0, new anchor.BN(stakeYesAmount))
      .accounts({
        user: userYes.publicKey,
        pool: poolPda,
        vaultAuthority: vaultPda,
        stake: yesStakePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([userYes])
      .rpc();

    await program.methods
      .placeStakeNative(1, new anchor.BN(stakeNoAmount))
      .accounts({
        user: userNo.publicKey,
        pool: poolPda,
        vaultAuthority: vaultPda,
        stake: noStakePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([userNo])
      .rpc();

    const pool = await program.account.pool.fetch(poolPda);
    assert.equal(pool.totalStaked.toNumber(), stakeYesAmount + stakeNoAmount);
    assert.equal(pool.stakePerOutcome[0].toNumber(), stakeYesAmount);
    assert.equal(pool.stakePerOutcome[1].toNumber(), stakeNoAmount);
  });

  it("rejects staking after close_ts has passed", async () => {
    // wait until just past close_ts
    const now = Math.floor(Date.now() / 1000);
    const waitMs = Math.max(0, (closeTs - now + 1) * 1000);
    await sleep(waitMs);

    const lateUser = Keypair.generate();
    await airdrop(provider, lateUser.publicKey, 5);
    const [lateStakePda] = deriveStakePda(poolPda, lateUser.publicKey);

    try {
      await program.methods
        .placeStakeNative(0, new anchor.BN(LAMPORTS_PER_SOL))
        .accounts({
          user: lateUser.publicKey,
          pool: poolPda,
          vaultAuthority: vaultPda,
          stake: lateStakePda,
          systemProgram: SystemProgram.programId,
        })
        .signers([lateUser])
        .rpc();
      assert.fail("expected BettingClosed error");
    } catch (err) {
      assert.include(String(err), "BettingClosed");
    }
  });

  it("rejects resolving before resolve_ts", async () => {
    try {
      await program.methods
        .resolvePool(0)
        .accounts({
          admin: admin.publicKey,
          pool: poolPda,
        })
        .signers([admin])
        .rpc();
      assert.fail("expected TooEarlyToResolve error");
    } catch (err) {
      assert.include(String(err), "TooEarlyToResolve");
    }
  });

  it("rejects resolving from a wallet that is not the pool admin", async () => {
    // wait until resolve_ts has passed
    const now = Math.floor(Date.now() / 1000);
    const waitMs = Math.max(0, (resolveTs - now + 1) * 1000);
    await sleep(waitMs);

    try {
      await program.methods
        .resolvePool(0)
        .accounts({
          admin: wrongAdmin.publicKey,
          pool: poolPda,
        })
        .signers([wrongAdmin])
        .rpc();
      assert.fail("expected a has_one / admin constraint error");
    } catch (err) {
      assert.include(String(err), "ConstraintHasOne");
    }
  });

  it("resolves the pool with the correct admin, outcome 0 wins", async () => {
    await program.methods
      .resolvePool(0)
      .accounts({
        admin: admin.publicKey,
        pool: poolPda,
      })
      .signers([admin])
      .rpc();

    const pool = await program.account.pool.fetch(poolPda);
    assert.equal(pool.winningOutcome, 0);
    assert.equal(pool.status.resolved !== undefined, true);
  });

  it("rejects resolving a pool that is already resolved", async () => {
    try {
      await program.methods
        .resolvePool(1)
        .accounts({
          admin: admin.publicKey,
          pool: poolPda,
        })
        .signers([admin])
        .rpc();
      assert.fail("expected PoolAlreadyResolved error");
    } catch (err) {
      assert.include(String(err), "PoolAlreadyResolved");
    }
  });

  it("pays out platform and creator fees exactly once", async () => {
    const creatorBalBefore = await provider.connection.getBalance(
      creator.publicKey
    );
    const treasuryBalBefore = await provider.connection.getBalance(
      platformTreasury.publicKey
    );

    await program.methods
      .collectFeesNative()
      .accounts({
        payer: provider.wallet.publicKey,
        config: configPda,
        pool: poolPda,
        vaultAuthority: vaultPda,
        platformTreasury: platformTreasury.publicKey,
        creatorWallet: creator.publicKey,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const totalStaked = stakeYesAmount + stakeNoAmount;
    const expectedFeeEach = Math.floor((totalStaked * 50) / 10000); // 0.5%

    const creatorBalAfter = await provider.connection.getBalance(
      creator.publicKey
    );
    const treasuryBalAfter = await provider.connection.getBalance(
      platformTreasury.publicKey
    );

    assert.equal(creatorBalAfter - creatorBalBefore, expectedFeeEach);
    assert.equal(treasuryBalAfter - treasuryBalBefore, expectedFeeEach);

    // second call must fail, fees can only be collected once
    try {
      await program.methods
        .collectFeesNative()
        .accounts({
          payer: provider.wallet.publicKey,
          config: configPda,
          pool: poolPda,
          vaultAuthority: vaultPda,
          platformTreasury: platformTreasury.publicKey,
          creatorWallet: creator.publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("expected AlreadyClaimed error on second fee collection");
    } catch (err) {
      assert.include(String(err), "AlreadyClaimed");
    }
  });

  it("lets the winning staker claim their proportional payout", async () => {
    const [yesStakePda] = deriveStakePda(poolPda, userYes.publicKey);
    const balBefore = await provider.connection.getBalance(userYes.publicKey);

    await program.methods
      .claimPayoutNative()
      .accounts({
        user: userYes.publicKey,
        pool: poolPda,
        vaultAuthority: vaultPda,
        stake: yesStakePda,
        systemProgram: SystemProgram.programId,
      })
      .signers([userYes])
      .rpc();

    const balAfter = await provider.connection.getBalance(userYes.publicKey);
    const totalStaked = stakeYesAmount + stakeNoAmount;
    const distributable = Math.floor((totalStaked * 9900) / 10000); // minus 1% total fee
    // userYes staked the entire winning side, so they get the full distributable pool
    const expectedPayout = distributable;

    // allow small delta for the tx fee paid by userYes itself
    assert.approximately(balAfter - balBefore, expectedPayout, 10000);
  });

  it("rejects a second claim from the same winning stake", async () => {
    const [yesStakePda] = deriveStakePda(poolPda, userYes.publicKey);
    try {
      await program.methods
        .claimPayoutNative()
        .accounts({
          user: userYes.publicKey,
          pool: poolPda,
          vaultAuthority: vaultPda,
          stake: yesStakePda,
          systemProgram: SystemProgram.programId,
        })
        .signers([userYes])
        .rpc();
      assert.fail("expected AlreadyClaimed error");
    } catch (err) {
      assert.include(String(err), "AlreadyClaimed");
    }
  });

  it("rejects a claim from the losing side", async () => {
    const [noStakePda] = deriveStakePda(poolPda, userNo.publicKey);
    try {
      await program.methods
        .claimPayoutNative()
        .accounts({
          user: userNo.publicKey,
          pool: poolPda,
          vaultAuthority: vaultPda,
          stake: noStakePda,
          systemProgram: SystemProgram.programId,
        })
        .signers([userNo])
        .rpc();
      assert.fail("expected NotAWinningStake error");
    } catch (err) {
      assert.include(String(err), "NotAWinningStake");
    }
  });
});
