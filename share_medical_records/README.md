# Medical Records - Re-encryption

Share your medical records with a new doctor and here's the problem: the platform operator, system administrators, and cloud provider all have access. This example transfers encrypted records from one key to another inside MPC -- the platform never sees plaintext.

## How It Works

1. Patient stores encrypted medical data on-chain
2. Patient initiates a share, specifying the recipient's public key
3. Arcium nodes decrypt inside MPC and re-encrypt for the recipient
4. Re-encrypted data is emitted in an event -- only the recipient can decrypt

## Implementation

### The Re-encryption Pattern

```rust
pub fn share_patient_data(
    receiver: Shared,
    input_ctxt: Enc<Shared, PatientData>,
) -> Enc<Shared, PatientData> {
    let input = input_ctxt.to_arcis();
    receiver.from_arcis(input)
}
```

Data enters MPC encrypted under the patient's key and leaves encrypted under the doctor's key. No intermediate plaintext is exposed outside the MPC environment.

> [Input/Output Patterns](https://docs.arcium.com/developers/arcis/input-output)

### Multi-field Encrypted Struct

```rust
pub struct PatientData {
    patient_id: u64,
    age: u8,
    gender: bool,
    blood_type: u8,
    weight: u16,
    height: u16,
    allergies: [bool; 5],
}
```

Stored on-chain as 11 encrypted fields (11 x 32 = 352 bytes). Currently the entire record is re-encrypted as a unit -- per-field selective disclosure would require separate circuits for each field combination.
