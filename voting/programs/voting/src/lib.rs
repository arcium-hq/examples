use anchor_lang::prelude::*;
use arcium_anchor::{
    comp_def_offset, init_comp_def, init_da_object, queue_computation, CLOCK_PDA_SEED,
    CLUSTER_PDA_SEED, COMP_DEF_PDA_SEED, DATA_OBJ_PDA_SEED, MEMPOOL_PDA_SEED, MXE_PDA_SEED,
    POOL_PDA_SEED,
};
use arcium_client::idl::arcium::{
    accounts::{
        ClockAccount, Cluster, ComputationDefinitionAccount, DataObjectAccount, Mempool,
        PersistentMXEAccount, StakingPoolAccount,
    },
    program::Arcium,
    types::Argument,
    types::OffChainReference,
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    init_data_object_accounts, queue_computation_accounts,
};

const COMP_DEF_OFFSET_VOTE: u32 = comp_def_offset("vote");
const COMP_DEF_OFFSET_REVEAL: u32 = comp_def_offset("reveal_result");

declare_id!("YFLJWFAbxhRZv4xYET3cgnno85DopTJvKazkmzpUjB2");

#[arcium_program]
pub mod voting {
    use super::*;

    pub fn create_new_poll(
        ctx: Context<CreateNewPoll>,
        id: u32,
        question: String,
        initial_vote_state: OffChainReference,
    ) -> Result<()> {
        msg!("Creating a new poll");

        init_da_object(
            ctx.accounts,
            initial_vote_state,
            ctx.accounts.vote_state.to_account_info(),
            id,
        )?;

        ctx.accounts.poll_acc.question = question;
        ctx.accounts.poll_acc.vote_state_da_obj = ctx.accounts.vote_state.key();
        ctx.accounts.poll_acc.bump = ctx.bumps.poll_acc;
        ctx.accounts.poll_acc.id = id;
        ctx.accounts.poll_acc.authority = ctx.accounts.payer.key();

        Ok(())
    }

    pub fn init_vote_comp_def(ctx: Context<InitVoteCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn vote(ctx: Context<Vote>, id: u32, vote_state: OffChainReference) -> Result<()> {
        let args = vec![Argument::MBool(vote_state), Argument::DataObj(id)];
        queue_computation(
            ctx.accounts,
            args,
            vec![ctx.accounts.vote_state.to_account_info()],
            vec![],
        )?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "vote")]
    pub fn vote_callback(ctx: Context<VoteCallback>, output: Vec<u8>) -> Result<()> {
        msg!("Arcium callback invoked with output {:?}", output);
        Ok(())
    }

    pub fn init_reveal_result_comp_def(ctx: Context<InitRevealResultCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn reveal_result(ctx: Context<RevealVotingResult>, id: u32) -> Result<()> {
        require!(
            ctx.accounts.payer.key() == ctx.accounts.poll_acc.authority,
            ErrorCode::InvalidAuthority
        );

        msg!("Revealing voting result for poll with id {}", id);

        let args = vec![Argument::DataObj(id)];

        queue_computation(
            ctx.accounts,
            args,
            vec![ctx.accounts.vote_state.to_account_info()],
            vec![],
        )?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "reveal_result")]
    pub fn reveal_result_callback(
        ctx: Context<RevealVotingResultCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        msg!("Arcium callback invoked with output {:?}", output);
        Ok(())
    }
}

#[init_data_object_accounts(payer)]
#[instruction(id: u32)]
pub struct CreateNewPoll<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + PollAccount::INIT_SPACE,
        seeds = [b"poll", payer.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
    )]
    pub poll_acc: Account<'info, PollAccount>,
    /// CHECK: Vote state data object will be initialized by CPI
    #[account(mut)]
    pub vote_state: UncheckedAccount<'info>,
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

#[queue_computation_accounts("vote", payer)]
#[instruction(id: u32)]
pub struct Vote<'info> {
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
        seeds = [COMP_DEF_PDA_SEED, COMP_DEF_OFFSET_VOTE.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        seeds = [CLUSTER_PDA_SEED, mxe_account.cluster.offset.to_le_bytes().as_ref()],
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
    /// CHECK: Poll authority pubkey
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"poll", authority.key().as_ref(), id.to_le_bytes().as_ref()],
        bump = poll_acc.bump,
        has_one = authority
    )]
    pub poll_acc: Account<'info, PollAccount>,
    #[account(
        mut,
        seeds = [DATA_OBJ_PDA_SEED, id.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        owner = ARCIUM_PROG_ID,
        bump = vote_state.bump,
    )]
    pub vote_state: Account<'info, DataObjectAccount>,
}

#[callback_accounts("vote", payer)]
pub struct VoteCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, COMP_DEF_OFFSET_VOTE.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("vote", payer)]
pub struct InitVoteCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = ID_CONST
    )]
    pub mxe: AccountInfo<'info>,
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

#[queue_computation_accounts("reveal_result", payer)]
#[instruction(id: u32)]
pub struct RevealVotingResult<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"poll", payer.key().as_ref(), id.to_le_bytes().as_ref()],
        bump = poll_acc.bump
    )]
    pub poll_acc: Account<'info, PollAccount>,
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
        seeds = [COMP_DEF_PDA_SEED, COMP_DEF_OFFSET_REVEAL.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        seeds = [CLUSTER_PDA_SEED, mxe_account.cluster.offset.to_le_bytes().as_ref()],
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
        seeds = [DATA_OBJ_PDA_SEED, id.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        owner = ARCIUM_PROG_ID,
        bump = vote_state.bump,
    )]
    pub vote_state: Account<'info, DataObjectAccount>,
}

#[callback_accounts("reveal_result", payer)]
pub struct RevealVotingResultCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, COMP_DEF_OFFSET_REVEAL.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("reveal_result", payer)]
pub struct InitRevealResultCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = ID_CONST
    )]
    pub mxe: AccountInfo<'info>,
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

#[account]
#[derive(InitSpace)]
pub struct PollAccount {
    #[max_len(50)]
    pub question: String,
    pub id: u32,
    pub authority: Pubkey,
    pub vote_state_da_obj: Pubkey,
    pub bump: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid authority")]
    InvalidAuthority,
}
