# Voting - Private Ballots, Public Results

Blockchain transparency makes voting dangerous: visible votes enable vote buying and coercion. Encrypt the ballots? Whoever holds the decryption key can still see every vote. This example tallies votes without decrypting individual ballots -- only aggregate results are revealed.

## How It Works

1. Votes are encrypted on the client before submission
2. Arcium nodes add each encrypted vote to running tallies without decrypting
3. Only the poll authority can reveal aggregate results
4. A `VoterRecord` PDA (seeded by poll + voter) prevents double-voting -- the account is initialized on vote, so a second attempt fails

## Implementation

### Encrypted State

Vote counters live on-chain as raw ciphertexts:

```rust
pub struct PollAccount {
    pub bump: u8,
    pub vote_state: [[u8; 32]; 2],  // [yes_count, no_count]
    pub id: u32,
    pub authority: Pubkey,
    pub nonce: u128,
    pub question: String,
}
```

### Byte Offsets

Arx nodes need precise byte locations to read encrypted data from accounts:

```rust
.account(
    ctx.accounts.poll_acc.key(),
    8 + 1,  // discriminator (8) + bump (1)
    32 * 2, // 2 ciphertexts x 32 bytes
)
```

```
Byte 0-7:   Anchor discriminator
Byte 8:     bump
Byte 9-40:  yes ciphertext
Byte 41-72: no ciphertext
Byte 73+:   id, authority, nonce, question
```

### Vote Accumulation

Circuit (runs inside MPC):

```rust
pub fn vote(
    vote_ctxt: Enc<Shared, UserVote>,
    vote_stats_ctxt: Enc<Mxe, VoteStats>,
) -> Enc<Mxe, VoteStats> {
    let user_vote = vote_ctxt.to_arcis();
    let mut vote_stats = vote_stats_ctxt.to_arcis();

    if user_vote.vote {
        vote_stats.yes += 1;
    } else {
        vote_stats.no += 1;
    }

    vote_stats_ctxt.owner.from_arcis(vote_stats)
}
```

Callback (writes updated ciphertexts back to the account):

```rust
pub fn vote_callback(ctx: Context<VoteCallback>, output: ...) -> Result<()> {
    let o = output.verify_output(...)?;
    ctx.accounts.poll_acc.vote_state = o.ciphertexts;
    ctx.accounts.poll_acc.nonce = o.nonce;
    Ok(())
}
```

> [Callback Type Generation](https://docs.arcium.com/developers/program/callback-type-generation), [Input/Output Patterns](https://docs.arcium.com/developers/arcis/input-output)

### Revealing Results

Only the poll authority can reveal, and only the comparison (not raw counts) is disclosed:

```rust
pub fn reveal_result(vote_stats_ctxt: Enc<Mxe, VoteStats>) -> bool {
    let vote_stats = vote_stats_ctxt.to_arcis();
    (vote_stats.yes > vote_stats.no).reveal()
}
```
