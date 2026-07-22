# Voting — private ballots, public results

A poll where votes are encrypted on the voter's machine and never decrypted: MPC nodes
add each ballot to encrypted counters, and only the poll authority can reveal the
outcome, as a single boolean rather than the tallies.

**Use this pattern when** you need to aggregate private inputs over time and reveal
only the aggregate: surveys, leaderboards, confidential analytics.

## How it works

1. `create_new_poll` initializes two encrypted `u64` counters (`init_vote_stats`
   circuit) and stores the ciphertexts in the `PollAccount` PDA.
2. Each voter encrypts a boolean client-side and calls `vote`. The circuit takes the
   client-encrypted ballot (`Enc<Shared, UserVote>`) and the cluster-owned tallies
   (`Enc<Mxe, VoteStats>`), increments the right counter inside MPC, and returns
   re-encrypted tallies; the plaintext never leaves the computation.
3. A `VoterRecord` PDA (seeded by poll and voter) is created on first vote, so a
   second vote fails at the account level.
4. The poll authority calls `reveal_result`, which decrypts inside MPC and reveals
   only whether yes votes exceed no votes.

No party, not even the authority or an individual node, sees a ballot or the
running tally.

## Concepts demonstrated

- Persistent encrypted state: `PollAccount.vote_state` holds the tally ciphertexts,
  passed to the circuit by byte offset via `ArgBuilder` ([invoking a computation](https://docs.arcium.com/developers/program)).
- Mixed ownership in one circuit: `Enc<Shared, UserVote>` ballot, `Enc<Mxe, VoteStats>`
  tallies, re-encrypted via `owner.from_arcis` ([encryption overview](https://docs.arcium.com/developers/encryption)).
- Wallet-derived encryption keys: the test's `deriveEncryptionKey` hashes a wallet
  signature into an x25519 key, so voters carry no extra key material.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how `reveal_result` reveals a comparison, never
  the counts.
- `programs/voting/src/lib.rs` — note how the `vote` handler reads the tallies by
  byte offset and how the callbacks persist the rotated nonce.
- `tests/voting.ts` — note how `deriveEncryptionKey` builds the x25519 key from a
  wallet signature instead of throwaway key material.

## Pitfalls

- `InvalidAuthority`: only the wallet that created the poll can call `reveal_result`.
- `ArgBuilder` order must match the circuit signature: `x25519_pubkey`, nonce, then
  ciphertext for the ballot; stored nonce plus account reference for the tallies.
- The account read skips 9 bytes (8-byte discriminator + 1-byte `bump`) to reach
  `vote_state`; reordering `PollAccount` fields silently feeds garbage to the circuit.

## Limitations

- Participation is public: `VoterRecord` PDAs and `VoteEvent` show who voted and
  when; only the ballot content is hidden.
- No poll close: the authority can reveal at any time, and small tallies leak
  ballots (a one-vote poll's result is that voter's ballot). Ties reveal as `false`.
- Votes must finalize sequentially: the tally nonce is fixed when queueing while
  account ciphertexts are fetched during computation, so overlapping votes can use
  incompatible state versions or overwrite an update.

See also: [Shared vs MXE encryption](https://docs.arcium.com/developers/program/callback-type-generation#encryption-types-shared-vs-mxe) · **Next:** [Medical Records](../share_medical_records/)
