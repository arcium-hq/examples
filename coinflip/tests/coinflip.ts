import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Coinflip } from "../target/types/coinflip";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getArciumEnv,
  getCompDefAccOffset,
  getArciumAccountBaseSeed,
  getArciumProgramId,
  uploadCircuit,
  RescueCipher,
  deserializeLE,
  getMXEAccAddress,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  x25519,
  getComputationAccAddress,
  getMXEPublicKey,
  getClusterAccAddress,
  getLookupTableAddress,
  getArciumProgram,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";

describe("Coinflip", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Coinflip as Program<Coinflip>;
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
  const clusterAccount = getClusterAccAddress(arciumEnv.arciumClusterOffset);

  it("flip a coin!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);

    console.log("Initializing flip computation definition");
    const initFlipSig = await initFlipCompDef(program, owner);
    console.log(
      "Flip computation definition initialized with signature",
      initFlipSig
    );

    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const choice = BigInt(true);
    const plaintext = [choice];

    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt(plaintext, nonce);

    const flipEventPromise = awaitEvent("flipEvent");

    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    const queueSig = await program.methods
      .flip(
        computationOffset,
        Array.from(ciphertext[0]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          arciumEnv.arciumClusterOffset,
          computationOffset
        ),
        clusterAccount,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(
          arciumEnv.arciumClusterOffset
        ),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("flip")).readUInt32LE()
        ),
      })
      .rpc({
        skipPreflight: true,
        preflightCommitment: "confirmed",
        commitment: "confirmed",
      });
    console.log("Queue sig is ", queueSig);

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      computationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const flipEvent = await flipEventPromise;

    if (flipEvent.result) {
      console.log("Congratulations! You won!");
    } else {
      console.log("Better luck next time!");
    }
  });

  async function initFlipCompDef(
    program: Program<Coinflip>,
    owner: anchor.web3.Keypair
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("flip");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const arciumProgram = getArciumProgram(provider as anchor.AnchorProvider);
    const mxeAccount = getMXEAccAddress(program.programId);
    const mxeAcc = await arciumProgram.account.mxeAccount.fetch(mxeAccount);
    const lutAddress = getLookupTableAddress(
      program.programId,
      mxeAcc.lutOffsetSlot
    );

    const sig = await program.methods
      .initFlipCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount,
        addressLookupTable: lutAddress,
      })
      .signers([owner])
      .rpc({
        preflightCommitment: "confirmed",
        commitment: "confirmed",
      });
    console.log("Init flip computation definition transaction", sig);

    const rawCircuit = fs.readFileSync("build/flip.arcis");
    await uploadCircuit(
      provider as anchor.AnchorProvider,
      "flip",
      program.programId,
      rawCircuit,
      true
    );

    return sig;
  }
});

async function getMXEPublicKeyWithRetry(
  provider: anchor.AnchorProvider,
  programId: PublicKey,
  maxRetries: number = 20,
  retryDelayMs: number = 500
): Promise<Uint8Array> {
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
