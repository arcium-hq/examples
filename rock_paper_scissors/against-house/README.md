# Rock Paper Scissors vs House - Fair Gaming

Player-versus-house games require trust that the house algorithm operates fairly. In traditional implementations, the house can observe player moves before generating responses, or use biased random number generation to favor house outcomes.

This example demonstrates fair gaming where the house cannot access player moves before generating its response, and randomness generation is cryptographically secure.

## Why can't you trust the house?

Traditional house games have a trust problem: the house can see your move before responding, bias the random number generator, hide how the algorithm works, and modify game behavior without telling anyone. Every layer requires trusting the operator not to cheat, creating information asymmetry that favors the house.

## How Provably Fair Gaming Works

The protocol ensures fairness through cryptographic isolation:

1. **Player commitment**: The player's move is encrypted and submitted to the blockchain
2. **House response generation**: Nodes in Arcium network generate a random house move without accessing player data
3. **Encrypted comparison**: Both moves are compared in encrypted form
4. **Result disclosure**: Only the game outcome (win/loss/tie) is revealed

The house algorithm cannot access the player's encrypted move during response generation. Random number generation uses cryptographic primitives that no single party can predict or bias.

## Running the Example

```bash
# Install dependencies
npm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates the complete protocol: player move encryption, house random response generation, encrypted comparison, and outcome verification.

## Technical Implementation

The player's move is encrypted (as `Enc<Shared, u8>` in the code) and committed on-chain. The house response is generated using network-generated randomness with the following properties:

- **Isolation**: House randomness generation occurs in an environment isolated from player data
- **Unbiasability**: No single network node can influence the random outcome
- **Integrity**: Arcium's maliciously secure protocol is designed to prevent manipulation even if some nodes act dishonestly

Key mechanisms:
- Encrypted move storage prevents house observation
- Network random generation provides fair house responses
- On-chain logic allows outcome verification

This is powered by Arcium's Cerberus MPC protocol, which prevents any single party from cheating or manipulating outcomes through maliciously secure multi-party computation requiring only one honest actor. This enables fair gaming without requiring players to trust the house operator.
