# Voting - Private Ballots, Public Results

Blockchain transparency makes voting dangerous: visible votes enable vote buying (prove your vote, get paid) and coercion. Encrypt the ballots? Whoever holds the decryption key to count them can still see every vote - and potentially sell or leak that data. Traditional encryption just shifts who holds the power.

This example demonstrates tallying votes without decrypting individual ballots. Votes stay encrypted throughout - only aggregate results are revealed.

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
2. **On-chain storage**: Encrypted votes are recorded on the blockchain
3. **Secure distributed tallying**: Arcium nodes collaboratively compute aggregate totals
4. **Result publication**: Only aggregate vote counts are revealed, not individual choices
5. **Security guarantee**: Arcium's MPC protocol preserves ballot secrecy even with a dishonest majority—individual votes remain private as long as one node is honest

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

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
- **Distributed computation**: Arcium nodes jointly compute aggregate tallies
- **Result accuracy**: Aggregate totals are computed correctly despite processing only encrypted data

## Implementation Details

### The Private Tallying Problem

**Conceptual Challenge**: How do you count votes without seeing individual ballots?

Traditional approaches all fail:

- **Encrypt then decrypt**: Someone holds the decryption key and can see votes
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

### Reading Encrypted Account Data

Arx nodes need precise byte locations to read encrypted data from accounts and deserialize it into the proper MPC function arguments.

To specify encrypted account data, provide exact byte offsets:

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

**Callback** (runs on-chain after MPC completes):

```rust
pub fn vote_callback(
    ctx: Context<VoteCallback>,
    output: SignedComputationOutputs<VoteOutput>,
) -> Result<()> {
    let o = match output.verify_output(
        &ctx.accounts.cluster_account,
        &ctx.accounts.computation_account,
    ) {
        Ok(VoteOutput { field_0 }) => field_0,
        Err(_) => return Err(ErrorCode::AbortedComputation.into()),
    };

    // Save new encrypted tallies + new nonce
    ctx.accounts.poll_acc.vote_state = o.ciphertexts;
    ctx.accounts.poll_acc.nonce = o.nonce;
    Ok(())
}
```

### Revealing Results

The program restricts result revelation to the poll authority:

```rust
pub fn reveal_result(votes: Enc<Mxe, VoteStats>) -> bool {
    let votes = votes.to_arcis();
    (votes.yes_count > votes.no_count).reveal()  // Only reveal comparison
}
```

### What This Example Demonstrates

This example shows how to:

- **Store encrypted data in Solana accounts**: Using raw bytes (`[[u8; 32]; 2]`) to persist encrypted values on-chain
- **Pass encrypted account data to MPC**: Using `Argument::Account()` with precise byte offsets to read encrypted state
- **Compute on encrypted state over time**: Accumulating encrypted values across multiple transactions (adding new votes to running tallies)

This pattern applies to any scenario requiring private aggregation: voting, surveys, sealed-bid auctions, confidential analytics, and private leaderboards.
