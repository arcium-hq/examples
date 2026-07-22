//! Rock-paper-scissors circuit against a house move drawn inside MPC. The
//! player's move stays encrypted; the house move comes from `ArcisRNG` via
//! rejection sampling. Only the result code is revealed. See README.md.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// The player's move: 0 = rock, 1 = paper, 2 = scissors.
    pub struct PlayerMove {
        player_move: u8,
    }

    /// Draws a uniform random house move and reveals only the result code:
    /// 0 = tie, 1 = player wins, 2 = house wins, 3 = invalid move.
    #[instruction]
    pub fn play_rps(player_move_ctxt: Enc<Shared, PlayerMove>) -> u8 {
        let player_move = player_move_ctxt.to_arcis();

        let mut house_move: u8 = 0;
        let mut selected = false;

        // Two random bits give a candidate in 0..=3; 3 is rejected so the
        // three moves stay uniform (a modulo would bias rock to 50%). Circuits
        // cannot break, so all 16 rounds run and `selected` latches the first
        // accepted candidate.
        for _ in 0..16 {
            let b0 = ArcisRNG::bool();
            let b1 = ArcisRNG::bool();

            let candidate: u8 = if b0 {
                if b1 {
                    3
                } else {
                    2
                }
            } else if b1 {
                1
            } else {
                0
            };

            let candidate_valid = candidate < 3;
            let take = (!selected) & candidate_valid;

            house_move = if take { candidate } else { house_move };
            selected = selected | candidate_valid;
        }

        let result = if player_move.player_move > 2 {
            3
        } else if player_move.player_move == house_move {
            0
        } else if (player_move.player_move == 0 && house_move == 2) || // Rock beats Scissors
                  (player_move.player_move == 1 && house_move == 0) || // Paper beats Rock
                  (player_move.player_move == 2 && house_move == 1)
        // Scissors beats Paper
        {
            1
        } else {
            2
        };

        result.reveal()
    }
}
