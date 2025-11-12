# Ed25519 Signatures - Confidential Key Management

This example demonstrates Ed25519 signing and verification using distributed key management through multi-party computation. The private key is split across multiple nodes and never exists in a single location.

## How It Works

**Message Signing**: A message is sent to the Arcium network where MPC nodes collectively generate a valid Ed25519 signature using their key shares. The signature can be verified by anyone using the public key.

**Signature Verification with Confidential Public Key**: The verifying key (public key) is provided in encrypted form, and the signature is verified within MPC. Only the verification result (valid/invalid) is revealed to a designated observer.

## Running the Example

```bash
# Install dependencies
yarn install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates both signing and verification flows with the MPC-managed key.

## Technical Implementation

### MPC Signing

Arcium's `MXESigningKey` enables signing through distributed key shares:

```rust
pub fn sign_message(message: [u8; 5]) -> ArcisEd25519Signature {
    let signature = MXESigningKey::sign(&message);
    signature.reveal()
}
```

Each MPC node holds a share of the private key and executes a distributed signing protocol to produce a standard Ed25519 signature without reconstructing the complete key.

### Confidential Public Key Verification

```rust
pub fn verify_signature(
    verifying_key_compressed_enc: Enc<Shared, CompressedVerifyingKey>,
    message: [u8; 5],
    signature: [u8; 64],
    observer: Shared,
) -> Enc<Shared, bool> {
    let verifying_key = VerifyingKey::from_compressed(verifying_key_compressed_enc.to_arcis());
    let signature = ArcisEd25519Signature::from_bytes(signature);
    let is_valid = verifying_key.verify(&message, &signature);
    observer.from_arcis(is_valid)
}
```

In some scenarios, revealing which public key is being verified could leak sensitive information (identity, organizational affiliations). This pattern enables verification without public key disclosure.
