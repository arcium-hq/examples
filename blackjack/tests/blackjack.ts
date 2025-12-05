import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { Blackjack } from "../target/types/blackjack";
import { randomBytes } from "crypto";
import {
  awaitComputationFinalization,
  getArciumEnv,
  getCompDefAccOffset,
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
  getArciumAccountBaseSeed,
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

// Helper function to calculate Blackjack hand value
function calculateHandValue(cards: number[]): {
  value: number;
  isSoft: boolean;
} {
  let value = 0;
  let aceCount = 0;
  let isSoft = false;

  for (const cardIndex of cards) {
    // Map card index (0-51) to value (Ace=11/1, K/Q/J=10, 2-10=face value)
    const rank = cardIndex % 13; // 0=Ace, 1=2, ..., 9=10, 10=J, 11=Q, 12=K
    if (rank === 0) {
      // Ace
      aceCount++;
      value += 11;
    } else if (rank >= 10) {
      // K, Q, J
      value += 10;
    } else {
      // 2-10
      value += rank + 1;
    }
  }

  // Adjust for Aces if value > 21
  while (value > 21 && aceCount > 0) {
    value -= 10;
    aceCount--;
  }

  // Check if the hand is "soft" (contains an Ace counted as 11)
  isSoft = aceCount > 0 && value <= 21;

  return { value, isSoft };
}

// Updated decompressHand to use hand size
function decompressHand(
  compressedHandValue: bigint,
  handSize: number
): number[] {
  let currentHandValue = compressedHandValue;
  const cards: number[] = [];
  const numCardSlots = 11; // Max possible slots in u128 encoding

  for (let i = 0; i < numCardSlots; i++) {
    const card = currentHandValue % BigInt(64); // Get the last 6 bits
    cards.push(Number(card));
    currentHandValue >>= BigInt(6); // Shift right by 6 bits
  }

  // Return only the actual cards based on handSize, reversing because they were pushed LSB first
  // Filter out potential padding/unused card slots (> 51)
  return cards
    .slice(0, handSize)
    .filter((card) => card <= 51)
    .reverse();
}

describe("Blackjack", () => {
  const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Blackjack as Program<Blackjack>;
  const provider = anchor.getProvider() as anchor.AnchorProvider;

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;
  const awaitEvent = async <E extends keyof Event>(
    eventName: E,
    timeoutMs = 60000
  ): Promise<Event[E]> => {
    let listenerId: number;
    let timeoutId: NodeJS.Timeout;
    const event = await new Promise<Event[E]>((res, rej) => {
      listenerId = program.addEventListener(eventName as any, (event) => {
        if (timeoutId) clearTimeout(timeoutId);
        res(event);
      });
      timeoutId = setTimeout(() => {
        program.removeEventListener(listenerId);
        rej(new Error(`Event ${eventName} timed out after ${timeoutMs}ms`));
      }, timeoutMs);
    });
    await program.removeEventListener(listenerId);
    return event;
  };

  const clusterAccount = getClusterAccount();

  it("Should play a full blackjack game with state awareness", async () => {
    console.log("Owner address:", owner.publicKey.toBase58());

    // --- Initialize Computation Definitions ---
    console.log("Initializing computation definitions...");
    await Promise.all([
      initShuffleAndDealCardsCompDef(program as any, owner, false, false).then(
        (sig) => console.log("Shuffle/Deal CompDef Init Sig:", sig)
      ),
      initPlayerHitCompDef(program as any, owner, false, false).then((sig) =>
        console.log("Player Hit CompDef Init Sig:", sig)
      ),
      initPlayerStandCompDef(program as any, owner, false, false).then((sig) =>
        console.log("Player Stand CompDef Init Sig:", sig)
      ),
      initPlayerDoubleDownCompDef(program as any, owner, false, false).then(
        (sig) => console.log("Player DoubleDown CompDef Init Sig:", sig)
      ),
      initDealerPlayCompDef(program as any, owner, false, false).then((sig) =>
        console.log("Dealer Play CompDef Init Sig:", sig)
      ),
      initResolveGameCompDef(program as any, owner, false, false).then((sig) =>
        console.log("Resolve Game CompDef Init Sig:", sig)
      ),
    ]);
    console.log("All computation definitions initialized.");
    await new Promise((res) => setTimeout(res, 2000));

    // --- Setup Game Cryptography ---
    const privateKey = x25519.utils.randomSecretKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const clientNonce = randomBytes(16);
    const dealerClientNonce = randomBytes(16);

    const gameId = BigInt(Math.floor(Math.random() * 1000000));
    const mxeNonce = randomBytes(16);
    const mxeAgainNonce = randomBytes(16);

    const computationOffsetInit = new anchor.BN(randomBytes(8));

    const gameIdBuffer = Buffer.alloc(8);
    gameIdBuffer.writeBigUInt64LE(gameId);

    const blackjackGamePDA = PublicKey.findProgramAddressSync(
      [Buffer.from("blackjack_game"), gameIdBuffer],
      program.programId
    )[0];

    console.log(`Game ID: ${gameId}, PDA: ${blackjackGamePDA.toBase58()}`);

    // --- Initialize Game ---
    const cardsShuffledAndDealtEventPromise = awaitEvent(
      "cardsShuffledAndDealtEvent"
    );
    console.log("Initializing Blackjack game...");

    const initGameSig = await program.methods
      .initializeBlackjackGame(
        computationOffsetInit,
        new anchor.BN(gameId.toString()),
        new anchor.BN(deserializeLE(mxeNonce).toString()),
        new anchor.BN(deserializeLE(mxeAgainNonce).toString()),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(clientNonce).toString()),
        new anchor.BN(deserializeLE(dealerClientNonce).toString())
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          computationOffsetInit
        ),
        clusterAccount: clusterAccount,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(
            getCompDefAccOffset("shuffle_and_deal_cards")
          ).readUInt32LE()
        ),
        blackjackGame: blackjackGamePDA,
      })
      .signers([owner])
      .rpc({ commitment: "confirmed" });
    console.log("Initialize game TX Signature:", initGameSig);

    console.log("Waiting for shuffle/deal computation finalization...");
    const finalizeInitSig = await awaitComputationFinalization(
      provider,
      computationOffsetInit,
      program.programId,
      "confirmed"
    );
    console.log(
      "Shuffle/deal computation finalized. Signature:",
      finalizeInitSig
    );

    const cardsShuffledAndDealtEvent = await cardsShuffledAndDealtEventPromise;
    console.log("Received CardsShuffledAndDealtEvent.");

    let gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
    expect(gameState.gameState).to.deep.equal({ playerTurn: {} });

    // Decrypt initial hands
    // Convert anchor.BN to Uint8Array (16 bytes for u128) - manual conversion
    let currentClientNonce = Uint8Array.from(
      cardsShuffledAndDealtEvent.clientNonce.toArray("le", 16)
    );

    console.log("Current client nonce:", currentClientNonce);
    let compressedPlayerHand = cipher.decrypt(
      [cardsShuffledAndDealtEvent.playerHand],
      currentClientNonce
    );
    let playerHand = decompressHand(
      compressedPlayerHand[0],
      gameState.playerHandSize
    );
    let { value: playerValue, isSoft: playerIsSoft } =
      calculateHandValue(playerHand);
    console.log(
      `Initial Player Hand: ${playerHand.join(", ")} (Value: ${playerValue}${playerIsSoft ? " Soft" : ""
      })`
    );

    let currentDealerClientNonce = Uint8Array.from(
      cardsShuffledAndDealtEvent.dealerClientNonce.toArray("le", 16)
    );
    console.log("Current dealer client nonce:", currentDealerClientNonce);
    let dealerFaceUpCardEncrypted = cipher.decrypt(
      [cardsShuffledAndDealtEvent.dealerFaceUpCard],
      currentDealerClientNonce
    );
    let dealerFaceUpCard = Number(dealerFaceUpCardEncrypted[0] % BigInt(64));
    console.log(`Dealer Face Up Card Index: ${dealerFaceUpCard}`);

    // --- Player's Turn Loop ---
    let playerBusted = false;
    let playerStood = false;

    while (
      gameState.gameState.hasOwnProperty("playerTurn") &&
      !playerBusted &&
      !playerStood
    ) {
      console.log(
        `\nPlayer's Turn. Hand: ${playerHand.join(
          ", "
        )} (Value: ${playerValue}${playerIsSoft ? " Soft" : ""})`
      );

      // Basic Strategy: Hit on 16 or less, Stand on 17 or more. Hit soft 17.
      let action: "hit" | "stand" = "stand";
      if (playerValue < 17 || (playerValue === 17 && playerIsSoft)) {
        action = "hit";
      }

      if (action === "hit") {
        console.log("Player decides to HIT.");
        const playerHitComputationOffset = new anchor.BN(randomBytes(8));
        const playerHitEventPromise = awaitEvent("playerHitEvent");
        const playerBustEventPromise = awaitEvent("playerBustEvent");

        const playerHitSig = await program.methods
          .playerHit(
            playerHitComputationOffset,
            new anchor.BN(gameId.toString())
          )
          .accountsPartial({
            computationAccount: getComputationAccAddress(
              getArciumEnv().arciumClusterOffset,
              playerHitComputationOffset
            ),
            clusterAccount: clusterAccount,
            mxeAccount: getMXEAccAddress(program.programId),
            mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
            executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
            compDefAccount: getCompDefAccAddress(
              program.programId,
              Buffer.from(getCompDefAccOffset("player_hit")).readUInt32LE()
            ),
            blackjackGame: blackjackGamePDA,
            payer: owner.publicKey,
          })
          .signers([owner])
          .rpc({ commitment: "confirmed" });
        console.log("Player Hit TX Signature:", playerHitSig);

        console.log("Waiting for player hit computation finalization...");
        const finalizeHitSig = await awaitComputationFinalization(
          provider,
          playerHitComputationOffset,
          program.programId,
          "confirmed"
        );
        console.log(
          "Player Hit computation finalized. Signature:",
          finalizeHitSig
        );

        try {
          const playerHitEvent = await Promise.race([
            playerHitEventPromise,
            playerBustEventPromise,
          ]);

          gameState = await program.account.blackjackGame.fetch(
            blackjackGamePDA
          );

          if ("playerHand" in playerHitEvent) {
            console.log("Received PlayerHitEvent.");
            currentClientNonce = Uint8Array.from(
              playerHitEvent.clientNonce.toArray("le", 16)
            );
            compressedPlayerHand = cipher.decrypt(
              [playerHitEvent.playerHand],
              currentClientNonce
            );
            playerHand = decompressHand(
              compressedPlayerHand[0],
              gameState.playerHandSize
            );
            ({ value: playerValue, isSoft: playerIsSoft } =
              calculateHandValue(playerHand));
            console.log(
              `New Player Hand: ${playerHand.join(
                ", "
              )} (Value: ${playerValue}${playerIsSoft ? " Soft" : ""})`
            );

            if (playerValue > 21) {
              console.error(
                "ERROR: Bust detected after PlayerHitEvent, expected PlayerBustEvent!"
              );
              playerBusted = true;
            }
          } else {
            console.log("Received PlayerBustEvent.");
            playerBusted = true;
            expect(gameState.gameState).to.deep.equal({ dealerTurn: {} });
            console.log("Player BUSTED!");
          }
        } catch (e) {
          console.error("Error waiting for player hit/bust event:", e);
          throw e;
        }
      } else {
        console.log("Player decides to STAND.");
        const playerStandComputationOffset = new anchor.BN(randomBytes(8));
        const playerStandEventPromise = awaitEvent("playerStandEvent");

        const playerStandSig = await program.methods
          .playerStand(
            playerStandComputationOffset,
            new anchor.BN(gameId.toString())
          )
          .accountsPartial({
            computationAccount: getComputationAccAddress(
              getArciumEnv().arciumClusterOffset,
              playerStandComputationOffset
            ),
            clusterAccount: clusterAccount,
            mxeAccount: getMXEAccAddress(program.programId),
            mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
            executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
            compDefAccount: getCompDefAccAddress(
              program.programId,
              Buffer.from(getCompDefAccOffset("player_stand")).readUInt32LE()
            ),
            blackjackGame: blackjackGamePDA,
            payer: owner.publicKey,
          })
          .signers([owner])
          .rpc({ commitment: "confirmed" });
        console.log("Player Stand TX Signature:", playerStandSig);

        console.log("Waiting for player stand computation finalization...");
        const finalizeStandSig = await awaitComputationFinalization(
          provider,
          playerStandComputationOffset,
          program.programId,
          "confirmed"
        );
        console.log(
          "Player Stand computation finalized. Signature:",
          finalizeStandSig
        );

        const playerStandEvent = await playerStandEventPromise;
        console.log(
          `Received PlayerStandEvent. Is Bust reported? ${playerStandEvent.isBust}`
        );
        expect(playerStandEvent.isBust).to.be.false;

        playerStood = true;
        gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
        expect(gameState.gameState).to.deep.equal({ dealerTurn: {} });
        console.log("Player stands. Proceeding to Dealer's Turn.");
      }

      if (!playerBusted && !playerStood) {
        await new Promise((res) => setTimeout(res, 1000));
        gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
      }
    }

    // --- Dealer's Turn ---
    gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
    if (gameState.gameState.hasOwnProperty("dealerTurn")) {
      console.log("Dealer's Turn...");
      const dealerPlayComputationOffset = new anchor.BN(randomBytes(8));
      const dealerPlayNonce = randomBytes(16);
      const dealerPlayEventPromise = awaitEvent("dealerPlayEvent");

      const dealerPlaySig = await program.methods
        .dealerPlay(
          dealerPlayComputationOffset,
          new anchor.BN(gameId.toString()),
          new anchor.BN(deserializeLE(dealerPlayNonce).toString())
        )
        .accountsPartial({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            dealerPlayComputationOffset
          ),
          clusterAccount: clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("dealer_play")).readUInt32LE()
          ),
          blackjackGame: blackjackGamePDA,
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
      console.log("Dealer Play TX Signature:", dealerPlaySig);

      console.log("Waiting for dealer play computation finalization...");
      const finalizeDealerPlaySig = await awaitComputationFinalization(
        provider,
        dealerPlayComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log(
        "Dealer Play computation finalized. Signature:",
        finalizeDealerPlaySig
      );

      const dealerPlayEvent = await dealerPlayEventPromise;
      console.log("Received DealerPlayEvent.");

      const finalDealerNonce = Uint8Array.from(
        dealerPlayEvent.clientNonce.toArray("le", 16)
      );
      const decryptedDealerHand = cipher.decrypt(
        [dealerPlayEvent.dealerHand],
        finalDealerNonce
      );
      const dealerHand = decompressHand(
        decryptedDealerHand[0],
        dealerPlayEvent.dealerHandSize
      );
      const { value: dealerValue } = calculateHandValue(dealerHand);
      console.log(
        `Final Dealer Hand: ${dealerHand.join(", ")} (Value: ${dealerValue})`
      );
      gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
      expect(gameState.gameState).to.deep.equal({ resolving: {} });
    } else if (playerBusted) {
      console.log("Player busted, skipping Dealer's Turn.");
      console.log(
        "Manually considering state as Resolving for test flow after player bust."
      );
    }

    gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
    if (
      gameState.gameState.hasOwnProperty("resolving") ||
      (playerBusted && gameState.gameState.hasOwnProperty("dealerTurn"))
    ) {
      console.log("Resolving Game...");
      const resolveComputationOffset = new anchor.BN(randomBytes(8));
      const resultEventPromise = awaitEvent("resultEvent");

      const resolveSig = await program.methods
        .resolveGame(resolveComputationOffset, new anchor.BN(gameId.toString()))
        .accountsPartial({
          computationAccount: getComputationAccAddress(
            getArciumEnv().arciumClusterOffset,
            resolveComputationOffset
          ),
          clusterAccount: clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("resolve_game")).readUInt32LE()
          ),
          blackjackGame: blackjackGamePDA,
          payer: owner.publicKey,
        })
        .signers([owner])
        .rpc({ commitment: "confirmed" });
      console.log("Resolve Game TX Signature:", resolveSig);

      console.log("Waiting for resolve game computation finalization...");
      const finalizeResolveSig = await awaitComputationFinalization(
        provider,
        resolveComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log(
        "Resolve Game computation finalized. Signature:",
        finalizeResolveSig
      );

      const resultEvent = await resultEventPromise;
      console.log("Received ResultEvent.");
      console.log(`GAME OVER! Winner: ${resultEvent.winner}`);
      expect(["Player", "Dealer", "Tie"]).to.include(resultEvent.winner);

      gameState = await program.account.blackjackGame.fetch(blackjackGamePDA);
      expect(gameState.gameState).to.deep.equal({ resolved: {} });
    } else {
      console.warn(
        `Skipping Resolve Game step. Current state: ${Object.keys(gameState.gameState)[0]
        }`
      );
    }
  });

  async function initShuffleAndDealCardsCompDef(
    program: Program<Blackjack>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("shuffle_and_deal_cards");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    console.log("Shuffle/Deal CompDef PDA:", compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log("Shuffle/Deal CompDef already initialized.");
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const sig = await program.methods
      .initShuffleAndDealCardsCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc();

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/shuffle_and_deal_cards.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "shuffle_and_deal_cards",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      console.log("Finalizing Shuffle/Deal CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log("Shuffle/Deal CompDef finalized.");
    }
    return sig;
  }

  async function initPlayerHitCompDef(
    program: Program<Blackjack>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("player_hit");
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];
    console.log("Player Hit CompDef PDA:", compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log("Player Hit CompDef already initialized.");
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const sig = await program.methods
      .initPlayerHitCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc();

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/player_hit.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "player_hit",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      console.log("Finalizing Player Hit CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log("Player Hit CompDef finalized.");
    }
    return sig;
  }

  async function initPlayerStandCompDef(
    program: Program<Blackjack>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("player_stand");
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];
    console.log("Player Stand CompDef PDA:", compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log("Player Stand CompDef already initialized.");
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const sig = await program.methods
      .initPlayerStandCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc({ commitment: "confirmed" });

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/player_stand.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "player_stand",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      console.log("Finalizing Player Stand CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log("Player Stand CompDef finalized.");
    }
    return sig;
  }

  async function initPlayerDoubleDownCompDef(
    program: Program<Blackjack>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("player_double_down");
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];
    console.log("Player DoubleDown CompDef PDA:", compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log("Player DoubleDown CompDef already initialized.");
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const sig = await program.methods
      .initPlayerDoubleDownCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc();

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/player_double_down.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "player_double_down",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      console.log("Finalizing Player DoubleDown CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log("Player DoubleDown CompDef finalized.");
    }
    return sig;
  }

  async function initDealerPlayCompDef(
    program: Program<Blackjack>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("dealer_play");
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];
    console.log("Dealer Play CompDef PDA:", compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log("Dealer Play CompDef already initialized.");
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const sig = await program.methods
      .initDealerPlayCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc();

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/dealer_play.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "dealer_play",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      console.log("Finalizing Dealer Play CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log("Dealer Play CompDef finalized.");
    }
    return sig;
  }

  async function initResolveGameCompDef(
    program: Program<Blackjack>,
    owner: Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("resolve_game");
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];
    console.log("Resolve Game CompDef PDA:", compDefPDA.toBase58());

    try {
      await program.account.computationDefinitionAccount.fetch(compDefPDA);
      console.log("Resolve Game CompDef already initialized.");
      return "Already Initialized";
    } catch (e) {
      // Not initialized, proceed
    }

    const sig = await program.methods
      .initResolveGameCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
      })
      .rpc();

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/resolve_game.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "resolve_game",
        program.programId,
        rawCircuit,
        true
      );
    } else if (!offchainSource) {
      console.log("Finalizing Resolve Game CompDef...");
      const finalizeTx = await buildFinalizeCompDefTx(
        provider,
        Buffer.from(offset).readUInt32LE(),
        program.programId
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx, [owner], {
        commitment: "confirmed",
      });
      console.log("Resolve Game CompDef finalized.");
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
