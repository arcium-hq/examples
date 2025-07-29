use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    // Consider 0 - Rock, 1 - Paper, 2 - Scissors
    pub struct GameMoves {
        player_a_move: u8,
        player_b_move: u8,
    }

    #[instruction]
    pub fn init_game(mxe: Mxe) -> Enc<Mxe, GameMoves> {
        let game_moves = GameMoves {
            player_a_move: 3, // Moves are 0-2, so 3 is invalid
            player_b_move: 3, // Moves are 0-2, so 3 is invalid
        };

        mxe.from_arcis(game_moves)
    }

    pub struct PlayersMove {
        player: u8,
        player_move: u8,
    }

    #[instruction]
    pub fn player_move(
        players_move_ctxt: Enc<Shared, PlayersMove>,
        game_ctxt: Enc<Mxe, GameMoves>,
    ) -> Enc<Mxe, GameMoves> {
        let players_move = players_move_ctxt.to_arcis();
        let mut game_moves = game_ctxt.to_arcis();

        // Check which player is moving, if the player hasn't played their move yet, and the move is valid
        if players_move.player == 0 && game_moves.player_a_move == 3 && players_move.player_move < 3
        {
            game_moves.player_a_move = players_move.player_move;
        } else if players_move.player == 1
            && game_moves.player_b_move == 3
            && players_move.player_move < 3
        {
            game_moves.player_b_move = players_move.player_move;
        }

        game_ctxt.owner.from_arcis(game_moves)
    }

    #[instruction]
    pub fn compare_moves(game_ctxt: Enc<Mxe, GameMoves>) -> u8 {
        let game_moves = game_ctxt.to_arcis();

        // 0 - tie, 1 - player A wins, 2 - player B wins, 3 - invalid move
        let mut result = 3;

        // If moves are the same, it's a tie
        if game_moves.player_a_move == game_moves.player_b_move {
            result = 0;
        } else if (game_moves.player_a_move == 0 && game_moves.player_b_move == 2) || // Rock beats Scissors
                  (game_moves.player_a_move == 1 && game_moves.player_b_move == 0) || // Paper beats Rock
                  (game_moves.player_a_move == 2 && game_moves.player_b_move == 1)
        // Scissors beats Paper
        {
            result = 1; // Player A wins
        } else {
            result = 2; // Player B wins
        }

        // If either player hasn't played their move yet, the result is invalid
        if game_moves.player_a_move == 3 || game_moves.player_b_move == 3 {
            result = 3;
        }

        result.reveal()
    }
}
