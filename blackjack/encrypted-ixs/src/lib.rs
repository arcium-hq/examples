//! Blackjack circuits: shuffle and deal an encrypted 52-card deck, apply player
//! actions and dealer play against it, and resolve the winner. The deck and the
//! dealer's hand stay encrypted to the MXE; the player learns only their own hand,
//! the dealer's face-up card, and revealed bust/result flags. See README.md.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// Standard 52-card deck represented as indices 0-51
    const INITIAL_DECK: [u8; 52] = [
        0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

    type Deck = Pack<[u8; 52]>;
    type Hand = Pack<[u8; 11]>;

    /// Shuffles the deck and deals two cards each. The deck and dealer hand are
    /// encrypted to the MXE; the player receives their hand and the dealer's face-up card.
    #[instruction]
    pub fn shuffle_and_deal_cards(
        client: Shared,
        client_again: Shared,
    ) -> (
        Enc<Mxe, Deck>,
        Enc<Mxe, Hand>,
        Enc<Shared, Hand>,
        Enc<Shared, u8>,
    ) {
        let mut initial_deck: [u8; 52] = INITIAL_DECK;
        ArcisRNG::shuffle(&mut initial_deck);

        let deck_packed: Deck = Pack::new(initial_deck);
        let deck = Mxe::get().from_arcis(deck_packed);

        // 53 marks an empty slot; only indices 0-51 count toward hand value.
        let mut dealer_cards = [53u8; 11];
        dealer_cards[0] = initial_deck[1];
        dealer_cards[1] = initial_deck[3];

        let dealer_hand = Mxe::get().from_arcis(Pack::new(dealer_cards));

        let mut player_cards = [53u8; 11];
        player_cards[0] = initial_deck[0];
        player_cards[1] = initial_deck[2];

        let player_hand = client.from_arcis(Pack::new(player_cards));

        (
            deck,
            dealer_hand,
            player_hand,
            client_again.from_arcis(initial_deck[1]),
        )
    }

    /// Draws the next card from the encrypted deck into the player's hand.
    /// Reveals only whether the player busted.
    #[instruction]
    pub fn player_hit(
        deck_ctxt: Enc<Mxe, Deck>,
        player_hand_ctxt: Enc<Shared, Hand>,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Shared, Hand>, bool) {
        let deck = deck_ctxt.to_arcis().unpack();

        let mut player_hand = player_hand_ctxt.to_arcis().unpack();

        let card_index = (player_hand_size + dealer_hand_size) as usize;
        player_hand[player_hand_size as usize] = deck[card_index];

        let is_bust = calculate_hand_value(&player_hand, player_hand_size + 1) > 21;

        (
            player_hand_ctxt.owner.from_arcis(Pack::new(player_hand)),
            is_bust.reveal(),
        )
    }

    /// Reveals only whether the player's final hand is a bust.
    #[instruction]
    pub fn player_stand(player_hand_ctxt: Enc<Shared, Hand>, player_hand_size: u8) -> bool {
        let player_hand = player_hand_ctxt.to_arcis().unpack();
        let value = calculate_hand_value(&player_hand, player_hand_size);
        (value > 21).reveal()
    }

    /// Same draw as `player_hit`; the program marks the player as stood afterward.
    /// Reveals only whether the player busted.
    #[instruction]
    pub fn player_double_down(
        deck_ctxt: Enc<Mxe, Deck>,
        player_hand_ctxt: Enc<Shared, Hand>,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Shared, Hand>, bool) {
        let deck = deck_ctxt.to_arcis().unpack();

        let mut player_hand = player_hand_ctxt.to_arcis().unpack();

        let card_index = (player_hand_size + dealer_hand_size) as usize;
        player_hand[player_hand_size as usize] = deck[card_index];

        let is_bust = calculate_hand_value(&player_hand, player_hand_size + 1) > 21;

        (
            player_hand_ctxt.owner.from_arcis(Pack::new(player_hand)),
            is_bust.reveal(),
        )
    }

    /// Dealer draws from the encrypted deck until reaching 17. Returns the hand
    /// re-encrypted to the MXE and to the player, and reveals only the final hand size.
    #[instruction]
    pub fn dealer_play(
        deck_ctxt: Enc<Mxe, Deck>,
        dealer_hand_ctxt: Enc<Mxe, Hand>,
        client: Shared,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Mxe, Hand>, Enc<Shared, Hand>, u8) {
        let deck_array = deck_ctxt.to_arcis().unpack();
        let mut dealer = dealer_hand_ctxt.to_arcis().unpack();
        let mut size = dealer_hand_size as usize;

        for _ in 0..9 {
            let val = calculate_hand_value(&dealer, size as u8);
            if val < 17 && size < 11 {
                let idx = player_hand_size as usize + size;
                dealer[size] = deck_array[idx];
                size += 1;
            }
        }

        (
            dealer_hand_ctxt.owner.from_arcis(Pack::new(dealer)),
            client.from_arcis(Pack::new(dealer)),
            (size as u8).reveal(),
        )
    }

    /// Standard blackjack hand value: aces count 11 then drop to 1 while the hand
    /// would bust; face cards count 10. Slots beyond `hand_length` are ignored.
    fn calculate_hand_value(hand: &[u8; 11], hand_length: u8) -> u8 {
        let mut value: u8 = 0;
        let mut ace_count: u8 = 0;

        for i in 0..11 {
            if i < hand_length as usize {
                let card = hand[i];
                if card <= 51 {
                    let rank = card % 13; // 0=Ace, 1=2, ..., 9=10, 10=J, 11=Q, 12=K
                    if rank == 0 {
                        value += 11;
                        ace_count += 1;
                    } else if rank <= 9 {
                        value += rank + 1;
                    } else {
                        value += 10;
                    }
                }
            }
        }

        for _ in 0..11 {
            if value > 21 && ace_count > 0 {
                value -= 10;
                ace_count -= 1;
            }
        }

        value
    }

    /// Compares final hands and reveals only a result code: 0 player bust,
    /// 1 dealer bust, 2 player wins, 3 dealer wins, 4 push.
    #[instruction]
    pub fn resolve_game(
        player_hand: Enc<Shared, Hand>,
        dealer_hand: Enc<Mxe, Hand>,
        player_hand_length: u8,
        dealer_hand_length: u8,
    ) -> u8 {
        let player_hand = player_hand.to_arcis().unpack();
        let dealer_hand = dealer_hand.to_arcis().unpack();

        let player_value = calculate_hand_value(&player_hand, player_hand_length);
        let dealer_value = calculate_hand_value(&dealer_hand, dealer_hand_length);

        let result = if player_value > 21 {
            0 // player bust
        } else if dealer_value > 21 {
            1 // dealer bust
        } else if player_value > dealer_value {
            2 // player wins
        } else if dealer_value > player_value {
            3 // dealer wins
        } else {
            4 // push
        };

        result.reveal()
    }
}
