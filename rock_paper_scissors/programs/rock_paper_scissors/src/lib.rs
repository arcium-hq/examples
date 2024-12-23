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
    init_data_object_accounts, queue_computation_accounts,
};
pub mod utils;
use utils::get_offset;

const COMP_DEF_OFFSET_COMMIT_CHOICE: u32 = comp_def_offset("commit_choice");
const COMP_DEF_OFFSET_DECIDE_WINNER: u32 = comp_def_offset("decide_winner");

declare_id!("2ihd38tySzhfWJmWvQt78zyyCC4ktCKs8HwjwYAXLjqa");

#[arcium_program]
pub mod rock_paper_scissors {
    use super::*;

    pub fn create_new_game(
        ctx: Context<CreateNewGame>,
        id: u32,
        initial_player_acc: OffChainReference,
    ) -> Result<()> {
        msg!("Instantiating new game!");

        init_da_object(
            ctx.accounts,
            initial_player_acc,
            ctx.accounts.player1_data_obj.to_account_info(),
            get_offset(id, 1),
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
        ctx: Context<CommitChoiceCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        msg!("Choice committed successfully");
        Ok(())
    }

    pub fn decide_winner(ctx: Context<DecideWinner>, id: u32) -> Result<()> {
        let args = vec![Argument::DataObject(), Argument::DataObject()];
        queue_computation(
            ctx.accounts,
            args,
            vec![
                ctx.accounts.player_one.to_account_info(),
                ctx.accounts.player_two.to_account_info(),
            ],
            vec![],
        )?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "decide_winner")]
    pub fn decide_winner_callback(
        ctx: Context<DecideWinnerCallbackCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        msg!("Winner decided successfully: {:?}", output[0]);
        Ok(())
    }
}

#[init_data_object_accounts(payer)]
#[instruction(id: u32)]
pub struct CreateNewGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + RPSGame::INIT_SPACE,
        seeds = [b"rps", id.to_le_bytes().as_ref(), payer.key().as_ref()],
        bump,
    )]
    pub game: Account<'info, RPSGame>,
    /// CHECK: Player state data object will be initialized by CPI
    #[account(mut)]
    pub player1_data_obj: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, ID_CONST.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe.bump
    )]
    pub mxe: Account<'info, PersistentMXEAccount>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_COMMIT_CHOICE.to_le_bytes().as_ref()],
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_COMMIT_CHOICE.to_le_bytes().as_ref()],
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

#[queue_computation_accounts("decide_winner", payer)]
pub struct DecideWinner<'info> {
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_DECIDE_WINNER.to_le_bytes().as_ref()],
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
    #[account(
        mut,
        seeds = [DATA_OBJ_PDA_SEED, &ID_CONST.to_bytes().as_ref(), id.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = player_one.bump,
    )]
    pub player_one: Account<'info, DataObjectAccount>,
    #[account(
        mut,
        seeds = [DATA_OBJ_PDA_SEED, &ID_CONST.to_bytes().as_ref(), id.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = player_two.bump,
    )]
    pub player_two: Account<'info, DataObjectAccount>,
}

#[callback_accounts("decide_winner", payer)]
pub struct DecideWinnerCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_DECIDE_WINNER.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct RPSGame {
    pub id: u32,
    pub player1: Pubkey,
    pub player2: Pubkey,
    pub player1_state: Pubkey,
    pub player2_state: Pubkey,
    pub winner: Pubkey,
    pub bump: u8,
}
