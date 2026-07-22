//! Ed25519 circuits: `sign_message` signs with the MXE's share-split key (the
//! private key never exists in one place), and `verify_signature` checks a
//! signature against an encrypted verifying key. See README.md for the flow.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// Signs the message with the MXE's distributed Ed25519 key and reveals the
    /// signature, which is standard and publicly verifiable.
    #[instruction]
    pub fn sign_message(message: [u8; 5]) -> ArcisEd25519Signature {
        let signature = MXESigningKey::sign(&message);
        signature.reveal()
    }

    /// Verifies the signature against an encrypted verifying key; only the
    /// observer can decrypt the boolean verdict. Message and signature are public.
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
