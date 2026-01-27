use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    // Consider 0 - Rock, 1 - Paper, 2 - Scissors
    pub struct PlayerMove {
        player_move: u8,
    }

    #[instruction]
    pub fn play_rps(player_move_ctxt: Enc<Shared, PlayerMove>) -> u8 {
        let player_move = player_move_ctxt.to_arcis();

        // Sample a near-uniform house move in {0,1,2} using rejection sampling over 2 random bits.
        // 00 -> 0, 01 -> 1, 10 -> 2, 11 -> reject and resample.
        // Fixed iterations bound runtime; the fallback introduces negligible bias (<= (1/4)^16).
        let mut house_move: u8 = 0;
        let mut selected = false;

        for _ in 0..16 {
            let b0 = ArcisRNG::bool();
            let b1 = ArcisRNG::bool();

            // Map (b0,b1) to 0..3 uniformly.
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

        // 0 - tie, 1 - player wins, 2 - house wins, 3 - invalid move
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
