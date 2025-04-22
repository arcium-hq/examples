# Anonymous Voting with Arcium

This project demonstrates how to implement truly anonymous voting on Solana using Arcium's confidential computing capabilities. It showcases how to create private polls where individual votes remain confidential while still allowing for verifiable results.

## Why Arcium is Necessary for Anonymous Voting

Traditional blockchains are transparent by design, making it impossible to implement truly anonymous voting without additional privacy layers. Here's why Arcium is essential:

- **Public Nature of Blockchains**: All data on a regular blockchain is visible to everyone
- **Privacy Requirements**: Votes must remain confidential to ensure anonymity
- **Security Concerns**: Even encrypted votes would require decryption keys, creating vulnerabilities
- **Distributed Trust**: Arcium uses Multi-Party Computation (MPC) to achieve a trust-minimized setup for confidential computing

## How It Works

### 1. Poll Creation

```typescript
const pollSig = await program.methods.createNewPoll(
  POLL_ID,
  `Poll ${POLL_ID}: $SOL to 500?`,
  new anchor.BN(deserializeLE(pollNonce).toString())
);
```

- Creates a new poll with a unique ID and title
- Uses a cryptographic nonce for security operations
- Establishes the voting context on-chain

### 2. Voting Process

```typescript
const vote = BigInt(true);
const plaintext = [vote];
const nonce = randomBytes(16);
const ciphertext = cipher.encrypt(plaintext, nonce);
```

- Votes are encrypted using x25519 (key exchange) and RescueCipher
- Each vote uses a unique nonce for security
- Votes remain confidential even when stored on-chain

### 3. Confidential Computation

```typescript
const queueVoteSig = await program.methods.vote(
  POLL_ID,
  Array.from(ciphertext[0]),
  Array.from(publicKey),
  new anchor.BN(deserializeLE(nonce).toString())
);
```

- Encrypted votes are processed using MPC across multiple parties
- Computation is distributed across the Arcium network
- Individual vote values remain confidential throughout the computation

### 4. Result Revealed

```typescript
const revealQueueSig = await program.methods.revealResult(POLL_ID);
```

- Only the final result (e.g., majority vote) is revealed
- Individual votes remain confidential
- Results are computed through MPC and only the outcome is published
