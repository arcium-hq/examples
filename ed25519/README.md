# Ed25519 Signatures - Confidential Key Management

Store a private key anywhere and you have to trust someone. Trust the server not to leak it, trust the hardware not to be compromised, or trust yourself not to lose it. There's no way to prove the key is safe.

This example shows how to perform Ed25519 signing and verification using distributed key management through multi-party computation.

## Why is key custody hard to trust?

Traditional key management has a fundamental custody problem: whoever holds the private key can use it, lose it, or have it stolen. Hot wallets are vulnerable to hacks, cold storage can be lost, and multi-sig still requires trusting individual keyholders. The requirement is performing cryptographic operations with a key that doesn't exist in any single location.

## How It Works

This example demonstrates two operations:

### 1. Message Signing
1. **Message submission**: A message is sent to the Arcium network
2. **Distributed signing**: MPC nodes collectively generate a signature using their key shares
3. **Signature output**: A valid Ed25519 signature is produced without reconstructing the private key
4. **Public verification**: Anyone can verify the signature using the public key

### 2. Signature Verification (with Confidential Public Key)
1. **Encrypted key input**: The verifying key (public key) is provided in encrypted form
2. **MPC verification**: The signature is verified against the encrypted key within MPC
3. **Confidential result**: Only the verification result (valid/invalid) is revealed
4. **Observer decryption**: The result is encrypted for a designated observer

The private key never exists in a single location throughout either process.

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

### The Distributed Key Problem

**Conceptual Challenge**: Traditional cryptographic operations require access to the complete private key:

- **Hot wallet**: Key stored on a connected system (vulnerable to hacks)
- **Cold storage**: Key stored offline (can be lost or stolen)
- **Multi-sig**: Multiple keys, but each is still a single point of failure
- **HSM**: Hardware security module (still a single custodian)

**The Question**: Can we perform signing operations where NO single party ever holds the complete private key?

### The MPC Signing Solution

Arcium's `MXESigningKey` enables signing through distributed key shares:

```rust
pub fn sign_message(message: [u8; 5]) -> ArcisEd25519Signature {
    let signature = MXESigningKey::sign(&message);
    signature.reveal()
}
```

**How it works**:

1. Each MPC node holds a share of the private key
2. Nodes execute a distributed signing protocol
3. Signature components are computed collaboratively
4. **The complete private key never exists** anywhere during the process
5. **The signature is valid**: Standard Ed25519 signature verifiable by anyone

### Confidential Public Key Verification

Unlike standard Ed25519, this example also demonstrates verification where the public key remains encrypted:

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

**Why encrypt the public key?** In some scenarios, revealing which public key is being verified could leak sensitive information (identity of a signer, organizational affiliations, etc.). This pattern enables verification without public key disclosure.

### Key Security Properties

- **No single point of failure**: No node can access the complete private key
- **Threshold security**: Key remains secure even if some nodes are compromised
- **Standard compatibility**: Produces standard Ed25519 signatures
- **Verifiable output**: Signatures can be verified by any Ed25519 implementation

### When to Use This Pattern

Use MPC Ed25519 signing (`MXESigningKey`) when:

- **High-value keys**: Treasury keys, organizational signing keys, identity keys
- **No trusted custody**: No single party should hold the complete key
- **Regulatory compliance**: Requirements for distributed key management
- **Long-term security**: Keys that must remain secure for extended periods

Use confidential verification when:

- **Privacy-preserving verification**: The public key identity should remain hidden
- **Selective disclosure**: Only specific parties should know verification results
- **Confidential authentication**: Proving identity without revealing public identifiers
