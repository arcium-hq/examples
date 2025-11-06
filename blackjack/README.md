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
- **Total size**: 52 × 32 bytes = **1,664 bytes**

**The Problem**: Solana's transaction size limit is **1,232 bytes**, but our encrypted deck is 1,664 bytes. Whether returning the generated deck from initialization or passing it in transactions to deal more cards, the deck won't fit in a single transaction.

### Deriving the Compression Solution

**Analysis**: Do we really need 8 bits per card?

- Cards range from 0 to 51 (52 total cards)
- 51 < 64 = 2^6
- We only need **6 bits** to represent any card (0-63 range, we use 0-51)

**The Math**:

- 52 cards × 6 bits per card = **312 bits total**
- A `u128` can store 128 bits
- 312 bits ÷ 128 bits = 2.4... → need **3 `u128` values**
- First two `u128`s: 21 cards each (21 × 6 = 126 bits)
- Third `u128`: 10 cards (10 × 6 = 60 bits)

**After encryption**:

- 3 `u128` values → 3 × 32 bytes = **96 bytes**
- **Savings**: 1,664 bytes → 96 bytes (94% reduction!)

### Base-64 Encoding Explained

**Why 6 bits = "base-64"?**

Normal data uses base-256 (8 bits): each position can be 0-255.

We're using base-64 (6 bits): each position can be 0-63.

**Encoding formula**:

```
value = card[0]×64^0 + card[1]×64^1 + card[2]×64^2 + ... + card[20]×64^20
```

**Example** - Encoding first 3 cards [2, 13, 51]:

```
card_one = 2×1 + 13×64 + 51×4096
         = 2 + 832 + 208,896
         = 209,730
```

This single `u128` (value 209,730) represents 3 cards compressed together.

**Decoding formula**:

```rust
card[i] = (value / 64^i) % 64
```

**Example** - Decoding card[1] from 209,730:

```
card[1] = (209,730 / 64) % 64
        = 3,276 % 64
        = 13  ✓
```

### Implementation: The Powers Lookup Table

```rust
const POWS_OF_SIXTY_FOUR: [u128; 21] = [
    1, 64, 4096, 262144, 16777216, ...  // 64^0, 64^1, 64^2, 64^3, ...
];
```

Pre-computing powers of 64 makes encoding/decoding efficient in MPC.

**Encoding** (from array to compressed):

```rust
let mut card_one: u128 = 0;
for i in 0..21 {
    card_one += POWS_OF_SIXTY_FOUR[i] * cards[i] as u128;
}
```

**Decoding** (from compressed to array):

```rust
let mut temp = card_one;
for i in 0..21 {
    cards[i] = (temp % 64) as u8;
    temp = temp / 64;
}
```

### Account Storage Structure

On-chain storage uses serialized byte arrays. The `Enc<Shared, u128>` values from MPC instructions are serialized to `[u8; 32]` format for account storage:

```rust
pub struct BlackjackGame {
    pub deck: [[u8; 32]; 3],      // 3 encrypted u128s (52 cards compressed)
    pub player_hand: [u8; 32],    // 1 encrypted u128 (max 11 cards compressed)
    pub dealer_hand: [u8; 32],    // 1 encrypted u128 (max 11 cards compressed)
    pub deck_nonce: u128,         // Nonce for deck encryption
    pub client_nonce: u128,       // Nonce for player hand
    pub dealer_nonce: u128,       // Nonce for dealer hand
    pub player_hand_size: u8,     // How many cards actually in player_hand
    pub dealer_hand_size: u8,     // How many cards actually in dealer_hand
    // ... other game state
}
```

**Why separate nonces?** Each encrypted value needs its own nonce for security.

**Why track hand sizes?** The compressed `u128` can hold up to 11 cards, but we need to know how many are actually present (could be 2, 3, 10, etc.).

### The Result

**Without compression**:

- Deck: 52 encrypted values (1,664 bytes)
- Player hand: 11 encrypted values (352 bytes)
- Dealer hand: 11 encrypted values (352 bytes)
- **Total**: 2,368 bytes ❌ (won't fit in Solana transaction or account efficiently)

**With compression**:

- Deck: 3 encrypted values (96 bytes)
- Player hand: 1 encrypted value (32 bytes)
- Dealer hand: 1 encrypted value (32 bytes)
- **Total**: 160 bytes ✓

**Additional benefits**:

- Fewer MPC operations (5 encryptions vs 74)
- Faster computation (less encrypted data to process)
- Lower costs (fewer computation units)

### When to Use This Pattern

Apply compression when:

- You have many small values (cards, pixels, flags, etc.)
- Values fit in less than 8 bits
- Transaction size is a constraint
- Values are processed together (e.g., entire deck shuffled at once)

Examples: game pieces, map tiles, bit flags, small integers, inventory items.
