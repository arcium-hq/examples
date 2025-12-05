# Medical Records - Privacy-Preserving Healthcare Data Sharing

Share your medical records with a new doctor and here's the problem: the platform operator, system administrators, database engineers, and cloud infrastructure provider all have access. You can't control who sees what.

This example demonstrates patient-controlled selective disclosure where you specify exactly which medical information each provider can access.

## What's wrong with healthcare data sharing today?

Medical records in centralized databases face multiple problems: patients have no control over who sees what, and every centralized database is a target for breaches. The system requires trusting multiple third parties not to misuse your data.

The challenge is enabling healthcare data sharing while giving patients control over access permissions.

## How Selective Data Sharing Works

The protocol enables patient-controlled data disclosure:

1. **Encrypted storage**: Patient medical records are encrypted and stored on-chain
2. **Access control**: Patient specifies which data fields to share with which providers
3. **Selective disclosure**: Arcium network enables transfer of authorized data only
4. **Provider access**: Authorized providers receive only the specific data fields granted access

The patient maintains granular control over data access. Providers receive only authorized information, while the platform facilitating the transfer operates on encrypted data.

## Running the Example

```bash
# Install dependencies
yarn install  # or npm install or pnpm install

# Build the program
arcium build

# Run tests
arcium test
```

The test suite demonstrates patient data encryption, selective authorization configuration, and controlled data disclosure to authorized providers.

## Technical Implementation

Medical records are encrypted and stored as patient data structures. Access control is enforced through encrypted authorization logic that determines which data fields specific providers can access.

## Implementation Details

### The Selective Sharing Problem

**Conceptual Challenge**: You want to share medical records with a new doctor, but you want to control exactly who can decrypt your information.

**The Question**: Can you transfer encrypted data from your key to doctor's key without decrypting it in transit?

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
2. Data is decrypted inside the MPC environment
3. Data is re-encrypted using doctor's public key
4. Re-encrypted data is emitted in event
5. Only the doctor can decrypt (they have the private key)

**Key insight**: Data is "handed over" inside MPC—re-encrypted from one key to another without intermediate plaintext exposure.

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

Stored as a single encrypted data structure containing 11 fields (352 bytes total). The entire record is encrypted together for on-chain storage.

### When to Use Re-encryption

Apply this pattern when:

- Data encrypted under one key needs to be accessible to another party
- No single party should see the unencrypted data during transfer
- Selective disclosure (future: share only age + blood_type, not full record)
- Examples: credential sharing, encrypted file transfer, confidential data markets
