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

const COMP_DEF_OFFSET_INIT_VOTE_STATS: u32 = comp_def_offset("init_vote_stats");
const COMP_DEF_OFFSET_VOTE: u32 = comp_def_offset("vote");
const COMP_DEF_OFFSET_REVEAL: u32 = comp_def_offset("reveal_result");

declare_id!("EGRyBhhe9pzvoznJuQh5x5Dx5V2eKQv1AwWdTsEMEYEQ");

#[arcium_program]
pub mod voting {
    use super::*;

    pub fn init_vote_stats_comp_def(ctx: Context<InitVoteStatsCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn create_new_poll(
        ctx: Context<CreateNewPoll>,
        id: u32,
        question: String,
        nonce: u128,
    ) -> Result<()> {
        msg!("Creating a new poll");

        ctx.accounts.poll_acc.question = question;
        ctx.accounts.poll_acc.bump = ctx.bumps.poll_acc;
        ctx.accounts.poll_acc.id = id;
        ctx.accounts.poll_acc.authority = ctx.accounts.payer.key();
        ctx.accounts.poll_acc.nonce = nonce;
        ctx.accounts.poll_acc.vote_state = [[0; 32]; 2];

        let args = vec![Argument::PlaintextU128(nonce)];

        queue_computation(
            ctx.accounts,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.poll_acc.key(),
                is_writable: true,
            }],
            None,
        )?;

        Ok(())
    }

    #[arcium_callback(confidential_ix = "init_vote_stats")]
    pub fn init_vote_stats_callback(
        ctx: Context<InitVoteStatsCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        let vote_stats: [[u8; 32]; 2] = output
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let mut poll_acc =
            PollAccount::try_deserialize(&mut &ctx.accounts.poll_acc.data.borrow()[..])?;
        poll_acc.vote_state = vote_stats;
        poll_acc.try_serialize(&mut *ctx.accounts.poll_acc.try_borrow_mut_data()?)?;

        Ok(())
    }

    pub fn init_vote_comp_def(ctx: Context<InitVoteCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn vote(
        ctx: Context<Vote>,
        id: u32,
        vote: [u8; 32],
        vote_encryption_pubkey: [u8; 32],
        vote_nonce: u128,
        vote_stats_nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::EncryptedBool(vote),
            Argument::PublicKey(vote_encryption_pubkey),
            Argument::PlaintextU128(vote_nonce),
            Argument::Account(
                ctx.accounts.poll_acc.key(),
                // Offset of 8 (discriminator), 1 (bump), 4 + 50 (question), 4 (id), 32 (authority), 16 (nonce)
                8 + 1 + (4 + 50) + 4 + 32 + 16,
                32 * 2, // 2 counts, each saved as a ciphertext (so 32 bytes each)
            ),
            Argument::PlaintextU128(vote_stats_nonce),
        ];

        queue_computation(
            ctx.accounts,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.poll_acc.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "vote")]
    pub fn vote_callback(ctx: Context<VoteCallback>, output: Vec<u8>) -> Result<()> {
        let vote_stats: [[u8; 32]; 2] = output
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let mut poll_acc =
            PollAccount::try_deserialize(&mut &ctx.accounts.poll_acc.data.borrow()[..])?;
        poll_acc.vote_state = vote_stats;
        poll_acc.try_serialize(&mut *ctx.accounts.poll_acc.try_borrow_mut_data()?)?;

        emit!(VoteEvent { output: output });

        Ok(())
    }

    pub fn init_reveal_result_comp_def(ctx: Context<InitRevealResultCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn reveal_result(
        ctx: Context<RevealVotingResult>,
        id: u32,
        vote_stats_nonce: u128,
    ) -> Result<()> {
        require!(
            ctx.accounts.payer.key() == ctx.accounts.poll_acc.authority,
            ErrorCode::InvalidAuthority
        );

        msg!("Revealing voting result for poll with id {}", id);

        let args = vec![
            Argument::Account(
                ctx.accounts.poll_acc.key(),
                // Offset of 8 (discriminator), 1 (bump), 4 + 50 (question), 4 (id), 32 (authority), 16 (nonce)
                8 + 1 + (4 + 50) + 4 + 32 + 16,
                32 * 2, // 2 counts, each saved as a ciphertext (so 32 bytes each)
            ),
            Argument::PlaintextU128(vote_stats_nonce),
        ];

        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "reveal_result")]
    pub fn reveal_result_callback(
        ctx: Context<RevealVotingResultCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        let result = output[0] != 0;
        emit!(RevealResultEvent { output: result });
        Ok(())
    }
}

#[queue_computation_accounts("init_vote_stats", payer)]
#[derive(Accounts)]
pub struct InitVoteStats<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_TOGETHER)
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

#[callback_accounts("add_together", payer)]
#[derive(Accounts)]
pub struct AddTogetherCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_ADD_TOGETHER)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("add_together", payer)]
#[derive(Accounts)]
pub struct InitAddTogetherCompDef<'info> {
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

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid authority")]
    InvalidAuthority,
}
