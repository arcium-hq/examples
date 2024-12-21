import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { RockPaperScissors } from "../target/types/rock_paper_scissors";
import {
  getClusterDAInfo,
  getArciumEnv,
  encryptAndEncodeInput,
  MScalar,
  DANodeClient,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  uploadCircuit,
  buildFinalizeCompDefTx,
  awaitComputationFinalization,
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";

describe("RockPaperScissors", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .RockPaperScissors as Program<RockPaperScissors>;
  const provider = anchor.getProvider();

  const arciumEnv = getArciumEnv();
  const daNodeClient = new DANodeClient(arciumEnv.DANodeURL);

  it("Is initialized!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing commit choice computation definition");
    const initCCSig = await initCommitChoiceCompDef(program, owner, false);
    console.log(
      "Commit choice computation definition initialized with signature",
      initCCSig
    );

    console.log("Initializing decide winner computation definition");
    const initDWSig = await initDecideWinnerCompDef(program, owner, false);
    console.log(
      "Decide winner computation definition initialized with signature",
      initDWSig
    );

    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );

    const val1 = BigInt(1) as MScalar;
    const val2 = BigInt(2) as MScalar;
    const req1 = encryptAndEncodeInput(val1, cluster_da_info);
    const req2 = encryptAndEncodeInput(val2, cluster_da_info);
    const oref1 = await daNodeClient.postOffchainReference(req1);
    const oref2 = await daNodeClient.postOffchainReference(req2);

    const queueSig = await program.methods
      .addTogether(oref1, oref2)
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
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

    const tx = await provider.connection.getTransaction(
      finalizeSig.finalizeSignature,
      {
        commitment: "confirmed",
        maxSupportedTransactionVersion: 0,
      }
    );
    console.log("Logs are ", tx.meta.logMessages);
  });

  async function initCommitChoiceCompDef(
    program: Program<RockPaperScissors>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("commit_choice");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initCommitChoiceCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init commit_choice computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/commit_choice.arcis"
      );

      await uploadCircuit(
        provider.connection,
        owner,
        "commit_choice",
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

  async function initDecideWinnerCompDef(
    program: Program<RockPaperScissors>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("decide_winner");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initDecideWinnerCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init decide_winner computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/decide_winner.arcis"
      );

      await uploadCircuit(
        provider.connection,
        owner,
        "decide_winner",
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
});

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
