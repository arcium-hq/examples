# Coinflip — trustless randomness

The player encrypts a heads-or-tails guess, the MPC cluster flips a coin using
randomness no individual node can predict or bias, and only the win/loss bit becomes
public. The guess and toss are never published directly, though the player can infer
the toss from their guess and the public outcome.

**Use this pattern when** no party (operator, player, or any single node) may
predict or influence a random outcome: lotteries, random drops, fair matchmaking.

## How it works

1. The client derives a shared secret with the MXE via x25519, encrypts the boolean
   guess with `RescueCipher`, and calls `flip` with ciphertext, public key, and nonce.
2. `flip` packs the arguments with `ArgBuilder` and queues the computation via
   `queue_computation`, registering `flip_callback` for the result.
3. The `flip` circuit draws a random boolean with `ArcisRNG::bool()` and reveals only
   its comparison against the guess; both operands remain secret shares throughout.
4. `flip_callback` verifies the signed output and emits `FlipEvent`; nothing is stored.

Anyone can see the win/loss bit. The guess remains known only to the player, and no
node sees either operand; the player can derive the toss from their guess and outcome.

## Concepts demonstrated

- MPC randomness: `ArcisRNG::bool()` in the `flip` circuit draws a toss no single node
  can bias ([random number generation](https://docs.arcium.com/developers/arcis/primitives#random-number-generation)).
- Selective reveal: `.reveal()` wraps only the equality check — exactly one bit of
  the computation becomes public.
- Stateless MXE program: no game accounts; the callback only emits an event.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how the circuit reveals only the comparison,
  never `toss` or the guess.
- `programs/coinflip/src/lib.rs` — note how `flip_callback` calls `verify_output`
  before trusting the result.
- `tests/coinflip.ts` — note how the `FlipEvent` listener is registered before
  queueing, so the one-shot result is not missed.

## Pitfalls

- `ArgBuilder` order must mirror the circuit signature: `Enc<Shared, UserChoice>` is
  `x25519_pubkey`, `plaintext_u128` nonce, then ciphertext; a wrong order fails at
  computation time, not at build time.

## Limitations

- The outcome is public. It does not reveal the guess to observers, but it lets the
  player infer the toss because they already know their guess.
- The result lives only in `FlipEvent`; nothing persists on chain, so other programs
  cannot consume the outcome. A production game would also need a wager and payout.

See also: [Callback type generation](https://docs.arcium.com/developers/program/callback-type-generation) · **Next:** [Rock Paper Scissors](../rock_paper_scissors/)
