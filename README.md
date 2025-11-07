# Arcium Examples - Privacy-Preserving Applications

Applications built on public blockchains face a fundamental limitation: all computation is transparent. These examples demonstrate how to build applications that can compute on encrypted data while preserving privacy.

## Getting Started

For installation instructions and setup, see the [Installation Guide](https://docs.arcium.com/developers/installation).

## Examples

### Games with Hidden Information

**[Coinflip](./coinflip/)** (Simple) - Implements cryptographically secure randomness generation where outcomes cannot be predicted or manipulated by any party. Players commit to their choice before the random result is computed.

**[Rock Paper Scissors](./rock_paper_scissors/)** (Simple) - Implements simultaneous move games where players commit to encrypted choices before any moves are revealed. Available in two variants:
- **[Player vs Player](./rock_paper_scissors/against-player/)** - Two players make asynchronous encrypted commitments
- **[Player vs House](./rock_paper_scissors/against-house/)** - Player competes against an on-chain algorithm with cryptographically secure randomness

**[Blackjack](./blackjack/)** (Complex) - Demonstrates card games where deck state remains hidden from all players until cards are revealed. The dealer, players, and platform cannot access card information before the appropriate game phase.

### Privacy and Governance

**[Medical Records Sharing](./share_medical_records/)** (Easy) - Demonstrates selective disclosure of healthcare data where patients control access permissions. Medical information can be shared with authorized providers without exposing data to intermediaries or platform operators.

**[Confidential Voting](./voting/)** (Medium) - Implements anonymous ballot systems where individual votes remain private while aggregate results are public.

## Getting Started with Examples

Each example includes complete source code, build instructions, and test suites. Start with any example that addresses your use case.

For questions and support, join the [Discord community](https://discord.com/invite/arcium). 
