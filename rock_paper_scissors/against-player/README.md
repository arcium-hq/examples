# Rock Paper Scissors - Player vs Player

Player-versus-player asynchronous games require encrypted moves to prevent information advantages. Without cryptographic enforcement, players may observe opponent moves before finalizing their own, or modify submitted moves retroactively.

This example demonstrates a player-versus-player protocol where both players submit encrypted moves asynchronously - neither can see the other's choice until both are submitted.

## Why do public blockchains break asynchronous gameplay?

Blockchains make everything public - which breaks games where information needs to stay hidden. Public blockchain state allows participants to observe transaction data before submitting their own moves, creating information advantages.

Traditional approaches have limitations: unencrypted storage makes moves visible to all participants immediately, decryption requires someone to view moves sequentially, and trusted referees who decrypt moves can potentially favor specific players.

The requirement is encrypted asynchronous submission where each player finalizes their choice before learning their opponent's move.

## How Encrypted Asynchronous Gameplay Works

The protocol enforces fairness through encrypted move submission and secure comparison:

1. **Player 1 encrypted submission**: First player's move is encrypted and recorded on-chain
2. **Player 2 encrypted submission**: Second player's move is encrypted and recorded on-chain
3. **Verification**: System confirms both encrypted moves are submitted
4. **Encrypted comparison**: Arcium nodes jointly compute the outcome
5. **Result disclosure**: Only the game outcome (player 1 wins/player 2 wins/tie) is revealed

The comparison occurs on encrypted values throughout. Only the final game outcome is revealed.

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite simulates two-player gameplay with encrypted move submissions, demonstrating the complete protocol from move submission through outcome determination.

## Technical Implementation

Both player moves are encrypted and submitted on-chain. The game outcome is computed using Arcium's confidential instructions, which process encrypted moves without decryption.

Key properties:
- **Asynchronous encrypted commitment**: Both moves locked in before comparison, despite asynchronous submission
- **Minimal revelation**: Only the game result is disclosed, not individual moves
- **Integrity**: The MPC protocol ensures correct game resolution even with a dishonest majority—neither player can manipulate the outcome as long as one node is honest

## Implementation Details

### The Asynchronous Information Hiding Problem

**Conceptual Challenge**: Rock Paper Scissors requires both players to commit to moves without seeing the opponent's choice. Online, players submit asynchronously - Player 1 first, then Player 2. On public blockchains, Player 2 can see Player 1's move before submitting their own.

**Traditional solutions**:
- **Commit-reveal**: Submit hash of move, then reveal. Problem: With only 3 options (rock/paper/scissors), attackers can precompute all possible hashes and reverse them.
- **Trusted referee**: Third party collects moves. Problem: Referee can peek or leak.
- **Time locks**: Strict submission windows. Problem: Network latency, timezone issues.

**The Question**: Can we hide moves on a public blockchain to enable fair asynchronous gameplay?

### The Encrypted State Pattern

```rust
pub struct GameMoves {
    player_a_move: u8,  // 0=Rock, 1=Paper, 2=Scissors, 3=Not submitted yet
    player_b_move: u8,
}
```

Stored on-chain as `Enc<Mxe, GameMoves>` - network-encrypted, no one can decrypt.

**Phase 1 - Initialization**:
```rust
GameMoves { player_a_move: 3, player_b_move: 3 }  // Both "empty"
```

**Phase 2 - Player submits move** (inside MPC):
```rust
pub fn player_move(
    players_move: Enc<Shared, PlayersMove>,
    game: Enc<Mxe, GameMoves>,
) -> Enc<Mxe, GameMoves> {
    let input = players_move.to_arcis();
    let mut game = game.to_arcis();

    // Validate: player hasn't moved yet (3 = invalid move, used as "empty" marker)
    if input.player == 0 && game.player_a_move == 3 && input.player_move < 3 {
        game.player_a_move = input.player_move;  // Update encrypted state
    }
    // Similar logic for player B...

    game.owner.from_arcis(game)  // Return updated encrypted moves
}
```

**Phase 3 - Comparison** (only after both submitted):
```rust
pub fn compare_moves(game: Enc<Mxe, GameMoves>) -> u8 {
    let moves = game.to_arcis();

    if moves.player_a_move == 3 || moves.player_b_move == 3 {
        return 3;  // Error: incomplete game
    }

    // Determine winner based on Rock-Paper-Scissors logic...
    result.reveal()  // Only reveal winner (0=tie, 1=A wins, 2=B wins)
}
```
