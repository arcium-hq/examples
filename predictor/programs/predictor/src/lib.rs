use anchor_lang::prelude::*;
use arcium_anchor::{
    comp_def_offset, init_comp_def, queue_computation, CLOCK_PDA_SEED, CLUSTER_PDA_SEED,
    COMP_DEF_PDA_SEED, MEMPOOL_PDA_SEED, MXE_PDA_SEED, POOL_PDA_SEED,
};
use arcium_client::idl::arcium::{
    accounts::{
        ClockAccount, Cluster, ComputationDefinitionAccount, Mempool, PersistentMXEAccount,
        StakingPoolAccount,
    },
    program::Arcium,
    types::{Argument, OffChainReference},
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    queue_computation_accounts,
};

const COMP_DEF_OFFSET_PREDICT: u32 = comp_def_offset("predict_proba");

declare_id!("GUDuZTaMjdTJtsHqhBBhcsMDpa5jWRHBDzj5QtB6FL4j");

#[arcium_program]
pub mod predictor {
    use super::*;

    pub fn init_predict_comp_def(ctx: Context<InitPredictCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn predictor(
        ctx: Context<Predict>,
        coef_1: OffChainReference,
        coef_2: OffChainReference,
        coef_3: OffChainReference,
        coef_4: OffChainReference,
        intercept: OffChainReference,
        input_1: OffChainReference,
        input_2: OffChainReference,
        input_3: OffChainReference,
        input_4: OffChainReference,
    ) -> Result<()> {
        let args = vec![
            Argument::MFloat(coef_1),
            Argument::MFloat(coef_2),
            Argument::MFloat(coef_3),
            Argument::MFloat(coef_4),
            Argument::MFloat(intercept),
            Argument::MFloat(input_1),
            Argument::MFloat(input_2),
            Argument::MFloat(input_3),
            Argument::MFloat(input_4),
        ];
        queue_computation(ctx.accounts, args, vec![], vec![])?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "predict_proba")]
    pub fn predict_proba_callback(ctx: Context<PredictCallback>, output: Vec<u8>) -> Result<()> {
        msg!("Prediction: {:?}", output);
        Ok(())
    }
}

#[queue_computation_accounts("predict_proba", payer)]
pub struct Predict<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [MXE_PDA_SEED, ID.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_account.bump
    )]
    pub mxe_account: Account<'info, PersistentMXEAccount>,
    #[account(
        mut,
        seeds = [MEMPOOL_PDA_SEED, ID.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mempool_account.bump
    )]
    pub mempool_account: Account<'info, Mempool>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_PREDICT.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        seeds = [CLUSTER_PDA_SEED,  mxe_account.cluster.offset.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = cluster_account.bump
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        seeds = [POOL_PDA_SEED],
        seeds::program = ARCIUM_PROG_ID,
        bump = pool_account.bump
    )]
    pub pool_account: Account<'info, StakingPoolAccount>,
    #[account(
        seeds = [CLOCK_PDA_SEED],
        seeds::program = ARCIUM_PROG_ID,
        bump = clock_account.bump
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("predict_proba", payer)]
pub struct PredictCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_PREDICT.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("predict_proba", payer)]
pub struct InitPredictCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, ID_CONST.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_acc.bump
    )]
    pub mxe_acc: Box<Account<'info, PersistentMXEAccount>>,
    #[account(mut)]
    pub comp_def_acc: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}
