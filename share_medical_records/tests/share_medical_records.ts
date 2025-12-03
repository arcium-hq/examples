import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { ShareMedicalRecords } from "../target/types/share_medical_records";
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

describe("ShareMedicalRecords", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace
    .ShareMedicalRecords as Program<ShareMedicalRecords>;
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

  it("can store and share patient data confidentially!", async () => {
    const owner = readKpJson(`${os.homedir()}/.config/solana/id.json`);

    const mxePublicKey = await getMXEPublicKeyWithRetry(
      provider as anchor.AnchorProvider,
      program.programId
    );

    console.log("MXE x25519 pubkey is", mxePublicKey);

    console.log("Initializing share patient data computation definition");
    const initSPDSig = await initSharePatientDataCompDef(
      program,
      owner,
      false,
      false
    );
    console.log(
      "Share patient data computation definition initialized with signature",
      initSPDSig
    );

    const senderPrivateKey = x25519.utils.randomSecretKey();
    const senderPublicKey = x25519.getPublicKey(senderPrivateKey);
    const sharedSecret = x25519.getSharedSecret(senderPrivateKey, mxePublicKey);
    const cipher = new RescueCipher(sharedSecret);

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

    const receiverSecretKey = x25519.utils.randomSecretKey();
    const receiverPubKey = x25519.getPublicKey(receiverSecretKey);
    const receiverNonce = randomBytes(16);

    const receivedPatientDataEventPromise = awaitEvent(
      "receivedPatientDataEvent"
    );

    const computationOffset = new anchor.BN(randomBytes(8), "hex");

    const queueSig = await program.methods
      .sharePatientData(
        computationOffset,
        Array.from(receiverPubKey),
        new anchor.BN(deserializeLE(receiverNonce).toString()),
        Array.from(senderPublicKey),
        new anchor.BN(deserializeLE(nonce).toString())
      )
      .accountsPartial({
        computationAccount: getComputationAccAddress(
          getArciumEnv().arciumClusterOffset,
          computationOffset
        ),
        clusterAccount: clusterAccount,
        mxeAccount: getMXEAccAddress(program.programId),
        mempoolAccount: getMempoolAccAddress(getArciumEnv().arciumClusterOffset),
        executingPool: getExecutingPoolAccAddress(getArciumEnv().arciumClusterOffset),
        compDefAccount: getCompDefAccAddress(
          program.programId,
          Buffer.from(getCompDefAccOffset("share_patient_data")).readUInt32LE()
        ),
        patientData: PublicKey.findProgramAddressSync(
          [Buffer.from("patient_data"), owner.publicKey.toBuffer()],
          program.programId
        )[0],
      })
      .rpc({ commitment: "confirmed" });
    console.log("Queue sig is ", queueSig);

    const finalizeSig = await awaitComputationFinalization(
      provider as anchor.AnchorProvider,
      computationOffset,
      program.programId,
      "confirmed"
    );
    console.log("Finalize sig is ", finalizeSig);

    const receiverSharedSecret = x25519.getSharedSecret(
      receiverSecretKey,
      mxePublicKey
    );
    const receiverCipher = new RescueCipher(receiverSharedSecret);

    const receivedPatientDataEvent = await receivedPatientDataEventPromise;

    // Decrypt all patient data fields
    const decryptedFields = receiverCipher.decrypt(
      [
        receivedPatientDataEvent.patientId,
        receivedPatientDataEvent.age,
        receivedPatientDataEvent.gender,
        receivedPatientDataEvent.bloodType,
        receivedPatientDataEvent.weight,
        receivedPatientDataEvent.height,
        ...receivedPatientDataEvent.allergies,
      ],
      new Uint8Array(receivedPatientDataEvent.nonce)
    );

    // Verify all fields match the original data
    expect(decryptedFields[0]).to.equal(patientData[0], "Patient ID mismatch");
    expect(decryptedFields[1]).to.equal(patientData[1], "Age mismatch");
    expect(decryptedFields[2]).to.equal(patientData[2], "Gender mismatch");
    expect(decryptedFields[3]).to.equal(patientData[3], "Blood type mismatch");
    expect(decryptedFields[4]).to.equal(patientData[4], "Weight mismatch");
    expect(decryptedFields[5]).to.equal(patientData[5], "Height mismatch");

    // Verify allergies
    for (let i = 0; i < 5; i++) {
      expect(decryptedFields[6 + i]).to.equal(
        patientData[6 + i],
        `Allergy ${i} mismatch`
      );
    }

    console.log("All patient data fields successfully decrypted and verified");
  });

  async function initSharePatientDataCompDef(
    program: Program<ShareMedicalRecords>,
    owner: anchor.web3.Keypair,
    uploadRawCircuit: boolean,
    offchainSource: boolean
  ): Promise<string> {
    const baseSeedCompDefAcc = getArciumAccountBaseSeed(
      "ComputationDefinitionAccount"
    );
    const offset = getCompDefAccOffset("share_patient_data");

    const compDefPDA = PublicKey.findProgramAddressSync(
      [baseSeedCompDefAcc, program.programId.toBuffer(), offset],
      getArciumProgramId()
    )[0];

    console.log("Comp def pda is ", compDefPDA);

    const sig = await program.methods
      .initSharePatientDataCompDef()
      .accounts({
        compDefAccount: compDefPDA,
        payer: owner.publicKey,
        mxeAccount: getMXEAccAddress(program.programId),
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
    } else if (!offchainSource) {
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
