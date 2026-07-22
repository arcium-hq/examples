//! Re-encryption (sealing) circuit: hands a patient's encrypted medical record
//! to a new owner. The record is decrypted only inside MPC and re-encrypted
//! under the receiver's public key; no plaintext ever leaves the computation.
//! See README.md for the full flow.

use arcis::*;

#[encrypted]
mod circuits {
    use arcis::*;

    /// A patient's medical record, encrypted as eleven field ciphertexts.
    pub struct PatientData {
        pub patient_id: u64,
        pub age: u8,
        pub gender: bool,
        pub blood_type: u8,
        pub weight: u16,
        pub height: u16,
        pub allergies: [bool; 5],
    }

    /// Re-encrypts the patient's record for `receiver`. Reveals nothing:
    /// the output is ciphertext only the receiver can decrypt.
    #[instruction]
    pub fn share_patient_data(
        receiver: Shared,
        input_ctxt: Enc<Shared, PatientData>,
    ) -> Enc<Shared, PatientData> {
        let input = input_ctxt.to_arcis();
        receiver.from_arcis(input)
    }
}
