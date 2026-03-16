# Ed25519 Signatures - Distributed Key Management

Ed25519 signing and verification using MPC. The private key is split across multiple nodes and never exists in a single location.

## How It Works

**Signing**: A message is sent to the Arcium network where MPC nodes collectively produce a valid Ed25519 signature using their key shares. The signature can be verified by anyone with the public key.

**Verification with confidential public key**: The verifying key is provided encrypted, and verification runs inside MPC. Only the result (valid/invalid) is revealed to a designated observer.

## Implementation

### MPC Signing

```rust
pub fn sign_message(message: [u8; 5]) -> ArcisEd25519Signature {
    let signature = MXESigningKey::sign(&message);
    signature.reveal()
}
```

Each MPC node holds a key share and executes a distributed signing protocol to produce a standard Ed25519 signature without reconstructing the complete key.

> [Arcis Primitives](https://docs.arcium.com/developers/arcis/primitives)

### Confidential Public Key Verification

```rust
pub fn verify_signature(
    verifying_key_enc: Enc<Shared, Pack<VerifyingKey>>,
    message: [u8; 5],
    signature: [u8; 64],
    observer: Shared,
) -> Enc<Shared, bool> {
    let verifying_key = verifying_key_enc.to_arcis().unpack();
    let signature = ArcisEd25519Signature::from_bytes(signature);
    let is_valid = verifying_key.verify(&message, &signature);
    observer.from_arcis(is_valid)
}
```

Revealing which public key is being verified could leak identity or organizational affiliations. This pattern enables verification without public key disclosure.
