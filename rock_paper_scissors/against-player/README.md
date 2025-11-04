# Rock Paper Scissors - Player vs Player

Player-versus-player simultaneous move games require both participants to commit to choices before either choice is revealed. Without cryptographic enforcement, players may attempt to observe opponent moves before finalizing their own, or modify committed moves retroactively.

This example demonstrates a player-versus-player protocol where both players commit to encrypted moves simultaneously, with neither party able to access opponent information before commitment finalization.

## How do blockchains break simultaneous moves?

Blockchains make everything public - which breaks games where information needs to stay hidden. Public blockchain state allows participants to observe transaction data before committing their own moves, creating information advantages.

Traditional approaches have limitations: unencrypted storage makes moves visible to all participants immediately, decryption requires someone to view moves sequentially, and trusted referees who decrypt moves can potentially favor specific players.

The requirement is simultaneous commitment where neither player can access opponent information during the commitment phase, and determining the winner happens without anyone seeing individual moves.

## How Simultaneous Moves Work

The protocol enforces fairness through encrypted commitment and comparison by the network:

1. **Player 1 commitment**: First player's move is encrypted and recorded on-chain
2. **Player 2 commitment**: Second player's move is encrypted and recorded on-chain
3. **Verification**: System confirms both commitments are finalized
4. **Encrypted comparison**: Encrypted moves are compared without decryption
5. **Result disclosure**: Only the game outcome (player 1 wins/player 2 wins/tie) is revealed

Neither player can access opponent moves during the commitment phase. The comparison occurs entirely on encrypted values, preventing information leakage to any party.

## Running the Example

```bash
# Install dependencies
npm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite simulates two-player gameplay with encrypted commitments, demonstrating the complete protocol from move submission through outcome determination.

## Technical Implementation

Both player moves are encrypted (as `Enc<Shared, u8>` in the code) and committed on-chain. The game outcome is computed using Arcium's confidential instructions, which process encrypted moves without decryption.

Key properties:
- **Fair comparison**: Network nodes jointly compute the outcome without individual move visibility
- **Minimal revelation**: Only the game result is disclosed, not individual moves

## Implementation Details

### The Simultaneous Commitment Problem

**Conceptual Challenge**: Rock Paper Scissors requires both players to choose simultaneously. Online, someone must submit first - and going first means the second player can see your move.

**Traditional solutions**:
- **Commit-reveal**: Submit hash of move, then reveal. Problem: With only 3 options (rock/paper/scissors), attackers can precompute all possible hashes and reverse them.
- **Trusted referee**: Third party collects moves. Problem: Referee can peek or leak.
- **Time locks**: Strict submission windows. Problem: Network latency, timezone issues.

**The Question**: Can we enforce simultaneous commitment where neither player can see opponent's move, even while stored on-chain?

### The Encrypted Commitment Pattern

```rust
pub struct GameMoves {
    player_a_move: u8,  // 0=Rock, 1=Paper, 2=Scissors, 3=Not committed yet
    player_b_move: u8,
}
```

Stored on-chain as `Enc<Mxe, GameMoves>` - network-encrypted, no one can decrypt.

**Phase 1 - Initialization**:
```rust
GameMoves { player_a_move: 3, player_b_move: 3 }  // Both "empty"
```

**Phase 2 - Player commits** (inside MPC):
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

**Phase 3 - Comparison** (only after both committed):
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

### What Makes This Secure

1. **Encrypted moves**: `Enc<Mxe, GameMoves>` means NO ONE can decrypt (not players, not platform)
2. **Validation in MPC**: The check `game.player_a_move == 3` happens inside encrypted computation
3. **One-time commitment**: Once move != 3, cannot change it (check fails)
4. **Delayed revelation**: Winner only revealed after both commit
5. **Individual moves NEVER revealed**: Only result (0/1/2) decrypted

### The Commitment Scheme Pattern

This demonstrates general commitment scheme on encrypted data:

```
1. Initialize with sentinel value (3 = "empty")
2. Accept input only if still at sentinel value (== 3)
3. Validate input before accepting (< 3 for valid moves)
4. Store updated encrypted state
5. Reveal result only after all parties committed
```

Applies to: sealed-bid auctions, prediction markets, blind voting, poker hands, any scenario requiring simultaneous secret inputs.

This is powered by Arcium's Cerberus MPC protocol, which prevents either player from gaining information advantage through maliciously secure multi-party computation requiring only one honest actor.
