import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Voting } from "../target/types/voting";
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
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";
import { randomBytes } from "crypto";

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
    const initRRSig = await initRRCompDef(program, owner, false);
    console.log(
      "Reveal result computation definition initialized with signature",
      initRRSig
    );

    const privateKey = x25519RandomPrivateKey();
    const publicKey = x25519GetPublicKey(privateKey);
    const mxePublicKey = [
      new Uint8Array([
        78, 96, 220, 218, 225, 248, 149, 140, 229, 147, 105, 183, 46, 82, 166,
        248, 146, 35, 137, 78, 122, 181, 200, 220, 217, 97, 20, 11, 71, 9, 113,
        6,
      ]),
      new Uint8Array([
        155, 202, 231, 73, 215, 1, 94, 193, 141, 26, 77, 66, 143, 114, 197, 172,
        160, 245, 64, 108, 236, 104, 149, 242, 103, 140, 199, 94, 70, 61, 162,
        118,
      ]),
      new Uint8Array([
        231, 24, 19, 12, 184, 40, 139, 11, 29, 176, 125, 231, 49, 53, 174, 225,
        183, 156, 234, 55, 49, 240, 169, 70, 252, 141, 70, 28, 113, 255, 70, 20,
      ]),
      new Uint8Array([
        120, 66, 73, 239, 247, 13, 25, 149, 162, 21, 108, 27, 236, 128, 93, 84,
        210, 18, 70, 106, 80, 82, 111, 61, 12, 178, 182, 23, 96, 12, 9, 1,
      ]),
      new Uint8Array([
        112, 133, 255, 66, 62, 138, 251, 232, 170, 239, 193, 225, 253, 152, 85,
        205, 19, 16, 50, 193, 41, 248, 39, 175, 49, 87, 207, 79, 54, 122, 78,
        125,
      ]),
    ];

    const { sig: pollSig, nonce: pollNonce } = await createNewPoll(
      program,
      POLL_ID,
      "$SOL to 500?",
      owner.publicKey
    );
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

    const pollPDA = PublicKey.findProgramAddressSync(
      [
        Buffer.from("poll"),
        owner.publicKey.toBuffer(),
        new anchor.BN(POLL_ID).toArrayLike(Buffer, "le", 4),
      ],
      program.programId
    )[0];

    const queueSig = await program.methods
      .vote(
        POLL_ID,
        Array.from(ciphertext[0]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString()),
        new anchor.BN(deserializeLE(pollNonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        authority: owner.publicKey,
        pollAcc: pollPDA,
      })
      .rpc({ commitment: "confirmed" });
    console.log("Vote queue sig is ", queueSig);

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      queueSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize vote sig is ", finalizeSig);

    const voteEvent = await voteEventPromise;
    console.log("Vote event is ", voteEvent);

    const revealEventPromise = awaitEvent("revealResultEvent");

    const revealQueueSig = await program.methods
      .revealResult(POLL_ID, new anchor.BN(deserializeLE(pollNonce).toString()))
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
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

    console.log("Vote stats comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initVoteStatsCompDef()
      .accounts({ compDefAccount: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init vote stats computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/init_vote_stats.arcis"
      );

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

    console.log("Vote comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initVoteCompDef()
      .accounts({ compDefAccount: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init vote computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("confidential-ixs/build/vote.arcis");

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

  async function initRRCompDef(
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

    console.log("RR comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initRevealResultCompDef()
      .accounts({ compDefAccount: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init reveal_result computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/reveal_result.arcis"
      );

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

  async function createNewPoll(
    program: Program<Voting>,
    pollId: number,
    question: string,
    owner: PublicKey
  ): Promise<{ sig: string; nonce: Buffer }> {
    const nonce = randomBytes(16);

    return {
      sig: await program.methods
        .createNewPoll(
          pollId,
          question,
          new anchor.BN(deserializeLE(nonce).toString())
        )
        .accountsPartial({
          payer: owner,
          clusterAccount: arciumEnv.arciumClusterPubkey,
        })
        .rpc(),
      nonce: nonce,
    };
  }
});

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
