# Sealed-Bid Auction

This example shows how to run a sealed-bid auction on Solana using Arcium for confidential bidding and winner determination.

## What it demonstrates

- Bids are submitted encrypted; the program never sees plaintext bid values.
- Auction state (highest bid, second-highest bid, bidder identity) is maintained inside an encrypted state blob.
- Two auction types are supported:
  - **First-price**: winner pays their bid.
  - **Vickrey**: winner pays the second-highest bid.

## Layout

- `programs/sealed_bid_auction`: Anchor program that manages auctions and queues MPC computations.
- `encrypted-ixs`: Arcis circuits implementing encrypted auction logic (init state, place bid, determine winner).
- `tests/sealed_bid_auction.ts`: End-to-end flow from creating an auction through bidding and winner resolution.

## Running the example

From this directory:

```bash
yarn install
arcium build
arcium test
```
