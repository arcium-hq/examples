use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    const INITIAL_DECK: [u8; 52] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

    // 1 << (i * 6) for i in 0..21
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

    pub struct Deck {
        pub card_one: u128,
        pub card_two: u128,
        pub card_three: u128,
    }

    impl Deck {
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

    pub struct InitialHandHidden {
        pub dealer_card_two: u8,
    }

    #[instruction]
    pub fn shuffle_and_deal_cards(
        mxe: Mxe,
        mxe_again: Mxe,
        client: Client,
    ) -> (
        Enc<Mxe, Deck>,                  // 16 + 32 x 3
        Enc<Mxe, InitialHandHidden>,     // 16 + 32 x 1
        Enc<Client, InitialHandVisible>, // 32 + 16 + 32 x 3
        (u8, u8),
    ) {
        let mut initial_deck = INITIAL_DECK;
        ArcisRNG::shuffle(&mut initial_deck);

        let deck = mxe.from_arcis(Deck::from_array(initial_deck));
        let initial_hand_hidden = mxe_again.from_arcis(InitialHandHidden {
            dealer_card_two: initial_deck[3],
        });

        // Cards are dealt clockwise, starting with the player
        let initial_hand_visible = client.from_arcis(InitialHandVisible {
            player_card_one: initial_deck[0],
            player_card_two: initial_deck[2],
            dealer_card_one: initial_deck[1],
        });

        (deck, initial_hand_hidden, initial_hand_visible, (2, 2))
    }

    // New function for player to hit (take another card)
    #[instruction]
    pub fn player_hit(
        deck_ctxt: Enc<Mxe, Deck>,
        client: Client,
        player_hand_size: u8,
    ) -> (
        Enc<Mxe, Deck>,  // Updated deck
        Enc<Client, u8>, // New card dealt to player
    ) {
        let deck = deck_ctxt.to_arcis();
        let deck_array = deck.to_array();

        // Calculate the index of the next card to deal (4 + player_hand_size)
        // 4 is the number of cards already dealt (2 player + 2 dealer)
        let card_index = 4 + player_hand_size as usize;

        // Get the next card from the deck
        let new_card = deck_array[card_index];

        // Return the updated deck and the new card
        (deck_ctxt, client.from_arcis(new_card))
    }

    // Function for player to stand (end turn)
    #[instruction]
    pub fn player_stand(deck_ctxt: Enc<Mxe, Deck>) -> Enc<Mxe, Deck> {
        // Simply return the deck unchanged
        // This is a placeholder for future logic that might be needed
        deck_ctxt
    }

    // Function for player to double down (double bet and take one more card)
    #[instruction]
    pub fn player_double_down(
        deck_ctxt: Enc<Mxe, Deck>,
        client: Client,
    ) -> (
        Enc<Mxe, Deck>,  // Updated deck
        Enc<Client, u8>, // New card dealt to player
    ) {
        let deck = deck_ctxt.to_arcis();
        let deck_array = deck.to_array();

        // For double down, we always deal exactly one more card
        // The index is 4 (2 player + 2 dealer) + 0 (no additional cards yet)
        let card_index = 4;

        // Get the next card from the deck
        let new_card = deck_array[card_index];

        // Return the updated deck and the new card
        (deck_ctxt, client.from_arcis(new_card))
    }

    // Function for dealer to play (reveal hole card and follow rules)
    #[instruction]
    pub fn dealer_play(
        deck_ctxt: Enc<Mxe, Deck>,
        client: Client,
        dealer_face_up_card: u8,
        dealer_face_down_card: u8,
    ) -> (
        Enc<Mxe, Deck>,       // Updated deck
        Enc<Client, [u8; 3]>, // Cards dealt to dealer (up to 3 cards)
    ) {
        let deck = deck_ctxt.to_arcis();
        let deck_array = deck.to_array();

        // For simplicity, we'll just reveal the hole card and deal one more card
        // In a real implementation, we would need to calculate hand value and follow dealer rules
        let start_index = 5; // 4 initial cards + 1 player card

        // Dealer's hand starts with the two initial cards and one more card
        let dealer_hand = [
            dealer_face_up_card,
            dealer_face_down_card,
            deck_array[start_index],
        ];

        // Return the updated deck and the dealer's hand
        (deck_ctxt, client.from_arcis(dealer_hand))
    }

    // Helper function to calculate the value of a hand
    // Takes a fixed array of 11 cards and a hand_length parameter
    fn calculate_hand_value(hand: &[u8; 11], hand_length: u8) -> u8 {
        let mut value = 0;
        let mut has_ace = false;

        // Process each card individually with conditional logic
        // Card 0
        let rank0 = if 0 < hand_length as usize {
            (hand[0] % 13) + 1
        } else {
            0
        };
        if 0 < hand_length as usize {
            if rank0 == 1 {
                value += 11;
                has_ace = true;
            } else if rank0 > 10 {
                value += 10;
            } else {
                value += rank0;
            }
        }

        // Card 1
        let rank1 = if 1 < hand_length as usize {
            (hand[1] % 13) + 1
        } else {
            0
        };
        if 1 < hand_length as usize {
            if rank1 == 1 {
                value += 11;
                has_ace = true;
            } else if rank1 > 10 {
                value += 10;
            } else {
                value += rank1;
            }
        }

        // Card 2
        let rank2 = if 2 < hand_length as usize {
            (hand[2] % 13) + 1
        } else {
            0
        };
        if 2 < hand_length as usize {
            if rank2 == 1 {
                value += 11;
                has_ace = true;
            } else if rank2 > 10 {
                value += 10;
            } else {
                value += rank2;
            }
        }

        // Card 3
        let rank3 = if 3 < hand_length as usize {
            (hand[3] % 13) + 1
        } else {
            0
        };
        if 3 < hand_length as usize {
            if rank3 == 1 {
                value += 11;
                has_ace = true;
            } else if rank3 > 10 {
                value += 10;
            } else {
                value += rank3;
            }
        }

        // Card 4
        let rank4 = if 4 < hand_length as usize {
            (hand[4] % 13) + 1
        } else {
            0
        };
        if 4 < hand_length as usize {
            if rank4 == 1 {
                value += 11;
                has_ace = true;
            } else if rank4 > 10 {
                value += 10;
            } else {
                value += rank4;
            }
        }

        // Card 5
        let rank5 = if 5 < hand_length as usize {
            (hand[5] % 13) + 1
        } else {
            0
        };
        if 5 < hand_length as usize {
            if rank5 == 1 {
                value += 11;
                has_ace = true;
            } else if rank5 > 10 {
                value += 10;
            } else {
                value += rank5;
            }
        }

        // Card 6
        let rank6 = if 6 < hand_length as usize {
            (hand[6] % 13) + 1
        } else {
            0
        };
        if 6 < hand_length as usize {
            if rank6 == 1 {
                value += 11;
                has_ace = true;
            } else if rank6 > 10 {
                value += 10;
            } else {
                value += rank6;
            }
        }

        // Card 7
        let rank7 = if 7 < hand_length as usize {
            (hand[7] % 13) + 1
        } else {
            0
        };
        if 7 < hand_length as usize {
            if rank7 == 1 {
                value += 11;
                has_ace = true;
            } else if rank7 > 10 {
                value += 10;
            } else {
                value += rank7;
            }
        }

        // Card 8
        let rank8 = if 8 < hand_length as usize {
            (hand[8] % 13) + 1
        } else {
            0
        };
        if 8 < hand_length as usize {
            if rank8 == 1 {
                value += 11;
                has_ace = true;
            } else if rank8 > 10 {
                value += 10;
            } else {
                value += rank8;
            }
        }

        // Card 9
        let rank9 = if 9 < hand_length as usize {
            (hand[9] % 13) + 1
        } else {
            0
        };
        if 9 < hand_length as usize {
            if rank9 == 1 {
                value += 11;
                has_ace = true;
            } else if rank9 > 10 {
                value += 10;
            } else {
                value += rank9;
            }
        }

        // Card 10
        let rank10 = if 10 < hand_length as usize {
            (hand[10] % 13) + 1
        } else {
            0
        };
        if 10 < hand_length as usize {
            if rank10 == 1 {
                value += 11;
                has_ace = true;
            } else if rank10 > 10 {
                value += 10;
            } else {
                value += rank10;
            }
        }

        // Adjust for aces if needed
        if value > 21 && has_ace {
            value -= 10;
        }

        value
    }

    // Function to resolve the game and determine the winner
    #[instruction]
    pub fn resolve_game(
        client: Client,
        player_hand: [u8; 11],
        dealer_hand: [u8; 11],
        player_hand_length: u8,
        dealer_hand_length: u8,
    ) -> Enc<Client, u8> {
        // Calculate hand values
        let player_value = calculate_hand_value(&player_hand, player_hand_length);
        let dealer_value = calculate_hand_value(&dealer_hand, dealer_hand_length);

        // Determine the winner
        // 0 = player busts (dealer wins)
        // 1 = dealer busts (player wins)
        // 2 = player wins
        // 3 = dealer wins
        // 4 = push (tie)
        let result = if player_value > 21 {
            0 // Player busts
        } else if dealer_value > 21 {
            1 // Dealer busts
        } else if player_value > dealer_value {
            2 // Player wins
        } else if dealer_value > player_value {
            3 // Dealer wins
        } else {
            4 // Push (tie)
        };

        client.from_arcis(result)
    }
}
