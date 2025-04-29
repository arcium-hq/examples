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

describe("Blackjack", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Blackjack as Program<Blackjack>;
  const provider = anchor.getProvider();

  type Event = anchor.IdlEvents<(typeof program)["idl"]>;
  const awaitEvent = async (eventName: string): Promise<any> => {
    let listenerId: number;
    const event = await new Promise<any>((res) => {
      listenerId = program.addEventListener(eventName as any, (evt: any) => {
        res(evt);
      });
    });
    await program.removeEventListener(listenerId);

    return event;
  };

  const arciumEnv = getArciumEnv();

  it("Should play a blackjack game", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    console.log("Initializing blackjack game");
    const initShuffleAndDealCardsCompDefSig =
      await initShuffleAndDealCardsCompDef(program, owner, false);
    console.log(
      "Shuffle and deal cards computation definition initialized with signature",
      initShuffleAndDealCardsCompDefSig
    );
    const initPlayerHitCompDefSig = await initPlayerHitCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Player hit computation definition initialized with signature",
      initPlayerHitCompDefSig
    );
    await new Promise((res) => setTimeout(res, 1000));
    const initPlayerStandCompDefSig = await initPlayerStandCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Player stand computation definition initialized with signature",
      initPlayerStandCompDefSig
    );
    await new Promise((res) => setTimeout(res, 1000));
    const initPlayerDoubleDownCompDefSig = await initPlayerDoubleDownCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Player double down computation definition initialized with signature",
      initPlayerDoubleDownCompDefSig
    );
    await new Promise((res) => setTimeout(res, 1000));
    const initDealerPlayCompDefSig = await initDealerPlayCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Dealer play computation definition initialized with signature",
      initDealerPlayCompDefSig
    );
    await new Promise((res) => setTimeout(res, 1000));
    const initResolveGameCompDefSig = await initResolveGameCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Resolve game computation definition initialized with signature",
      initResolveGameCompDefSig
    );

    const privateKey = x25519.utils.randomPrivateKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const mxePublicKey = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);
    const clientNonce = randomBytes(16);

    const gameId = BigInt(1);
    const mxeNonce = randomBytes(16);
    const mxeAgainNonce = randomBytes(16);

    const cardsShuffledAndDealtEventPromise = awaitEvent(
      "cardsShuffledAndDealtEvent"
    );
    await new Promise((res) => setTimeout(res, 1000));

    // Initialize the blackjack game
    const initGameSig = await program.methods
      .initializeBlackjackGame(
        new anchor.BN(gameId.toString()),
        new anchor.BN(deserializeLE(mxeNonce).toString()),
        new anchor.BN(deserializeLE(mxeAgainNonce).toString()),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(clientNonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(
            getCompDefAccOffset("shuffle_and_deal_cards")
          ).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Initialize game sig is ", initGameSig);

    const finalizeInitSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      initGameSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize init sig is ", finalizeInitSig);

    // Wait for cards to be shuffled
    const cardsShuffledAndDealtEvent = await cardsShuffledAndDealtEventPromise;
    console.log("Cards shuffled and dealt");

    const cards = cipher.decrypt(
      [
        ...cardsShuffledAndDealtEvent.userHand,
        cardsShuffledAndDealtEvent.dealerFaceUpCard,
      ],
      new Uint8Array(cardsShuffledAndDealtEvent.clientNonce)
    );

    console.log("User hand is ", cards[0], cards[1]);
    console.log("Dealer face up card is ", cards[2]);

    // Full gameplay: player hit, stand, dealer play, and resolve game
    const playerHitEventPromise = awaitEvent("playerHitEvent");
    const playerHitSig = await program.methods
      .playerHit(new anchor.BN(gameId.toString()))
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_hit")).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Player hit sig:", playerHitSig);
    const finalizeHitSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      playerHitSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize hit sig:", finalizeHitSig);
    const playerHitEvent = await playerHitEventPromise;
    const decryptedHitCard = cipher.decrypt(
      [playerHitEvent.card],
      new Uint8Array(playerHitEvent.clientNonce)
    )[0];
    console.log("Decrypted hit card:", decryptedHitCard);

    const playerStandEventPromise = awaitEvent("playerStandEvent");
    const playerStandSig = await program.methods
      .playerStand(new anchor.BN(gameId.toString()))
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("player_stand")).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Player stand sig:", playerStandSig);
    const finalizeStandSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      playerStandSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize stand sig:", finalizeStandSig);
    const playerStandEvent = await playerStandEventPromise;
    console.log("Player stand event is bust?", playerStandEvent.isBust);
    expect(typeof playerStandEvent.isBust).to.equal("boolean");

    const dealerPlayEventPromise = awaitEvent("dealerPlayEvent");
    const dealerPlaySig = await program.methods
      .dealerPlay(new anchor.BN(gameId.toString()))
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("dealer_play")).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Dealer play sig:", dealerPlaySig);
    const finalizeDealerPlaySig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      dealerPlaySig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize dealer play sig:", finalizeDealerPlaySig);
    const dealerPlayEvent = await dealerPlayEventPromise;
    console.log(
      "Dealer hand:",
      dealerPlayEvent.dealerHand,
      "size:",
      dealerPlayEvent.dealerHandSize
    );
    const decryptedDealerHand = cipher.decrypt(
      dealerPlayEvent.dealerHand.slice(0, dealerPlayEvent.dealerHandSize),
      new Uint8Array(dealerPlayEvent.clientNonce)
    );
    console.log("Decrypted dealer hand:", decryptedDealerHand);

    const resultEventPromise = awaitEvent("resultEvent");
    const resolveSig = await (program as any).methods
      .resolveGame(new anchor.BN(gameId.toString()))
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("resolve_game")).readUInt32LE()
        ),
      })
      .rpc({ commitment: "confirmed" });
    console.log("Resolve game sig:", resolveSig);
    const finalizeResolveSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      resolveSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize resolve sig:", finalizeResolveSig);
    const resultEvent = await resultEventPromise;
    console.log("Result winner:", resultEvent.winner);
    expect(["Player", "Dealer", "Tie"]).to.include(resultEvent.winner);
  });

  async function initShuffleAndDealCardsCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("shuffle_and_deal_cards");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initShuffleAndDealCardsCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log(
      "Init shuffle and deal cards computation definition transaction",
      sig
    );

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/shuffle_and_deal_cards.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "shuffle_and_deal_cards",
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

  async function initPlayerHitCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("player_hit");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initPlayerHitCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init player hit computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/player_hit.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "player_hit",
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

  async function initPlayerStandCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("player_stand");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initPlayerStandCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init player stand computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/player_stand.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "player_stand",
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

  async function initPlayerDoubleDownCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("player_double_down");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initPlayerDoubleDownCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log(
      "Init player double down computation definition transaction",
      sig
    );

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/player_double_down.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "player_double_down",
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

  async function initDealerPlayCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("dealer_play");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initDealerPlayCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init dealer play computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/dealer_play.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "dealer_play",
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

  async function initResolveGameCompDef(
    program: Program<Blackjack>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("resolve_game");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initResolveGameCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAcc(program.programId),
      })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    console.log("Init resolve game computation definition transaction", sig);

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/resolve_game.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "resolve_game",
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
