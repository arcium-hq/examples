use arcis_imports::*;

#[instruction]
mod circuits {
    use arcis_imports::*;

    pub struct GameMoves {
        player_move: u8, // 0 = rock, 1 = paper, 2 = scissors
        house_move: u8,
    }

    // Arcomorphic Encryption (Homomorphic Encryption à la Arcium)
    #[encrypted]
    pub fn compare_moves(input_ctxt: Enc<ClientCipher, GameMoves>) -> u8 {
        let input = input_ctxt.decrypt();
        let mut result = 1;  // Default to player win
        
        // 0 = tie, 1 = player wins, 2 = house wins
        if input.player_move == input.house_move {
            result = 0;  // tie
        } else if (input.player_move == 0 && input.house_move == 1) ||    // rock vs paper
                  (input.player_move == 1 && input.house_move == 2) ||    // paper vs scissors
                  (input.player_move == 2 && input.house_move == 0) {     // scissors vs rock
            result = 2;  // house wins
        }
        
        result.reveal()
    }
}
