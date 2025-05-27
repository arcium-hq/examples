# Confidential Rock Paper Scissors Against the House on Solana

This project demonstrates building a confidential on-chain Rock Paper Scissors game using Arcium, where a player competes against the house. The player's move remains private, and the house's move is generated randomly within Arcium's secure computation environment.

## How It Works

### The Challenge of On-Chain Games

In typical on-chain games, all data is public. For a Player vs. House game, if the house's move generation were predictable or manipulatable on-chain, the game could be unfair.

## Game Flow

1.  The player initializes a game session on Solana.
2.  The player submits their encrypted move.
3.  The Solana program triggers the confidential computation on the Arcium network.
4.  Within Arcium's secure environment:
    - The house's move is randomly generated.
    - The winner is determined based on both moves.
5.  The result (win, lose, draw) is sent back to the Solana program.
6.  The game outcome is recorded on-chain.

## Getting Started

Refer to the [Arcium documentation](https://docs.arcium.com) for setup instructions.
