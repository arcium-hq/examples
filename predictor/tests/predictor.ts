import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Predictor } from "../target/types/predictor";
import {
  getClusterDAInfo,
  getArciumEnv,
  encryptAndEncodeInput,
  MFloat,
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

describe("PricePredict", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Predictor as Program<Predictor>;
  const provider = anchor.getProvider();

  const arciumEnv = getArciumEnv();
  const daNodeClient = new DANodeClient(arciumEnv.DANodeURL);

  it("Is initialized!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing predict computation definition");
    const initPredictSig = await initPredictCompDef(program, owner, false);
    console.log(
      "Predict computation definition initialized with signature",
      initPredictSig
    );

    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );

    // Demo values for a logistic regression model
    // y = 1.1 * x + 0.1
    const coeff = 1.1 as MFloat;
    const intercept = 0.1 as MFloat;
    const inputVal = 1.0 as MFloat;
    const req1 = encryptAndEncodeInput(coeff, cluster_da_info);
    const req2 = encryptAndEncodeInput(intercept, cluster_da_info);
    const req3 = encryptAndEncodeInput(inputVal, cluster_da_info);

    const oref1 = await daNodeClient.postOffchainReference(req1);
    const oref2 = await daNodeClient.postOffchainReference(req2);
    const oref3 = await daNodeClient.postOffchainReference(req3);

    const queueSig = await program.methods
      .predictor(oref1, oref2, oref3)
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

  async function initPredictCompDef(
    program: Program<Predictor>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("predict");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initPredictCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init predict computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/predict.arcis"
      );

      await uploadCircuit(
        provider.connection,
        owner,
        "predict",
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
