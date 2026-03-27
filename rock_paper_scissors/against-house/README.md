# Rock Paper Scissors vs House - Fair Gaming

In traditional player-vs-house games, the house can observe player moves before generating responses, or use biased RNG to favor house outcomes. This example demonstrates fair gaming where the house cannot access the player's move before generating its response.

## How It Works

1. Player's move is encrypted and submitted on-chain
2. Arcium nodes generate a random house move inside MPC
3. Both moves are compared in encrypted form
4. Only the game outcome (win/loss/tie) is revealed

## Implementation

The full game runs in a single MPC computation -- the player's encrypted move enters, the house move is generated via `ArcisRNG`, and the comparison happens without either move being exposed.

The house move uses rejection sampling over 2-bit random candidates to ensure exactly 1/3 probability per move (a naive 2-bit mapping would bias one outcome to 50%).
