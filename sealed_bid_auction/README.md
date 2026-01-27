# Sealed-Bid Auction - Private Bids, Fair Outcomes

Traditional auction platforms have access to all bids. Even "sealed" bids are only sealed from other bidders - the platform sees everything. This creates opportunities for bid manipulation, information leakage, and requires trusting the auctioneer not to exploit their privileged position.

This example demonstrates sealed-bid auctions where bid amounts remain encrypted throughout the auction. The platform never sees individual bid values - only the final winner and payment amount are revealed.

## Why are sealed-bid auctions hard?

Transparent blockchain architectures conflict with bid privacy requirements:

1. **Bid visibility**: All blockchain data is publicly accessible by default - competitors see your bid
2. **Strategic manipulation**: Visible bids enable last-minute sniping and bid shading
3. **Platform trust**: Traditional sealed-bid auctions require trusting the auctioneer to not peek at bids or collude with bidders
4. **Winner determination**: Computing the highest bid without revealing all bid amounts is non-trivial
5. **Pricing mechanisms**: Supporting different auction types (first-price vs Vickrey) requires tracking both highest and second-highest bids privately

The requirement is determining the auction winner and payment amount without revealing individual bids, while ensuring the process is verifiable and tamper-resistant.

## How Sealed-Bid Auctions Work

The protocol maintains bid privacy while providing accurate winner determination:

1. **Auction creation**: Authority creates an auction specifying the type (first-price or Vickrey), minimum bid, and end time
2. **Bid encryption**: Bidders encrypt their bid amounts locally before submission using the MXE public key
3. **Encrypted comparison**: Arcium nodes compare new bids against the encrypted auction state without decrypting
4. **State update**: Highest and second-highest bids are tracked in encrypted form on-chain
5. **Winner revelation**: After the auction closes, only the winner identity and payment amount are revealed - not individual bid values
6. **Security guarantee**: Arcium's MPC protocol ensures auction integrity even with a dishonest majority - bid values remain private as long as one node is honest

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates complete auction flows for both first-price and Vickrey auction types, including auction creation, encrypted bid submission, and winner determination.

## Technical Implementation

Bids are encrypted using X25519 key exchange with the MXE public key before submission. The auction state stores five encrypted values on-chain: highest bid, highest bidder (split into two u128s), second-highest bid, and bid count.

Key properties:

- **Bid secrecy**: Individual bid amounts remain encrypted throughout the auction lifecycle
- **Distributed computation**: Arcium nodes jointly compare and update encrypted auction state
- **Selective revelation**: Only the winner and payment amount are revealed, not the losing bids

## Implementation Details

### The Sealed Bid Problem

**Conceptual Challenge**: How do you find the maximum value in a set without revealing any individual values?

Traditional approaches all fail:

- **Encrypt then decrypt**: Someone holds the decryption key and can see all bids
- **Trusted auctioneer**: Requires trusting the platform not to leak or exploit bid information
- **Commit-reveal**: Bidders can see others' bids before revealing, enabling strategic behavior

**The Question**: Can we compare encrypted bids and track the highest one without ever decrypting individual values?

### The Encrypted Auction State Pattern

Sealed-bid auction demonstrates storing encrypted comparison state in Anchor accounts:

```rust
pub struct AuctionState {
    pub highest_bid: u64,
    pub highest_bidder_lo: u128,  // Lower 128 bits of winner pubkey
    pub highest_bidder_hi: u128,  // Upper 128 bits of winner pubkey
    pub second_highest_bid: u64,  // Required for Vickrey auctions
    pub bid_count: u8,
}
```

**Why split the bidder identity?** Solana public keys are 32 bytes, but Arcis encrypts each primitive separately. Splitting into two u128s (16 bytes each) allows efficient encrypted storage and comparison.

**On-chain storage**: The encrypted state is stored as `[[u8; 32]; 5]` - five 32-byte ciphertexts representing each field.

> Learn more about [Arcis Types](https://docs.arcium.com/developers/arcis/types) for encrypted value handling.

### The Bid Comparison Logic

**MPC instruction** (runs inside encrypted computation):

```rust
pub fn place_bid(
    bid_ctxt: Enc<Shared, Bid>,       // Bidder's encrypted bid
    state_ctxt: Enc<Mxe, AuctionState>, // Current encrypted auction state
) -> Enc<Mxe, AuctionState> {
    let bid = bid_ctxt.to_arcis();      // Decrypt in MPC (never exposed)
    let mut state = state_ctxt.to_arcis();

    if bid.amount > state.highest_bid {
        // New highest bid - shift current highest to second place
        state.second_highest_bid = state.highest_bid;
        state.highest_bid = bid.amount;
        state.highest_bidder_lo = bid.bidder_lo;
        state.highest_bidder_hi = bid.bidder_hi;
    } else if bid.amount > state.second_highest_bid {
        // New second-highest bid
        state.second_highest_bid = bid.amount;
    }

    state.bid_count += 1;
    state_ctxt.owner.from_arcis(state)  // Re-encrypt updated state
}
```

**Key insight**: The comparison `bid.amount > state.highest_bid` happens inside MPC - decrypted values never leave the secure environment.

### First-Price vs Vickrey Auctions

This example supports two auction mechanisms with different economic properties:

**First-price auction**: Winner pays their bid amount.

```rust
pub fn determine_winner_first_price(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
    let state = state_ctxt.to_arcis();
    AuctionResult {
        winner_lo: state.highest_bidder_lo,
        winner_hi: state.highest_bidder_hi,
        payment_amount: state.highest_bid,  // Pay your bid
    }.reveal()
}
```

**Vickrey (second-price) auction**: Winner pays the second-highest bid.

```rust
pub fn determine_winner_vickrey(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
    let state = state_ctxt.to_arcis();
    AuctionResult {
        winner_lo: state.highest_bidder_lo,
        winner_hi: state.highest_bidder_hi,
        payment_amount: state.second_highest_bid,  // Pay second-highest
    }.reveal()
}
```

**Why Vickrey matters**: In a Vickrey auction, bidding your true valuation is the dominant strategy - you can't benefit from bidding lower (you might lose) or higher (you'd overpay). This incentive-compatibility property, discovered by economist William Vickrey (Nobel Prize 1996), is widely used in ad auctions (Google, Meta) and spectrum auctions.

### When to Use This Pattern

Apply sealed-bid auctions when:

- **Bid privacy is critical**: Prevent competitors from seeing and reacting to bids
- **Strategic behavior is harmful**: Eliminate sniping, bid shading, and collusion
- **Verifiable fairness is required**: Prove the winner determination was correct without revealing losing bids
- **Multiple pricing mechanisms**: Need flexibility between first-price and second-price rules

**Example applications**: NFT auctions, ad bidding systems, procurement contracts, treasury bond auctions, spectrum license sales.

This pattern extends to any scenario requiring private comparison and selection: hiring decisions, grant allocations, or matching markets where selection criteria should remain confidential.
