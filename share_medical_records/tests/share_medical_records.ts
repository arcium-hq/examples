import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { ShareMedicalRecords } from "../target/types/share_medical_records";
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

describe("Share Medical Records", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .ShareMedicalRecords as Program<ShareMedicalRecords>;
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

    console.log("Initializing share patient data computation definition");
    const initSPDSig = await initSharePatientDataCompDef(program, owner, false);
    console.log(
      "Share patient data computation definition initialized with signature",
      initSPDSig
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

    const patientId = BigInt(420);
    const age = BigInt(69);
    const gender = BigInt(true);
    const bloodType = BigInt(1); // A+
    const weight = BigInt(70);
    const height = BigInt(170);
    // allergies are [peanuts, latex, bees, wasps, cats]
    const allergies = [
      BigInt(false),
      BigInt(true),
      BigInt(false),
      BigInt(true),
      BigInt(false),
    ];

    const patientData = [
      patientId,
      age,
      gender,
      bloodType,
      weight,
      height,
      ...allergies,
    ];

    const nonce = randomBytes(16);
    const ciphertext = cipher.encrypt(patientData, nonce);

    const storeSig = await program.methods
      .storePatientData(
        ciphertext[0],
        ciphertext[1],
        ciphertext[2],
        ciphertext[3],
        ciphertext[4],
        ciphertext[5],
        [
          ciphertext[6],
          ciphertext[7],
          ciphertext[8],
          ciphertext[9],
          ciphertext[10],
        ]
      )
      .rpc({ commitment: "confirmed" });
    console.log("Store sig is ", storeSig);

    const receiverKp = Keypair.generate();
    const receiverPubKey = receiverKp.publicKey;
    const receiverNonce = randomBytes(16);

    const receivedPatientDataEventPromise = awaitEvent(
      "receivedPatientDataEvent"
    );

    const queueSig = await program.methods
      .sharePatientData(
        Array.from(receiverPubKey.toBuffer()),
        new anchor.BN(deserializeLE(receiverNonce).toString()),
        ciphertext[0],
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        clusterAccount: arciumEnv.arciumClusterPubkey,
        mxeAccount: getMXEAccAcc(program.programId),
        mempoolAccount: getMempoolAcc(program.programId),
        executingPool: getExecutingPoolAcc(program.programId),
        compDefAccount: getCompDefAcc(
          program.programId,
          Buffer.from(getCompDefAccOffset("share_patient_data")).readUInt32LE()
        ),
        patientData: PublicKey.findProgramAddressSync(
          [Buffer.from("patient_data"), owner.publicKey.toBuffer()],
          program.programId
        )[0],
      })
      .rpc({ commitment: "confirmed" });
    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      queueSig,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const receivedPatientDataEvent = await receivedPatientDataEventPromise;
    const decrypted = cipher.decrypt(
      [receivedPatientDataEvent.patientId],
      nonce
    )[0];
    // expect(decrypted).to.equal(patientData.patientId);
    console.log("Decrypted patient data is ", decrypted);
  });

  async function initSharePatientDataCompDef(
    program: Program<ShareMedicalRecords>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("share_patient_data");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgAddress()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initSharePatientDataCompDef()
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
      "Init share patient data computation definition transaction",
      sig
    );

    if (uploadRawCircuit) {
      const rawCircuit = fs.readFileSync("build/share_patient_data.arcis");

      await uploadCircuit(
        provider as anchor.AnchorProvider,
        "share_patient_data",
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
