use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    /// Represents the player's choice in the coin flip game.
    pub struct UserChoice {
        pub choice: bool, // Player's choice: true for heads, false for tails
    }

    /// Performs a confidential coin flip and compares it with the player's choice.
    ///
    /// This function generates a cryptographically secure random boolean value within
    /// the MPC environment and compares it with the player's encrypted choice.
    /// The comparison result (win/lose) is revealed while keeping both the player's
    /// choice and the actual coin flip result confidential.
    ///
    /// # Arguments
    /// * `input_ctxt` - Player's encrypted choice (heads or tails)
    ///
    /// # Returns
    /// * `true` if the player's choice matches the coin flip (player wins)
    /// * `false` if the player's choice doesn't match (player loses)
    #[instruction]
    pub fn flip(input_ctxt: Enc<Shared, UserChoice>) -> bool {
        let input = input_ctxt.to_arcis();

        // Generate a cryptographically secure random boolean (the coin flip)
        let toss = ArcisRNG::bool();

        // Compare player's choice with the coin flip result and reveal only the outcome
        (input.choice == toss).reveal()
    }
}
