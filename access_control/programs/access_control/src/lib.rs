use anchor_lang::prelude::*;
use arcium_anchor::{
    comp_def_offset, derive_cluster_pda, derive_comp_def_pda, derive_execpool_pda,
    derive_mempool_pda, derive_mxe_pda, init_comp_def, queue_computation,
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

const COMP_DEF_OFFSET_ACCESS_CONTROL: u32 =
    comp_def_offset("share_answer_to_ultimate_question_of_life");

declare_id!("8ezhbEqmpbQJmWGyJMgsyQLVp6uP8fEsncJzWN5Wdgkv");

#[arcium_program]
pub mod access_control {
    use super::*;

    pub fn init_share_answer_to_ultimate_question_of_life_comp_def(
        ctx: Context<InitShareAnswerToUltimateQuestionOfLifeCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn share_answer_to_ultimate_question_of_life(
        ctx: Context<ShareAnswerToUltimateQuestionOfLife>,
        ciphertext_0: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::PublicKey(pub_key),
            Argument::PlaintextU128(nonce),
            Argument::EncryptedU8(ciphertext_0),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "share_answer_to_ultimate_question_of_life")]
    pub fn share_answer_to_ultimate_question_of_life_callback(
        ctx: Context<ShareAnswerToUltimateQuestionOfLifeCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        emit!(AnswerToUltimateQuestionOfLifeEvent {
            answer: output[0..32].try_into().unwrap(),
        });
        Ok(())
    }
}

#[queue_computation_accounts("share_answer_to_ultimate_question_of_life", payer)]
#[derive(Accounts)]
pub struct ShareAnswerToUltimateQuestionOfLife<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_ACCESS_CONTROL)
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

#[callback_accounts("share_answer_to_ultimate_question_of_life", payer)]
#[derive(Accounts)]
pub struct ShareAnswerToUltimateQuestionOfLifeCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_ACCESS_CONTROL)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("share_answer_to_ultimate_question_of_life", payer)]
#[derive(Accounts)]
pub struct InitShareAnswerToUltimateQuestionOfLifeCompDef<'info> {
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
pub struct AnswerToUltimateQuestionOfLifeEvent {
    pub answer: [u8; 32],
}
