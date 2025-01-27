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

const COMP_DEF_OFFSET_BID: u32 = comp_def_offset("bid");
const COMP_DEF_OFFSET_SELL: u32 = comp_def_offset("sell");

declare_id!("5vMrxtWm3xKVgHjJGbBHmsAk7hwzXt8xmRUkEU5mmqaw");

#[arcium_program]
pub mod sealed_bid_auction {
    use super::*;

    pub fn init_bid_comp_def(ctx: Context<InitBidCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn init_sell_comp_def(ctx: Context<InitSellCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn bid(
        ctx: Context<Bid>,
        price: OffChainReference,
        bidder: OffChainReference,
    ) -> Result<()> {
        let args = vec![Argument::MU128(price), Argument::MU128(bidder)];
        queue_computation(ctx.accounts, args, vec![], vec![])?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "bid")]
    pub fn bid_callback(ctx: Context<BidCallback>, output: Vec<u8>) -> Result<()> {
        Ok(())
    }

    pub fn sell(ctx: Context<Sell>) -> Result<()> {
        Ok(())
    }

    #[arcium_callback(confidential_ix = "sell")]
    pub fn sell_callback(ctx: Context<SellCallback>, output: Vec<u8>) -> Result<()> {
        emit!(ItemSold {
            seller: ctx.accounts.seller.key(),
            price: output[0],
            buyer: output[1],
        });
        Ok(())
    }
}

#[queue_computation_accounts("bid", payer)]
#[derive(Accounts)]
pub struct Bid<'info> {
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_BID.to_le_bytes().as_ref()],
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

#[callback_accounts("add_together", payer)]
#[derive(Accounts)]
pub struct AddTogetherCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_BID.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("bid", payer)]
#[derive(Accounts)]
pub struct InitBidCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, ID_CONST.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_account.bump
    )]
    pub mxe_account: Box<Account<'info, PersistentMXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("sell", payer)]
#[derive(Accounts)]
pub struct InitSellCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, ID_CONST.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_account.bump
    )]
    pub mxe_account: Box<Account<'info, PersistentMXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Debug, InitSpace)]
#[account]
pub struct Auction {
    pub item: Pubkey,
    pub seller: Pubkey,
    pub status: AuctionStatus,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug, InitSpace)]
pub enum AuctionStatus {
    Open,
    Closed,
}


#[event]
pub struct ItemSold {
    pub seller: Pubkey,
    pub price: u128,
    pub buyer: Pubkey,
}
