# Medical Records — re-encrypting data to a new owner

A patient stores an encrypted medical record on-chain and later shares it with a doctor: the record is decrypted only inside MPC and re-encrypted under the doctor's public key. No platform, node, or observer ever sees the plaintext.

**Use this pattern when** data encrypted for one party must become readable by another without any intermediary seeing the plaintext: credential handoffs, compliance disclosures, encrypted data markets.

## How it works

1. `store_patient_data` writes eleven client-encrypted ciphertexts (id, age, gender, blood type, weight, height, five allergy flags) into the `PatientData` PDA — a plain Anchor write, no MPC involved.
2. To share, the patient calls `share_patient_data` with the recipient's x25519 public key; the stored record is passed to the circuit by account reference.
3. The `share_patient_data` circuit takes the record as `Enc<Shared, PatientData>` and the recipient as a bare `Shared` owner: `to_arcis()` decrypts inside MPC, `receiver.from_arcis()` re-encrypts.
4. `share_patient_data_callback` verifies the signed output and emits `ReceivedPatientDataEvent`; the stored record is left untouched.

The patient can still decrypt the stored record, the recipient can decrypt the event, and everyone else sees only ciphertext.

## Concepts demonstrated

- [Sealing](https://docs.arcium.com/developers/encryption/sealing): a `Shared` parameter names the new owner; `from_arcis` seals the data to them.
- On-chain ciphertext as circuit input: `ArgBuilder` `.account()` feeds stored account data to MPC by pubkey, offset, and length.
- Event-based delivery: the callback emits the re-encrypted record instead of persisting it; the recipient collects it off-chain.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how the circuit body is two lines; a pure ownership handoff needs no computation on the data.
- `programs/share_medical_records/src/lib.rs` — note how `ArgBuilder` interleaves pubkeys and nonces to reconstruct both `Shared` values for the circuit.
- `tests/share_medical_records.ts` — note how the receiver derives their own shared secret with the MXE and decrypts using the nonce carried in the event.

## Pitfalls

- `ArgBuilder` order must mirror the circuit signature: receiver pubkey and nonce, then sender pubkey, nonce, and ciphertexts. Wrong order fails decryption inside MPC, not at submit time.
- The account reference starts at offset 8 to skip Anchor's discriminator; reading from 0 feeds discriminator bytes into the circuit as ciphertext.
- Decrypt the event with `ReceivedPatientDataEvent.nonce`, not the submitted `receiver_nonce` — the MXE assigns a fresh output nonce.

## Limitations

- Sharing is all-or-nothing: the whole `PatientData` struct is re-encrypted. Per-field selective disclosure would need separate circuits.
- Sharing metadata is public: anyone can see a record was shared and to which key.
- Delivery is a one-shot event; a recipient not listening at callback time must replay transaction logs.

See also: [Sealing (re-encryption)](https://docs.arcium.com/developers/encryption/sealing) · **Next:** [Sealed-Bid Auction](../sealed_bid_auction/)
