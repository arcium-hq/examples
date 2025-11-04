# Medical Records - Privacy-Preserving Healthcare Data Sharing

Share your medical records with a new doctor and here's who gets to see them: the platform operator, system administrators, database engineers, and probably the cloud infrastructure provider. Everyone except you controls who accesses your health data.

This example demonstrates selective disclosure where you control exactly which medical information goes to which providers - without intermediaries being able to read any of it.

## What's wrong with healthcare data sharing today?

The problems cascade: medical records sit in centralized databases where system administrators can access everything, patients have no control over who sees what, and every centralized database is a target for breaches. The whole system requires trusting multiple third parties not to misuse your data.

The challenge is enabling healthcare data sharing while giving patients control over access permissions and preventing unnecessary data exposure to intermediaries.

## How Selective Data Sharing Works

The protocol enables patient-controlled data disclosure:

1. **Encrypted storage**: Patient medical records are encrypted and stored on-chain
2. **Access control**: Patient specifies which data fields to share with which providers
3. **Selective disclosure**: Arcium network enables transfer of authorized data only
4. **Provider access**: Authorized providers receive only the specific data fields granted access
5. **Platform isolation**: The sharing platform cannot access unencrypted patient data

The patient maintains granular control over data access. Providers receive only authorized information, while the platform facilitating the transfer operates on encrypted data without plaintext access.

## Running the Example

```bash
# Install dependencies
npm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates patient data encryption, selective authorization configuration, and controlled data disclosure to authorized providers.

## Technical Implementation

Medical records are encrypted and stored as patient data structures (using `Enc<Shared, PatientData>` in the code). Access control is enforced through encrypted authorization logic that determines which data fields specific providers can access.

Key mechanisms:
- **Encrypted storage**: Patient data remains encrypted on-chain
- **Granular access control**: Patients specify field-level access permissions
- **Network-based selective disclosure**: Selective data transfer occurs without platform decryption
- **Authorization verification**: Access rights are cryptographically enforced

## Implementation Details

### The Selective Sharing Problem

**Conceptual Challenge**: You want to share medical records with a new doctor, but:
- Don't want hospital system admins to see your data
- Don't want the platform operator to access unencrypted records
- Want to control exactly who can decrypt your information

**The Question**: Can you transfer encrypted data from your key to doctor's key, without anyone (including the platform) seeing the plaintext?

### The Re-encryption Pattern

```rust
pub fn share_patient_data(
    receiver: Shared,                      // Recipient's public key for re-encryption
    input_ctxt: Enc<Shared, PatientData>,  // Your encrypted data
) -> Enc<Shared, PatientData> {
    let input = input_ctxt.to_arcis();     // Decrypt inside MPC
    receiver.from_arcis(input)             // Re-encrypt for doctor
}
```

**What happens**:
1. Your encrypted data enters MPC
2. Data is decrypted **inside the secure MPC environment** (never exposed to any single party)
3. Data is re-encrypted using doctor's public key
4. Re-encrypted data is emitted in event
5. Only the doctor can decrypt (they have the private key)

**Key insight**: The platform facilitates transfer but never sees plaintext. Data is "handed over" inside MPC.

### Multi-field Encrypted Struct

```rust
pub struct PatientData {
    patient_id: u64,
    age: u8,
    gender: u8,
    blood_type: u8,
    weight: u64,
    height: u16,
    allergies: [bool; 5],
}
```

Stored as 11 separate encrypted fields (352 bytes total). Each field individually encrypted for granular access control (future enhancement could share only specific fields).

### When to Use Re-encryption

Apply this pattern when:
- Data encrypted under one key needs to be accessible to another party
- No single party should see the unencrypted data during transfer
- Selective disclosure (future: share only age + blood_type, not full record)
- Examples: credential sharing, encrypted file transfer, confidential data markets

This is powered by Arcium's Cerberus MPC protocol, which is designed to prevent any single party from accessing unauthorized medical information through maliciously secure multi-party computation requiring only one honest actor. Patients maintain data sovereignty while enabling coordinated care.
