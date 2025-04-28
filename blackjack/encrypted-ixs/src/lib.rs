use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    const INITIAL_DECK: [u8; 52] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51,
    ];

    // 1 << i * 6 for i in 0..21
    const FIRST_21_POWERS_OF_2_TIMES_6: [u128; 21] = [
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
                card_one += FIRST_21_POWERS_OF_2_TIMES_6[i] * array[i] as u128;
            }

            let mut card_two = 0;
            for i in 21..42 {
                card_two += FIRST_21_POWERS_OF_2_TIMES_6[i - 21] * array[i] as u128;
            }

            let mut card_three = 0;
            for i in 42..52 {
                card_three += FIRST_21_POWERS_OF_2_TIMES_6[i - 42] * array[i] as u128;
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
        Enc<Mxe, Deck>,
        Enc<Mxe, InitialHandHidden>,
        Enc<Client, InitialHandVisible>,
    ) {
        let mut initial_deck = INITIAL_DECK;
        ArcisRNG::shuffle(&mut initial_deck);

        let deck = mxe.from_arcis(Deck::from_array(initial_deck));
        let initial_hand_hidden = mxe_again.from_arcis(InitialHandHidden {
            dealer_card_two: initial_deck[3],
        });

        let initial_hand_visible = client.from_arcis(InitialHandVisible {
            player_card_one: initial_deck[0],
            player_card_two: initial_deck[1],
            dealer_card_one: initial_deck[2],
        });

        (deck, initial_hand_hidden, initial_hand_visible)
    }

    // #[instruction]
    // pub fn deal_cards(deck_ctxt: Enc<Mxe, Deck>) -> u8 {
    //     let deck = deck_ctxt.to_arcis();

    //     deck.to_array()[3].reveal()
    // }

    #[instruction]
    pub fn deal_cards(initial_hand_visible_ctxt: Enc<Client, InitialHandVisible>) -> (u8, u8, u8) {
        let initial_hand_visible = initial_hand_visible_ctxt.to_arcis();

        (
            initial_hand_visible.player_card_one.reveal(),
            initial_hand_visible.player_card_two.reveal(),
            initial_hand_visible.dealer_card_one.reveal(),
        )
    }
}
