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
  getMXEAccAcc,
  getMempoolAcc,
  getCompDefAcc,
  getExecutingPoolAcc,
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

  it("Plays Rock Paper Scissors games!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing compare moves computation definition");
    const initSig = await initCompareMovesCompDef(program, owner, false);
    console.log(
      "Compare moves computation definition initialized with signature",
      initSig
    );

    const privateKey = x25519RandomPrivateKey();
    const publicKey = x25519GetPublicKey(privateKey);
    const mxePublicKey = [
      new Uint8Array([
        34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
        253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
      ]),
      new Uint8Array([
        107, 1, 201, 151, 195, 126, 155, 84, 228, 85, 185, 142, 62, 220, 161,
        29, 179, 36, 112, 163, 201, 103, 172, 207, 55, 89, 53, 120, 73, 208,
        234, 63,
      ]),
      new Uint8Array([
        217, 186, 137, 28, 190, 167, 128, 220, 100, 71, 90, 160, 130, 162, 96,
        15, 191, 147, 184, 4, 151, 89, 186, 211, 72, 212, 173, 31, 98, 187, 65,
        59,
      ]),
      new Uint8Array([
        51, 66, 84, 103, 52, 182, 174, 177, 134, 163, 224, 196, 127, 102, 81,
        61, 12, 136, 171, 212, 230, 171, 242, 47, 221, 48, 152, 231, 239, 0,
        183, 15,
      ]),
      new Uint8Array([
        162, 140, 124, 61, 16, 202, 184, 56, 39, 7, 37, 95, 225, 104, 229, 25,
        48, 246, 35, 136, 99, 106, 110, 253, 188, 86, 201, 42, 112, 211, 129,
        34,
      ]),
    ];
    const rescueKey = x25519GetSharedSecretWithMXE(privateKey, mxePublicKey);
    const cipher = new RescueCipher(rescueKey);

    // Play multiple games
    const games = [
      { player: 0, house: 0 }, // Rock vs Rock (Tie)
      { player: 0, house: 2 }, // Rock vs Scissors (Win)
      { player: 1, house: 0 }, // Paper vs Rock (Win)
      { player: 2, house: 1 }, // Scissors vs Paper (Win)
      { player: 2, house: 0 }, // Scissors vs Rock (Loss)
      { player: 1, house: 2 }, // Paper vs Scissors (Loss)
    ];

    for (const game of games) {
      const gameEventPromise = awaitEvent("compareMovesEvent");

      const nonce = randomBytes(16);
      const ciphertext = cipher.encrypt(
        [BigInt(game.player), BigInt(game.house)],
        nonce
      );

      const queueSig = await program.methods
        .compareMoves(
          Array.from(ciphertext[0]),
          Array.from(ciphertext[1]),
          Array.from(publicKey),
          new anchor.BN(deserializeLE(nonce).toString())
        )
        .accounts({
          payer: owner.publicKey,
          mxeAccount: getMXEAccAcc(program.programId),
          mempoolAccount: getMempoolAcc(program.programId),
          executingPool: getExecutingPoolAcc(program.programId),
          compDefAccount: getCompDefAcc(
            program.programId,
            Buffer.from(getCompDefAccOffset("compare_moves")).readUInt32LE()
          ),
          clusterAccount: arciumEnv.arciumClusterPubkey,
        })
        .rpc({ commitment: "confirmed" });

      console.log("Queue signature:", queueSig);

      const finalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        queueSig,
        program.programId,
        "confirmed"
      );
      console.log("Finalize signature:", finalizeSig);

      const gameEvent = await gameEventPromise;
      console.log(`Game result: ${gameEvent.result}`);

      // Verify the result
      let expectedResult: string;
      if (game.player === game.house) {
        expectedResult = "Tie";
      } else if (
        (game.player === 0 && game.house === 2) || // Rock beats Scissors
        (game.player === 1 && game.house === 0) || // Paper beats Rock
        (game.player === 2 && game.house === 1) // Scissors beats Paper
      ) {
        expectedResult = "Win";
      } else {
        expectedResult = "Loss";
      }

      expect(gameEvent.result).to.equal(expectedResult);
    }
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

    console.log("Comp def PDA:", compDefPDA);

    const sig = await program.methods
      .initCompareMovesCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init compare moves computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/compare_moves.arcis");
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
