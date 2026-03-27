# Arcium Examples - Privacy-Preserving Applications

Applications built on public blockchains face a fundamental limitation: all computation is transparent. These examples demonstrate how to build applications that can compute on encrypted data while preserving privacy.

All examples use Arcium's MPC protocol, which preserves privacy even with a dishonest majority -- data remains confidential as long as one node is honest.

## Getting Started

For installation instructions and setup, see the [Installation Guide](https://docs.arcium.com/developers/installation).

Each example follows the same build/test flow:

```bash
yarn install
arcium build
arcium test
```

New to Arcium? Start with Coinflip and progress through the tiers. For conceptual background, see [Mental Model](https://docs.arcium.com/developers/arcis/mental-model).

## Examples

### Getting Started

**[Coinflip](./coinflip/)** - Trustless randomness using `ArcisRNG`. Stateless design, simplest example.

**[Rock Paper Scissors](./rock_paper_scissors/)** - Encrypted gameplay with hidden moves.
- [Player vs Player](./rock_paper_scissors/against-player/) - Two encrypted submissions, async commitment
- [Player vs House](./rock_paper_scissors/against-house/) - Provably fair randomized opponent

### Intermediate

**[Voting](./voting/)** - Private ballots with public results. Encrypted state accumulation and callbacks.

**[Medical Records](./share_medical_records/)** - Patient-controlled data sharing via re-encryption.

**[Sealed-Bid Auction](./sealed_bid_auction/)** - Encrypted bid comparison with first-price and Vickrey mechanisms.

### Advanced

**[Blackjack](./blackjack/)** - Hidden deck state with `Pack<T>` for efficient encrypted storage.

**[Ed25519 Signatures](./ed25519/)** - Distributed key management. Private keys never exist in single location.

## Documentation

- [Mental Model](https://docs.arcium.com/developers/arcis/mental-model) - Conceptual foundation
- [Computation Lifecycle](https://docs.arcium.com/developers/computation-lifecycle) - How MPC computations execute
- [Arcis Framework](https://docs.arcium.com/developers/arcis) - Programming model reference
- [Best Practices](https://docs.arcium.com/developers/arcis/best-practices) - Performance optimization

For questions and support, join the [Discord community](https://discord.com/invite/arcium).
