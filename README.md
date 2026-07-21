# Arcium Examples

Example applications built with [Arcium](https://arcium.com), the encrypted compute
network on Solana. Each example computes on encrypted data using multi-party
computation (MPC): inputs stay encrypted end to end, and no single party, including
the nodes running the computation, ever sees them.

## How Arcium works

Every example follows the same lifecycle:

1. The client encrypts inputs locally (x25519 key exchange + RescueCipher) and
   submits them in a Solana transaction.
2. The Anchor program queues a computation with the Arcium program.
3. An MPC cluster executes the encrypted instruction defined in `encrypted-ixs/`.
4. The result returns on-chain via a callback instruction, which stores or reveals it.

Data stays private as long as one node in the cluster is honest; the protocol
tolerates a dishonest majority. See [Core Concepts](https://docs.arcium.com/developers/core-concepts)
and [Computation Lifecycle](https://docs.arcium.com/developers/computation-lifecycle)
for the full picture.

## Examples

Recommended path: work through them in order. Each introduces one new pattern.

| Example | Pattern | Encrypted on-chain state | Main concept |
|---|---|---|---|
| [Coinflip](./coinflip/) | Trustless randomness | No | `ArcisRNG`, revealing results |
| [Rock Paper Scissors](./rock_paper_scissors/) | Hidden moves | Yes ([vs player](./rock_paper_scissors/against-player/)), No ([vs house](./rock_paper_scissors/against-house/)) | `Enc<Shared, T>` inputs, RNG opponent |
| [Voting](./voting/) | Private aggregation | Yes | `Enc<Mxe, T>` accumulators, callbacks |
| [Medical Records](./share_medical_records/) | Controlled sharing | Yes | Re-encryption to a new owner |
| [Sealed-Bid Auction](./sealed_bid_auction/) | Encrypted comparison | Yes | First-price and Vickrey settlement |
| [Blackjack](./blackjack/) | Hidden game state | Yes | `Pack<T>` storage efficiency |
| [Ed25519 Signatures](./ed25519/) | Distributed signing | Yes | Keys that never exist in one place |

## Running an example

Prerequisites: the [Arcium CLI](https://docs.arcium.com/developers/installation),
which requires Rust, the Solana CLI, Anchor, and Docker. Toolchain versions are
pinned per example in `Cargo.toml` and `rust-toolchain.toml`.

Every example builds and tests the same way:

```bash
cd voting        # or any example
yarn install
arcium build     # compiles the encrypted circuit and the Anchor program
arcium test      # runs the tests against a local Arcium cluster
```

Each example has the same layout:

```
encrypted-ixs/   # Arcis circuit: the code that runs inside MPC
programs/        # Anchor program: queues computations, handles callbacks
tests/           # TypeScript tests: encryption happens client-side here
```

## Troubleshooting

- **`arcium test` hangs at keygen after a version bump**: stale localnet cache.
  Remove `artifacts/localnet/mxe_utility_pubkeys.bin` and
  `artifacts/localnet/private_shares_node_*`, then rerun.
- **Computation stuck in `queued`**: the local cluster did not finalize; rerun the
  test. Persistent failures usually mean the circuit and program are out of sync:
  run `arcium build` again.
- **`AbortedComputation` in a callback**: the MPC output failed verification. Check
  that the argument order in the queue call matches the circuit signature exactly.

## Documentation

[Mental Model](https://docs.arcium.com/developers/arcis/mental-model) ·
[Arcis reference](https://docs.arcium.com/developers/arcis) ·
[Best Practices](https://docs.arcium.com/developers/arcis/best-practices) ·
[Discord](https://discord.com/invite/arcium)

These examples are for learning. They are not audited and cut corners a production
deployment must not; see each example's Limitations section.
