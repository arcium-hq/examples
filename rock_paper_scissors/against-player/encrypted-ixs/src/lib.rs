//! Circuits for two-player rock paper scissors. Game state lives in
//! `Enc<Mxe, GameMoves>`, so neither player can read the other's move;
//! only `compare_moves` reveals anything. See README.md for the full flow.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// Both players' moves (0 = Rock, 1 = Paper, 2 = Scissors);
    /// `3` marks an empty slot.
    pub struct GameMoves {
        player_a_move: u8,
        player_b_move: u8,
    }

    /// Creates game state with both slots empty. Reveals nothing.
    #[instruction]
    pub fn init_game() -> Enc<Mxe, GameMoves> {
        let game_moves = GameMoves {
            player_a_move: 3,
            player_b_move: 3,
        };

        Mxe::get().from_arcis(game_moves)
    }

    /// A player's submission: `player` selects the slot (0 = A, 1 = B),
    /// `player_move` is the move (0-2).
    pub struct PlayersMove {
        player: u8,
        player_move: u8,
    }

    /// Writes the move into its slot if the slot is empty and the move is
    /// valid; otherwise returns the state unchanged. Reveals nothing.
    #[instruction]
    pub fn player_move(
        players_move_ctxt: Enc<Shared, PlayersMove>,
        game_ctxt: Enc<Mxe, GameMoves>,
    ) -> Enc<Mxe, GameMoves> {
        let players_move = players_move_ctxt.to_arcis();
        let mut game_moves = game_ctxt.to_arcis();

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

    /// Reveals only the outcome: 0 tie, 1 player A wins, 2 player B wins,
    /// 3 at least one slot still empty. The moves themselves stay encrypted.
    #[instruction]
    pub fn compare_moves(game_ctxt: Enc<Mxe, GameMoves>) -> u8 {
        let game_moves = game_ctxt.to_arcis();

        let result = if game_moves.player_a_move == 3 || game_moves.player_b_move == 3 {
            3 // Invalid - at least one player hasn't moved
        } else if game_moves.player_a_move == game_moves.player_b_move {
            0 // Tie
        } else if (game_moves.player_a_move == 0 && game_moves.player_b_move == 2) || // Rock beats Scissors
                  (game_moves.player_a_move == 1 && game_moves.player_b_move == 0) || // Paper beats Rock
                  (game_moves.player_a_move == 2 && game_moves.player_b_move == 1)
        // Scissors beats Paper
        {
            1 // Player A wins
        } else {
            2 // Player B wins
        };

        result.reveal()
    }
}
