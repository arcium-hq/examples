//! Stores a patient's encrypted medical record on-chain and re-encrypts it for
//! a chosen recipient via MPC, emitting the result as an event.
//!
//! The `share_patient_data` circuit reads the record directly from the
//! `PatientData` account: 8-byte Anchor discriminator, then eleven `[u8; 32]`
//! ciphertexts (patient_id, age, gender, blood_type, weight, height, five
//! allergy flags) — 352 bytes of ciphertext starting at offset 8.

use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_SHARE_PATIENT_DATA: u32 = comp_def_offset("share_patient_data");

declare_id!("671bRuGEhWu7N9tsc38xE9Zp8ABAJaBcZRo29UzsftHg");

#[arcium_program]
pub mod share_medical_records {
    use super::*;

    /// Stores the patient's client-encrypted record in the `PatientData` PDA.
    /// A plain Anchor write; no MPC computation is queued.
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
        init_computation_def(ctx.accounts, None)?;
        Ok(())
    }

    /// Queues the MPC computation that re-encrypts the stored record for
    /// `receiver`. The stored data is not modified.
    pub fn share_patient_data(
        ctx: Context<SharePatientData>,
        computation_offset: u64,
        receiver: [u8; 32],
        receiver_nonce: u128,
        sender_pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        // Argument order must match the circuit signature: `receiver: Shared`
        // expands to pubkey + nonce, then `Enc<Shared, PatientData>` expands to
        // pubkey + nonce + ciphertexts. The ciphertexts are read from the
        // account, starting at offset 8 to skip the Anchor discriminator.
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

        // Persist the bump before queueing so the callback can verify the signer PDA.
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![SharePatientDataCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
            0,
        )?;
        Ok(())
    }

    /// Emits the re-encrypted record as `ReceivedPatientDataEvent`; only the
    /// receiver can decrypt the ciphertexts, using the nonce from the event.
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
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account)
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account)
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHARE_PATIENT_DATA)
    )]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [b"patient_data", payer.key().as_ref()],
        bump,
    )]
    pub patient_data: Account<'info, PatientData>,
}

#[callback_accounts("share_patient_data")]
#[derive(Accounts)]
pub struct SharePatientDataCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHARE_PATIENT_DATA)
    )]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(address = ::arcium_anchor::solana_instructions_sysvar::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: UncheckedAccount<'info>,
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
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
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
