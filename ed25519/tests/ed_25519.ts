import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Ed25519 } from "../target/types/ed_25519";
import { randomBytes } from "crypto";
import {
  arcisEd25519,
  awaitComputationFinalization,
  compressUint128,
  getArciumEnv,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgramId,
  uploadCircuit,
  buildFinalizeCompDefTx,
  RescueCipher,
  deserializeLE,
  getMXEAccAddress,
  getMXEArcisEd25519VerifyingKey,
  getMXEPublicKey,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  getComputationAccAddress,
  getClusterAccAddress,
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("Ed25519", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .Ed25519 as Program<Ed25519>;
  const provider = anchor.getProvider();

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;
  const awaitEvent = async <E extends keyof Event>(
    eventName: E
  ): Promise<Event[E]> => {
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

  it("sign and verify with MPC Ed25519", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing computation definitions");
    const initSMSig = await initSignMessageCompDef(program, owner, false, false);
    console.log("Sign message computation definition initialized with signature", initSMSig);

    const initVSSig = await initVerifySignatureCompDef(program, owner, false, false);
    console.log("Verify signature computation definition initialized with signature", initVSSig);

    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey:", mxePublicKey);

    console.log("\nSigning message with MPC Ed25519");
    let message = new TextEncoder().encode('hello');

    const signMessageEventPromise = awaitEvent("signMessageEvent");
    const computationOffsetSignMessage = new anchor.BN(randomBytes(8), "hex");

    const queueSigSignMessage = await program.methods
      .signMessage(
        computationOffsetSignMessage,
        Array.from(message),
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffsetSignMessage
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("sign_message")).readUInt32LE()
        ),
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      computationOffsetSignMessage,
      program.programId,
      "confirmed"
    );

    const signMessageEvent = await signMessageEventPromise;
    const mxeSignature = new Uint8Array(signMessageEvent.signature);
    const mxeVerifyingKey = await getMXEArcisEd25519VerifyingKey(
      provider as anchor.AnchorProvider,
      program.programId
    );

    const isValid = arcisEd25519.verify(mxeSignature, message, mxeVerifyingKey);
    console.log("Signature verified successfully");
    expect(isValid).to.equal(true);

    console.log("\nVerifying signature with encrypted public key");

    // ephemeral x25519 key to encrypt verifyingKey
    const oneTimePrivateKey = x25519.utils.randomSecretKey();
    const oneTimePublicKey = x25519.getPublicKey(oneTimePrivateKey);
    const oneTimeSharedSecret = x25519.getSharedSecret(oneTimePrivateKey, mxePublicKey);
    const oneTimeCipher = new RescueCipher(oneTimeSharedSecret);
    const oneTimeNonce = randomBytes(16);

    const secretKey = arcisEd25519.utils.randomSecretKey();
    let verifyingKey = arcisEd25519.getPublicKey(secretKey);
    let signature = arcisEd25519.sign(message, secretKey);

    let isValidSignature = randomBytes(1)[0] % 2;

    if (!isValidSignature) {
      const isFakeSignature = randomBytes(1)[0] % 2;
      if (isFakeSignature === 1) {
        signature[32] += 1;
      }

      const isFakeVerifyingKey = randomBytes(1)[0] % 2;
      if (isFakeVerifyingKey === 1) {
        verifyingKey = randomBytes(32);
      }

      const isFakeMessage = randomBytes(1)[0] % 2;
      if (isFakeMessage === 1 || isFakeSignature === 0 && isFakeVerifyingKey === 0) {
        message[0] += 1;
      }
    }

    // we compress verifyingKey as two 128-bit numbers
    let verifyingKeyCompressed = compressUint128(verifyingKey);
    const verifyingKeyCompressedEnc = oneTimeCipher.encrypt(verifyingKeyCompressed, oneTimeNonce);

    // observer who can decrypt isValid
    const observerPrivateKey = x25519.utils.randomSecretKey();
    const observerPublicKey = x25519.getPublicKey(observerPrivateKey);
    const observerSharedSecret = x25519.getSharedSecret(observerPrivateKey, mxePublicKey);
    const observerCipher = new RescueCipher(observerSharedSecret);
    const observerNonce = randomBytes(16);

    const verifySignatureEventPromise = awaitEvent("verifySignatureEvent");
    const computationOffsetVerifySignature = new anchor.BN(randomBytes(8), "hex");

    const queueSigVerifySignature = await program.methods
      .verifySignature(
        computationOffsetVerifySignature,
        Array.from(oneTimePublicKey),
        new anchor.BN(deserializeLE(oneTimeNonce).toString()),
        Array.from(verifyingKeyCompressedEnc[0]),
        Array.from(verifyingKeyCompressedEnc[1]),
        Array.from(message),
        Array.from(signature),
        Array.from(observerPublicKey),
        new anchor.BN(deserializeLE(observerNonce).toString()),
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffsetVerifySignature
        ),
        clusterAccount: getClusterAccAddress(arciumEnv.arciumClusterOffset),
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(arciumEnv.arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("verify_signature")).readUInt32LE()
        ),
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      computationOffsetVerifySignature,
      program.programId,
      "confirmed"
    );

    const verifySignatureEvent = await verifySignatureEventPromise;
    const decrypted = observerCipher.decrypt([verifySignatureEvent.isValid], new Uint8Array(verifySignatureEvent.nonce))[0];
    console.log(`Encrypted verification completed, result: ${decrypted === BigInt(1) ? 'valid' : 'invalid'}`);
    expect(decrypted).to.equal(BigInt(isValidSignature));
  });

  async function initSignMessageCompDef(
    program: Program<Ed25519>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("sign_message");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initSignMessageCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .signers([owner])
      .rpc();
    console.log("\nInit sign message computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/sign_message.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "sign_message",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
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

  async function initVerifySignatureCompDef(
    program: Program<Ed25519>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("verify_signature");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initVerifySignatureCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .signers([owner])
      .rpc();
    console.log("\nInit verify signature computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/verify_signature.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "verify_signature",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
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

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries: number = 20,
  retryDelayMs: number = 500
): Promise<Uint8Array> {
  console.log("");
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      const mxePublicKey = await getMXEPublicKey(provider, programId);
      if (mxePublicKey) {
        return mxePublicKey;
      }
    } catch (error) {
      console.log(`Attempt ${attempt} failed to fetch MXE public key:`, error);
    }

    if (attempt < maxRetries) {
      console.log(
        `Retrying in ${retryDelayMs}ms... (attempt ${attempt}/${maxRetries})`
      );
      await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
    }
  }

  throw new Error(
    `Failed to fetch MXE public key after ${maxRetries} attempts`
  );
}

function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}
