# Sealed-Bid Auction - Private Bids, Fair Outcomes

Traditional auction platforms see all bids. Even "sealed" bids are only sealed from other bidders -- the platform sees everything. This example keeps bid amounts encrypted throughout the auction. Only the final winner and payment amount are revealed.

## How It Works

1. Authority creates an auction with type (first-price or Vickrey), minimum bid, and duration (end time computed on-chain)
2. Bidders encrypt their bids locally and submit to the network
3. Arcium nodes compare each new bid against encrypted auction state without decrypting
4. Highest and second-highest bids are tracked in encrypted form on-chain
5. After the auction closes, only the winner and payment amount are revealed

## Implementation

### Encrypted Auction State

```rust
pub struct AuctionState {
    pub highest_bid: u64,
    pub highest_bidder: SerializedSolanaPublicKey,
    pub second_highest_bid: u64,
    pub bid_count: u16,
}
```

`SerializedSolanaPublicKey` splits a 32-byte pubkey into lo/hi u128 pairs to fit Arcis field elements. On-chain storage is `[[u8; 32]; 5]` -- five ciphertexts (the bidder pubkey occupies two).

> [Arcis Types](https://docs.arcium.com/developers/arcis/types)

### Bid Comparison

```rust
pub fn place_bid(
    bid_ctxt: Enc<Shared, Bid>,
    state_ctxt: Enc<Mxe, AuctionState>,
) -> Enc<Mxe, AuctionState> {
    let bid = bid_ctxt.to_arcis();
    let mut state = state_ctxt.to_arcis();

    if bid.amount > state.highest_bid {
        state.second_highest_bid = state.highest_bid;
        state.highest_bid = bid.amount;
        state.highest_bidder = bid.bidder;
    } else if bid.amount > state.second_highest_bid {
        state.second_highest_bid = bid.amount;
    }

    state.bid_count += 1;
    state_ctxt.owner.from_arcis(state)
}
```

The comparison `bid.amount > state.highest_bid` happens inside MPC -- decrypted values never leave the secure environment.

### First-Price vs Vickrey

**First-price**: Winner pays their bid.

```rust
pub fn determine_winner_first_price(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
    let state = state_ctxt.to_arcis();
    AuctionResult {
        winner: state.highest_bidder,
        payment_amount: state.highest_bid,
    }.reveal()
}
```

**Vickrey (second-price)**: Winner pays the second-highest bid. Bidding your true valuation is the dominant strategy -- you can't benefit from bidding lower (you might lose) or higher (you'd overpay).

```rust
pub fn determine_winner_vickrey(state_ctxt: Enc<Mxe, AuctionState>) -> AuctionResult {
    let state = state_ctxt.to_arcis();
    AuctionResult {
        winner: state.highest_bidder,
        payment_amount: state.second_highest_bid,
    }.reveal()
}
```

## Known Limitations

**`min_bid` not enforced against encrypted bids.** On-chain validation is impossible (the bid is encrypted), and circuit-side validation would require passing `min_bid` as a plaintext argument. For production, pass `min_bid` into the circuit and compare before updating state.

**No per-bidder deduplication.** A bidder can submit multiple bids. This is non-exploitable: in Vickrey mode, duplicate bids can only increase the second-highest price (hurting the bidder). In first-price mode, the bidder always pays their highest bid regardless. `bid_count` reflects total bids, not unique bidders.
