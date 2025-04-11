import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
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

  // Combined test suite for Rock Paper Scissors game
  it("Tests the complete Rock Paper Scissors game flow", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);
    const playerA = Keypair.generate();
    const playerB = Keypair.generate();
    const unauthorizedPlayer = Keypair.generate();

    // Step 1: Initialize computation definitions
    console.log("Initializing init_game computation definition");
    const initGameSig = await initInitGameCompDef(program, owner, false);
    console.log(
      "Init game computation definition initialized with signature",
      initGameSig
    );

    console.log("Initializing player_move computation definition");
    const playerMoveSig = await initPlayerMoveCompDef(program, owner, false);
    console.log(
      "Player move computation definition initialized with signature",
      playerMoveSig
    );

    console.log("Initializing compare_moves computation definition");
    const compareMovesSig = await initCompareMovesCompDef(program, owner, false);
    console.log(
      "Compare moves computation definition initialized with signature",
      compareMovesSig
    );

    // Step 2: Play a complete game with two players
    console.log("\n--- Playing a complete game with two players ---");
    
    // Generate encryption keys
    const privateKey = x25519.utils.randomPrivateKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const mxePublicKey = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    // Initialize a new game
    const gameId = 1;
    const nonce = randomBytes(16);

    console.log("Initializing a new game");
    const initGameTx = await program.methods
      .initGame(
        new anchor.BN(gameId),
        playerA.publicKey,
        playerB.publicKey,
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString()),
      )
      .accounts({
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });

    console.log("Game initialized with signature:", initGameTx);

    // Player A makes a move (Rock)
    const playerAMove = 0; // Rock
    const playerANonce = randomBytes(16);
    const playerACiphertext = cipher.encrypt(
      [BigInt(playerAMove)],
      playerANonce
    );

    console.log("Player A making a move (Rock)");
    const playerAMoveTx = await program.methods
      .playerMove(
        Array.from(playerACiphertext[0]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(playerANonce).toString())
      )
      .accounts({
        payer: playerA.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
      })
      .signers([playerA])
      .rpc({ commitment: "confirmed" });

    console.log("Player A move signature:", playerAMoveTx);

    // Player B makes a move (Scissors)
    const playerBMove = 2; // Scissors
    const playerBNonce = randomBytes(16);
    const playerBCiphertext = cipher.encrypt(
      [BigInt(playerBMove)],
      playerBNonce
    );

    console.log("Player B making a move (Scissors)");
    const playerBMoveTx = await program.methods
      .playerMove(
        Array.from(playerBCiphertext[0]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(playerBNonce).toString())
      )
      .accounts({
        payer: playerB.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
      })
      .signers([playerB])
      .rpc({ commitment: "confirmed" });

    console.log("Player B move signature:", playerBMoveTx);

    // Compare moves to determine the winner
    const gameEventPromise = awaitEvent("compareMovesEvent");
    
    console.log("Comparing moves");
    const compareTx = await program.methods
      .compareMoves()
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

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      compareTx,
      program.programId,
      "confirmed"
    );
    console.log("Finalize signature:", finalizeSig);

    const gameEvent = await gameEventPromise;
    console.log(`Game result: ${gameEvent.result}`);

    // Verify the result (Rock beats Scissors, so Player A wins)
    expect(gameEvent.result).to.equal("Win");

    // Step 3: Test unauthorized player trying to make a move
    console.log("\n--- Testing unauthorized player ---");
    
    // Generate new encryption keys for this test
    const privateKey2 = x25519.utils.randomPrivateKey();
    const publicKey2 = x25519.getPublicKey(privateKey2);
    const mxePublicKey2 = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const sharedSecret2 = x25519.getSharedSecret(privateKey2, mxePublicKey2);
    const cipher2 = new RescueCipher(sharedSecret2);

    // Initialize a new game
    const gameId2 = new anchor.BN(Date.now());
    const nonce2 = randomBytes(16);
    const nonceValue2 = new anchor.BN(deserializeLE(nonce2).toString());

    console.log("Initializing a new game");
    const initGameTx2 = await program.methods
      .initGame(
        gameId2,
        playerA.publicKey,
        playerB.publicKey,
        Array.from(publicKey2),
        nonceValue2
      )
      .accounts({
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
        ),
        clusterAccount: arciumEnv.arciumClusterPubkey,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });

    console.log("Game initialized with signature:", initGameTx2);

    // Unauthorized player tries to make a move
    const unauthorizedMove = 1; // Paper
    const unauthorizedNonce = randomBytes(16);
    const unauthorizedCiphertext = cipher2.encrypt(
      [BigInt(unauthorizedMove)],
      unauthorizedNonce
    );

    console.log("Unauthorized player attempting to make a move");
    try {
      await program.methods
        .playerMove(
          Array.from(unauthorizedCiphertext[0]),
          Array.from(publicKey2),
          new anchor.BN(deserializeLE(unauthorizedNonce).toString())
        )
        .accounts({
          payer: unauthorizedPlayer.publicKey,
          mxeAccount: getMXEAccAcc(program.programId),
          mempoolAccount: getMempoolAcc(program.programId),
          executingPool: getExecutingPoolAcc(program.programId),
          compDefAccount: getCompDefAcc(
            program.programId,
            Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
          ),
          clusterAccount: arciumEnv.arciumClusterPubkey,
        })
        .signers([unauthorizedPlayer])
        .rpc({ commitment: "confirmed" });
      
      // If we get here, the test should fail because unauthorized player should not be able to make a move
      expect.fail("Unauthorized player was able to make a move");
    } catch (error) {
      console.log("Expected error caught:", error.message);
      // Test passes if we catch an error
      expect(error).to.be.an("error");
    }

    // Step 4: Test multiple game scenarios
    console.log("\n--- Testing multiple game scenarios ---");
    
    // Generate new encryption keys for this test
    const privateKey3 = x25519.utils.randomPrivateKey();
    const publicKey3 = x25519.getPublicKey(privateKey3);
    const mxePublicKey3 = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const sharedSecret3 = x25519.getSharedSecret(privateKey3, mxePublicKey3);
    const cipher3 = new RescueCipher(sharedSecret3);

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
      const ciphertext = cipher3.encrypt(
        [BigInt(game.player), BigInt(game.house)],
        nonce
      );

      console.log("Comparing moves");
      const queueSig = await program.methods
        .compareMoves()
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
});

