import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { DarkPool } from "../target/types/dark_pool";
import { AddOrder } from "../confidential-ixs/build/add_order";
import {
  RawConfidentialInstructionInputs,
  getClusterDAInfo,
  getArciumEnv,
  buildOffchainRefRequest,
  MScalar,
  DANodeClient,
  getArciumAccountBaseSeed,
  getArciumProgAddress,
  NodeDAInfo,
  uploadCircuit,
  getCompDefAccOffset,
  getDataObjPDA,
  buildFinalizeCompDefTx,
} from "@arcium-hq/arcium-sdk";
import * as fs from "fs";
import * as os from "os";

// TODO: Reading these out directly from the conf circuit interface would be nicer
type OrderBook = {
  orders: [
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order,
    Order
  ];
};

type Order = {
  size: bigint;
  bid: boolean;
  owner: bigint;
};

describe("Dark pool", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.DarkPool as Program<DarkPool>;
  const provider = anchor.getProvider();

  const arciumEnv = getArciumEnv();
  const daNodeClient = new DANodeClient(arciumEnv.DANodeURL);

  it("Is initialized!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);
    const initProgramSig = await initProgram(program, daNodeClient);
    const initAOSig = await initAddOrderCompDef(program, owner, false);
    const initNMSig = await initFindNextMatchCompDef(program, owner, false);

    const inputVal: RawConfidentialInstructionInputs<AddOrder> = [
      { value: [BigInt(1) as MScalar, true, BigInt(3) as MScalar] },
    ];
    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );
    const req = buildOffchainRefRequest(inputVal, cluster_da_info);
    const oref = await daNodeClient.postOffchainReference(req);
    const queueRawTx = await program.methods
      .addOrder(oref)
      .accounts({ payer: owner.publicKey })
      .rpc();
    const correctObPDA = getDataObjPDA(getArciumProgAddress(), 42);
  });

  async function initProgram(
    program: Program<DarkPool>,
    daNodeClient: DANodeClient
  ): Promise<string> {
    const oderBookPDA = getDataObjPDA(getArciumProgAddress(), 42);

    // Empty orderbook is 16 orders times three scalars per order = 48 scalars
    const emptyBook: { value: bigint }[] = new Array(48).fill({
      value: BigInt(0),
    });
    const cluster_da_info = await getClusterDAInfo(
      provider.connection,
      arciumEnv.arciumClusterPubkey
    );
    const req = buildOffchainRefRequest(emptyBook, cluster_da_info);
    const oref = await daNodeClient.postOffchainReference(req);

    return program.methods.init(oref).accounts({ ob: oderBookPDA }).rpc();
  }

  async function initFindNextMatchCompDef(
    program: Program<DarkPool>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("find_next_match");

    // Initialize the add together computation definition
    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, offset],
      getArciumProgAddress()
    )[0];

    const sig = await program.methods
      .initNextMatchCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc();
    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/find_next_match.arcis"
      );
      await uploadCircuit(
        provider.connection,
        owner,
        "find_next_match",
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        owner.publicKey,
        Buffer.from(offset).readUInt32LE()
      );
      const latestBlockhash = await provider.connection.getLatestBlockhash();
      finalizeTx.recentBlockhash = latestBlockhash.blockhash;
      finalizeTx.lastValidBlockHeight = latestBlockhash.lastValidBlockHeight;
      finalizeTx.sign(owner);
      await provider.sendAndConfirm(finalizeTx);
    }
    return sig;
  }

  async function initAddOrderCompDef(
    program: Program<DarkPool>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("add_order");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, offset],
      getArciumProgAddress()
    )[0];

    console.log("compDefPDA internally is ", compDefPDA.toBase58());

    const sig = await program.methods
      .initAddOrderCompDef()
      .accounts({ compDefAcc: compDefPDA, payer: owner.publicKey })
      .signers([owner])
      .rpc({
        commitment: "confirmed",
      });
    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync(
        "confidential-ixs/build/add_order.arcis"
      );
      await uploadCircuit(
        provider.connection,
        owner,
        "add_order",
        rawCircuit,
        true
      );
    } else {
      const finalizeTx = await buildFinalizeCompDefTx(
        owner.publicKey,
        Buffer.from(offset).readUInt32LE()
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
