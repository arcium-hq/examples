use anchor_lang::prelude::*;
use arcium_anchor::{
    comp_def_offset, init_comp_def, init_da_object, queue_computation, CLOCK_PDA_SEED,
    CLUSTER_PDA_SEED, COMP_DEF_PDA_SEED, MEMPOOL_PDA_SEED, MXE_PDA_SEED, POOL_PDA_SEED,
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

const COMP_DEF_OFFSET_ADD_TOGETHER: u32 = comp_def_offset("add_together");

declare_id!("2ihd38tySzhfWJmWvQt78zyyCC4ktCKs8HwjwYAXLjqa");

#[arcium_program]
pub mod rock_paper_scissors {
    use super::*;

    pub fn create_new_game(
        ctx: Context<CreateNewGame>,
        initial_player_acc: OffChainReference,
    ) -> Result<()> {
        init_da_object(
            ctx.accounts,
            initial_player_acc,
            ctx.accounts.player1_data_obj.to_account_info(),
            data_obj_offset,
        )?;

        Ok(())
    }

    pub fn init_commit_choice_comp_def(ctx: Context<InitCommitChoiceCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn init_decide_winner_comp_def(ctx: Context<InitDecideWinnerCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn commit_choice(ctx: Context<CommitChoice>, choice: OffChainReference) -> Result<()> {
        let args = vec![Argument::MU8(choice), Argument::DataObject()];
        queue_computation(ctx.accounts, args, vec![], vec![])?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "commit_choice")]
    pub fn commit_choice_callback(
        ctx: Context<AddTogetherCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        msg!("Choice committed successfully");
        Ok(())
    }
}

#[queue_computation_accounts("commit_choice", payer)]
pub struct CommitChoice<'info> {
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_ADD_TOGETHER.to_le_bytes().as_ref()],
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

#[callback_accounts("commit_choice", payer)]
pub struct CommitChoiceCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_ADD_TOGETHER.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("commit_choice", payer)]
pub struct InitCommitChoiceCompDef<'info> {
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

#[init_computation_definition_accounts("decide_winner", payer)]
pub struct InitDecideWinnerCompDef<'info> {
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
