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

const COMP_DEF_OFFSET_PREDICT_PROBABILITY: u32 = comp_def_offset("predict_probability");

declare_id!("KzAUiBZMUTv4vbuFhJeeJ6xZWkhVKK7bC3ZCgQd1bfQ");

#[arcium_program]
pub mod predictor {
    use super::*;

    pub fn init_predict_probability_comp_def(
        ctx: Context<InitPredictProbabilityCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn predict_probability(
        ctx: Context<PredictProbability>,
        coef_1: [u8; 32],
        coef_2: [u8; 32],
        intercept: [u8; 32],
        input_1: [u8; 32],
        input_2: [u8; 32],
        encryption_pub_key: [u8; 32],
        encryption_nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::PublicKey(encryption_pub_key),
            Argument::PlaintextU128(encryption_nonce),
            Argument::EncryptedU8(coef_1),
            Argument::EncryptedU8(coef_2),
            Argument::EncryptedU8(intercept),
            Argument::EncryptedU8(input_1),
            Argument::EncryptedU8(input_2),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "predict_probability")]
    pub fn predict_probability_callback(
        ctx: Context<PredictProbabilityCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        emit!(PredictProbabilityEvent {
            probability: bytes[..32].try_into().unwrap(),
        });
        Ok(())
    }
}

#[queue_computation_accounts("predict_probability", payer)]
#[derive(Accounts)]
pub struct PredictProbability<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PREDICT_PROBABILITY)
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
}

#[callback_accounts("predict_probability", payer)]
#[derive(Accounts)]
pub struct PredictProbabilityCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PREDICT_PROBABILITY)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("predict_probability", payer)]
#[derive(Accounts)]
pub struct InitPredictProbabilityCompDef<'info> {
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
pub struct PredictProbabilityEvent {
    pub probability: [u8; 32],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
