use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_FLIP: u32 = comp_def_offset("flip");

declare_id!("EiFoAJkimEAju8gcjR53yQmfoXDGrwY7F53Nv5BUKkXe");

#[arcium_program]
pub mod coinflip {
    use super::*;

    /// Initializes the computation definition for the coin flip operation.
    /// This sets up the MPC environment for generating secure randomness and comparing it with the player's choice.
    pub fn init_flip_comp_def(ctx: Context<InitFlipCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    /// Initiates a coin flip game with the player's encrypted choice.
    ///
    /// The player submits their choice (heads or tails) in encrypted form along with their
    /// public key and nonce. The MPC computation will generate a cryptographically secure
    /// random boolean and compare it with the player's choice to determine if they won.
    ///
    /// # Arguments
    /// * `user_choice` - Player's encrypted choice (true for heads, false for tails)
    /// * `pub_key` - Player's public key for encryption operations
    /// * `nonce` - Cryptographic nonce for the encryption
    pub fn flip(
        ctx: Context<Flip>,
        computation_offset: u64,
        user_choice: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::ArcisPubkey(pub_key),
            Argument::PlaintextU128(nonce),
            Argument::EncryptedU8(user_choice),
        ];
        queue_computation(ctx.accounts, computation_offset, args, vec![], None)?;
        Ok(())
    }

    /// Handles the result of the coin flip MPC computation.
    ///
    /// This callback receives the result of comparing the player's choice with the
    /// randomly generated coin flip. The result is a boolean indicating whether
    /// the player won (true) or lost (false).
    #[arcium_callback(encrypted_ix = "flip")]
    pub fn flip_callback(
        ctx: Context<FlipCallback>,
        output: ComputationOutputs<FlipOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(FlipOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        emit!(FlipEvent { result: o });

        Ok(())
    }
}

#[queue_computation_accounts("flip", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct Flip<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!()
    )]
    /// CHECK: mempool_account, checked by the arcium program
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_FLIP)
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

#[callback_accounts("flip", payer)]
#[derive(Accounts)]
pub struct FlipCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_FLIP)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("flip", payer)]
#[derive(Accounts)]
pub struct InitFlipCompDef<'info> {
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

/// Event emitted when a coin flip game completes.
#[event]
pub struct FlipEvent {
    /// Whether the player won the coin flip (true = won, false = lost)
    pub result: bool,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
