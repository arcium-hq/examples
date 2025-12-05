# Rock Paper Scissors - Encrypted Moves

Rock Paper Scissors only works if neither player can see the other's move before submitting their own. Online, moves are submitted asynchronously - Player 1 submits first, Player 2 submits hours or days later. How do you prevent Player 2 from seeing Player 1's encrypted move before making their choice?

This example demonstrates how encrypted moves enable fair asynchronous gameplay where neither player can see the other's choice, even when both moves are stored on a public blockchain.

## Why can't you hide encrypted moves on a public blockchain?

Physical Rock Paper Scissors works because moves are revealed simultaneously. Digital games are asynchronous: Player 1 submits their encrypted move on-chain, then Player 2 submits hours or days later. The challenge is preventing Player 2 from learning Player 1's move before submitting their own.

Traditional approaches fail:

- **Unencrypted storage**: Player 2 sees Player 1's move immediately
- **Simple hashing (commit-reveal)**: With only 3 moves, Player 2 can brute-force all possible hashes
- **Trusted servers**: Requires trusting a third party not to leak moves
- **Time windows**: Don't solve the information hiding problem

The solution: Player 1's move is **encrypted and immutable** (can't be changed, but hidden from Player 2), even when stored on a public blockchain.

## How Encrypted Asynchronous Gameplay Works

1. **Player 1 submits encrypted move**: Encrypted move submitted and stored on-chain
2. **Asynchronous delay**: Player 2 can submit hours or days later
3. **Player 2 submits encrypted move**: Submits encrypted move without seeing Player 1's choice
4. **Encrypted comparison**: Moves compared without anyone being able to see them
5. **Result revelation**: Only game outcome (win/loss/tie) revealed

Player 1's move is **encrypted and immutable** (stored on-chain, can't be changed) but **hidden** (Player 2 can't decrypt it). Neither player gains information advantage, regardless of submission timing.

## Variants

Two implementations demonstrate different encrypted gameplay scenarios:

### [Player vs Player](./against-player/)

Two players submit encrypted moves. Neither party can access the opponent's choice until both encrypted moves are submitted.

### [Player vs House](./against-house/)

Player competes against an on-chain algorithm. The system cannot access the player's move before generating its response, ensuring provable fairness.
