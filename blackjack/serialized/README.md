# Confidential Blackjack on Solana

This example demonstrates a fully confidential blackjack game implemented using Arcium's Multi-Party Computation network. Players can enjoy a complete blackjack experience while keeping all card information private throughout the game.

## How Blackjack Works

Blackjack is a card game where players try to get their hand value as close to 21 as possible without going over (busting). Card values are:

- Number cards (2-10): Face value
- Face cards (Jack, Queen, King): 10 points each  
- Aces: 1 or 11 points (whichever is better for the hand)

The player receives two cards initially and can choose to "hit" (take another card), "stand" (keep current hand), or "double down" (double the bet and take exactly one more card). The dealer follows fixed rules: hit on 16 or less, stand on 17 or more.

## Why Arcium is Essential

Traditional on-chain card games face a fundamental problem: blockchain transparency means all data is public. In blackjack, if card values were visible, players could see the dealer's hole card and upcoming cards in the deck, completely breaking the game's fairness.

Arcium solves this by:

- **Confidential Deck Shuffling**: The 52-card deck is shuffled using cryptographically secure randomness within MPC
- **Private Card Values**: Player and dealer hands remain encrypted throughout gameplay
- **Hidden Information**: Players can't see the dealer's hole card or future cards in the deck
- **Fair Gameplay**: Only necessary information is revealed (like whether a player busted) while maintaining game integrity

## Technical Implementation

### Deck Encoding Innovation

The most complex part of this implementation is efficiently storing a 52-card deck in encrypted form. The solution uses a clever base-64 encoding scheme:

- Each card is represented as a 6-bit value (0-63 range)
- Multiple cards are packed into u128 integers using powers of 64
- The full deck splits across three u128 values for storage efficiency
- Cards 0-20 go in the first u128, cards 21-41 in the second, cards 42-51 in the third

This encoding allows the entire shuffled deck to be stored and manipulated within MPC while remaining completely confidential.

### Game Flow

1. **Initialization**: Player creates a game session and the deck is shuffled in MPC
2. **Deal**: Initial cards are dealt (2 to player, 2 to dealer with 1 face up)  
3. **Player Turn**: Player can hit, stand, or double down based on their encrypted hand
4. **Dealer Turn**: Dealer follows standard rules within MPC computation
5. **Resolution**: Final hand comparison determines the winner

### MPC Operations

Each game action triggers a specific MPC computation:

- `shuffle_and_deal_cards`: Initial deck shuffle and card dealing
- `player_hit`: Drawing additional cards for the player
- `player_stand`: Checking if player's current hand is valid
- `player_double_down`: Taking exactly one more card with doubled stakes
- `dealer_play`: Dealer follows hitting rules until reaching 17+
- `resolve_game`: Final comparison to determine the winner

All computations maintain card confidentiality while revealing only the minimum information needed for gameplay.

## Project Structure

**In order to build this project, cargo will require access to the arcium registry where the arcium dependencies are published to.
This is done by editing the generated `.cargo/credentials.toml` file to the root of the project with the provided token.**

The project follows Arcium's standard structure:

- `programs/blackjack/` - Solana program handling game state and user interactions
- `encrypted-ixs/` - MPC computations for confidential card operations  
- `tests/` - Integration tests demonstrating complete game flows
- `app/` - Frontend application for playing the game

The confidential computations in `encrypted-ixs/` handle all card-related logic while the Solana program manages game sessions, player accounts, and state transitions.
