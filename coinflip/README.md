# Coinflip - Trustless Randomness

Flip a coin online and you have to trust someone. Trust the server not to rig the flip, or trust yourself not to inspect the code and game the system. There's no way to prove it's actually random.

This example shows how to generate verifiably random outcomes using distributed entropy generation.

## Why is randomness hard to trust online?

Traditional random number generation has a fundamental trust problem: whoever generates the random number can potentially influence or predict it. Server-side RNG can be biased by operators, client-side generation can be manipulated, and pseudorandom algorithms may have predictable patterns. The requirement is generating randomness that remains unpredictable and unbiased even when participants don't trust each other.

## How It Works

The coinflip follows this flow:

1. **Player commitment**: The player's choice (heads/tails) is encrypted and submitted to the network
2. **Random generation**: Arcium nodes work together to generate a random boolean outcome
3. **Encrypted comparison**: The system compares the encrypted choice against the encrypted random result
4. **Result disclosure**: Only the win/loss outcome is revealed

The comparison occurs on encrypted values throughout the computation.

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates the complete flow: player choice submission, secure random generation, encrypted comparison, and result verification.

## Technical Implementation

The player's choice is encrypted as a boolean, allowing result verification without exposing the choice prematurely. Random generation uses Arcium's cryptographic primitives, where Arcium nodes contribute entropy that no single node can predict or control.

## Implementation Details

### The Trustless Randomness Problem

**Conceptual Challenge**: In traditional online systems, randomness comes from somewhere - a server, a third-party service, your browser's `Math.random()`. Each source requires trusting that entity:

- **Server-generated**: Trust the operator doesn't rig outcomes
- **Third-party service**: Trust the service provider is honest
- **Client-side**: Trust the player doesn't inspect and manipulate

**The Question**: Can we generate randomness where NO single party can predict or bias the outcome?

### The MPC Randomness Solution

Arcium's `ArcisRNG` generates randomness through multi-party computation:

```rust
pub fn flip(input_ctxt: Enc<Shared, UserChoice>) -> bool {
    let input = input_ctxt.to_arcis();
    let toss = ArcisRNG::bool();  // MPC-generated randomness
    (input.choice == toss).reveal()
}
```

**How it works**:

1. Arcium nodes each generate local random values
2. Nodes combine their randomness using secure multi-party computation
3. Final random value is deterministic given all inputs
4. **No single node can predict the result** before all contribute
5. **No subset of nodes can bias the outcome**: The MPC protocol guarantees unbiased randomness even with a dishonest majority—the outcome remains unpredictable as long as one node is honest

### Stateless Design

Unlike Voting or Blackjack, Coinflip has **no game state account**:

- Receive encrypted player choice → Generate MPC random → Compare → Emit result
- Each flip is independent
- No persistent storage needed

When randomness generation itself is the primary feature, stateless design is simplest.

### When to Use This Pattern

Use MPC randomness (`ArcisRNG`) when:

- **No one should control outcome**: Lotteries, random drops, fair matchmaking
- **Platform can't be trusted**: House games where operator could cheat
- **Randomness is high-value**: Large prizes or critical game mechanics
