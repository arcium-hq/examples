import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Voting } from "../target/types/voting";
import {
  getClusterDAInfo,
  getArciumEnv,
  encryptAndEncodeInput,
  DANodeClient,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  uploadCircuit,
  buildFinalizeCompDefTx,
  awaitComputationFinalization,
  MBoolean,
  getDataObjPDA,
  MScalar,
  encryptAndEncodeInputArray,
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";

describe("Voting", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Voting as Program<Voting>;
  const provider = anchor.getProvider();

  const arciumEnv = getArciumEnv();
  const daNodeClient = new DANodeClient(arciumEnv.DANodeURL);

  it("Is initialized!", async () => {
    const POLL_ID = 420;
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing add together computation definition");
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

    const pollSig = await createNewPoll(program, daNodeClient, POLL_ID);
    console.log("Poll created with signature", pollSig);

    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );

    const vote = true as MBoolean;
    const voteReq = encryptAndEncodeInput(vote, cluster_da_info);
    const oref1 = await daNodeClient.postOffchainReference(voteReq);
    console.log("Oref1 is ", oref1);

    const queueSig = await program.methods
      .vote(POLL_ID, oref1)
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        authority: owner.publicKey,
      })
      .rpc({ commitment: "confirmed" });
    console.log("Queue sig is ", queueSig);

    const finalizeSig = await awaitComputationFinalization(
      provider.connection,
      queueSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const revealQueueSig = await program.methods
      .revealResult(POLL_ID)
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
      })
      .rpc({ commitment: "confirmed" });
    console.log("Reveal queue sig is ", revealQueueSig);
    const revealFinalizeSig = await awaitComputationFinalization(
      provider.connection,
      revealQueueSig,
      program.programId,
      "confirmed"
    );
    console.log("Reveal finalize sig is ", revealFinalizeSig);

    const tx = await provider.connection.getTransaction(
      revealFinalizeSig.finalizeSignature,
      {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      }
    );
    console.log("Logs are ", tx.meta.logMessages);
  });

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
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init vote computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("confidential-ixs/build/vote.arcis");

      await uploadCircuit(
        provider.connection,
        owner,
        "vote",
        program.programId,
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        owner.publicKey,
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
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
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
        provider.connection,
        owner,
        "reveal_result",
        program.programId,
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        owner.publicKey,
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
    daNodeClient: DANodeClient,
    pollId: number
  ): Promise<string> {
    const votePDA = getDataObjPDA(
      getArciumProgAddress(),
      program.programId,
      pollId
    );

    // Empty vote state is 2 scalars
    const emptyVoteState: MScalar[] = new Array(2).fill(BigInt(0) as MScalar);

    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );
    const req = encryptAndEncodeInputArray(emptyVoteState, cluster_da_info);
    const oref = await daNodeClient.postOffchainReference(req);

    return program.methods
      .createNewPoll(pollId, "$SOL to 500?", oref)
      .accounts({ voteState: votePDA })
      .rpc();
  }
});

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
