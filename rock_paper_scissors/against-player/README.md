# Rock Paper Scissors vs Player — asynchronous hidden moves

Two players submit rock paper scissors moves at different times, each encrypted client-side and merged into MXE-owned game state that no individual party can decrypt. Neither player can see the other's move before both commit; the final outcome lets each infer the other's move.

**Use this pattern when** multiple parties submit hidden inputs asynchronously and only the result of comparing them should become public: sealed matches, blind negotiations, simultaneous-commitment games.

## How it works

1. `init_game` creates the `RPSGame` PDA with both players' public keys and queues the `init_game` circuit, which returns `Enc<Mxe, GameMoves>` with both move slots set to the sentinel `3` (valid moves are 0-2). The callback stores the two ciphertexts and the nonce in `RPSGame`.
2. Each player encrypts a `PlayersMove` (slot index plus move) client-side and calls `player_move`. The program checks the signer is `player_a` or `player_b`, then passes the client-encrypted input and the ciphertexts stored in `RPSGame` into the `player_move` circuit.
3. Inside MPC, the circuit writes the move into its slot only if the slot is still empty and the move is valid, then returns re-encrypted state; the callback overwrites `RPSGame` with the new ciphertexts and nonce.
4. Once both slots are filled, anyone calls `compare_moves`. The circuit reveals a single `u8` outcome and the callback emits it as a `CompareMovesEvent` ("Tie", "Player A Wins", "Player B Wins", or "Invalid Move" if a slot was still empty).

Moves stay MXE-owned end to end and are never published directly. The public learns only the outcome; each player can combine it with their own move to infer the other's.

## Concepts demonstrated

- Client-encrypted input merged into MXE-owned state: `player_move` takes `Enc<Shared, PlayersMove>` alongside `Enc<Mxe, GameMoves>`, the same shape as the order book in [Input/output](https://docs.arcium.com/developers/arcis/input-output).
- Passing stored ciphertexts back into MPC: `ArgBuilder`'s `account` method references the `RPSGame` account so the circuit computes over previously stored encrypted state ([Invoking computations](https://docs.arcium.com/developers/program)).
- Sentinel-guarded writes: "has this player already moved" is decided inside the circuit by comparing against the sentinel, so slot occupancy and guarded-write success remain hidden.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how `player_move` validates slot and move entirely inside MPC and returns the state unchanged when the guard fails.
- `programs/rock_paper_scissors/src/lib.rs` — note how `player_move` orders its `ArgBuilder` arguments: the shared input's pubkey, nonce, and ciphertexts before the MXE state's nonce and account slice.
- `tests/rock_paper_scissors.ts` — note how each player derives their own x25519 shared secret with the MXE, and how every move awaits finalization before the next is queued.

## Pitfalls

- `NotAuthorized`: `player_move` requires the signer to be the `player_a` or `player_b` recorded at `init_game`; signing with any other keypair fails on-chain.
- `ArgBuilder` order must mirror the circuit signature: for `Enc<Shared, PlayersMove>`, pubkey then nonce then ciphertexts in struct field order (`player` before `player_move`); for `Enc<Mxe, GameMoves>`, the stored nonce then the account slice. The slice starts at offset 8 to skip the Anchor discriminator, so `moves` must remain the first field of `RPSGame`.
- Rejected submissions are silent: a move above 2 or a write to an already-filled slot leaves state unchanged with no error; the problem only surfaces when `compare_moves` reports "Invalid Move".

## Limitations

- The signer check does not bind a signer to a slot: the slot claimed comes from the encrypted `player` field, so either registered player could fill the opponent's empty slot.
- Moves must finalize sequentially: the nonce is fixed when queueing while account ciphertexts are fetched during computation, so overlapping moves can use incompatible state versions or overwrite an update.
- No timeout or account closure: if one player never moves, the game stays open indefinitely, and the pairing (`player_a`, `player_b`, game id) is public.

See also: [Input/output](https://docs.arcium.com/developers/arcis/input-output) · **Next:** [Rock Paper Scissors vs House](../against-house/)
