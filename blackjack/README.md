# Blackjack - Hidden Game State

Physical blackjack naturally hides the dealer's hole card. Digital blackjack has a different problem: the card's value must be stored somewhere, and whoever stores it can peek. This example keeps cards encrypted until game rules require them to be revealed.

## How It Works

1. A 52-card deck is shuffled using Arcium's cryptographic randomness
2. The entire deck, player hand, and dealer hand remain encrypted throughout gameplay
3. Players view their own cards; the dealer's hole card stays hidden
4. Game actions (hit, stand, double down) are processed against encrypted state
5. At resolution, hands are compared inside MPC and only the winner is revealed

## Implementation

### The Size Problem and `Pack<T>`

Encrypting 52 cards individually produces 52 x 32 = 1,664 bytes -- exceeds Solana's 1,232-byte transaction limit. Arcis `Pack<T>` compresses byte arrays into fewer field elements:

- Each field element holds up to 26 bytes
- `Pack<[u8; 52]>` = 2 field elements = **64 bytes** (96% reduction)
- `Pack<[u8; 11]>` = 1 field element = **32 bytes** per hand

```rust
type Deck = Pack<[u8; 52]>;
type Hand = Pack<[u8; 11]>;

let deck_packed: Deck = Pack::new(initial_deck);
let deck = Mxe::get().from_arcis(deck_packed);
```

Accessing individual cards requires unpacking:

```rust
let deck_array = deck_ctxt.to_arcis().unpack();
let card = deck_array[index];
```

> [Best Practices](https://docs.arcium.com/developers/arcis/best-practices)

### Account Storage

Encrypted `Pack` values are stored as `[u8; 32]` ciphertexts:

```rust
pub struct BlackjackGame {
    pub deck: [[u8; 32]; 2],      // Pack<[u8; 52]> = 2 field elements
    pub player_hand: [u8; 32],    // Pack<[u8; 11]> = 1 field element
    pub dealer_hand: [u8; 32],    // Pack<[u8; 11]> = 1 field element
    pub deck_nonce: u128,
    pub client_nonce: u128,
    pub dealer_nonce: u128,
    pub player_hand_size: u8,
    pub dealer_hand_size: u8,
    // ... other game state
}
```

Each encrypted value needs its own nonce. Hand sizes are tracked separately because the packed array always holds 11 slots but only some contain real cards.

### Size Comparison

| | Without Pack | With Pack |
|---|---|---|
| Deck | 1,664 bytes (52 x 32) | 64 bytes (2 x 32) |
| Player hand | 352 bytes (11 x 32) | 32 bytes (1 x 32) |
| Dealer hand | 352 bytes (11 x 32) | 32 bytes (1 x 32) |
| **Total** | **2,368 bytes** | **128 bytes** |
