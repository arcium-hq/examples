use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    // Consider 0 - Rock, 1 - Paper, 2 - Scissors
    pub struct GameMoves {
        player_move: u8,
        house_move: u8,
    }

    #[instruction]
    pub fn compare_moves(game_ctxt: Enc<Client, GameMoves>) -> u8 {
        let game_moves = game_ctxt.to_arcis();

        // 0 - tie, 1 - player wins, 2 - house wins
        let mut result = 0;

        // If moves are the same, it's a tie
        if game_moves.player_move == game_moves.house_move {
            result = 0;
        } else if (game_moves.player_move == 0 && game_moves.house_move == 2) || // Rock beats Scissors
                  (game_moves.player_move == 1 && game_moves.house_move == 0) || // Paper beats Rock
                  (game_moves.player_move == 2 && game_moves.house_move == 1)
        // Scissors beats Paper
        {
            result = 1; // Player wins
        } else {
            result = 2; // House wins
        }

        result
    }
}
