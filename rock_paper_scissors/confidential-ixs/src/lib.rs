use arcis::prelude::*;

arcis_linker!();

#[derive(ArcisObject, Copy, Clone)]
pub struct Player {
    pub choice: mu8, // 0: rock, 1: paper, 2: scissors
}

#[confidential]
pub fn commit_choice(choice: mu8, player: &mut Player) {
    player.choice = choice;
}

#[confidential]
pub fn decide_winner(player1: &Player, player2: &Player) -> u8 {
    // One option is to reveal both choices and then match.
    // Another is conditional reveal.
    //
    // Here, we use conditional reveal.

    let mut res: mu8 = 0.into();
    arcis! {
        res = if player1.choice.eq(player2.choice) {
            0u8 // draw
        } else {
            if player1.choice.eq(0u8) && player2.choice.eq(2u8) {
                1u8 // rock beats scissors
            } else if player1.choice.eq(1u8) && player2.choice.eq(0u8) {
                1u8 // paper beats rock
            } else if player1.choice.eq(2u8) && player2.choice.eq(1u8) {
                1u8 // scissors beats paper
            } else {
                2u8 // otherwise player2 wins
            }
        }
    }

    res.reveal()
}

// #[confidential]
// pub fn decide_winner_reveal(player1: &Player, player2: &Player) -> u8 {
//     // One option is to reveal both choices and then match.
//     // Another is conditional reveal.
//     //
//     // Here, we reveal both choices and then match.

//     let choice1 = player1.choice.reveal();
//     let choice2 = player2.choice.reveal();

//     let mut res = 0.into();

//     match (choice1, choice2) {
//         (0u8, 2u8) | (1u8, 0u8) | (2u8, 1u8) => res = 1u8, // player1 wins
//         (2u8, 0u8) | (0u8, 1u8) | (1u8, 2u8) => res = 2,   // player2 wins
//         _ => res = 0,                                      // draw
//     }
// }
