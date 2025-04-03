# Share Medical Records

## How This Example Demonstrates Arcium's MPC Solution

This example showcases how Arcium's MPC (Multi-Party Computation) solution enables decentralized, trust-minimized confidential computing on Solana. Here's how it works:

## Architecture Overview

- The project implements a medical records sharing system on Solana using Arcium's MPC network
- It's structured with two main components:
  - Regular Solana program code in the `programs` directory
  - Confidential computing instructions in the `encrypted-ixs` directory using Arcium's Arcis framework

## Confidential Data Handling

- The system demonstrates how to handle sensitive medical data (patient ID, age, gender, blood type, weight, height, allergies) in a privacy-preserving way
- Data is encrypted using Arcium's encryption scheme (using x25519 for key exchange and RescueCipher for encryption)
- The actual computation happens off-chain in Arcium's MPC network, ensuring the data never exists in plaintext on the blockchain

## Trust-Minimized Architecture

- The system uses a decentralized network of MPC nodes
- The computation is split across multiple parties (nodes) who must cooperate to perform operations
- No single node has access to the complete data, making it impossible for any single party to compromise privacy

## Key Components

- **Encrypted Circuit**: Defined in `encrypted-ixs/src/lib.rs`, the `share_patient_data` circuit handles the confidential transfer of patient data
- **Program Instructions**:
  - `init_share_patient_data_comp_def`: Initializes the confidential computation definition
  - `store_patient_data`: Stores encrypted patient data on-chain
  - `share_patient_data`: Initiates the confidential data sharing process
  - `share_patient_data_callback`: Handles the result of the confidential computation

## Security Features

- Uses a threshold encryption scheme where multiple parties must cooperate
- Implements proper key management with separate encryption keys for sender and receiver
- Employs nonces to prevent replay attacks
- Uses Arcium's secure enclave environment for computation

## Integration with Solana

- Seamlessly integrates with Solana's account model and program structure
- Uses Anchor framework for program development
- Maintains on-chain state for encrypted data while keeping the actual computation off-chain

## Practical Implementation

The test file (`share_medical_records.ts`) demonstrates the complete flow:

1. Initializes the computation definition
2. Encrypts and stores patient data
3. Shares the data with a receiver
4. Verifies the secure transfer through events

This example effectively showcases how Arcium's MPC solution enables:

- Decentralized computation without any single trusted party
- Privacy-preserving data sharing on public blockchains
- Secure handling of sensitive medical information
- Integration with existing blockchain infrastructure
- Practical implementation of complex privacy-preserving protocols
