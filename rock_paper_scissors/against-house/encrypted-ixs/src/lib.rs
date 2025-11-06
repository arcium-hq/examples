use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    // Consider 0 - Rock, 1 - Paper, 2 - Scissors
    pub struct PlayerMove {
        player_move: u8,
    }

    #[instruction]
    pub fn play_rps(player_move_ctxt: Enc<Shared, PlayerMove>) -> u8 {
        let player_move = player_move_ctxt.to_arcis();

        let first_bit = ArcisRNG::bool();
        let second_bit = ArcisRNG::bool();

        let house_move = if first_bit {
            if second_bit {
                0
            } else {
                2
            }
        } else if second_bit {
            1
        } else {
            0
        };

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
