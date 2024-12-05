import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Voting } from "../target/types/voting";
import { Vote } from "../confidential-ixs/build/vote";
import {
  ConfidentialInstructionInputs,
  getClusterDAInfo,
  getArciumEnv,
  buildOffchainRefRequest,
  DANodeClient,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  uploadCircuit,
  buildFinalizeCompDefTx,
  trackComputationProgress,
  getDataObjPDA,
  MBoolean,
} from "@elusiv-privacy/arcium-sdk";
import * as fs from "fs";
import * as os from "os";

describe("Voting", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Voting as Program<Voting>;
  const provider = anchor.getProvider();

  const arciumEnv = getArciumEnv();
  const daNodeClient = new DANodeClient(arciumEnv.DANodeURL);

  it("Is initialized and can create new polls!", async () => {
    const POLL_ID = 420;
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log(owner.publicKey.toBase58());
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

    // Vote on the poll
    const inputVal: ConfidentialInstructionInputs<Vote> = [
      {
        value: true as MBoolean,
      },
      {
        offset: 0,
        isMutable: true,
      },
    ];
    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );
    const req = buildOffchainRefRequest(inputVal, cluster_da_info);
    const oref = await daNodeClient.postOffchainReference(req);
    console.log("Built offchain request");
    const queueSig = await program.methods
      .vote(POLL_ID, oref)
      .accounts({
        authority: owner.publicKey,
      })
      .rpc({ commitment: "confirmed" });
    console.log("Voting queue sig is ", queueSig);

    const finalizeSig = await trackComputationProgress(
      provider.connection,
      queueSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize voting sig is ", finalizeSig);

    const revealQueueSig = await program.methods
      .revealResult(POLL_ID)
      .accounts({})
      .rpc({ commitment: "confirmed" });
    console.log("Reveal queue sig is ", revealQueueSig);
    const revealFinalizeSig = await trackComputationProgress(
      provider.connection,
      revealQueueSig,
      program.programId,
      "confirmed"
    );
    console.log("Reveal finalize sig is ", revealFinalizeSig);
  });

  async function initVoteCompDef(
    program: Program<Voting>,
    payer: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("vote");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, offset],
      getArciumProgAddress()
    )[0];

    const sig = await program.methods
      .initVoteCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: payer.publicKey })
      .signers([payer])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init vote computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("confidential-ixs/build/vote.arcis");
      await uploadCircuit(provider.connection, payer, "vote", rawCircuit, true);
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        payer.publicKey,
        Buffer.from(offset).readUInt32LE()
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(payer);
      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }

  async function initRRCompDef(
    program: Program<Voting>,
    payer: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("reveal_result");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, offset],
      getArciumProgAddress()
    )[0];

    const sig = await program.methods
      .initRevealResultCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: payer.publicKey })
      .signers([payer])
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
        payer,
        "reveal_result",
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        payer.publicKey,
        Buffer.from(offset).readUInt32LE()
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(payer);
      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }

  async function createNewPoll(
    program: Program<Voting>,
    daNodeClient: DANodeClient,
    pollId: number
  ): Promise<string> {
    const votePDA = getDataObjPDA(getArciumProgAddress(), pollId);

    // Empty vote stats is 2 scalars
    const emptyVoteState: [{ value: bigint }, { value: bigint }] = [
      {
        value: BigInt(0),
      },
      {
        value: BigInt(0),
      },
    ];

    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );
    const req = buildOffchainRefRequest(emptyVoteState, cluster_da_info);
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
