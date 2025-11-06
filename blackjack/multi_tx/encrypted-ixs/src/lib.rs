use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    /// Standard 52-card deck represented as indices 0-51
    const INITIAL_DECK: [u8; 52] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

    /// Powers of 64 used for encoding cards into u128 values.
    /// Each card takes 6 bits (values 0-63), so we can pack multiple cards efficiently.
    /// This array contains 64^i for i in 0..21, allowing us to encode up to 21 cards per u128.
    const POWS_OF_SIXTY_FOUR: [u128; 21] = [
        1,
        64,
        4096,
        262144,
        16777216,
        1073741824,
        68719476736,
        4398046511104,
        281474976710656,
        18014398509481984,
        1152921504606846976,
        73786976294838206464,
        4722366482869645213696,
        302231454903657293676544,
        19342813113834066795298816,
        1237940039285380274899124224,
        79228162514264337593543950336,
        5070602400912917605986812821504,
        324518553658426726783156020576256,
        20769187434139310514121985316880384,
        1329227995784915872903807060280344576,
    ];

    /// Represents a full 52-card deck encoded into three u128 values for efficiency.
    ///
    /// Each card is represented by 6 bits (0-63 range), allowing us to pack:
    /// - Cards 0-20 in card_one (21 cards × 6 bits = 126 bits < 128 bits)
    /// - Cards 21-41 in card_two (21 cards × 6 bits = 126 bits < 128 bits)  
    /// - Cards 42-51 in card_three (10 cards × 6 bits = 60 bits < 128 bits)
    pub struct Deck {
        pub card_one: u128,
        pub card_two: u128,
        pub card_three: u128,
    }

    impl Deck {
        /// Converts a 52-card array into the packed Deck representation.
        /// Uses base-64 encoding where each card index is treated as a digit in base 64.
        pub fn from_array(array: [u8; 52]) -> Deck {
            let mut card_one = 0;
            for i in 0..21 {
                card_one += POWS_OF_SIXTY_FOUR[i] * array[i] as u128;
            }

            let mut card_two = 0;
            for i in 21..42 {
                card_two += POWS_OF_SIXTY_FOUR[i - 21] * array[i] as u128;
            }

            let mut card_three = 0;
            for i in 42..52 {
                card_three += POWS_OF_SIXTY_FOUR[i - 42] * array[i] as u128;
            }

            Deck {
                card_one,
                card_two,
                card_three,
            }
        }

        /// Converts the packed Deck representation back to a 52-card array.
        /// Reverses the base-64 encoding by extracting 6 bits at a time.
        fn to_array(&self) -> [u8; 52] {
            let mut card_one = self.card_one;
            let mut card_two = self.card_two;
            let mut card_three = self.card_three;

            let mut bytes = [0u8; 52];
            for i in 0..21 {
                bytes[i] = (card_one % 64) as u8;
                bytes[i + 21] = (card_two % 64) as u8;
                card_one >>= 6;
                card_two >>= 6;
            }

            for i in 42..52 {
                bytes[i] = (card_three % 64) as u8;
                card_three >>= 6;
            }

            bytes
        }
    }

    // Initial hand is 2 player cards and 2 dealer cards (1 face up, 1 face down)
    pub struct InitialHandVisible {
        pub player_card_one: u8,
        pub player_card_two: u8,
        pub dealer_card_one: u8,
    }

    pub struct Hand {
        pub cards: u128,
    }

    impl Hand {
        pub fn from_array(array: [u8; 11]) -> Hand {
            let mut cards = 0;
            for i in 0..11 {
                cards += POWS_OF_SIXTY_FOUR[i] * array[i] as u128;
            }

            Hand { cards }
        }

        fn to_array(&self) -> [u8; 11] {
            let mut cards = self.cards;

            let mut bytes = [0u8; 11];
            for i in 0..11 {
                bytes[i] = (cards % 64) as u8;
                cards >>= 6;
            }

            bytes
        }
    }

    #[instruction]
    pub fn shuffle_and_deal_cards(
        mxe: Mxe,
        mxe_again: Mxe,
        client: Shared,
        client_again: Shared,
    ) -> (
        Enc<Mxe, Deck>,    // 16 + 32 x 3
        Enc<Mxe, Hand>,    // 16 + 32
        Enc<Shared, Hand>, // 32 + 16 + 32
        Enc<Shared, u8>,   // 32 + 16 + 32
    ) {
        let mut initial_deck = INITIAL_DECK;
        ArcisRNG::shuffle(&mut initial_deck);

        let deck = mxe.from_arcis(Deck::from_array(initial_deck));

        let mut dealer_cards = [53; 11];
        dealer_cards[0] = initial_deck[1];
        dealer_cards[1] = initial_deck[3];

        let dealer_hand = mxe_again.from_arcis(Hand::from_array(dealer_cards));

        let mut player_cards = [53; 11];
        player_cards[0] = initial_deck[0];
        player_cards[1] = initial_deck[2];

        let player_hand = client.from_arcis(Hand::from_array(player_cards));

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
        let deck = deck_ctxt.to_arcis().to_array();

        let mut player_hand = player_hand_ctxt.to_arcis().to_array();

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

        let player_updated_hand_value = calculate_hand_value(&player_hand, player_hand_size + 1);

        (
            player_hand_ctxt
                .owner
                .from_arcis(Hand::from_array(player_hand)),
            is_bust.reveal(),
        )
    }

    // Returns true if the player has busted
    #[instruction]
    pub fn player_stand(player_hand_ctxt: Enc<Shared, Hand>, player_hand_size: u8) -> bool {
        let player_hand = player_hand_ctxt.to_arcis().to_array();
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
        let deck = deck_ctxt.to_arcis();
        let deck_array = deck.to_array();

        let mut player_hand = player_hand_ctxt.to_arcis().to_array();

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
                .from_arcis(Hand::from_array(player_hand)),
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
        let deck = deck_ctxt.to_arcis();
        let mut deck_array = deck.to_array();
        let mut dealer = dealer_hand_ctxt.to_arcis().to_array();
        let mut size = dealer_hand_size as usize;

        for i in 0..7 {
            let val = calculate_hand_value(&dealer, size as u8);
            if val < 17 {
                let idx = (player_hand_size as usize + size) as usize;
                dealer[size] = deck_array[idx];
                size += 1;
            }
        }

        (
            dealer_hand_ctxt.owner.from_arcis(Hand::from_array(dealer)),
            client.from_arcis(Hand::from_array(dealer)),
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
                (hand[i] % 13) // Card rank (0=Ace, 1-9=pip cards, 10-12=face cards)
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
        let player_hand = player_hand.to_arcis().to_array();
        let dealer_hand = dealer_hand.to_arcis().to_array();

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
