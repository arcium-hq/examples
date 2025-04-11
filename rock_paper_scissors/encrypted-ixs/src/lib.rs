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
            player_a_move: 0,
            player_b_move: 0,
        };

        mxe.from_arcis(game_moves)
    }

    pub struct PlayerMove {
        player: u8,
        player_move: u8,
    }

    #[instruction]
    pub fn player_move(
        player_move_ctxt: Enc<Client, PlayerMove>,
        game_ctxt: Enc<Mxe, GameMoves>,
    ) -> Enc<Mxe, GameMoves> {
        let player_move = player_move_ctxt.to_arcis();
        let mut game_moves = game_ctxt.to_arcis();

        if player_move.player == 0 {
            game_moves.player_a_move = player_move.player_move;
        } else {
            game_moves.player_b_move = player_move.player_move;
        }

        game_ctxt.owner.from_arcis(game_moves)
    }

    #[instruction]
    pub fn compare_moves(game_ctxt: Enc<Mxe, GameMoves>) -> u8 {
        let game_moves = game_ctxt.to_arcis();

        // 0 - tie, 1 - player A wins, 2 - player B wins
        let mut result = 0;

        // If moves are the same, it's a tie
        if game_moves.player_a_move == game_moves.player_b_move {
            result = 0;
        } else if (game_moves.player_a_move == 0 && game_moves.player_b_move == 2) || // Rock beats Scissors
                  (game_moves.player_a_move == 1 && game_moves.player_b_move == 0) || // Paper beats Rock
                  (game_moves.player_a_move == 2 && game_moves.player_b_move == 1)    // Scissors beats Paper
        {
            result = 1; // Player A wins
        } else {
            result = 2; // Player B wins
        }

        result.reveal()
    }
}
