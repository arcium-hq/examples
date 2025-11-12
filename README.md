# Arcium Examples - Privacy-Preserving Applications

Applications built on public blockchains face a fundamental limitation: all computation is transparent. These examples demonstrate how to build applications that can compute on encrypted data while preserving privacy.

## Getting Started

For installation instructions and setup, see the [Installation Guide](https://docs.arcium.com/developers/installation).

## Examples

### Games

**[Rock Paper Scissors](./rock_paper_scissors/)** - Simultaneous move games with asynchronous submission where neither player can see the other's choice until both commit.

- **[Player vs Player](./rock_paper_scissors/against-player/)** - Two players make encrypted moves independently
- **[Player vs House](./rock_paper_scissors/against-house/)** - Player competes against provably fair randomized opponent

**[Blackjack](./blackjack/)** - Card games where deck state remains hidden from all players until cards are revealed.

### Privacy Applications

**[Medical Records Sharing](./share_medical_records/)** - Share healthcare data with patient-controlled access permissions.

**[Confidential Voting](./voting/)** - Anonymous ballot systems where individual votes remain private while aggregate results are public.

### Primitives

**[Coinflip](./coinflip/)** - Generate trustless randomness where no party can predict outcomes.

**[Ed25519 Signatures](./ed25519/)** - Distributed Ed25519 signing where private keys are split across MPC nodes and never exist in a single location.

## Getting Started with Examples

Each example includes complete source code, build instructions, and test suites. Start with any example that addresses your use case.

For questions and support, join the [Discord community](https://discord.com/invite/arcium).
