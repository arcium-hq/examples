import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Voting } from "../target/types/voting";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getArciumEnv,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  uploadCircuit,
  buildFinalizeCompDefTx,
  RescueCipher,
  x25519RandomPrivateKey,
  x25519GetPublicKey,
  x25519GetSharedSecretWithMXE,
  deserializeLE,
  getMXEAccAcc,
  getMempoolAcc,
  getCompDefAcc,
  getExecutingPoolAcc,
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("Voting", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Voting as Program<Voting>;
  const provider = anchor.getProvider();

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;
  const awaitEvent = async <E extends keyof Event>(eventName: E) => {
    let listenerId: number;
    const event = await new Promise<Event[E]>((res) => {
      listenerId = program.addEventListener(eventName, (event) => {
        res(event);
      });
    });
    await program.removeEventListener(listenerId);

    return event;
  };

  const arciumEnv = getArciumEnv();

  it("Is initialized!", async () => {
    const POLL_ID = 420;
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing vote stats computation definition");
    const initVoteStatsSig = await initVoteStatsCompDef(program, owner, false);
    console.log(
      "Vote stats computation definition initialized with signature",
      initVoteStatsSig
    );

    console.log("Initializing voting computation definition");
    const initVoteSig = await initVoteCompDef(program, owner, false);
    console.log(
      "Vote computation definition initialized with signature",
      initVoteSig
    );

    console.log("Initializing reveal result computation definition");
    const initRRSig = await initRevealResultCompDef(program, owner, false);
    console.log(
      "Reveal result computation definition initialized with signature",
      initRRSig
    );

    const privateKey = x25519RandomPrivateKey();
    const publicKey = x25519GetPublicKey(privateKey);
    const mxePublicKey = [
      new Uint8Array([
        34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
        253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
      ]),
      new Uint8Array([
        107, 1, 201, 151, 195, 126, 155, 84, 228, 85, 185, 142, 62, 220, 161,
        29, 179, 36, 112, 163, 201, 103, 172, 207, 55, 89, 53, 120, 73, 208,
        234, 63,
      ]),
      new Uint8Array([
        217, 186, 137, 28, 190, 167, 128, 220, 100, 71, 90, 160, 130, 162, 96,
        15, 191, 147, 184, 4, 151, 89, 186, 211, 72, 212, 173, 31, 98, 187, 65,
        59,
      ]),
      new Uint8Array([
        51, 66, 84, 103, 52, 182, 174, 177, 134, 163, 224, 196, 127, 102, 81,
        61, 12, 136, 171, 212, 230, 171, 242, 47, 221, 48, 152, 231, 239, 0,
        183, 15,
      ]),
      new Uint8Array([
        162, 140, 124, 61, 16, 202, 184, 56, 39, 7, 37, 95, 225, 104, 229, 25,
        48, 246, 35, 136, 99, 106, 110, 253, 188, 86, 201, 42, 112, 211, 129,
        34,
      ]),
    ];

    const pollNonce = randomBytes(16);

    const pollSig = await program.methods
      .createNewPoll(
        POLL_ID,
        "$SOL to 500?",
        new anchor.BN(deserializeLE(pollNonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_vote_stats")).readUInt32LE()
        ),
      })
      .rpc();

    console.log("Poll created with signature", pollSig);

    const finalizePollSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      pollSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize poll sig is ", finalizePollSig);

    const rescueKey = x25519GetSharedSecretWithMXE(privateKey, mxePublicKey);
    const cipher = new RescueCipher(rescueKey);

    const vote = BigInt(true);
    const plaintext = [vote];

    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt(plaintext, nonce);

    const voteEventPromise = awaitEvent("voteEvent");

    // TODO: Remove this sleep once the CI bug is solved
    await new Promise((resolve) => setTimeout(resolve, 100));

    console.log("Voting");

    const queueVoteSig = await program.methods
      .vote(
        POLL_ID,
        Array.from(ciphertext[0]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString()),
        new anchor.BN(deserializeLE(pollNonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("vote")).readUInt32LE()
        ),
        authority: owner.publicKey,
      })
      .rpc({ commitment: "confirmed" });
    console.log("Queue vote sig is ", queueVoteSig);

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      queueVoteSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const voteEvent = await voteEventPromise;
    console.log("Vote casted at timestamp ", voteEvent.timestamp.toString());

    const revealEventPromise = awaitEvent("revealResultEvent");

    const revealQueueSig = await program.methods
      .revealResult(POLL_ID, new anchor.BN(deserializeLE(pollNonce).toString()))
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("reveal_result")).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Reveal queue sig is ", revealQueueSig);

    const revealFinalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      revealQueueSig,
      program.programId,
      "confirmed"
    );
    console.log("Reveal finalize sig is ", revealFinalizeSig);

    const revealEvent = await revealEventPromise;

    console.log("Decrypted winner is ", revealEvent.output);
    expect(revealEvent.output).to.be.true;
  });

  async function initVoteStatsCompDef(
    program: Program<Voting>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("init_vote_stats");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Init vote stats computation definition pda is ", compDefPDA);

    const sig = await program.methods
      .initVoteStatsCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init vote stats computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/init_vote_stats.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "init_vote_stats",
        program.programId,
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        provider as anchor.AnchorProvider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

      finalizeTx.sign(owner);

      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }

  async function initVoteCompDef(
    program: Program<Voting>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("vote");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Vote computation definition pda is ", compDefPDA);

    const sig = await program.methods
      .initVoteCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init vote computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/vote.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "vote",
        program.programId,
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        provider as anchor.AnchorProvider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

      finalizeTx.sign(owner);

      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }

  async function initRevealResultCompDef(
    program: Program<Voting>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("reveal_result");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Reveal result computation definition pda is ", compDefPDA);

    const sig = await program.methods
      .initRevealResultCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init reveal result computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/reveal_result.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "reveal_result",
        program.programId,
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        provider as anchor.AnchorProvider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );

      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

      finalizeTx.sign(owner);

      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }
});

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
