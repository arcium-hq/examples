# Blackjack - Hidden Game State

Physical blackjack naturally hides the dealer's hole card. Digital blackjack has a different problem: the card's value must be stored somewhere - whether that's on a game server, in a database, or in code. Trusting a server to "hide" it just means betting they won't peek.

This example shows how to implement blackjack where cards remain encrypted until they need to be revealed according to game rules.

## Why is hidden information hard in digital card games?

Physical blackjack maintains three types of hidden information: the dealer's hole card, undealt cards in the deck, and random shuffle order. In digital implementations, this information must be stored as data - whether on a server or on a public blockchain - where it becomes vulnerable to inspection or manipulation.

Blockchain implementations face an additional challenge: transparent state. If card data is stored on-chain unencrypted, all participants can view the dealer's hidden cards and remaining deck order, completely breaking the game.

## How Hidden Game State Works

At game initialization, a 52-card deck is shuffled using Arcium's cryptographic randomness. The entire deck, including player cards and the dealer's hole card, remains encrypted throughout gameplay.

Information disclosure follows game rules: players view their own cards and the dealer's face-up card. Game actions (hit, stand, double down) are processed against encrypted hand values. The dealer's hole card and undealt cards remain encrypted until game resolution.

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates a complete game flow: deck shuffling with secure randomness, dealing encrypted cards, processing game actions against hidden state, and verifying final game result.

## Technical Implementation

The deck is stored as encrypted values, with multiple cards packed together for efficiency. Game logic processes encrypted hand values without ever decrypting them, using Arcium's confidential instructions.

The system works through three key mechanisms: network-generated randomness creates unpredictable deck order, selective disclosure reveals only authorized information per game rules, and the MPC protocol ensures no party can manipulate game state or outcomes even with a dishonest majority—game integrity is preserved as long as one node is honest.

## Implementation Details

### The Encryption Size Problem

**Requirement**: Encrypt a deck of 52 playing cards for an on-chain Blackjack game.

**Naive Approach**: Store each card as a separate encrypted value.

- Each card can be represented as `u8` (values 0-51)
- 52 cards = 52 `u8` values
- After encryption: each `u8` becomes a 32-byte ciphertext
- **Total size**: 52 x 32 bytes = **1,664 bytes**

**The Problem**: Solana's transaction size limit is **1,232 bytes**, but our encrypted deck is 1,664 bytes. Whether returning the generated deck from initialization or passing it in transactions to deal more cards, the deck won't fit in a single transaction.

### The Solution: `Pack<T>`

Arcis provides `Pack<T>`, a built-in type that compresses byte arrays into fewer field elements for encryption. Instead of encrypting each card individually, `Pack` packs multiple bytes into a single field element.

**How it works**:

- Each field element holds up to 26 bytes (at 8 bits per byte)
- `Pack<[u8; 52]>` = 52 bytes / 26 bytes per element = **2 field elements**
- `Pack<[u8; 11]>` = 11 bytes / 26 bytes per element = **1 field element**

**Usage in the circuit**:

```rust
type Deck = Pack<[u8; 52]>;
type Hand = Pack<[u8; 11]>;

let deck_packed: Deck = Pack::new(initial_deck);
let deck = Mxe::get().from_arcis(deck_packed);
```

**After encryption**:

- 2 field elements -> 2 x 32 bytes = **64 bytes**
- **Savings**: 1,664 bytes -> 64 bytes (96% reduction)

Accessing individual cards requires unpacking first:

```rust
let deck_array = deck_ctxt.to_arcis().unpack();
let card = deck_array[index];
```

### Account Storage Structure

On-chain storage uses serialized byte arrays. Encrypted `Pack` values are stored as `[u8; 32]` ciphertexts:

```rust
pub struct BlackjackGame {
    pub deck: [[u8; 32]; 2],      // Pack<[u8; 52]> = 2 field elements
    pub player_hand: [u8; 32],    // Pack<[u8; 11]> = 1 field element
    pub dealer_hand: [u8; 32],    // Pack<[u8; 11]> = 1 field element
    pub deck_nonce: u128,         // Nonce for deck encryption
    pub client_nonce: u128,       // Nonce for player hand
    pub dealer_nonce: u128,       // Nonce for dealer hand
    pub player_hand_size: u8,     // How many cards actually in player_hand
    pub dealer_hand_size: u8,     // How many cards actually in dealer_hand
    // ... other game state
}
```

**Why separate nonces?** Each encrypted value needs its own nonce for security.

**Why track hand sizes?** The packed hand can hold up to 11 cards, but we need to know how many are actually present (could be 2, 3, 10, etc.).

### The Result

**Without packing**:

- Deck: 52 encrypted values (1,664 bytes)
- Player hand: 11 encrypted values (352 bytes)
- Dealer hand: 11 encrypted values (352 bytes)
- **Total**: 2,368 bytes (won't fit in Solana transaction or account efficiently)

**With `Pack<T>`**:

- Deck: 2 encrypted values (64 bytes)
- Player hand: 1 encrypted value (32 bytes)
- Dealer hand: 1 encrypted value (32 bytes)
- **Total**: 128 bytes

**Additional benefits**:

- Fewer MPC operations (4 encryptions vs 74)
- Faster computation (less encrypted data to process)
- Lower costs (fewer computation units)
- No manual encoding/decoding logic needed

> For more optimization techniques, see [Best Practices](https://docs.arcium.com/developers/arcis/best-practices).

### When to Use This Pattern

Use `Pack<T>` when you have arrays of small values that are processed together. Arcis handles the packing and unpacking automatically — define a type alias, and the framework manages compression.

Examples: game pieces, map tiles, bit flags, small integers, inventory items.
