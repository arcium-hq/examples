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
    types::Argument,
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    queue_computation_accounts,
};

const COMP_DEF_VICKREY_AUCTION_BID: u32 = comp_def_offset("vickrey_bid");
const COMP_DEF_VICKREY_AUCTION_REVEAL: u32 = comp_def_offset("vickrey_reveal");

declare_id!("HAmy4nhyS3uobDtyjDy5GutzCp3osznvBpbjzoNg6LzU");

#[arcium_program]
pub mod sealed_bid_auction {
    use super::*;

    pub fn init_vickrey_auction_bid_comp_def(
        ctx: Context<InitVickreyAuctionBidCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn init_vickrey_auction_reveal_comp_def(
        ctx: Context<InitVickreyAuctionRevealCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn vickrey_bid(
        ctx: Context<VickreyAuctionBid>,
        bid: [u8; 32],
        bidder_pubkey_one: [u8; 32],
        bidder_pubkey_two: [u8; 32],
        encryption_pub_key: [u8; 32],
        bid_nonce: u128,
        nonce: u128,
    ) -> Result<()> {
        let args = vec![
            Argument::PlaintextU128(nonce),
            // Argument::Account(),
            Argument::EncryptedU128(encryption_pub_key),
            Argument::PlaintextU128(bid_nonce),
            Argument::EncryptedU128(bid),
            Argument::EncryptedU128(bidder_pubkey_one),
            Argument::EncryptedU128(bidder_pubkey_two),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "vickrey_bid")]
    pub fn vickrey_bid_callback(
        ctx: Context<VickreyAuctionBidCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        emit!(BidEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn vickrey_reveal(ctx: Context<VickreyAuctionReveal>) -> Result<()> {
        let args = vec![
            Argument::PublicKey(ctx.accounts.auction_account.encryption_pubkey),
            Argument::PlaintextU128(ctx.accounts.auction_account.nonce),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "vickrey_reveal")]
    pub fn vickrey_reveal_callback(
        ctx: Context<VickreyAuctionRevealCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        emit!(RevealEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }
}

#[queue_computation_accounts("vickrey_bid", payer)]
#[derive(Accounts)]
pub struct VickreyAuctionBid<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_VICKREY_AUCTION_BID)
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

#[callback_accounts("vickrey_bid", payer)]
#[derive(Accounts)]
pub struct VickreyAuctionBidCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_VICKREY_AUCTION_BID)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("vickrey_bid", payer)]
#[derive(Accounts)]
pub struct InitVickreyAuctionBidCompDef<'info> {
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

#[queue_computation_accounts("vickrey_reveal", payer)]
#[derive(Accounts)]
pub struct VickreyAuctionReveal<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_VICKREY_AUCTION_REVEAL)
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
    #[account(mut)]
    pub auction_account: Account<'info, Auction>,
}

#[callback_accounts("vickrey_reveal", payer)]
#[derive(Accounts)]
pub struct VickreyAuctionRevealCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_VICKREY_AUCTION_REVEAL)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("vickrey_reveal", payer)]
#[derive(Accounts)]
pub struct InitVickreyAuctionRevealCompDef<'info> {
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
pub struct Auction {
    pub encryption_pubkey: [u8; 32],
    pub nonce: u128,
}

#[event]
pub struct BidEvent {
    pub timestamp: i64,
}

#[event]
pub struct RevealEvent {
    pub timestamp: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
