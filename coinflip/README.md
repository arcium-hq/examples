# Coinflip - Trustless Randomness

Flip a coin online and you have to trust someone. Trust the server not to rig the flip, or trust yourself not to inspect the code and game the system. There's no way to prove it's actually random.

This example shows how to generate random outcomes that no one can predict or manipulate - not the players, not the platform, not even the nodes running the computation.

## Why is randomness hard to trust online?

Digital randomness typically fails in three ways: server operators can influence outcomes, users can manipulate their local environment, or pseudorandom algorithms turn out to be predictable with enough information.

The challenge is generating randomness that no single party can control, predict, or manipulate. Arcium network solves this by distributing random generation across multiple parties who don't trust each other - no single party can influence the outcome, but together they generate randomness no one can predict or bias.

## How It Works

The coinflip follows this flow:

1. **Player commitment**: The player's choice (heads/tails) is encrypted and submitted to the network
2. **Random generation**: Multiple nodes in Arcium network work together to generate a random boolean outcome
3. **Encrypted comparison**: The system compares the encrypted choice against the encrypted random result
4. **Result disclosure**: Only the win/loss outcome is revealed, maintaining privacy of intermediate values

No single party can view or manipulate the random generation process. The comparison occurs on encrypted values, preventing result manipulation.

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

The player's choice is encrypted as a boolean (technically `Enc<Shared, bool>` in the code), allowing result verification without exposing the choice prematurely. Random generation uses Arcium's cryptographic primitives, where multiple nodes contribute entropy that no single node can predict or control.

Key properties:
- **Unpredictability**: No single party can predict the random outcome before generation
- **Unbiasability**: No single party can influence the random outcome toward a desired result
- **Integrity**: Arcium's maliciously secure protocol is designed to prevent manipulation even if some nodes act dishonestly

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
1. Multiple MPC nodes each generate local random values
2. Nodes combine their randomness using secure multi-party computation
3. Final random value is deterministic given all inputs
4. **No single node can predict the result** before all contribute
5. **No subset of nodes can bias the outcome** (requires only 1 honest node)

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
- **Dishonest majority security**: Need to work even if most nodes are malicious
