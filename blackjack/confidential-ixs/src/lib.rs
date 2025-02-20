use arcis::prelude::*;
use crypto::*;

arcis_linker!();

#[derive(ArcisType, Copy, Clone, ArcisEncryptable)]
struct Card {
    rank: mu8, // 1-13 (Ace = 1, Jack = 11, Queen = 12, King = 13)
    suit: mu8, // 1-4 (Hearts = 1, Diamonds = 2, Clubs = 3, Spades = 4)
}

#[derive(ArcisType, Copy, Clone, ArcisEncryptable)]
struct Hand {
    cards: ArcisArray<Card, 5>, // Max 5 cards per hand
    num_cards: mu8,
}

#[confidential]
pub fn init_hand(nonce: u128) -> [Ciphertext; 11] {
    let cipher = RescueCipher::new_for_mxe();

    cipher.encrypt::<11, _>(
        Hand {
            cards: ArcisArray::new(
                [Card {
                    rank: 0.into(),
                    suit: 0.into(),
                }; 5],
            ),
            num_cards: 0.into(),
        },
        nonce,
    )
}

// Function to calculate hand value
#[confidential]
pub fn calculate_hand_value(hand: [Ciphertext; 11], nonce: u128) -> [Ciphertext; 1] {
    let cipher = RescueCipher::new_for_mxe();
    let hand = cipher.decrypt::<11, Hand>(hand, nonce);

    let mut value = mu8::from(0u8);
    let mut num_aces = mu8::from(0u8);

    for i in 0..5 {
        let card = hand.cards[i as usize];
        let rank = card.rank;

        arcis! {
            if rank == 1 {  // Ace
                num_aces += 1;
                value += 11;
            } else if rank >= 10 {  // Face cards
                value += 10;
            } else {
                value += rank;
            }
        }
    }

    arcis! {
        // If we're over 21 and have aces, convert them from 11 to 1
        let over_amount = if value > 21 {
            value - 21
        } else {
            0
        };

        let aces_to_convert = if (over_amount / 10) + 1 > num_aces {
            num_aces
        } else {
            (over_amount / 10) + 1
        };
    }

    value -= aces_to_convert * 10;

    cipher.encrypt::<1, _>(value, nonce)
}

// Function to add a card to hand
#[confidential]
pub fn add_card_to_hand(
    hand: [Ciphertext; 11],
    hand_nonce: u128,
    new_card: [Ciphertext; 2],
    public_key: PublicKey,
    nonce: u128,
) -> [Ciphertext; 11] {
    let hand_cipher = RescueCipher::new_for_mxe();
    let mut hand = hand_cipher.decrypt::<11, Hand>(hand, hand_nonce);
    let card_cipher = RescueCipher::new_with_client(public_key);
    let new_card = card_cipher.decrypt::<2, Card>(new_card, nonce);

    arcis! {
        if hand.num_cards < mu8::from(5) {
            for i in 0..5 {
                if hand.cards[i].rank == 0 && hand.cards[i].suit == 0 {
                    hand.cards[i] = new_card;
                    hand.num_cards += 1;
                }
            }
        }
    }

    hand_cipher.encrypt::<11, _>(hand, hand_nonce)
}

// Function to show a hand to the user
#[confidential]
pub fn show_player_hand_to_user(
    hand: [Ciphertext; 11],
    hand_nonce: u128,
    encryption_public_key: PublicKey,
    encryption_nonce: u128,
) -> [Ciphertext; 11] {
    let cipher = RescueCipher::new_for_mxe();
    let hand = cipher.decrypt::<11, Hand>(hand, hand_nonce);

    let encryption_cipher = RescueCipher::new_with_client(encryption_public_key);

    encryption_cipher.encrypt::<11, _>(hand, encryption_nonce)
}

// Function to reveal dealer hand
#[confidential]
pub fn reveal_dealer_hand(
    hand: [Ciphertext; 11],
    hand_nonce: u128,
) -> (u8, u8, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
    let cipher = RescueCipher::new_for_mxe();
    let hand = cipher.decrypt::<11, Hand>(hand, hand_nonce);

    (
        hand.cards[0].rank.reveal(),
        hand.cards[0].suit.reveal(),
        hand.cards[1].rank.reveal(),
        hand.cards[1].suit.reveal(),
        hand.cards[2].rank.reveal(),
        hand.cards[2].suit.reveal(),
        hand.cards[3].rank.reveal(),
        hand.cards[3].suit.reveal(),
        hand.cards[4].rank.reveal(),
        hand.cards[4].suit.reveal(),
        hand.num_cards.reveal(),
    )
}
