use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

const COMP_DEF_OFFSET_INIT_GAME: u32 = comp_def_offset("init_game");
const COMP_DEF_OFFSET_PLAYER_MOVE: u32 = comp_def_offset("player_move");
const COMP_DEF_OFFSET_COMPARE_MOVES: u32 = comp_def_offset("compare_moves");

declare_id!("6FasviktxsUBZBst1sAA5uEN2WVnW7MuSiP6hiY9g1XT");

#[arcium_program]
pub mod rock_paper_scissors {
    use super::*;

    pub fn init_init_game_comp_def(ctx: Context<InitInitGameCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_game(
        ctx: Context<InitGame>,
        computation_offset: u64,
        id: u64,
        player_a: Pubkey,
        player_b: Pubkey,
        nonce: u128,
    ) -> Result<()> {
        let game = &mut ctx.accounts.rps_game;
        game.id = id;
        game.player_a = player_a;
        game.player_b = player_b;
        game.nonce = nonce;

        let args = ArgBuilder::new().plaintext_u128(nonce).build();

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![InitGameCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: ctx.accounts.rps_game.key(),
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "init_game")]
    pub fn init_game_callback(
        ctx: Context<InitGameCallback>,
        output: SignedComputationOutputs<InitGameOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(InitGameOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let nonce = o.nonce;

        let moves: [[u8; 32]; 2] = o.ciphertexts;

        let game = &mut ctx.accounts.rps_game;
        game.moves = moves;
        game.nonce = nonce;

        Ok(())
    }

    pub fn init_player_move_comp_def(ctx: Context<InitPlayerMoveCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn player_move(
        ctx: Context<PlayerMove>,
        computation_offset: u64,
        player_id: [u8; 32],
        player_move: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        require!(
            ctx.accounts.payer.key() == ctx.accounts.rps_game.player_a
                || ctx.accounts.payer.key() == ctx.accounts.rps_game.player_b,
            ErrorCode::NotAuthorized
        );

        let args = ArgBuilder::new()
            .x25519_pubkey(pub_key)
            .plaintext_u128(nonce)
            .encrypted_u8(player_id)
            .encrypted_u8(player_move)
            .plaintext_u128(ctx.accounts.rps_game.nonce)
            .account(ctx.accounts.rps_game.key(), 8, 32 * 2)
            .build();

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![PlayerMoveCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: ctx.accounts.rps_game.key(),
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "player_move")]
    pub fn player_move_callback(
        ctx: Context<PlayerMoveCallback>,
        output: SignedComputationOutputs<PlayerMoveOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(PlayerMoveOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let nonce = o.nonce;

        let moves: [[u8; 32]; 2] = o.ciphertexts;

        let game = &mut ctx.accounts.rps_game;
        game.moves = moves;
        game.nonce = nonce;

        Ok(())
    }

    pub fn init_compare_moves_comp_def(ctx: Context<InitCompareMovesCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn compare_moves(ctx: Context<CompareMoves>, computation_offset: u64) -> Result<()> {
        let args = ArgBuilder::new()
            .plaintext_u128(ctx.accounts.rps_game.nonce)
            .account(ctx.accounts.rps_game.key(), 8, 32 * 2)
            .build();
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![CompareMovesCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "compare_moves")]
    pub fn compare_moves_callback(
        ctx: Context<CompareMovesCallback>,
        output: SignedComputationOutputs<CompareMovesOutput>,
    ) -> Result<()> {
        let result = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(CompareMovesOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let result_str = match result {
            0 => "Tie",
            1 => "Player A Wins",
            2 => "Player B Wins",
            3 => "Invalid Move",
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
#[instruction(computation_offset: u64, id: u64)]
pub struct InitGame<'info> {
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
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
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

#[callback_accounts("init_game")]
#[derive(Accounts)]
pub struct InitGameCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
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
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("player_move", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct PlayerMove<'info> {
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
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_MOVE)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut)]
    pub rps_game: Account<'info, RPSGame>,
}

#[callback_accounts("player_move")]
#[derive(Accounts)]
pub struct PlayerMoveCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_MOVE)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub rps_game: Account<'info, RPSGame>,
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
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("compare_moves", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct CompareMoves<'info> {
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
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_COMPARE_MOVES)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS,
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(mut)]
    pub rps_game: Account<'info, RPSGame>,
}

#[callback_accounts("compare_moves")]
#[derive(Accounts)]
pub struct CompareMovesCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_COMPARE_MOVES)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
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
    pub mxe_account: Box<Account<'info, MXEAccount>>,
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
    #[msg("Not authorized")]
    NotAuthorized,
    #[msg("Cluster not set")]
    ClusterNotSet,
}
