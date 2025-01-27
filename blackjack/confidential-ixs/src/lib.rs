use arcis::prelude::*;

arcis_linker!();

// Struct to hold encrypted hand of 2 cards
// We consider deck of cards to be 0-51 (52 cards)
// where order of suits is Spades, Hearts, Diamonds, Clubs
#[derive(ArcisObject, Copy, Clone)]
struct EncryptedHand {
    first_card: mu8,
    second_card: mu8,
}

#[confidential]
pub fn setup_blackjack_game(
    seed: mu8,
    player_hand: &mut EncryptedHand,
    dealer_hand: &mut EncryptedHand,
) {
    player_hand.first_card = (seed + 0) % 52;
    player_hand.second_card = (seed + 1) % 52;
    dealer_hand.first_card = (seed + 2) % 52;
    dealer_hand.second_card = (seed + 3) % 52;
}

#[confidential]
pub fn reveal_dealer_card(dealer_hand: &mut EncryptedHand, card_index: u8) -> u8 {
    arcis! {
        let card: mu8 = if card_index.eq(Into::<u8>::into(0)) {
            dealer_hand.first_card
        } else {
            dealer_hand.second_card
        };
    }
    card.reveal()
}
