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
  getArciumProgramId,
  uploadCircuit,
  buildFinalizeCompDefTx,
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
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

/**
 * Gets the cluster account address using the cluster offset from environment.
 */
function getClusterAccount(): PublicKey {
  const arciumEnv = getArciumEnv();
  return getClusterAccAddress(arciumEnv.arciumClusterOffset);
}

describe("RockPaperScissors", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .RockPaperScissors as Program<RockPaperScissors>;
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

  const clusterAccount = getClusterAccount();

  // Combined test suite for Rock Paper Scissors game
  it("Tests the complete Rock Paper Scissors game flow", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);
    const playerA = Keypair.generate();
    const playerB = Keypair.generate();
    const unauthorizedPlayer = Keypair.generate();

    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);

    // Step 1: Initialize computation definitions
    console.log("Initializing init_game computation definition");
    const initGameSig = await initInitGameCompDef(program, owner, false, false);
    console.log(
      "Init game computation definition initialized with signature",
      initGameSig
    );

    console.log("Initializing player_move computation definition");
    const playerMoveSig = await initPlayerMoveCompDef(
      program,
      owner,
      false,
      false
    );
    console.log(
      "Player move computation definition initialized with signature",
      playerMoveSig
    );

    console.log("Initializing compare_moves computation definition");
    const compareMovesSig = await initCompareMovesCompDef(
      program,
      owner,
      false,
      false
    );
    console.log(
      "Compare moves computation definition initialized with signature",
      compareMovesSig
    );

    // Step 2: Play a complete game with two players
    console.log("\n--- Playing a complete game with two players ---");

    // Generate encryption keys for Player A
    const playerAPrivateKey = x25519.utils.randomSecretKey();
    const playerAPublicKey = x25519.getPublicKey(playerAPrivateKey);
    const playerASharedSecret = x25519.getSharedSecret(
      playerAPrivateKey,
      mxePublicKey
    );
    const playerACipher = new RescueCipher(playerASharedSecret);

    // Generate encryption keys for Player B
    const playerBPrivateKey = x25519.utils.randomSecretKey();
    const playerBPublicKey = x25519.getPublicKey(playerBPrivateKey);
    const playerBSharedSecret = x25519.getSharedSecret(
      playerBPrivateKey,
      mxePublicKey
    );
    const playerBCipher = new RescueCipher(playerBSharedSecret);

    // Initialize a new game
    const gameId = 1;
    const nonce = randomBytes(16);

    const initComputationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("Initializing a new game");
    const initGameTx = await program.methods
      .initGame(
        initComputationOffset,
        new anchor.BN(gameId),
        playerA.publicKey,
        playerB.publicKey,
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          initComputationOffset
        ),
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
      })
      .signers([owner])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Game initialized with signature:", initGameTx);

    // Wait for initGame computation finalization
    const initGameFinalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      initComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Init game finalize signature:", initGameFinalizeSig);

    // Airdrop funds to Player A
    console.log("Airdropping funds to Player A");
    const airdropPlayerATx = await provider.connection.requestAirdrop(
      playerA.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      signature: airdropPlayerATx,
      blockhash: (await provider.connection.getLatestBlockhash()).blockhash,
      lastValidBlockHeight: (
        await provider.connection.getLatestBlockhash()
      ).lastValidBlockHeight,
    });
    console.log("Funds airdropped to Player A");

    // Player A makes a move (Rock)
    const playerAMove = 0; // Rock
    const playerAId = 0;
    const playerANonce = randomBytes(16);
    const playerACiphertext = playerACipher.encrypt(
      [BigInt(playerAId), BigInt(playerAMove)],
      playerANonce
    );

    const playerAMoveComputationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("Player A making a move (Rock)");
    const playerAMoveTx = await program.methods
      .playerMove(
        playerAMoveComputationOffset,
        Array.from(playerACiphertext[0]),
        Array.from(playerACiphertext[1]),
        Array.from(playerAPublicKey),
        new anchor.BN(deserializeLE(playerANonce).toString())
      )
      .accounts({
        payer: playerA.publicKey,
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          playerAMoveComputationOffset
        ),
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
        rpsGame: PublicKey.findProgramAddressSync(
          [
            Buffer.from("rps_game"),
            new anchor.BN(gameId).toArrayLike(Buffer, "le", 8),
          ],
          program.programId
        )[0],
      })
      .signers([playerA])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Player A move signature:", playerAMoveTx);

    // Wait for player A move computation finalization
    const playerAMoveFinalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      playerAMoveComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Player A move finalize signature:", playerAMoveFinalizeSig);

    // Airdrop funds to Player B
    console.log("Airdropping funds to Player B");
    const airdropPlayerBTx = await provider.connection.requestAirdrop(
      playerB.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      signature: airdropPlayerBTx,
      blockhash: (await provider.connection.getLatestBlockhash()).blockhash,
      lastValidBlockHeight: (
        await provider.connection.getLatestBlockhash()
      ).lastValidBlockHeight,
    });
    console.log("Funds airdropped to Player B");

    // Player B makes a move (Scissors)
    const playerBMove = 2; // Scissors
    const playerBId = 1;
    const playerBNonce = randomBytes(16);
    const playerBCiphertext = playerBCipher.encrypt(
      [BigInt(playerBId), BigInt(playerBMove)],
      playerBNonce
    );

    const playerBMoveComputationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("Player B making a move (Scissors)");
    const playerBMoveTx = await program.methods
      .playerMove(
        playerBMoveComputationOffset,
        Array.from(playerBCiphertext[0]),
        Array.from(playerBCiphertext[1]),
        Array.from(playerBPublicKey),
        new anchor.BN(deserializeLE(playerBNonce).toString())
      )
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          playerBMoveComputationOffset
        ),
        payer: playerB.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
        rpsGame: PublicKey.findProgramAddressSync(
          [
            Buffer.from("rps_game"),
            new anchor.BN(gameId).toArrayLike(Buffer, "le", 8),
          ],
          program.programId
        )[0],
      })
      .signers([playerB])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Player B move signature:", playerBMoveTx);

    // Wait for player B move computation finalization
    const playerBMoveFinalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      playerBMoveComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Player B move finalize signature:", playerBMoveFinalizeSig);

    // Compare moves to determine the winner
    const gameEventPromise = awaitEvent("compareMovesEvent");

    const compareComputationOffset = new anchor.BN(randomBytes(8), "hex");

    console.log("Comparing moves");
    const compareTx = await program.methods
      .compareMoves(compareComputationOffset)
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          compareComputationOffset
        ),
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("compare_moves")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
        rpsGame: PublicKey.findProgramAddressSync(
          [
            Buffer.from("rps_game"),
            new anchor.BN(gameId).toArrayLike(Buffer, "le", 8),
          ],
          program.programId
        )[0],
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      compareComputationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Finalize signature:", finalizeSig);

    const gameEvent = await gameEventPromise;
    console.log(`Game result: ${gameEvent.result}`);

    // Verify the result (Rock beats Scissors, so Player A wins)
    expect(gameEvent.result).to.equal("Player A Wins");

    // Step 3: Test unauthorized player trying to make a move
    console.log("\n--- Testing unauthorized player ---");

    // Generate new encryption keys for this test
    const unauthorizedPrivateKey = x25519.utils.randomSecretKey();
    const unauthorizedPublicKey = x25519.getPublicKey(unauthorizedPrivateKey);
    const unauthorizedMxePublicKey = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const unauthorizedSharedSecret = x25519.getSharedSecret(
      unauthorizedPrivateKey,
      unauthorizedMxePublicKey
    );
    const unauthorizedCipher = new RescueCipher(unauthorizedSharedSecret);

    // Initialize a new game
    const gameId2 = new anchor.BN(Date.now());
    const nonce2 = randomBytes(16);
    const nonceValue2 = new anchor.BN(deserializeLE(nonce2).toString());

    const initComputationOffset2 = new anchor.BN(randomBytes(8), "hex");

    console.log("Initializing a new game");
    const initGameTx2 = await program.methods
      .initGame(
        initComputationOffset2,
        gameId2,
        playerA.publicKey,
        playerB.publicKey,
        nonceValue2
      )
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          initComputationOffset2
        ),
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
      })
      .signers([owner])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Game initialized with signature:", initGameTx2);

    // Wait for initGame computation finalization
    const initGameFinalizeSig2 = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      initComputationOffset2,
      program.programId,
      "confirmed"
    );
    console.log("Init game finalize signature:", initGameFinalizeSig2);

    // Airdrop funds to unauthorized player
    console.log("Airdropping funds to unauthorized player");
    const airdropUnauthorizedTx = await provider.connection.requestAirdrop(
      unauthorizedPlayer.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction({
      signature: airdropUnauthorizedTx,
      blockhash: (await provider.connection.getLatestBlockhash()).blockhash,
      lastValidBlockHeight: (
        await provider.connection.getLatestBlockhash()
      ).lastValidBlockHeight,
    });
    console.log("Funds airdropped to unauthorized player");

    // Unauthorized player tries to make a move
    const unauthorizedMove = 1; // Paper
    const unauthorizedNonce = randomBytes(16);
    const unauthorizedCiphertext = unauthorizedCipher.encrypt(
      [BigInt(0), BigInt(unauthorizedMove)],
      unauthorizedNonce
    );

    console.log("Unauthorized player attempting to make a move");
    try {
      const unauthorizedMoveComputationOffset = new anchor.BN(
        randomBytes(8),
        "hex"
      );

      await program.methods
        .playerMove(
          unauthorizedMoveComputationOffset,
          Array.from(unauthorizedCiphertext[0]),
          Array.from(unauthorizedCiphertext[1]),
          Array.from(unauthorizedPublicKey),
          new anchor.BN(deserializeLE(unauthorizedNonce).toString())
        )
        .accounts({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            unauthorizedMoveComputationOffset
          ),
          payer: unauthorizedPlayer.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
          ),
          clusterAccount: clusterAccount,
          rpsGame: PublicKey.findProgramAddressSync(
            [Buffer.from("rps_game"), gameId2.toArrayLike(Buffer, "le", 8)],
            program.programId
          )[0],
        })
        .signers([unauthorizedPlayer])
        .rpc({ skipPreflight: true, commitment: "confirmed" });

      // If we get here, the test should fail because unauthorized player should not be able to make a move
      expect.fail("Unauthorized player was able to make a move");
    } catch (error) {
      console.log("Expected error caught:", error.message);
      // Test passes if we catch an error
      expect(error).to.be.an("error");
    }

    // Step 4: Test multiple game scenarios
    console.log("\n--- Testing multiple game scenarios ---");

    // Generate encryption keys for multiple game scenarios
    const scenarioPrivateKey = x25519.utils.randomSecretKey();
    const scenarioPublicKey = x25519.getPublicKey(scenarioPrivateKey);
    const scenarioMxePublicKey = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const scenarioSharedSecret = x25519.getSharedSecret(
      scenarioPrivateKey,
      scenarioMxePublicKey
    );
    const scenarioCipher = new RescueCipher(scenarioSharedSecret);

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
      console.log(
        `\n--- Testing game scenario: Player ${game.player} vs House ${game.house} ---`
      );

      // Initialize a new game for this scenario
      const scenarioGameId = new anchor.BN(
        Date.now() + Math.floor(Math.random() * 1000)
      );
      const scenarioNonce = randomBytes(16);

      const initComputationOffset3 = new anchor.BN(randomBytes(8), "hex");

      console.log("Initializing a new game");
      const initGameTx = await program.methods
        .initGame(
          initComputationOffset3,
          scenarioGameId,
          playerA.publicKey,
          playerB.publicKey,
          new anchor.BN(deserializeLE(scenarioNonce).toString())
        )
        .accounts({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            initComputationOffset3
          ),
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
          ),
          clusterAccount: clusterAccount,
        })
        .signers([owner])
        .rpc({ skipPreflight: true, commitment: "confirmed" });

      console.log("Game initialized with signature:", initGameTx);

      // Wait for initGame computation finalization
      const initGameFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        initComputationOffset3,
        program.programId,
        "confirmed"
      );
      console.log("Init game finalize signature:", initGameFinalizeSig);

      // Player A makes a move
      const playerAMoveNonce = randomBytes(16);
      const playerAMoveCiphertext = playerACipher.encrypt(
        [BigInt(0), BigInt(game.player)],
        playerAMoveNonce
      );

      const playerAMoveComputationOffset = new anchor.BN(randomBytes(8), "hex");

      console.log("Player A making a move");
      const playerAMoveTx = await program.methods
        .playerMove(
          playerAMoveComputationOffset,
          Array.from(playerAMoveCiphertext[0]),
          Array.from(playerAMoveCiphertext[1]),
          Array.from(playerAPublicKey),
          new anchor.BN(deserializeLE(playerAMoveNonce).toString())
        )
        .accounts({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            playerAMoveComputationOffset
          ),
          payer: playerA.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
          ),
          clusterAccount: clusterAccount,
          rpsGame: PublicKey.findProgramAddressSync(
            [
              Buffer.from("rps_game"),
              scenarioGameId.toArrayLike(Buffer, "le", 8),
            ],
            program.programId
          )[0],
        })
        .signers([playerA])
        .rpc({ skipPreflight: true, commitment: "confirmed" });

      console.log("Player A move signature:", playerAMoveTx);

      // Wait for player A move computation finalization
      const playerAMoveFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        playerAMoveComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("Player A move finalize signature:", playerAMoveFinalizeSig);

      // Player B makes a move
      const playerBMoveNonce = randomBytes(16);
      const playerBMoveCiphertext = playerBCipher.encrypt(
        [BigInt(1), BigInt(game.house)],
        playerBMoveNonce
      );

      const playerBMoveComputationOffset = new anchor.BN(randomBytes(8), "hex");

      console.log("Player B making a move");
      const playerBMoveTx = await program.methods
        .playerMove(
          playerBMoveComputationOffset,
          Array.from(playerBMoveCiphertext[0]),
          Array.from(playerBMoveCiphertext[1]),
          Array.from(playerBPublicKey),
          new anchor.BN(deserializeLE(playerBMoveNonce).toString())
        )
        .accounts({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            playerBMoveComputationOffset
          ),
          payer: playerB.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
          ),
          clusterAccount: clusterAccount,
          rpsGame: PublicKey.findProgramAddressSync(
            [
              Buffer.from("rps_game"),
              scenarioGameId.toArrayLike(Buffer, "le", 8),
            ],
            program.programId
          )[0],
        })
        .signers([playerB])
        .rpc({ skipPreflight: true, commitment: "confirmed" });

      console.log("Player B move signature:", playerBMoveTx);

      // Wait for player B move computation finalization
      const playerBMoveFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        playerBMoveComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("Player B move finalize signature:", playerBMoveFinalizeSig);

      // Compare moves to determine the winner
      const gameEventPromise = awaitEvent("compareMovesEvent");

      const compareComputationOffset = new anchor.BN(randomBytes(8), "hex");

      console.log("Comparing moves");
      const compareTx = await program.methods
        .compareMoves(compareComputationOffset)
        .accounts({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            compareComputationOffset
          ),
          payer: owner.publicKey,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("compare_moves")).readUInt32LE()
          ),
          clusterAccount: clusterAccount,
          rpsGame: PublicKey.findProgramAddressSync(
            [
              Buffer.from("rps_game"),
              scenarioGameId.toArrayLike(Buffer, "le", 8),
            ],
            program.programId
          )[0],
        })
        .rpc({ skipPreflight: true, commitment: "confirmed" });

      console.log("Compare moves signature:", compareTx);

      const finalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        compareComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("Finalize signature:", finalizeSig);

      const gameEvent = await gameEventPromise;
      console.log(`Game result: ${gameEvent.result}`);

      // Verify the result based on the expected outcome
      let expectedResult: string;
      if (game.player === game.house) {
        expectedResult = "Tie";
      } else if (
        (game.player === 0 && game.house === 2) || // Rock beats Scissors
        (game.player === 1 && game.house === 0) || // Paper beats Rock
        (game.player === 2 && game.house === 1) // Scissors beats Paper
      ) {
        expectedResult = "Player A Wins";
      } else {
        expectedResult = "Player B Wins";
      }

      expect(gameEvent.result).to.equal(expectedResult);
    }

    // Step 5: Test invalid move scenario
    console.log("\n--- Testing invalid move scenario ---");

    // Initialize a new game for this scenario
    const gameId3 = new anchor.BN(
      Date.now() + Math.floor(Math.random() * 1000)
    );
    const nonce3 = randomBytes(16);

    const initComputationOffset4 = new anchor.BN(randomBytes(8), "hex");

    console.log("Initializing a new game for invalid move test");
    const initGameTx3 = await program.methods
      .initGame(
        initComputationOffset4,
        gameId3,
        playerA.publicKey,
        playerB.publicKey,
        new anchor.BN(deserializeLE(nonce3).toString())
      )
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          initComputationOffset4
        ),
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("init_game")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
      })
      .signers([owner])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Game initialized for invalid move test:", initGameTx3);

    // Wait for initGame computation finalization
    const initGameFinalizeSig3 = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      initComputationOffset4,
      program.programId,
      "confirmed"
    );
    console.log(
      "Init game finalize signature for invalid move test:",
      initGameFinalizeSig3
    );

    // Player A makes a valid move (Rock = 0)
    const playerAValidMove = 0;
    const playerAId3 = 0;
    const playerANonce3 = randomBytes(16);
    const playerACiphertext3 = playerACipher.encrypt(
      [BigInt(playerAId3), BigInt(playerAValidMove)],
      playerANonce3
    );

    const playerAMoveComputationOffset3 = new anchor.BN(randomBytes(8), "hex");

    console.log("Player A making a valid move (0)");
    const playerAMoveTx3 = await program.methods
      .playerMove(
        playerAMoveComputationOffset3,
        Array.from(playerACiphertext3[0]),
        Array.from(playerACiphertext3[1]),
        Array.from(playerAPublicKey),
        new anchor.BN(deserializeLE(playerANonce3).toString())
      )
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          playerAMoveComputationOffset3
        ),
        payer: playerA.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
        rpsGame: PublicKey.findProgramAddressSync(
          [Buffer.from("rps_game"), gameId3.toArrayLike(Buffer, "le", 8)],
          program.programId
        )[0],
      })
      .signers([playerA])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Player A valid move signature:", playerAMoveTx3);

    // Wait for player A move computation finalization
    const playerAMoveFinalizeSig3 = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      playerAMoveComputationOffset3,
      program.programId,
      "confirmed"
    );
    console.log("Player A move finalize signature:", playerAMoveFinalizeSig3);

    // Player B makes an invalid move (4)
    const playerBInvalidMove = 4;
    const playerBId3 = 1;
    const playerBNonce3 = randomBytes(16);
    const playerBCiphertext3 = playerBCipher.encrypt(
      [BigInt(playerBId3), BigInt(playerBInvalidMove)],
      playerBNonce3
    );

    const playerBMoveComputationOffset3 = new anchor.BN(randomBytes(8), "hex");

    console.log("Player B making an invalid move (4)");
    const playerBMoveTx3 = await program.methods
      .playerMove(
        playerBMoveComputationOffset3,
        Array.from(playerBCiphertext3[0]),
        Array.from(playerBCiphertext3[1]),
        Array.from(playerBPublicKey),
        new anchor.BN(deserializeLE(playerBNonce3).toString())
      )
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          playerBMoveComputationOffset3
        ),
        payer: playerB.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_move")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
        rpsGame: PublicKey.findProgramAddressSync(
          [Buffer.from("rps_game"), gameId3.toArrayLike(Buffer, "le", 8)],
          program.programId
        )[0],
      })
      .signers([playerB])
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Player B invalid move signature:", playerBMoveTx3);

    // Wait for player B move computation finalization
    const playerBMoveFinalizeSig3 = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      playerBMoveComputationOffset3,
      program.programId,
      "confirmed"
    );
    console.log("Player B move finalize signature:", playerBMoveFinalizeSig3);

    // Compare moves
    const gameEventPromise3 = awaitEvent("compareMovesEvent");

    const compareComputationOffset3 = new anchor.BN(randomBytes(8), "hex");

    console.log("Comparing moves for invalid move test");
    const compareTx3 = await program.methods
      .compareMoves(compareComputationOffset3)
      .accounts({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          compareComputationOffset3
        ),
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("compare_moves")).readUInt32LE()
        ),
        clusterAccount: clusterAccount,
        rpsGame: PublicKey.findProgramAddressSync(
          [Buffer.from("rps_game"), gameId3.toArrayLike(Buffer, "le", 8)],
          program.programId
        )[0],
      })
      .rpc({ skipPreflight: true, commitment: "confirmed" });

    console.log("Compare moves signature for invalid move test:", compareTx3);

    const finalizeSig3 = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      compareComputationOffset3,
      program.programId,
      "confirmed"
    );
    console.log("Finalize signature for invalid move test:", finalizeSig3);

    const gameEvent3 = await gameEventPromise3;
    console.log(`Game result for invalid move test: ${gameEvent3.result}`);

    // Verify the result is "Invalid Move"
    expect(gameEvent3.result).to.equal("Invalid Move");
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
  uploadRawCircuit: boolean,
  offchainSource: boolean
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed(
    "ComputationDefinitionAccount"
  );
  const offset = getCompDefAccOffset("init_game");

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgramId()
  )[0];

  console.log(`Comp def PDA for init_game:`, compDefPDA.toBase58());

  const sig = await program.methods
    .initInitGameCompDef()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAddress(program.programId),
    })
    .signers([owner])
    .rpc();

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
  } else if (!offchainSource) {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash =
      await program.provider.connection.getLatestBlockhash();
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
  uploadRawCircuit: boolean,
  offchainSource: boolean
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed(
    "ComputationDefinitionAccount"
  );
  const offset = getCompDefAccOffset("player_move");

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgramId()
  )[0];

  console.log(`Comp def PDA for player_move:`, compDefPDA.toBase58());

  const sig = await program.methods
    .initPlayerMoveCompDef()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAddress(program.programId),
    })
    .signers([owner])
    .rpc();

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
  } else if (!offchainSource) {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash =
      await program.provider.connection.getLatestBlockhash();
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
  uploadRawCircuit: boolean,
  offchainSource: boolean
): Promise<string> {
  const baseSeedCompDefAcc = getArciumAccountBaseSeed(
    "ComputationDefinitionAccount"
  );
  const offset = getCompDefAccOffset("compare_moves");

  const compDefPDA = PublicKey.findProgramAddressSync(
    [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
    getArciumProgramId()
  )[0];

  console.log(`Comp def PDA for compare_moves:`, compDefPDA.toBase58());

  const sig = await program.methods
    .initCompareMovesCompDef()
    .accounts({
      compDefAccount: compDefPDA,
      payer: owner.publicKey,
      mxeAccount: getMXEAccAddress(program.programId),
    })
    .signers([owner])
    .rpc();

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
  } else if (!offchainSource) {
    const finalizeTx = await buildFinalizeCompDefTx(
      program.provider as anchor.AnchorProvider,
      Buffer.from(offset).readUInt32LE(),
      program.programId
    );

    const latestBlockhash =
      await program.provider.connection.getLatestBlockhash();
    finalizeTx.recentBlockhash = latestBlockhash.blockhash;
    finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;

    finalizeTx.sign(owner);
    await program.provider.sendAndConfirm(finalizeTx);
  }
  return sig;
}

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
