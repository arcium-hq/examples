# Blackjack — hidden game state

A single-player blackjack game where the shuffled deck and the dealer's hole card exist only as ciphertext.
MPC nodes shuffle and deal inside the computation; the player decrypts only their own hand and the dealer's
face-up card, and no server, node, or chain observer can peek at undealt cards.

**Use this pattern when** encrypted state must persist on-chain across many turns with rule-driven selective
reveals: card games, fog-of-war strategy, any turn-based game with private state.

## How it works

1. `initialize_blackjack_game` creates the `BlackjackGame` PDA and queues `shuffle_and_deal_cards`, which
   shuffles a 52-card deck with `ArcisRNG::shuffle` and deals two cards each: deck and dealer hand return as
   `Enc<Mxe, ...>` (decryptable by no one), player hand and dealer face-up card as `Enc<Shared, ...>`
   (decryptable only by the player).
2. The callback stores every ciphertext in `BlackjackGame` and emits `CardsShuffledAndDealtEvent`; the
   player decrypts their hand client-side.
3. `player_hit` and `player_double_down` feed the stored deck and hand ciphertexts back into MPC by byte
   offset (`ArgBuilder`), draw the next card, and reveal only a bust boolean. `player_stand` ends the turn.
4. `dealer_play` draws against the encrypted deck inside MPC until reaching 17, then returns the dealer hand
   re-encrypted to both the MXE and the player.
5. `resolve_game` compares both hands inside MPC and reveals a single result code.

Everyone sees hand sizes, `GameState` transitions, bust flags, and the final winner; only the player sees
card values; nobody ever sees the undealt deck.

## Concepts demonstrated

- Packed encrypted state: `Pack<[u8; 52]>` fits the whole deck into two ciphertexts. Size math is in the
  module header of `programs/blackjack/src/lib.rs`; see
  [Data packing](https://docs.arcium.com/developers/arcis/primitives#data-packing).
- Reading encrypted account state back into computations: each action passes stored ciphertexts via
  `ArgBuilder` account references instead of re-uploading them.
- A multi-computation state machine: `GameState` gates which of the six circuits may run.
- [`ArcisRNG::shuffle`](https://docs.arcium.com/developers/arcis/primitives#shuffling): unbiased deck order
  no party can predict.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how one circuit returns both `Enc<Mxe, ...>` and `Enc<Shared, ...>` outputs.
- `programs/blackjack/src/lib.rs` — note how actions rebuild circuit inputs from stored ciphertexts by byte offset.
- `tests/blackjack.ts` — note how `unpackHand` shifts cards out of a decrypted field element to read a `Pack`'d hand.

## Pitfalls

- `ArgBuilder` argument order and byte offsets must match the circuit signature and the `BlackjackGame`
  field layout exactly; offsets count from the 8-byte discriminator (deck 8, player hand 72, dealer 104).
- `InvalidGameState`: each action checks the state machine; `dealer_play` before a stand or bust is rejected.
- `InvalidMove`: hitting after standing, or growing a hand past the 11-card `Hand` capacity.

## Limitations

- Hand sizes, bust flags, and the winner are public: observers see exactly when the player busts.
- The dealer's face-up card is encrypted to the player rather than published, unlike real blackjack.
- No bets, splits, insurance, or multiplayer, and no timeout path for abandoned games.

See also: [Arcis best practices](https://docs.arcium.com/developers/arcis/best-practices) · **Next:** [Ed25519](../ed25519/)
