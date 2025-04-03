# Confidential Rock Paper Scissors on Solana

This project demonstrates how to build a confidential on-chain Rock Paper Scissors game using Arcium's confidential computing capabilities. The game ensures that neither player can see the other's move until both moves are committed, preventing any potential cheating.

## How It Works

### The Challenge of On-Chain Games

Traditional on-chain games face a fundamental challenge: all data on the blockchain is public. This means that in a game like Rock Paper Scissors, if one player's move is visible on-chain, the other player could simply wait to see it before making their own move, making the game unfair.

### Arcium's Solution

Arcium solves this by enabling confidential computing on Solana. Here's how it works:

1. **Confidential Move Submission**: Players submit their moves (Rock, Paper, or Scissors) in encrypted form
2. **Off-Chain Computation**: The Arcium network processes the game logic in a confidential environment
3. **Fair Resolution**: The result is computed without revealing either player's move
4. **On-Chain Result**: Only the final outcome (win, lose, or draw) is published to the blockchain

### Technical Implementation

The project is structured into two main components:

1. **Encrypted Instructions** (`encrypted-ixs/`):
   - Contains the confidential game logic
   - Processes encrypted moves without revealing them
   - Returns only the game result

2. **Solana Program** (`programs/rock_paper_scissors/`):
   - Handles on-chain state management
   - Manages player accounts and game sessions
   - Interfaces with Arcium's confidential computing network

## Game Flow

1. Players initialize a game session
2. Each player submits their move (encrypted)
3. The Arcium network processes the moves confidentially
4. The result is published on-chain
5. Players can claim their winnings based on the outcome

## Security Benefits

- **Move Privacy**: Neither player can see the other's move until both are committed
- **Fair Play**: The game logic runs in a trusted execution environment
- **Transparent Resolution**: While moves are private, the outcome is publicly verifiable
- **No Front-Running**: Players cannot manipulate the game by observing on-chain data

## Getting Started

For detailed setup instructions and API documentation, please refer to the [Arcium documentation](https://docs.arcium.com).
