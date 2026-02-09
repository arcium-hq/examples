use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    // Consider 0 - split, 1 - steal
    pub struct GameMoves {
        player_a_move: u8,
        player_b_move: u8,
    }

    #[instruction]
    pub fn init_game(mxe: Mxe) -> Enc<Mxe, GameMoves> {
        let game_moves = GameMoves {
            player_a_move: 2, // invalid move
            player_b_move: 2,
        };

        mxe.from_arcis(game_moves)
    }

    pub struct PlayerMove {
        player: u8,
        player_move: u8,
    }

    #[instruction]
    pub fn player_move(
        player_move_ctxt: Enc<Shared, PlayerMove>,
        game_ctxt: Enc<Mxe, GameMoves>,
    ) -> Enc<Mxe, GameMoves> {
        let player_move = player_move_ctxt.to_arcis();
        let mut game_moves = game_ctxt.to_arcis();

        // Check which player is moving, if the player hasn't played their move yet, and the move is valid
        if player_move.player == 0 && game_moves.player_a_move == 2 && player_move.player_move < 2 {
            game_moves.player_a_move = player_move.player_move;
        } else if player_move.player == 1
            && game_moves.player_b_move == 2
            && player_move.player_move < 2
        {
            game_moves.player_b_move = player_move.player_move;
        }

        game_ctxt.owner.from_arcis(game_moves)
    }

    #[instruction]
    pub fn compare_moves(game_ctxt: Enc<Mxe, GameMoves>) -> u8 {
        let game_moves = game_ctxt.to_arcis();

        // 0 - both players splits
        // 1 - player A steals
        // 2 - player B steals
        // 3 - both players steal
        // 4 - invalid move

        let mut result = 4;

        if game_moves.player_a_move == game_moves.player_b_move {
            if game_moves.player_a_move == 0 {
                //both split
                result = 0;
            } else if game_moves.player_a_move == 1 {
                // both steal
                result = 3;
            }
        } else if (game_moves.player_a_move == 1 && game_moves.player_b_move == 0) {
            result = 1; // Player A steals
        } else if (game_moves.player_a_move == 0 && game_moves.player_b_move == 1) {
            result = 2; // Player B steals
        }

        result.reveal()
    }
}
