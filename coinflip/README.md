# Confidential Coin Flip on Solana

This project demonstrates building a confidential on-chain Coin Flip game using Arcium. A player chooses Heads or Tails, and the outcome is determined by a random boolean generated securely within Arcium's confidential computation environment.

## How It Works

### The Challenge of On-Chain Randomness

Generating true, unpredictable randomness on a public blockchain is difficult. On-chain pseudo-random number generators can often be predicted or influenced, making games like Coin Flip potentially unfair if the outcome could be known beforehand.

### Arcium's Solution

Arcium provides confidential computing on Solana, enabling secure random number generation for games:

1.  **Confidential Player Choice**: The player submits their choice (Heads or Tails) encrypted.
2.  **Secure Random Boolean Generation**: The Arcium network generates a random boolean value (representing the coin flip) securely within its confidential computation environment.
3.  **Confidential Computation**: The Arcium network compares the player's encrypted choice against the securely generated random boolean.
4.  **Result Calculation**: The result (win or lose) is computed without revealing the player's choice or the random boolean during the process.
5.  **On-Chain Result**: Only the final outcome (win or lose) is published to the Solana blockchain.

## Game Flow

1.  The player initializes a game session on Solana.
2.  The player submits their encrypted choice (Heads or Tails).
3.  The Solana program triggers the confidential computation on the Arcium network.
4.  Within Arcium's secure environment:
    - A random boolean (the coin flip) is securely generated.
    - The winner is determined based on the player's choice and the generated boolean.
5.  The result (win or lose) is sent back to the Solana program.
6.  The game outcome is recorded on-chain.

## Security Features

- **Player Choice Privacy**: The player's choice (Heads/Tails) is not revealed on-chain or during computation.
- **Unpredictable Outcome**: The coin flip result (random boolean) is generated securely within Arcium's trusted environment, preventing prediction or manipulation.
- **Fair Computation**: The game logic runs securely within Arcium.
- **Verifiable Outcome**: The final result is recorded transparently on the blockchain.

## Getting Started

Refer to the [Arcium documentation](https://docs.arcium.com) for setup instructions.
