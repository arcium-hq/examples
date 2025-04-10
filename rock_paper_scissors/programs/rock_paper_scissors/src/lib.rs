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
    types::{Argument, CallbackAccount},
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    queue_computation_accounts,
};

const COMP_DEF_OFFSET_INIT_GAME: u32 = comp_def_offset("init_game");
const COMP_DEF_OFFSET_PLAYER_MOVE: u32 = comp_def_offset("player_move");
const COMP_DEF_OFFSET_COMPARE_MOVES: u32 = comp_def_offset("compare_moves");

declare_id!("2PiEBVRRcLRQcAyEPFQYoX7rPNTTB3Q8Up7XZJmKEeuQ");

#[arcium_program]
pub mod rock_paper_scissors {
    use super::*;

    pub fn init_init_game_comp_def(ctx: Context<InitInitGameCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn init_game(
        ctx: Context<InitGame>,
        id: u64,
        player_a: Pubkey,
        player_b: Pubkey,
        encryption_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let game = &mut ctx.accounts.rps_game;
        game.id = id;
        game.player_a = player_a;
        game.player_b = player_b;
        game.encryption_key = encryption_key;
        game.nonce = nonce;

        let args = vec![
            Argument::EncryptedU8(encryption_key),
            Argument::PlaintextU128(nonce),
        ];

        queue_computation(
            ctx.accounts,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.rps_game.key(),
                is_writable: true,
            }],
            None,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "init_game")]
    pub fn init_game_callback(
        ctx: Context<InitGameCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        let moves: [[u8; 32]; 2] = bytes[16..]
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let game = &mut ctx.accounts.rps_game;
        game.moves = moves;

        Ok(())
    }

    pub fn init_player_move_comp_def(ctx: Context<InitPlayerMoveCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn init_compare_moves_comp_def(ctx: Context<InitCompareMovesCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn compare_moves(
        ctx: Context<CompareMoves>,
        player_move: [u8; 32],
        house_move: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::ArcisPubkey(pub_key),
            Argument::PlaintextU128(nonce),
            Argument::EncryptedU8(player_move),
            Argument::EncryptedU8(house_move),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "compare_moves")]
    pub fn compare_moves_callback(
        ctx: Context<CompareMovesCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        msg!("output: {:?}", bytes);

        let result = bytes[0];
        let result_str = match result {
            0 => "Tie",
            1 => "Win",
            2 => "Loss",
            _ => "Unknown",
        };

        emit!(CompareMovesEvent {
            result: result_str.to_string(),
        });
        Ok(())
    }
}

#[queue_computation_accounts("init_game", payer)]
#[derive(Accounts)]
#[instruction(id: u64)]
pub struct InitGame<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_GAME)
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
    #[account(init,
        payer = payer,
        space = 8 + RPSGame::INIT_SPACE,
        seeds = [b"rps_game", id.to_le_bytes().as_ref()],
        bump,
    )]
    pub rps_game: Account<'info, RPSGame>,
}

#[callback_accounts("init_game", payer)]
#[derive(Accounts)]
pub struct InitGameCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub rps_game: Account<'info, RPSGame>,
}

#[init_computation_definition_accounts("init_game", payer)]
#[derive(Accounts)]
pub struct InitInitGameCompDef<'info> {
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

#[init_computation_definition_accounts("player_move", payer)]
#[derive(Accounts)]
pub struct InitPlayerMoveCompDef<'info> {
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

#[queue_computation_accounts("compare_moves", payer)]
#[derive(Accounts)]
pub struct CompareMoves<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_COMPARE_MOVES)
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

#[callback_accounts("compare_moves", payer)]
#[derive(Accounts)]
pub struct CompareMovesCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_COMPARE_MOVES)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("compare_moves", payer)]
#[derive(Accounts)]
pub struct InitCompareMovesCompDef<'info> {
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

#[account]
#[derive(InitSpace)]
pub struct RPSGame {
    pub moves: [[u8; 32]; 2],
    pub player_a: Pubkey,
    pub player_b: Pubkey,
    pub encryption_key: [u8; 32],
    pub nonce: u128,
    pub id: u64,
}

#[event]
pub struct CompareMovesEvent {
    pub result: String,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
