import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { Blackjack } from "../target/types/blackjack";
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
  serializeLE,
  deserializeLE,
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("Blackjack", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .Blackjack as Program<Blackjack>;
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

    console.log("Initializing add together computation definition");
    const initATSig = await initAddTogetherCompDef(program, owner, false);
    console.log(
      "Add together computation definition initialized with signature",
      initATSig
    );

    const privateKey = x25519RandomPrivateKey();
    const publicKey = x25519GetPublicKey(privateKey);
    const mxePublicKey = [
      new Uint8Array([
        78, 96, 220, 218, 225, 248, 149, 140,
        229, 147, 105, 183, 46, 82, 166, 248,
        146, 35, 137, 78, 122, 181, 200, 220,
        217, 97, 20, 11, 71, 9, 113, 6
      ]),
      new Uint8Array([
        155, 202, 231, 73, 215, 1, 94, 193,
        141, 26, 77, 66, 143, 114, 197, 172,
        160, 245, 64, 108, 236, 104, 149, 242,
        103, 140, 199, 94, 70, 61, 162, 118
      ]),
      new Uint8Array([
        231, 24, 19, 12, 184, 40, 139, 11,
        29, 176, 125, 231, 49, 53, 174, 225,
        183, 156, 234, 55, 49, 240, 169, 70,
        252, 141, 70, 28, 113, 255, 70, 20
      ]),
      new Uint8Array([
        120, 66, 73, 239, 247, 13, 25, 149,
        162, 21, 108, 27, 236, 128, 93, 84,
        210, 18, 70, 106, 80, 82, 111, 61,
        12, 178, 182, 23, 96, 12, 9, 1
      ]),
      new Uint8Array([
        112, 133, 255, 66, 62, 138, 251, 232,
        170, 239, 193, 225, 253, 152, 85, 205,
        19, 16, 50, 193, 41, 248, 39, 175,
        49, 87, 207, 79, 54, 122, 78, 125
      ])
    ];
    const rescueKey = x25519GetSharedSecretWithMXE(privateKey, mxePublicKey);
    const cipher = new RescueCipher(rescueKey);

    const val1 = BigInt(1);
    const val2 = BigInt(2);
    const plaintext = [val1, val2];

    const nonce = randomBytes(16);
    const ciphertext = cipher
      .encrypt(plaintext, nonce);

    const sumEventPromise = awaitEvent("sumEvent");

    const queueSig = await program.methods
      .addTogether(
        Array.from(ciphertext[0]),
        Array.from(ciphertext[1]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString()),
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
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

    const sumEvent = await sumEventPromise;
    const decrypted = cipher.decrypt([sumEvent.sum], nonce)[0]
    expect(decrypted).to.equal(val1 + val2);
  });

  async function initAddTogetherCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("add_together");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initAddTogetherCompDef()
      .accounts({ compDefAccount: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init add together computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/add_together.arcis"
      );

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "add_together",
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
