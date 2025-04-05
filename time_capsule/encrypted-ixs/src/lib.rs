use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct ShareSecretToLife {
        pub secret: u16,
    }

    #[instruction]
    pub fn reveal_secret(
        input_ctxt: Enc<Client, ShareSecretToLife>,
        receiver: Client,
    ) -> Enc<Client, ShareSecretToLife> {
        let input = input_ctxt.to_arcis();
        receiver.from_arcis(input)
    }
}
