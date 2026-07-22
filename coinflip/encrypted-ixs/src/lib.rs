//! Coinflip circuit: draws an MPC random boolean and compares it against the
//! player's encrypted guess. Only the win/loss bit is revealed directly; combined
//! with their guess, it lets the player infer the toss. See README.md for the flow.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// The player's guess: true for heads, false for tails.
    pub struct UserChoice {
        pub choice: bool,
    }

    /// Flips a coin inside MPC and reveals only whether the player guessed right.
    #[instruction]
    pub fn flip(input_ctxt: Enc<Shared, UserChoice>) -> bool {
        let input = input_ctxt.to_arcis();

        let toss = ArcisRNG::bool();

        // Reveal only the comparison; neither operand is revealed directly.
        (input.choice == toss).reveal()
    }
}
