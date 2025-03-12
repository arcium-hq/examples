import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { RockPaperScissors } from "../target/types/rock_paper_scissors";
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
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

describe("RockPaperScissors", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .RockPaperScissors as Program<RockPaperScissors>;
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

  it("Plays a game of rock paper scissors!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing compare moves computation definition");
    const initSig = await initCompareMovesCompDef(program, owner, false);
    console.log(
      "Compare moves computation definition initialized with signature",
      initSig
    );

    // Setup encryption
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

    // Player chooses rock (0)
    const playerMove = BigInt(0);
    // House randomly chooses paper (1)
    const houseMove = BigInt(1);
    
    const moves = [playerMove, houseMove];
    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt(moves, nonce);

    const gameEventPromise = awaitEvent("gameResultEvent");

    // Queue the game computation
    const queueSig = await program.methods
      .playGame(
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

    // Wait for computation to finish
    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      queueSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    // Get the game result
    const gameEvent = await gameEventPromise;
    const result = gameEvent.result;
    
    // House should win (2) since paper beats rock
    expect(result).to.equal(2);
  });

  async function initCompareMovesCompDef(
    program: Program<RockPaperScissors>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("compare_moves");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initCompareMovesCompDef()
      .accounts({ compDefAccount: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init compare moves computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "build/compare_moves.arcis"
      );

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "compare_moves",
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
