use arcis_imports::*;
#[module]
mod circuits{
    use arcis_imports::*;

    pub struct InputValues {
        v1: u8,
        v2: u8,
    }

    // Arcomorphic Encryption (Homomorphic Encryption à la Arcium)
    #[circuit]
    pub fn add_together(input_ctxt: Ciphertext<ClientCipher, InputValues>) -> Ciphertext<ClientCipher, u16> {
        let input = input_ctxt.decrypt();
        let sum = input.v1 as u16 + input.v2 as u16;
        input_ctxt.cipher.encrypt(sum)
    }

}
