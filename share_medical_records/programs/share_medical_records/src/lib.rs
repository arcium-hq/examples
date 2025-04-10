use anchor_lang::prelude::*;
use arcium_anchor::{
    comp_def_offset, derive_cluster_pda, derive_comp_def_pda, derive_execpool_pda,
    derive_mempool_pda, derive_mxe_pda, init_comp_def, queue_computation, ComputationOutputs,
    ARCIUM_CLOCK_ACCOUNT_ADDRESS, ARCIUM_STAKING_POOL_ACCOUNT_ADDRESS, CLUSTER_PDA_SEED,
    COMP_DEF_PDA_SEED, EXECPOOL_PDA_SEED, MEMPOOL_PDA_SEED, MXE_PDA_SEED,
};
use arcium_client::idl::arcium::{
    accounts::{
        ClockAccount, Cluster, ComputationDefinitionAccount, ExecutingPool, Mempool,
        PersistentMXEAccount, StakingPoolAccount,
    },
    program::Arcium,
    types::Argument,
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    queue_computation_accounts,
};

const COMP_DEF_OFFSET_SHARE_PATIENT_DATA: u32 = comp_def_offset("share_patient_data");

declare_id!("4AJmvihTL5s1fwQYQGNA5c49sypuz5iScWnCD4HJZPp4");

#[arcium_program]
pub mod share_medical_records {
    use super::*;

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
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn share_patient_data(
        ctx: Context<SharePatientData>,
        receiver: [u8; 32],
        receiver_nonce: u128,
        sender_pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::ArcisPubkey(receiver),
            Argument::PlaintextU128(receiver_nonce),
            Argument::ArcisPubkey(sender_pub_key),
            Argument::PlaintextU128(nonce),
            Argument::Account(
                ctx.accounts.patient_data.key(),
                8,
                PatientData::INIT_SPACE as u32,
            ),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "share_patient_data")]
    pub fn share_patient_data_callback(
        ctx: Context<SharePatientDataCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        let bytes = bytes.iter().skip(32).cloned().collect::<Vec<_>>();

        emit!(ReceivedPatientDataEvent {
            nonce: bytes[0..16].try_into().unwrap(),
            patient_id: bytes[16..48].try_into().unwrap(),
            age: bytes[48..80].try_into().unwrap(),
            gender: bytes[80..112].try_into().unwrap(),
            blood_type: bytes[112..144].try_into().unwrap(),
            weight: bytes[144..176].try_into().unwrap(),
            height: bytes[176..208].try_into().unwrap(),
            allergies: [
                bytes[208..240].try_into().unwrap(),
                bytes[240..272].try_into().unwrap(),
                bytes[272..304].try_into().unwrap(),
                bytes[304..336].try_into().unwrap(),
                bytes[336..368].try_into().unwrap(),
            ],
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
pub struct SharePatientData<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, PersistentMXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    pub mempool_account: Account<'info, Mempool>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    pub executing_pool: Account<'info, ExecutingPool>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHARE_PATIENT_DATA)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_STAKING_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, StakingPoolAccount>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    pub patient_data: Account<'info, PatientData>,
}

#[callback_accounts("share_patient_data", payer)]
#[derive(Accounts)]
pub struct SharePatientDataCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHARE_PATIENT_DATA)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
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
    pub mxe_account: Box<Account<'info, PersistentMXEAccount>>,
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

#[account]
#[derive(InitSpace)]
pub struct PatientData {
    pub patient_id: [u8; 32],
    pub age: [u8; 32],
    pub gender: [u8; 32],
    pub blood_type: [u8; 32],
    pub weight: [u8; 32],
    pub height: [u8; 32],
    pub allergies: [[u8; 32]; 5],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
