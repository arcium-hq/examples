# Rock Paper Scissors vs House - Fair Gaming

Player-versus-house games require trust that the house algorithm operates fairly. In traditional implementations, the house can observe player moves before generating responses, or use biased random number generation to favor house outcomes.

This example demonstrates fair gaming where the house cannot access player moves before generating its response, and randomness generation is cryptographically secure.

## Why can't you trust the house?

Traditional house games have multiple trust problems: the house can see player moves before responding, bias the random number generator, or modify game behavior without transparency. Every layer requires trusting the operator not to cheat, creating information asymmetry that favors the house.

## How Provably Fair Gaming Works

The protocol ensures fairness through cryptographic isolation:

1. **Player encrypted submission**: The player's move is encrypted and submitted to the blockchain
2. **Random house move**: Arcium nodes generate a random house move using cryptographic randomness
3. **Encrypted comparison**: Both moves are compared in encrypted form
4. **Result disclosure**: Only the game outcome (win/loss/tie) is revealed

Random number generation uses cryptographic primitives that no single party can predict or bias.

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates the complete protocol: player move encryption, house random response generation, encrypted comparison, and outcome verification.

## Technical Implementation

The player's move is encrypted on the client and stored on-chain as a ciphertext. The house move is generated using Arcium's cryptographic randomness (similar to Coinflip), where Arcium nodes contribute entropy that no single node can predict or control.

Both moves are compared inside MPC on encrypted values.

Key properties:

- **Cryptographic randomness**: Arcium nodes contribute entropy; no single node or subset can predict or bias the outcome
- **Fair comparison**: Both moves processed in encrypted form throughout game resolution
- **Integrity**: The MPC protocol ensures correct game resolution even with a dishonest majority—neither the house nor the player can manipulate the outcome as long as at least one node is honest
