# Coinflip - Trustless Randomness

Flip a coin online and you have to trust someone. Trust the server not to rig the flip, or trust yourself not to inspect the code and game the system. This example generates verifiably random outcomes where no single party can predict or bias the result.

## How It Works

1. Player's choice (heads/tails) is encrypted and submitted
2. Inside one MPC computation, Arcium nodes generate a random boolean, compare it against the encrypted choice, and produce the result
3. Only the win/loss outcome is revealed

## Implementation

### MPC Randomness

```rust
pub fn flip(input_ctxt: Enc<Shared, UserChoice>) -> bool {
    let input = input_ctxt.to_arcis();
    let toss = ArcisRNG::bool();
    (input.choice == toss).reveal()
}
```

Each Arcium node contributes local entropy. The MPC protocol combines these into a final value that no single node could predict before all contributed.

> See [Arcis Primitives](https://docs.arcium.com/developers/arcis/primitives) for all randomness and cryptographic operations.

### Stateless Design

Unlike Voting or Blackjack, Coinflip has no game state account. Each flip is independent -- receive encrypted choice, generate random, compare, emit result.