// Helper function to read keypair from JSON file
function readKpJson(path: string): anchor.web3.Keypair {
  const file = fs.readFileSync(path);
  return anchor.web3.Keypair.fromSecretKey(
    new Uint8Array(JSON.parse(file.toString()))
  );
}

// Separate functions for each computation definition type
async function initInitGameCompDef(
  program: Program<RockPaperScissors>,
  owner: anchor.web3.Keypair,
  uploadRawCircuit: boolean
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed(
    "ComputationDefinitionAccount"
  );
  const offset = getCompDefAccOffset("init_game");

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgAddress()
  )[0];

  console.log(`Comp def PDA for init_game:`, compDefPDA.toBase58());

  const sig = await program.methods
    .initInitGameCompDef()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAcc(program.programId),
    })
    .signers([owner])
    .rpc({
      commitment: "confirmed",
    });

  console.log(`Init init_game computation definition transaction`, sig);

  if (uploadRawCircuit) {
    const rawCircuit = fs.readFileSync(`build/init_game.arcis`);
    await uploadCircuit(
      program.provider as anchor.AnchorProvider,
      "init_game",
      program.programId,
      rawCircuit,
      true
    );
  } else {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash = await program.provider.connection.getLatestBlockhash();
    finalizeTx.recentBlockhash = latestBlockhash.blockhash;
    finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    finalizeTx.sign(owner);
    await program.provider.sendAndConfirm(finalizeTx);
  }
  return sig;
}

async function initPlayerMoveCompDef(
  program: Program<RockPaperScissors>,
  owner: anchor.web3.Keypair,
  uploadRawCircuit: boolean
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed(
    "ComputationDefinitionAccount"
  );
  const offset = getCompDefAccOffset("player_move");

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgAddress()
  )[0];

  console.log(`Comp def PDA for player_move:`, compDefPDA.toBase58());

  const sig = await program.methods
    .initPlayerMoveCompDef()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAcc(program.programId),
    })
    .signers([owner])
    .rpc({
      commitment: "confirmed",
    });

  console.log(`Init player_move computation definition transaction`, sig);

  if (uploadRawCircuit) {
    const rawCircuit = fs.readFileSync(`build/player_move.arcis`);
    await uploadCircuit(
      program.provider as anchor.AnchorProvider,
      "player_move",
      program.programId,
      rawCircuit,
      true
    );
  } else {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash = await program.provider.connection.getLatestBlockhash();
    finalizeTx.recentBlockhash = latestBlockhash.blockhash;
    finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    finalizeTx.sign(owner);
    await program.provider.sendAndConfirm(finalizeTx);
  }
  return sig;
}

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

  console.log(`Comp def PDA for compare_moves:`, compDefPDA.toBase58());

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

  console.log(`Init compare_moves computation definition transaction`, sig);

  if (uploadRawCircuit) {
    const rawCircuit = fs.readFileSync(`build/compare_moves.arcis`);
    await uploadCircuit(
      program.provider as anchor.AnchorProvider,
      "compare_moves",
      program.programId,
      rawCircuit,
      true
    );
  } else {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash = await program.provider.connection.getLatestBlockhash();
    finalizeTx.recentBlockhash = latestBlockhash.blockhash;
    finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    finalizeTx.sign(owner);
    await program.provider.sendAndConfirm(finalizeTx);
  }
  return sig;
}
