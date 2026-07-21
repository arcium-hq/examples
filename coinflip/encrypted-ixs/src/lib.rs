//! Coinflip circuit: draws an MPC random boolean and compares it against the
//! player's encrypted guess. Only the win/loss bit is revealed; the guess and
//! the toss stay secret. See README.md for the full flow.

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

        // Reveal only the comparison; the guess and the toss remain secret.
        (input.choice == toss).reveal()
    }
}
