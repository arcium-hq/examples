use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    #[instruction]
    pub fn sign_message(message: [u8; 5]) -> ArcisEd25519Signature {
        let signature = MXESigningKey::sign(&message);
        signature.reveal()
    }

    #[instruction]
    pub fn verify_signature(
        verifying_key_enc: Enc<Shared, Pack<VerifyingKey>>,
        message: [u8; 5],
        signature: [u8; 64],
        observer: Shared,
    ) -> Enc<Shared, bool> {
        let verifying_key = verifying_key_enc.to_arcis().unpack();
        let signature = ArcisEd25519Signature::from_bytes(signature);
        let is_valid = verifying_key.verify(&message, &signature);
        observer.from_arcis(is_valid)
    }
}
