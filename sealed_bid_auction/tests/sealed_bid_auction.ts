import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { SealedBidAuction } from "../target/types/sealed_bid_auction";
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

describe("SealedBidAuction", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .SealedBidAuction as Program<SealedBidAuction>;
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

    console.log("Initializing vickrey auction bid computation definition");
    const initVickreyAuctionBidSig = await initVickreyAuctionBidCompDef(
      program,
      owner,
      false
    );
    console.log(
      "Vickrey auction bid computation definition initialized with signature",
      initVickreyAuctionBidSig
    );

    const privateKey = x25519.utils.randomPrivateKey();
    const publicKey = x25519.getPublicKey(privateKey);
    const mxePublicKey = new Uint8Array([
      34, 56, 246, 3, 165, 122, 74, 68, 14, 81, 107, 73, 129, 145, 196, 4, 98,
      253, 120, 15, 235, 108, 37, 198, 124, 111, 38, 1, 210, 143, 72, 87,
    ]);
    const sharedSecret = x25519.getSharedSecret(privateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

    const bid = randomBytes(32);
    const bidderKp = Keypair.generate();
    const bidderPubkey = bidderKp.publicKey;
    const nonce = randomBytes(16);

    const ciphertext = cipher.encrypt(
      [bid, ...encodeOwner(bidderPubkey)],
      nonce
    );

    const bidEventPromise = awaitEvent("bidEvent");

    const queueSig = await program.methods
      .vickreyBid(
        Array.from(ciphertext[0]),
        Array.from(ciphertext[1]),
        Array.from(ciphertext[2]),
        Array.from(publicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("vickrey_auction_bid")).readUInt32LE()
        ),
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

    const bidEvent = await bidEventPromise;
    console.log("Bid event is ", bidEvent);
  });

  async function initVickreyAuctionBidCompDef(
    program: Program<SealedBidAuction>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("vickrey_auction_bid");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initVickreyAuctionBidCompDef()
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
      "Init vickrey auction bid computation definition transaction",
      sig
    );

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/vickrey_auction_bid.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "vickrey_auction_bid",
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

function encodeOwner(owner: anchor.web3.PublicKey): [bigint, bigint] {
  const ownerBytes = owner.toBytes();
  return [
    bytesToBigIntLE(ownerBytes.slice(0, 16)),
    bytesToBigIntLE(ownerBytes.slice(16, 32)),
  ];
}

function bytesToBigIntLE(bytes: Uint8Array): bigint {
  return bytes.reduce(
    (acc: bigint, byte) => (acc << BigInt(8)) + BigInt(byte),
    BigInt(0)
  );
}
