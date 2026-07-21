# Ed25519 — distributed key management

An MXE signs messages with an Ed25519 key that exists only as secret shares across the cluster's
nodes: MPC among the shares produces a standard signature without the key ever being assembled.
A second circuit verifies a signature against an encrypted public key, hiding the checked identity.

**Use this pattern when** you need standard signatures (attestations, cross-chain messages)
without any single party ever holding the key, or checks that hide whose key was verified.

## How it works

1. `sign_message` queues a plaintext 5-byte message; inside MPC, `MXESigningKey::sign` signs
   over the nodes' key shares and reveals the resulting `ArcisEd25519Signature`.
2. `sign_message_callback` reassembles the 64-byte signature from the two halves (r, s) the
   circuit outputs and emits `SignMessageEvent`, verifiable by anyone against the MXE's key.
3. For blind verification, the client encrypts a packed key (`Enc<Shared, Pack<VerifyingKey>>`)
   under a one-time x25519 key and calls `verify_signature` with plaintext message and signature.
4. The circuit checks the signature inside MPC and encrypts the boolean verdict for the
   `observer`; `verify_signature_callback` emits it in `VerifySignatureEvent`.

Signed messages and signatures are public by design; in verification only the verifying key
stays hidden, and only the observer can read the verdict.

## Concepts demonstrated

- Distributed signing with [`MXESigningKey`](https://docs.arcium.com/developers/arcis/primitives#mxe-cluster-signing):
  the key already lives inside the MXE, and the signature is deliberately revealed — it is only
  useful when public.
- [Data packing](https://docs.arcium.com/developers/arcis/primitives#data-packing):
  `Pack<VerifyingKey>` fits the 32-byte key into two field elements, hence exactly two
  `encrypted_u128` ciphertexts.
- Mixed plaintext and encrypted arguments in one `ArgBuilder` chain, including the dedicated
  `arcis_ed25519_signature` argument type.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how `sign_message` takes no encrypted inputs at all, yet
  its output depends on a secret no one holds.
- `programs/ed_25519/src/lib.rs` — note how `verify_signature` assembles arguments in exactly
  the order the circuit signature dictates.
- `tests/ed_25519.ts` — note how `createPacker` builds the `VerifyingKey` packer by hand and
  how the test randomly corrupts inputs to exercise the invalid path.

## Pitfalls

- `ArgBuilder` order must mirror the circuit signature: the encrypted key param expands to
  `x25519_pubkey` + nonce + two `encrypted_u128` ciphertexts, `observer: Shared` to pubkey + nonce last.
- The client packer's field name must match the Arcis struct field (`public_key_encoded`)
  exactly; a mismatch corrupts data silently.
- Results arrive only via events; this program stores nothing on chain.

## Limitations

- Messages are fixed at 5 bytes by the circuit signatures; other lengths need their own circuits.
- `verify_signature` hides only the key and the verdict; the message and signature are public.
- One signing key per MXE — no rotation, no per-user keys; every caller signs as the same identity.

See also: [Arcis cryptographic operations](https://docs.arcium.com/developers/arcis/primitives#cryptographic-operations)
