# Rock Paper Scissors vs House — hidden move, unbiased opponent

Rock-paper-scissors against a house whose move is drawn inside the MPC computation
itself. The player's move is encrypted client-side, the house move is sampled from
cluster randomness, and only a result code becomes public — neither move is ever
visible to the operator, observers, or any individual node.

**Use this pattern when** a secret user input must be judged against randomness no
party can predict or bias: casino-style games, random rewards on a private choice.

## How it works

1. The client derives a shared secret with the MXE via x25519, encrypts the move
   (`0` rock, `1` paper, `2` scissors) with `RescueCipher`, and calls `play_rps`,
   which packs the arguments with `ArgBuilder` and queues the computation.
2. The `play_rps` circuit receives the move as `Enc<Shared, PlayerMove>` and draws
   the house move inside MPC: 16 rounds of rejection sampling over two
   `ArcisRNG::bool()` bits, discarding value 3 so all three moves are equally likely.
3. The circuit compares the moves and reveals only a result code: `0` tie, `1`
   player wins, `2` house wins, `3` invalid move.
4. `play_rps_callback` verifies the signed output and emits `PlayRpsEvent`; no game
   state is written.

Anyone watching the chain sees the outcome; the program, observers, and individual
nodes see neither the player's move nor the house's.

## Concepts demonstrated

- Client secret meets cluster randomness: the `play_rps` circuit combines a
  client-encrypted input with `ArcisRNG` output in one computation, so neither side
  can react to the other.
- Rejection sampling under circuit constraints: the loop runs a fixed 16 iterations
  and latches the first valid candidate with a `selected` flag, because circuits
  cannot `break` or return early
  ([Arcis language constraints](https://docs.arcium.com/developers/limitations#arcis-language-constraints)).

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how uniformity comes from discarding candidate
  value 3 rather than taking a modulo, which would make rock twice as likely.
- `programs/rock_paper_scissors_against_rng/src/lib.rs` — note how `ArgBuilder`
  supplies `Enc<Shared, PlayerMove>` as pubkey, nonce, then ciphertext, exactly the
  order the circuit expects.
- `tests/rock_paper_scissors_against_rng.ts` — note how the client never decrypts
  anything: the outcome arrives as plaintext in `PlayRpsEvent`.

## Pitfalls

- Move encoding is `0`/`1`/`2`. An out-of-range value cannot be rejected on-chain
  (it is ciphertext there) and is not an error: the game completes and publicly
  reveals result `3`, telling every observer the move was invalid.

## Limitations

- The outcome is public to every observer, not just the player.
- If all 16 sampling rounds reject (probability 4^-16), `house_move` silently stays
  `0` and the house plays rock — negligible, but a production version should handle
  that case explicitly.
- Each call is an independent round: no wager, no score, no persistent state.

See also: [Random number generation](https://docs.arcium.com/developers/arcis/primitives#random-number-generation) ·
**Next:** [Voting](../../voting/)
