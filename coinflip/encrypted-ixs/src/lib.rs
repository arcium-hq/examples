use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct UserChoice {
        pub choice: bool,
    }

    #[instruction]
    pub fn flip(input_ctxt: Enc<Client, UserChoice>) -> bool {
        let input = input_ctxt.to_arcis();

        let flip = ArcisRNG::bool();

        (input.choice == flip).reveal()
    }
}
