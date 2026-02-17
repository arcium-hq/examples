import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { SealedBidAuction } from "../target/types/sealed_bid_auction";
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
  getMXEPublicKey,
  getMXEAccAddress,
  getMempoolAccAddress,
  getCompDefAccAddress,
  getExecutingPoolAccAddress,
  getComputationAccAddress,
  getClusterAccAddress,
  getLookupTableAddress,
  getArciumProgram,
  x25519,
} from "@arcium-hq/client";
import * as fs from "fs";
import * as os from "os";
import { expect } from "chai";

/**
 * Splits a 32-byte public key into two u128 values (lo and hi parts).
 * This is required because Arcis encrypts each primitive separately.
 */
function splitPubkeyToU128s(pubkey: Uint8Array): { lo: bigint; hi: bigint } {
  // Lower 128 bits (first 16 bytes)
  const loBytes = pubkey.slice(0, 16);
  // Upper 128 bits (last 16 bytes)
  const hiBytes = pubkey.slice(16, 32);

  // Convert to bigint (little-endian)
  const lo = deserializeLE(loBytes);
  const hi = deserializeLE(hiBytes);

  return { lo, hi };
}

describe("SealedBidAuction", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .SealedBidAuction as Program<SealedBidAuction>;
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

  let owner: anchor.web3.Keypair;
  let mxePublicKey: Uint8Array;
  let compDefsInitialized = false;

  before(async () => {
    owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    // Get MXE public key for encryption
    mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );
    console.log("MXE x25519 pubkey is", mxePublicKey);

    // Initialize all computation definitions sequentially (like voting/ed25519)
    if (!compDefsInitialized) {
      console.log("\n=== Initializing Computation Definitions ===\n");

      console.log("1. Initializing init_auction_state comp def...");
      await initCompDef(program, owner, "init_auction_state");
      console.log("   Done.");

      console.log("2. Initializing place_bid comp def...");
      await initCompDef(program, owner, "place_bid");
      console.log("   Done.");

      console.log("3. Initializing determine_winner_first_price comp def...");
      await initCompDef(program, owner, "determine_winner_first_price");
      console.log("   Done.");

      console.log("4. Initializing determine_winner_vickrey comp def...");
      await initCompDef(program, owner, "determine_winner_vickrey");
      console.log("   Done.\n");

      compDefsInitialized = true;
    }
  });

  describe("First-Price Auction", () => {
    it("creates an auction, accepts bids, and determines winner (pays their bid)", async () => {
      console.log("\n=== First-Price Auction Test ===\n");

      // Bidder setup - using owner as bidder for simplicity
      const bidder = owner;
      const bidderPubkey = bidder.publicKey.toBytes();
      const { lo: bidderLo, hi: bidderHi } = splitPubkeyToU128s(bidderPubkey);

      // Create encryption keys for bidder
      const privateKey = x25519.utils.randomSecretKey();
      const publicKey = x25519.getPublicKey(privateKey);
      const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
      const cipher = new RescueCipher(sharedSecret);

      // Step 1: Create First-Price Auction
      console.log("Step 1: Creating first-price auction...");
      const auctionCreatedPromise = awaitEvent("auctionCreatedEvent");
      const createComputationOffset = new anchor.BN(randomBytes(8), "hex");

      const [auctionPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("auction"), owner.publicKey.toBuffer()],
        program.programId
      );

      const createSig = await program.methods
        .createAuction(
          createComputationOffset,
          { firstPrice: {} }, // AuctionType::FirstPrice
          new anchor.BN(100), // min_bid: 100 lamports
          new anchor.BN(Date.now() / 1000 + 3600) // end_time: 1 hour from now
        )
        .accountsPartial({
          authority: owner.publicKey,
          auction: auctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            createComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(
              getCompDefAccOffset("init_auction_state")
            ).readUInt32LE()
          ),
        })
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Create auction tx:", createSig);

      // Wait for MPC computation to finalize
      const createFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        createComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("   Finalize tx:", createFinalizeSig);

      const auctionCreatedEvent = await auctionCreatedPromise;
      console.log(
        "   Auction created:",
        auctionCreatedEvent.auction.toBase58()
      );
      expect(auctionCreatedEvent.minBid.toNumber()).to.equal(100);

      // Step 2: Place a bid
      console.log("\nStep 2: Placing bid of 500 lamports...");
      const bidPlacedPromise = awaitEvent("bidPlacedEvent");
      const bidComputationOffset = new anchor.BN(randomBytes(8), "hex");

      const bidAmount = BigInt(500);
      const nonce = randomBytes(16);

      // Encrypt bid data: [bidder_lo, bidder_hi, amount]
      const bidPlaintext = [bidderLo, bidderHi, bidAmount];
      const bidCiphertext = cipher.encrypt(bidPlaintext, nonce);

      const placeBidSig = await program.methods
        .placeBid(
          bidComputationOffset,
          Array.from(bidCiphertext[0]), // encrypted_bidder_lo
          Array.from(bidCiphertext[1]), // encrypted_bidder_hi
          Array.from(bidCiphertext[2]), // encrypted_amount
          Array.from(publicKey),
          new anchor.BN(deserializeLE(nonce).toString())
        )
        .accountsPartial({
          bidder: bidder.publicKey,
          auction: auctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            bidComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("place_bid")).readUInt32LE()
          ),
        })
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Place bid tx:", placeBidSig);

      const bidFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        bidComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("   Finalize tx:", bidFinalizeSig);

      const bidPlacedEvent = await bidPlacedPromise;
      console.log("   Bid placed, count:", bidPlacedEvent.bidCount);
      expect(bidPlacedEvent.bidCount).to.equal(1);

      // Step 3: Close auction
      console.log("\nStep 3: Closing auction...");
      const auctionClosedPromise = awaitEvent("auctionClosedEvent");

      const closeSig = await program.methods
        .closeAuction()
        .accountsPartial({
          authority: owner.publicKey,
          auction: auctionPDA,
        })
        .rpc({ preflightCommitment: "confirmed", commitment: "confirmed" });

      console.log("   Close auction tx:", closeSig);

      const auctionClosedEvent = await auctionClosedPromise;
      console.log("   Auction closed, bid count:", auctionClosedEvent.bidCount);

      // Step 4: Determine winner (first-price)
      console.log("\nStep 4: Determining winner (first-price)...");
      const auctionResolvedPromise = awaitEvent("auctionResolvedEvent");
      const resolveComputationOffset = new anchor.BN(randomBytes(8), "hex");

      const resolveSig = await program.methods
        .determineWinnerFirstPrice(resolveComputationOffset)
        .accountsPartial({
          authority: owner.publicKey,
          auction: auctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            resolveComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(
              getCompDefAccOffset("determine_winner_first_price")
            ).readUInt32LE()
          ),
        })
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Determine winner tx:", resolveSig);

      const resolveFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        resolveComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("   Finalize tx:", resolveFinalizeSig);

      const auctionResolvedEvent = await auctionResolvedPromise;
      console.log("\n=== First-Price Auction Results ===");
      console.log(
        "   Winner pubkey (bytes):",
        Buffer.from(auctionResolvedEvent.winner).toString("hex")
      );
      console.log(
        "   Payment amount:",
        auctionResolvedEvent.paymentAmount.toNumber(),
        "lamports"
      );

      // Verify: In first-price, winner pays their bid (500)
      expect(auctionResolvedEvent.paymentAmount.toNumber()).to.equal(500);

      // Verify winner matches bidder
      const expectedWinner = Buffer.from(bidderPubkey).toString("hex");
      const actualWinner = Buffer.from(auctionResolvedEvent.winner).toString(
        "hex"
      );
      expect(actualWinner).to.equal(expectedWinner);

      console.log("\n   First-price auction test PASSED!");
    });
  });

  describe("Vickrey (Second-Price) Auction", () => {
    it("creates an auction with multiple bids, winner pays second-highest", async () => {
      console.log("\n=== Vickrey Auction Test ===\n");

      // Use a different seed for this auction to avoid PDA collision
      // We'll use a separate keypair as authority
      const vickreyAuthority = anchor.web3.Keypair.generate();

      // Fund the new authority
      const fundSig = await provider.connection.requestAirdrop(
        vickreyAuthority.publicKey,
        2 * anchor.web3.LAMPORTS_PER_SOL
      );
      await provider.connection.confirmTransaction(fundSig);

      // Bidder setup
      const bidder1 = owner; // Bid 1000
      const bidder1Pubkey = bidder1.publicKey.toBytes();
      const { lo: bidder1Lo, hi: bidder1Hi } =
        splitPubkeyToU128s(bidder1Pubkey);

      // Create encryption keys for bidder1
      const privateKey1 = x25519.utils.randomSecretKey();
      const publicKey1 = x25519.getPublicKey(privateKey1);
      const sharedSecret1 = x25519.getSharedSecret(privateKey1, mxePublicKey);
      const cipher1 = new RescueCipher(sharedSecret1);

      // Step 1: Create Vickrey Auction
      console.log("Step 1: Creating Vickrey auction...");
      const auctionCreatedPromise = awaitEvent("auctionCreatedEvent");
      const createComputationOffset = new anchor.BN(randomBytes(8), "hex");

      const [vickreyAuctionPDA] = PublicKey.findProgramAddressSync(
        [Buffer.from("auction"), vickreyAuthority.publicKey.toBuffer()],
        program.programId
      );

      const createSig = await program.methods
        .createAuction(
          createComputationOffset,
          { vickrey: {} }, // AuctionType::Vickrey
          new anchor.BN(50), // min_bid: 50 lamports
          new anchor.BN(Date.now() / 1000 + 3600)
        )
        .accountsPartial({
          authority: vickreyAuthority.publicKey,
          auction: vickreyAuctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            createComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(
              getCompDefAccOffset("init_auction_state")
            ).readUInt32LE()
          ),
        })
        .signers([vickreyAuthority])
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Create auction tx:", createSig);

      const createFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        createComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("   Finalize tx:", createFinalizeSig);

      const auctionCreatedEvent = await auctionCreatedPromise;
      console.log(
        "   Vickrey auction created:",
        auctionCreatedEvent.auction.toBase58()
      );

      // Step 2: Place first bid (1000 lamports)
      console.log("\nStep 2: Placing first bid of 1000 lamports...");
      const bidPlaced1Promise = awaitEvent("bidPlacedEvent");
      const bid1ComputationOffset = new anchor.BN(randomBytes(8), "hex");

      const bid1Amount = BigInt(1000);
      const nonce1 = randomBytes(16);
      const bid1Plaintext = [bidder1Lo, bidder1Hi, bid1Amount];
      const bid1Ciphertext = cipher1.encrypt(bid1Plaintext, nonce1);

      const placeBid1Sig = await program.methods
        .placeBid(
          bid1ComputationOffset,
          Array.from(bid1Ciphertext[0]),
          Array.from(bid1Ciphertext[1]),
          Array.from(bid1Ciphertext[2]),
          Array.from(publicKey1),
          new anchor.BN(deserializeLE(nonce1).toString())
        )
        .accountsPartial({
          bidder: bidder1.publicKey,
          auction: vickreyAuctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            bid1ComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("place_bid")).readUInt32LE()
          ),
        })
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Place bid tx:", placeBid1Sig);

      await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        bid1ComputationOffset,
        program.programId,
        "confirmed"
      );

      const bidPlaced1Event = await bidPlaced1Promise;
      console.log("   First bid placed, count:", bidPlaced1Event.bidCount);

      // Step 3: Place second bid (700 lamports) - this becomes second-highest
      console.log("\nStep 3: Placing second bid of 700 lamports...");
      const bidPlaced2Promise = awaitEvent("bidPlacedEvent");
      const bid2ComputationOffset = new anchor.BN(randomBytes(8), "hex");

      // Use same bidder but different bid amount
      const bid2Amount = BigInt(700);
      const nonce2 = randomBytes(16);
      const privateKey2 = x25519.utils.randomSecretKey();
      const publicKey2 = x25519.getPublicKey(privateKey2);
      const sharedSecret2 = x25519.getSharedSecret(privateKey2, mxePublicKey);
      const cipher2 = new RescueCipher(sharedSecret2);

      const bid2Plaintext = [bidder1Lo, bidder1Hi, bid2Amount];
      const bid2Ciphertext = cipher2.encrypt(bid2Plaintext, nonce2);

      const placeBid2Sig = await program.methods
        .placeBid(
          bid2ComputationOffset,
          Array.from(bid2Ciphertext[0]),
          Array.from(bid2Ciphertext[1]),
          Array.from(bid2Ciphertext[2]),
          Array.from(publicKey2),
          new anchor.BN(deserializeLE(nonce2).toString())
        )
        .accountsPartial({
          bidder: bidder1.publicKey,
          auction: vickreyAuctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            bid2ComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(getCompDefAccOffset("place_bid")).readUInt32LE()
          ),
        })
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Place bid tx:", placeBid2Sig);

      await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        bid2ComputationOffset,
        program.programId,
        "confirmed"
      );

      const bidPlaced2Event = await bidPlaced2Promise;
      console.log("   Second bid placed, count:", bidPlaced2Event.bidCount);
      expect(bidPlaced2Event.bidCount).to.equal(2);

      // Step 4: Close auction
      console.log("\nStep 4: Closing Vickrey auction...");
      const auctionClosedPromise = awaitEvent("auctionClosedEvent");

      const closeSig = await program.methods
        .closeAuction()
        .accountsPartial({
          authority: vickreyAuthority.publicKey,
          auction: vickreyAuctionPDA,
        })
        .signers([vickreyAuthority])
        .rpc({ preflightCommitment: "confirmed", commitment: "confirmed" });

      console.log("   Close auction tx:", closeSig);

      const auctionClosedEvent = await auctionClosedPromise;
      console.log("   Auction closed, bid count:", auctionClosedEvent.bidCount);

      // Step 5: Determine winner (Vickrey)
      console.log("\nStep 5: Determining winner (Vickrey)...");
      const auctionResolvedPromise = awaitEvent("auctionResolvedEvent");
      const resolveComputationOffset = new anchor.BN(randomBytes(8), "hex");

      const resolveSig = await program.methods
        .determineWinnerVickrey(resolveComputationOffset)
        .accountsPartial({
          authority: vickreyAuthority.publicKey,
          auction: vickreyAuctionPDA,
          computationAccount: getComputationAccAddress(
            arciumEnv.arciumClusterOffset,
            resolveComputationOffset
          ),
          clusterAccount,
          mxeAccount: getMXEAccAddress(program.programId),
          mempoolAccount: getMempoolAccAddress(arciumEnv.arciumClusterOffset),
          executingPool: getExecutingPoolAccAddress(
            arciumEnv.arciumClusterOffset
          ),
          compDefAccount: getCompDefAccAddress(
            program.programId,
            Buffer.from(
              getCompDefAccOffset("determine_winner_vickrey")
            ).readUInt32LE()
          ),
        })
        .signers([vickreyAuthority])
        .rpc({
          skipPreflight: true,
          preflightCommitment: "confirmed",
          commitment: "confirmed",
        });

      console.log("   Determine winner tx:", resolveSig);

      const resolveFinalizeSig = await awaitComputationFinalization(
        provider as anchor.AnchorProvider,
        resolveComputationOffset,
        program.programId,
        "confirmed"
      );
      console.log("   Finalize tx:", resolveFinalizeSig);

      const auctionResolvedEvent = await auctionResolvedPromise;
      console.log("\n=== Vickrey Auction Results ===");
      console.log(
        "   Winner pubkey (bytes):",
        Buffer.from(auctionResolvedEvent.winner).toString("hex")
      );
      console.log(
        "   Payment amount:",
        auctionResolvedEvent.paymentAmount.toNumber(),
        "lamports"
      );

      // Verify: In Vickrey, winner pays second-highest bid (700)
      // Winner bid 1000, but pays second-highest (700)
      expect(auctionResolvedEvent.paymentAmount.toNumber()).to.equal(700);

      // Verify winner matches highest bidder
      const expectedWinner = Buffer.from(bidder1Pubkey).toString("hex");
      const actualWinner = Buffer.from(auctionResolvedEvent.winner).toString(
        "hex"
      );
      expect(actualWinner).to.equal(expectedWinner);

      console.log(
        "\n   Vickrey auction test PASSED! Winner paid second-highest bid."
      );
    });
  });

  async function initCompDef(
    program: Program<SealedBidAuction>,
    owner: anchor.web3.Keypair,
    circuitName: string
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset(circuitName);

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    // Fetch MXE account for LUT address
    const arciumProgram = getArciumProgram(provider as anchor.AnchorProvider);
    const mxeAccount = getMXEAccAddress(program.programId);
    const mxeAcc = await arciumProgram.account.mxeAccount.fetch(mxeAccount);
    const lutAddress = getLookupTableAddress(
      program.programId,
      mxeAcc.lutOffsetSlot
    );

    // Map circuit name to the correct init method
    // Using preflightCommitment to get fresh blockhash for each transaction
    let sig: string;
    switch (circuitName) {
      case "init_auction_state":
        sig = await program.methods
          .initAuctionStateCompDef()
          .accounts({
            compDefAccount: compDefPDA,
            payer: owner.publicKey,
            mxeAccount,
            addressLookupTable: lutAddress,
          })
          .signers([owner])
          .rpc({ preflightCommitment: "confirmed", commitment: "confirmed" });
        break;
      case "place_bid":
        sig = await program.methods
          .initPlaceBidCompDef()
          .accounts({
            compDefAccount: compDefPDA,
            payer: owner.publicKey,
            mxeAccount,
            addressLookupTable: lutAddress,
          })
          .signers([owner])
          .rpc({ preflightCommitment: "confirmed", commitment: "confirmed" });
        break;
      case "determine_winner_first_price":
        sig = await program.methods
          .initDetermineWinnerFirstPriceCompDef()
          .accounts({
            compDefAccount: compDefPDA,
            payer: owner.publicKey,
            mxeAccount,
            addressLookupTable: lutAddress,
          })
          .signers([owner])
          .rpc({ preflightCommitment: "confirmed", commitment: "confirmed" });
        break;
      case "determine_winner_vickrey":
        sig = await program.methods
          .initDetermineWinnerVickreyCompDef()
          .accounts({
            compDefAccount: compDefPDA,
            payer: owner.publicKey,
            mxeAccount,
            addressLookupTable: lutAddress,
          })
          .signers([owner])
          .rpc({ preflightCommitment: "confirmed", commitment: "confirmed" });
        break;
      default:
        throw new Error(`Unknown circuit: ${circuitName}`);
    }

    const rawCircuit = fs.readFileSync(`build/${circuitName}.arcis`);
    await uploadCircuit(
      provider as anchor.AnchorProvider,
      circuitName,
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
