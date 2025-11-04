# Voting - Private Ballots, Public Results

How do you count votes on a blockchain where everything is public? Traditional encryption just moves the problem - someone has to hold the decryption key, and whoever holds it can reveal votes.

This example demonstrates anonymous voting where individual ballots remain permanently encrypted while aggregate tallies can be computed and publicly audited. No party - including poll creators, system administrators, or infrastructure operators - can access individual votes.

## Why is blockchain voting hard?

Transparent blockchain architectures conflict with ballot secrecy requirements:

1. **Transaction visibility**: All blockchain data is publicly accessible by default
2. **Ballot privacy**: People may not want peers, family, or colleagues knowing how they voted on sensitive issues - votes need to stay private to prevent social pressure and judgment
3. **Vote buying**: If you can prove how you voted, someone can pay you to vote a certain way and verify you followed through
4. **Public tallying**: Everyone needs to be able to check that the final count is correct, without seeing how individual people voted

The requirement is computing aggregate vote tallies without revealing individual ballots, while providing accurate and tamper-resistant final counts.

## How Private Voting Works

The protocol maintains ballot secrecy while providing accurate results:

1. **Ballot encryption**: Votes are encrypted on the client's computer before submission
2. **On-chain storage**: Encrypted ballots are recorded on the blockchain
3. **Secure distributed tallying**: Arcium network nodes collaboratively compute aggregate totals without any single node being able to see individual ballots
4. **Result publication**: Only aggregate vote counts are revealed, not individual choices
5. **Result accuracy**: Arcium's maliciously secure protocol is designed to prevent manipulation, even if some nodes act dishonestly, requiring only one honest actor

Arcium network nodes jointly compute the tally by processing encrypted votes. No single node can access individual ballot contents, and Arcium's maliciously secure protocol is designed to detect and prevent cheating by any single party.

## Running the Example

```bash
# Install dependencies
npm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates poll creation, encrypted ballot submission, secure distributed tallying, and result verification.

## Technical Implementation

Votes are sent as encrypted booleans and stored as encrypted vote counts on-chain (using `Enc<Shared, bool>` in the code). Arcium's confidential instructions enable aggregate computation over encrypted ballots.

Key properties:
- **Ballot secrecy**: Individual votes remain encrypted throughout the tallying process
- **Distributed computation**: Arcium network nodes jointly compute tallies without individual ballot access
- **Result accuracy**: Aggregate totals are computed correctly despite processing only encrypted data
- **Result integrity**: Arcium's maliciously secure protocol is designed to prevent manipulation even with dishonest participants, using built-in cheater detection requiring only one honest actor

## Implementation Details

### The Private Tallying Problem

**Conceptual Challenge**: How do you count votes without seeing individual ballots?

Traditional approaches all fail:
- **Encrypt then decrypt**: Someone holds the decryption key and can see votes
- **Zero-knowledge proofs**: Complex, doesn't solve the tallying problem
- **Trusted counter**: Requires trusting the tallying authority

**The Question**: Can we compute "yes_votes + no_votes" on encrypted data without ever decrypting individual votes?

### The Encrypted State Pattern

Voting demonstrates storing encrypted counters directly in Anchor accounts:

```rust
#[account]
pub struct PollAccount {
    pub vote_state: [[u8; 32]; 2],  // Two 32-byte ciphertexts
    pub nonce: u128,                // Cryptographic nonce
    pub authority: Pubkey,          // Who can reveal results
    // ... other fields
}
```

**What's stored**: Two encrypted `u64` counters (yes_count, no_count) as raw ciphertexts.

**Storage vs MPC types**: On-chain, these counters are stored as raw bytes `[[u8; 32]; 2]`. When passed to MPC instructions via `Argument::Account()`, Arcium deserializes them into typed `Enc<Mxe, VoteStats>` for computation.

### Reading Encrypted Account Data

MPC instructions need precise byte locations to read encrypted data from accounts. Unlike normal deserialization, MPC must know exactly where each encrypted value starts:

To use encrypted account data in MPC, specify exact byte offsets:

```rust
Argument::Account(
    ctx.accounts.poll_acc.key(),
    8 + 1,  // Skip: Anchor discriminator (8 bytes) + bump (1 byte)
    64,     // Read: 2 ciphertexts × 32 bytes = 64 bytes
)
```

**Memory layout**:
```
Byte 0-7:   Anchor discriminator
Byte 8:     bump
Byte 9-40:  yes_count ciphertext (Enc<Mxe, u64>)
Byte 41-72: no_count ciphertext (Enc<Mxe, u64>)
Byte 73+:   other fields...
```

### The Vote Accumulation Logic

**MPC instruction** (runs inside encrypted computation):
```rust
pub fn vote(
    input: Enc<Shared, UserVote>,    // Voter's encrypted choice
    votes: Enc<Mxe, VoteStats>,      // Current encrypted tallies
) -> Enc<Mxe, VoteStats> {
    let input = input.to_arcis();     // Decrypt in MPC (never exposed)
    let mut votes = votes.to_arcis(); // Decrypt tallies in MPC

    if input.vote {
        votes.yes_count += 1;  // Increment happens inside MPC
    } else {
        votes.no_count += 1;
    }

    votes.owner.from_arcis(votes)  // Re-encrypt updated tallies
}
```

**Key insight**: The `+= 1` happens on encrypted values inside MPC. No one sees the actual counts.

**Callback** (runs on-chain after MPC completes):
```rust
pub fn vote_callback(
    ctx: Context<VoteCallback>,
    output: ComputationOutputs<VoteStatsOutput>,
) -> Result<()> {
    let VoteStatsOutput { nonce, ciphertext } = extract_output(output)?;

    // Save new encrypted tallies + new nonce
    ctx.accounts.poll_acc.nonce = nonce;
    ctx.accounts.poll_acc.vote_state = ciphertext;
    Ok(())
}
```

### Nonce Management

**Critical**: Every MPC operation returns a new nonce. Must update it:
- Old nonce used for current encryption
- MPC operation produces new encryption with new nonce
- Callback must save both new ciphertext AND new nonce
- Using old nonce with new ciphertext breaks decryption

### Revealing Results

Only the poll authority can reveal results:
```rust
pub fn reveal_result(votes: Enc<Mxe, VoteStats>) -> bool {
    let votes = votes.to_arcis();
    (votes.yes_count > votes.no_count).reveal()  // Only reveal comparison
}
```

Notice: Returns **boolean** (yes won?), not actual vote counts. Further privacy preservation.

### When to Use This Pattern

Apply encrypted state accumulation when:
- Need to compute on private data over time (running totals, aggregations)
- Individual inputs must stay private
- Final result can be aggregate or comparison
- Examples: voting, surveys, private auctions, confidential analytics

This demonstrates how Arcium's Cerberus MPC protocol enables voting systems that satisfy both ballot secrecy and result integrity guarantees, suitable for governance systems, polls, and democratic decision-making on blockchain platforms.
