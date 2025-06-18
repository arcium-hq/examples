use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_SETUP_VICKREY_AUCTION: u32 = comp_def_offset("setup_vickrey_auction");
const COMP_DEF_OFFSET_VICKREY_AUCTION_PLACE_BID: u32 = comp_def_offset("vickrey_auction_place_bid");
const COMP_DEF_OFFSET_VICKREY_AUCTION_REVEAL_RESULT: u32 =
    comp_def_offset("vickrey_auction_reveal_result");

declare_id!("5H6iGeeL3jiLs9NSkHd8EXMC1HECBX7dV8TfwzNm6sQV");

#[arcium_program]
pub mod sealed_bid_auction {
    use super::*;

    pub fn init_setup_vickrey_auction_comp_def(
        ctx: Context<InitSetupVickreyAuctionCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn setup_vickrey_auction(
        ctx: Context<SetupVickreyAuction>,
        computation_offset: u64,
    ) -> Result<()> {
        let args = vec![];
        queue_computation(ctx.accounts, computation_offset, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "setup_vickrey_auction")]
    pub fn setup_vickrey_auction_callback(
        ctx: Context<SetupVickreyAuctionCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        emit!(VickreyAuctionSetupEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }
}

#[queue_computation_accounts("setup_vickrey_auction", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct SetupVickreyAuction<'info> {
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
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!()
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SETUP_VICKREY_AUCTION)
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
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("setup_vickrey_auction", payer)]
#[derive(Accounts)]
pub struct SetupVickreyAuctionCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SETUP_VICKREY_AUCTION)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("setup_vickrey_auction", payer)]
#[derive(Accounts)]
pub struct InitSetupVickreyAuctionCompDef<'info> {
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

#[event]
pub struct VickreyAuctionSetupEvent {
    pub timestamp: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
