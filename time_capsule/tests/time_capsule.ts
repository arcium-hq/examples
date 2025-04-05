import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { TimeCapsule } from "../target/types/time_capsule";
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
  deserializeLE,
  getMXEAccAcc,
  getMempoolAcc,
  getCompDefAcc,
  getExecutingPoolAcc,
  x25519,
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("TimeCapsule", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.TimeCapsule as Program<TimeCapsule>;
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
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing reveal secret computation definition");
    const initRevealSecretCompDefSig = await initRevealSecretCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Reveal secret computation definition initialized with signature",
      initRevealSecretCompDefSig
    );

    const privateKey = x25519.utils.randomPrivateKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const mxePublicKey = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const secret = BigInt(42); // The answer to the universe
    const plaintext = [secret];

    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt(plaintext, nonce);

    const receiver = Keypair.generate();
    const receiverPublicKey = receiver.publicKey;
    const receiverNonce = randomBytes(16);

    const receiverSharedSecret = x25519.getSharedSecret(
      receiver.secretKey,
      publicKey
    );
    const receiverCipher = new RescueCipher(receiverSharedSecret);

    const storeSecretSig = await program.methods
      .storeSecret(
        Array.from(ciphertext[0]),
        new anchor.BN(5),
        Array.from(receiverPublicKey.toBuffer()),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .rpc();
    console.log("Store secret sig is ", storeSecretSig);

    const revealSecretEventPromise = awaitEvent("revealSecretEvent");

    const queueSig = await program.methods
      .revealSecret(
        Array.from(receiverPublicKey.toBuffer()),
        new anchor.BN(deserializeLE(receiverNonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("reveal_secret")).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Queue sig is ", queueSig);

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      queueSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const revealSecretEvent = await revealSecretEventPromise;
    const decrypted = receiverCipher.decrypt(
      [revealSecretEvent.secret],
      receiverNonce
    )[0];
    expect(decrypted).to.equal(secret);
  });

  async function initRevealSecretCompDef(
    program: Program<TimeCapsule>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("reveal_secret");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initRevealSecretCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init reveal secret computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/reveal_secret.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "reveal_secret",
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
