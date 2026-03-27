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

    #[instruction]
    pub fn shuffle_and_deal_cards(
        client: Shared,
        client_again: Shared,
    ) -> (
        Enc<Mxe, Deck>,    // 16 + 32 x 2
        Enc<Mxe, Hand>,    // 16 + 32
        Enc<Shared, Hand>, // 32 + 16 + 32
        Enc<Shared, u8>,   // 32 + 16 + 32
    ) {
        let mut initial_deck: [u8; 52] = INITIAL_DECK;
        ArcisRNG::shuffle(&mut initial_deck);

        let deck_packed: Deck = Pack::new(initial_deck);
        let deck = Mxe::get().from_arcis(deck_packed);

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

    // Checks the player's current hand value; returns true if already bust (>21)
    #[instruction]
    pub fn player_stand(player_hand_ctxt: Enc<Shared, Hand>, player_hand_size: u8) -> bool {
        let player_hand = player_hand_ctxt.to_arcis().unpack();
        let value = calculate_hand_value(&player_hand, player_hand_size);
        (value > 21).reveal()
    }

    // Deals one card and forces stand; returns (updated hand, is_bust)
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

    // Function for dealer to play (reveal hole card and follow rules)
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

    /// Calculates the blackjack value of a hand according to standard rules.
    ///
    /// Card values: Ace = 1 or 11 (whichever is better), Face cards = 10, Others = face value.
    /// Aces are initially valued at 11, but automatically reduced to 1 if the hand would bust.
    ///
    /// # Arguments
    /// * `hand` - Array of up to 11 cards (more than enough for blackjack)
    /// * `hand_length` - Number of actual cards in the hand
    ///
    /// # Returns
    /// The total value of the hand (1-21, or >21 if busted)
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

    /// Determines the final winner of the blackjack game.
    ///
    /// Compares the final hand values according to blackjack rules and returns
    /// a numeric result indicating the outcome. Both hands are evaluated for busts
    /// and compared for the winner.
    ///
    /// # Returns
    /// * 0 = Player busts (dealer wins)
    /// * 1 = Dealer busts (player wins)
    /// * 2 = Player wins (higher value, no bust)
    /// * 3 = Dealer wins (higher value, no bust)
    /// * 4 = Push/tie (same value, no bust)
    #[instruction]
    pub fn resolve_game(
        player_hand: Enc<Shared, Hand>,
        dealer_hand: Enc<Mxe, Hand>,
        player_hand_length: u8,
        dealer_hand_length: u8,
    ) -> u8 {
        let player_hand = player_hand.to_arcis().unpack();
        let dealer_hand = dealer_hand.to_arcis().unpack();

        // Calculate final hand values
        let player_value = calculate_hand_value(&player_hand, player_hand_length);
        let dealer_value = calculate_hand_value(&dealer_hand, dealer_hand_length);

        // Apply blackjack rules to determine winner
        let result = if player_value > 21 {
            0 // Player busts - dealer wins automatically
        } else if dealer_value > 21 {
            1 // Dealer busts - player wins automatically
        } else if player_value > dealer_value {
            2 // Player has higher value without busting
        } else if dealer_value > player_value {
            3 // Dealer has higher value without busting
        } else {
            4 // Equal values - push (tie)
        };

        result.reveal()
    }
}
