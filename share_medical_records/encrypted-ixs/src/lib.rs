use arcis_imports::*;

#[encrypted]
mod circuits {
    use arcis_imports::*;

    pub struct PatientData {
        pub patient_id: u64,
        pub age: u8,
        pub gender: bool,
        pub blood_type: u8,
        pub weight: u16,
        pub height: u16,
        pub allergies: [bool; 5],
    }

    #[instruction]
    pub fn share_patient_data(
        input_ctxt: Enc<Client, PatientData>,
        receiver: Client,
    ) -> Enc<Client, PatientData> {
        let input = input_ctxt.to_arcis();
        receiver.from_arcis(input)
    }
}
