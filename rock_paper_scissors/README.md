# Rock Paper Scissors - Simultaneous Move Commitment

Rock Paper Scissors only works if both players reveal at the same time. Online, someone has to go first - and going first means the second player can see your move before choosing theirs. How do you make simultaneous moves work when everything happens in sequence?

This example demonstrates cryptographic commitment schemes that make truly simultaneous moves possible in digital games, where neither party can access opponent information before finalizing their own choice.

## Why is "at the same time" impossible online?

Physical simultaneous move games ensure fairness through synchronized action. Digital implementations face ordering constraints: data must be submitted sequentially, creating opportunities for information leakage.

Traditional approaches all fail for the same reason - someone can always see something. Servers can view moves before finalizing them, public blockchains expose all transaction data, and sequential submission means the second player watches the first player's transaction go through.

The solution requires cryptographic commitment schemes to enforce simultaneity without trusted intermediaries.

## How Simultaneous Commitment Works

The commitment scheme follows this protocol:

1. **Encrypted submission**: Each player's move is encrypted before submission
2. **Commitment phase**: Both encrypted moves are recorded on-chain
3. **Verification**: The system confirms both commitments are finalized
4. **Revelation**: Moves are compared and the winner is determined

During the commitment phase, neither party can access the opponent's move. The comparison occurs on encrypted data, revealing only the game outcome without exposing individual moves unnecessarily.

## Variants

Two implementations demonstrate different commitment scenarios:

### [Player vs Player](./against-player/)
Two players submit encrypted moves. Neither party can access the opponent's choice until both commitments are finalized.

### [Player vs House](./against-house/)
Player competes against an on-chain algorithm. The system cannot access the player's move before generating its response, ensuring provable fairness.

## Technical Implementation

Both variants use Arcium's confidential instructions to implement commitment schemes. Player moves are encrypted (as `Enc<Shared, u8>` in the code), preventing information disclosure during the commitment phase.

Arcium's protocol prevents either party from gaining information advantage during the commitment phase, ensuring fair gameplay.

This is powered by Arcium's Cerberus MPC protocol, which prevents information advantage through maliciously secure multi-party computation requiring only one honest actor (Arcium's dishonest majority security model).
