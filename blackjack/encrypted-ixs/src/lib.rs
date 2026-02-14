use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// Standard 52-card deck represented as indices 0-51
    const INITIAL_DECK: [u8; 52] = [
        0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
        24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51,
    ];

    type Deck = Pack<[u8; 52]>;
    type Hand = Pack<[u8; 11]>;

    #[instruction]
    pub fn shuffle_and_deal_cards(
        mxe: Mxe,
        mxe_again: Mxe,
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
        let deck = mxe.from_arcis(deck_packed);

        let mut dealer_cards = [53u8; 11];
        dealer_cards[0] = initial_deck[1];
        dealer_cards[1] = initial_deck[3];

        let dealer_hand = mxe_again.from_arcis(Pack::new(dealer_cards));

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

        let player_hand_value = calculate_hand_value(&player_hand, player_hand_size);

        let is_bust = player_hand_value > 21;

        let new_card = if !is_bust {
            let card_index = (player_hand_size + dealer_hand_size) as usize;

            // Get the next card from the deck
            deck[card_index]
        } else {
            53
        };

        player_hand[player_hand_size as usize] = new_card;

        (
            player_hand_ctxt
                .owner
                .from_arcis(Pack::new(player_hand)),
            is_bust.reveal(),
        )
    }

    // Returns true if the player has busted
    #[instruction]
    pub fn player_stand(player_hand_ctxt: Enc<Shared, Hand>, player_hand_size: u8) -> bool {
        let player_hand = player_hand_ctxt.to_arcis().unpack();
        let value = calculate_hand_value(&player_hand, player_hand_size);
        (value > 21).reveal()
    }

    // Returns true if the player has busted, if not, returns the new card
    #[instruction]
    pub fn player_double_down(
        deck_ctxt: Enc<Mxe, Deck>,
        player_hand_ctxt: Enc<Shared, Hand>,
        player_hand_size: u8,
        dealer_hand_size: u8,
    ) -> (Enc<Shared, Hand>, bool) {
        let deck_array = deck_ctxt.to_arcis().unpack();

        let mut player_hand = player_hand_ctxt.to_arcis().unpack();

        let player_hand_value = calculate_hand_value(&player_hand, player_hand_size);

        let is_bust = player_hand_value > 21;

        let new_card = if !is_bust {
            let card_index = (player_hand_size + dealer_hand_size) as usize;

            // Get the next card from the deck
            deck_array[card_index]
        } else {
            53
        };

        player_hand[player_hand_size as usize] = new_card;

        (
            player_hand_ctxt
                .owner
                .from_arcis(Pack::new(player_hand)),
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

        for _ in 0..7 {
            let val = calculate_hand_value(&dealer, size as u8);
            if val < 17 {
                let idx = (player_hand_size as usize + size) as usize;
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
        let mut value = 0;
        let mut has_ace = false;

        // Process each card in the hand
        for i in 0..11 {
            let rank = if i < hand_length as usize {
                hand[i] % 13 // Card rank (0=Ace, 1-9=pip cards, 10-12=face cards)
            } else {
                0
            };

            if i < hand_length as usize {
                if rank == 0 {
                    // Ace: start with value of 11
                    value += 11;
                    has_ace = true;
                } else if rank > 10 {
                    // Face cards (Jack, Queen, King): value of 10
                    value += 10;
                } else {
                    // Pip cards (2-10): face value (rank 1-9 becomes value 1-9)
                    value += rank;
                }
            }
        }

        // Convert Ace from 11 to 1 if hand would bust with 11
        if value > 21 && has_ace {
            value -= 10;
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
