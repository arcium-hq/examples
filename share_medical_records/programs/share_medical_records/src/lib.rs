use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_SHARE_PATIENT_DATA: u32 = comp_def_offset("share_patient_data");

declare_id!("NEnkfYAYz9epwXkXChP3hz2y1L8wUgf2xkrUKAmfxBD");

#[arcium_program]
pub mod share_medical_records {
    use super::*;

    /// Stores encrypted patient medical data on-chain.
    ///
    /// This function stores patient medical information in encrypted form. All data fields
    /// are provided as encrypted 32-byte arrays that can only be decrypted by authorized parties.
    /// The data remains confidential while being stored on the public Solana blockchain.
    ///
    /// # Arguments
    /// * `patient_id` - Encrypted unique identifier for the patient
    /// * `age` - Encrypted patient age
    /// * `gender` - Encrypted patient gender information
    /// * `blood_type` - Encrypted blood type information
    /// * `weight` - Encrypted patient weight
    /// * `height` - Encrypted patient height
    /// * `allergies` - Array of encrypted allergy information (up to 5 entries)
    pub fn store_patient_data(
        ctx: Context<StorePatientData>,
        patient_id: [u8; 32],
        age: [u8; 32],
        gender: [u8; 32],
        blood_type: [u8; 32],
        weight: [u8; 32],
        height: [u8; 32],
        allergies: [[u8; 32]; 5],
    ) -> Result<()> {
        let patient_data = &mut ctx.accounts.patient_data;
        patient_data.patient_id = patient_id;
        patient_data.age = age;
        patient_data.gender = gender;
        patient_data.blood_type = blood_type;
        patient_data.weight = weight;
        patient_data.height = height;
        patient_data.allergies = allergies;

        Ok(())
    }

    pub fn init_share_patient_data_comp_def(
        ctx: Context<InitSharePatientDataCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    /// Initiates confidential sharing of patient data with a specified receiver.
    ///
    /// This function triggers an MPC computation that re-encrypts the patient's medical data
    /// for a specific receiver. The receiver will be able to decrypt the data using their
    /// private key, while the data remains encrypted for everyone else. The original
    /// stored data is not modified and remains encrypted for the original owner.
    ///
    /// # Arguments
    /// * `receiver` - Public key of the authorized recipient
    /// * `receiver_nonce` - Cryptographic nonce for the receiver's encryption
    /// * `sender_pub_key` - Sender's public key for the operation
    /// * `nonce` - Cryptographic nonce for the sender's encryption
    pub fn share_patient_data(
        ctx: Context<SharePatientData>,
        computation_offset: u64,
        receiver: [u8; 32],
        receiver_nonce: u128,
        sender_pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let args = ArgBuilder::new()
            .x25519_pubkey(receiver)
            .plaintext_u128(receiver_nonce)
            .x25519_pubkey(sender_pub_key)
            .plaintext_u128(nonce)
            .account(
                ctx.accounts.patient_data.key(),
                8,
                PatientData::INIT_SPACE as u32,
            )
            .build();

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![SharePatientDataCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    /// Handles the result of the patient data sharing MPC computation.
    ///
    /// This callback processes the re-encrypted patient data that has been prepared for
    /// the specified receiver. It emits an event containing all the medical data fields
    /// encrypted specifically for the receiver's public key.
    #[arcium_callback(encrypted_ix = "share_patient_data")]
    pub fn share_patient_data_callback(
        ctx: Context<SharePatientDataCallback>,
        output: SignedComputationOutputs<SharePatientDataOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(SharePatientDataOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        emit!(ReceivedPatientDataEvent {
            nonce: o.nonce.to_le_bytes(),
            patient_id: o.ciphertexts[0],
            age: o.ciphertexts[1],
            gender: o.ciphertexts[2],
            blood_type: o.ciphertexts[3],
            weight: o.ciphertexts[4],
            height: o.ciphertexts[5],
            allergies: o.ciphertexts[6..11]
                .try_into()
                .map_err(|_| ErrorCode::InvalidAllergyData)?,
        });
        Ok(())
    }
}

#[derive(Accounts)]
pub struct StorePatientData<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    #[account(
        init,
        payer = payer,
        space = 8 + PatientData::INIT_SPACE,
        seeds = [b"patient_data", payer.key().as_ref()],
        bump,
    )]
    pub patient_data: Account<'info, PatientData>,
}

#[queue_computation_accounts("share_patient_data", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct SharePatientData<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHARE_PATIENT_DATA)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    pub patient_data: Account<'info, PatientData>,
}

#[callback_accounts("share_patient_data")]
#[derive(Accounts)]
pub struct SharePatientDataCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHARE_PATIENT_DATA)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("share_patient_data", payer)]
#[derive(Accounts)]
pub struct InitSharePatientDataCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct ReceivedPatientDataEvent {
    pub nonce: [u8; 16],
    pub patient_id: [u8; 32],
    pub age: [u8; 32],
    pub gender: [u8; 32],
    pub blood_type: [u8; 32],
    pub weight: [u8; 32],
    pub height: [u8; 32],
    pub allergies: [[u8; 32]; 5],
}

/// Stores encrypted patient medical information.
#[account]
#[derive(InitSpace)]
pub struct PatientData {
    /// Encrypted unique patient identifier
    pub patient_id: [u8; 32],
    /// Encrypted patient age
    pub age: [u8; 32],
    /// Encrypted gender information
    pub gender: [u8; 32],
    /// Encrypted blood type
    pub blood_type: [u8; 32],
    /// Encrypted weight measurement
    pub weight: [u8; 32],
    /// Encrypted height measurement
    pub height: [u8; 32],
    /// Array of encrypted allergy information (up to 5 allergies)
    pub allergies: [[u8; 32]; 5],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Invalid allergy data format")]
    InvalidAllergyData,
    #[msg("Cluster not set")]
    ClusterNotSet,
}
