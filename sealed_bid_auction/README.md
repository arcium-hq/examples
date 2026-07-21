# Sealed-Bid Auction — private bids, public winner

First-price and Vickrey (second-price) auctions where bid amounts stay encrypted end to end. MPC nodes compare each incoming bid against encrypted running state, and only the winner's public key and the payment amount are ever revealed — losing bids stay hidden from other bidders, the auctioneer, and the nodes themselves.

**Use this pattern when** you need to select a maximum (or ranking) over private inputs and reveal only the outcome: auctions, procurement, hiring, matching markets.

## How it works

1. `create_auction` stores the type (`FirstPrice` or `Vickrey`), `min_bid`, and `end_time` in the `Auction` PDA and queues the `init_auction_state` circuit, whose callback writes a zeroed, MXE-encrypted `AuctionState` into `encrypted_state`.
2. Each bidder encrypts `{bidder, amount}` client-side and calls `place_bid`. The `place_bid` circuit takes the bid (`Enc<Shared, Bid>`) plus the auction state read straight from the account bytes (`Enc<Mxe, AuctionState>`), updates the highest and second-highest bids inside MPC, and the callback writes the re-encrypted state and fresh `state_nonce` back.
3. After `end_time` the authority calls `close_auction`, then `determine_winner_first_price` or `determine_winner_vickrey` to match the auction type.
4. The winner circuit decrypts inside MPC and reveals only the winner's pubkey and the clearing price — the top bid in first-price mode, the second-highest bid in Vickrey mode — which the callback emits as `AuctionResolvedEvent`.

Only the winner and the price they pay become public; losing bids, and in Vickrey mode even the winning bid amount, stay encrypted forever.

## Concepts demonstrated

- Encrypted state in an ordinary Anchor account: `Auction.encrypted_state` holds five 32-byte ciphertexts that the cluster reads in place via `ArgBuilder`'s `.account(pubkey, offset, size)` — the byte layout lives in the program module header.
- `SerializedSolanaPublicKey`: a 32-byte Solana pubkey carried as lo/hi `u128` halves so it fits Arcis field elements ([public key types](https://docs.arcium.com/developers/arcis/types#public-key-types)).
- Two reveal policies over one encrypted state: `determine_winner_first_price` and `determine_winner_vickrey` read the same `AuctionState` but disclose different fields as the price.

## Run

```bash
yarn install && arcium build && arcium test
```

Setup and troubleshooting: [repo README](../README.md#running-an-example).

## Key files

- `encrypted-ixs/src/lib.rs` — note how `place_bid` keeps highest and second-highest with plain Rust comparisons; MPC evaluates both branches without learning which was taken.
- `programs/sealed_bid_auction/src/lib.rs` — note how `ENCRYPTED_STATE_OFFSET` pins the ciphertext location and how every state-mutating callback also persists the new `state_nonce`.
- `tests/sealed_bid_auction.ts` — note how `splitPubkeyToU128s` prepares the bidder key for encryption and how the test waits on the validator clock, not wall time, before `close_auction`.

## Pitfalls

- `place_bid`'s argument order is fixed by the circuit signature: bidder x25519 pubkey, bid nonce, the two bidder-pubkey ciphertexts, the amount ciphertext, then the state nonce and account reference. A reordering fails at decryption time, not at build time.
- The lifecycle guards are strict: `AuctionNotOpen` and `AuctionEnded` gate `place_bid`, `AuctionNotEnded` gates `close_auction`, and `AuctionNotClosed`, `WrongAuctionType`, and `NoBids` gate the winner instructions.
- If you reorder or resize `Auction` fields, recompute `ENCRYPTED_STATE_OFFSET`: the cluster reads ciphertexts by raw byte offset, and a stale offset hands the circuit garbage.

## Limitations

- `min_bid` is stored and emitted but never enforced: the program cannot compare an encrypted amount, and the circuit is never given `min_bid`. A production version would pass it to `place_bid` as a plaintext argument and reject low bids in-circuit.
- No per-bidder limit or deposit: `bid_count` counts bids, not bidders, and in Vickrey mode a bidder's own extra bid can raise the price they pay.
- One auction per authority: the `Auction` PDA is seeded only by the authority key, so a second `create_auction` from the same key fails.

See also: [invoking circuits with ArgBuilder](https://docs.arcium.com/developers/program) · **Next:** [Blackjack](../blackjack/)
