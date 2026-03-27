# Rock Paper Scissors - Player vs Player

On public blockchains, Player 2 can see Player 1's move before submitting their own. Commit-reveal doesn't help -- with only 3 possible moves, all hashes can be precomputed. This example uses encrypted state so neither player can see the other's choice until both are submitted.

## How It Works

1. Both players submit encrypted moves on-chain (order doesn't matter)
2. Game state tracks moves as `Enc<Mxe, GameMoves>` -- network-encrypted, nobody can decrypt
3. Once both are in, Arcium nodes compare and reveal only the outcome (tie/A wins/B wins)

## Implementation

### Encrypted State

```rust
pub struct GameMoves {
    player_a_move: u8,  // 0=Rock, 1=Paper, 2=Scissors, 3=Not submitted
    player_b_move: u8,
}
```

Initialized with both moves set to `3` (empty). Each `player_move` call validates the player hasn't already submitted and the move is valid (<3), then updates the encrypted state.

### Move Submission (inside MPC)

```rust
pub fn player_move(
    players_move_ctxt: Enc<Shared, PlayersMove>,
    game_ctxt: Enc<Mxe, GameMoves>,
) -> Enc<Mxe, GameMoves> {
    let players_move = players_move_ctxt.to_arcis();
    let mut game_moves = game_ctxt.to_arcis();

    if players_move.player == 0 && game_moves.player_a_move == 3 && players_move.player_move < 3 {
        game_moves.player_a_move = players_move.player_move;
    }
    // Similar for player B...

    game_ctxt.owner.from_arcis(game_moves)
}
```

### Comparison (only after both submitted)

```rust
pub fn compare_moves(game_ctxt: Enc<Mxe, GameMoves>) -> u8 {
    let game_moves = game_ctxt.to_arcis();

    let result = if game_moves.player_a_move == 3 || game_moves.player_b_move == 3 {
        3  // Incomplete game
    } else if game_moves.player_a_move == game_moves.player_b_move {
        0  // Tie
    } else {
        // Rock-Paper-Scissors win logic...
    };

    result.reveal()  // 0=tie, 1=A wins, 2=B wins
}
```
