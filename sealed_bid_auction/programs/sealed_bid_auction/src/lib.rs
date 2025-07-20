use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

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
        auction_id: u64,
    ) -> Result<()> {
        let args = vec![];

        let mut auction = &mut ctx.accounts.vickrey_auction_account;
        auction.auctioneer = ctx.accounts.payer.key();
        auction.bump = ctx.bumps.vickrey_auction_account;
        auction.status = 0;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.vickrey_auction_account.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "setup_vickrey_auction")]
    pub fn setup_vickrey_auction_callback(
        ctx: Context<SetupVickreyAuctionCallback>,
        output: ComputationOutputs<SetupVickreyAuctionOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(SetupVickreyAuctionOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let auction = &mut ctx.accounts.vickrey_auction_account;
        auction.nonce = o.nonce.to_le_bytes();
        auction.vickrey_auction_data = o.ciphertexts;
        auction.status = 1;

        emit!(VickreyAuctionSetupEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn init_vickrey_auction_place_bid_comp_def(
        ctx: Context<InitVickreyAuctionPlaceBidCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn vickrey_auction_place_bid(
        ctx: Context<VickreyAuctionPlaceBid>,
        computation_offset: u64,
        auction_id: u64,
        bid_value: [u8; 32],
        bid_pubkey_hi: [u8; 32],
        bid_pubkey_lo: [u8; 32],
        bid_encryption_pubkey: [u8; 32],
        bid_encryption_nonce: u128,
    ) -> Result<()> {
        require_eq!(ctx.accounts.vickrey_auction_account.status, 1);
        let args = vec![
            Argument::ArcisPubkey(bid_encryption_pubkey),
            Argument::PlaintextU128(bid_encryption_nonce),
            Argument::EncryptedU128(bid_value),
            Argument::EncryptedU128(bid_pubkey_hi),
            Argument::EncryptedU128(bid_pubkey_lo),
            Argument::PlaintextU128(u128::from_le_bytes(
                ctx.accounts.vickrey_auction_account.nonce,
            )),
            Argument::Account(
                ctx.accounts.vickrey_auction_account.key(),
                // Offset of 8 (discriminator) + 32 (authority pubkey)
                8 + 32,
                // 32 bytes for each of the 6 vickrey auction data points stored as ciphertext
                32 * 6,
            ),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.vickrey_auction_account.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "vickrey_auction_place_bid")]
    pub fn vickrey_auction_place_bid_callback(
        ctx: Context<VickreyAuctionPlaceBidCallback>,
        output: ComputationOutputs<VickreyAuctionPlaceBidOutput>,
    ) -> Result<()> {
        require_eq!(ctx.accounts.vickrey_auction_account.status, 1);

        let o = match output {
            ComputationOutputs::Success(VickreyAuctionPlaceBidOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let auction = &mut ctx.accounts.vickrey_auction_account;
        auction.nonce = o.nonce.to_le_bytes();
        auction.vickrey_auction_data = o.ciphertexts;

        emit!(VickreyAuctionPlaceBidEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn init_vickrey_auction_reveal_result_comp_def(
        ctx: Context<InitVickreyAuctionRevealResultCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn vickrey_auction_reveal_result(
        ctx: Context<VickreyAuctionRevealResult>,
        computation_offset: u64,
    ) -> Result<()> {
        require_eq!(ctx.accounts.vickrey_auction_account.status, 1);

        let args = vec![
            Argument::PlaintextU128(u128::from_le_bytes(
                ctx.accounts.vickrey_auction_account.nonce,
            )),
            Argument::Account(
                ctx.accounts.vickrey_auction_account.key(),
                // Offset of 8 (discriminator) + 32 (authority pubkey)
                8 + 32,
                // 32 bytes for each of the 6 vickrey auction data points stored as ciphertext
                32 * 6,
            ),
        ];

        let auction = &mut ctx.accounts.vickrey_auction_account;
        auction.status = 2;

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.vickrey_auction_account.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "vickrey_auction_reveal_result")]
    pub fn vickrey_auction_reveal_result_callback(
        ctx: Context<VickreyAuctionRevealResultCallback>,
        output: ComputationOutputs<VickreyAuctionRevealResultOutput>,
    ) -> Result<()> {
        let (winning_pubkey, bid) = match output {
            ComputationOutputs::Success(VickreyAuctionRevealResultOutput {
                field_0:
                    VickreyAuctionRevealResultTupleStruct0 {
                        field_0: winning_pubkey,
                        field_1: bid,
                    },
            }) => (winning_pubkey, bid),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        require_eq!(ctx.accounts.vickrey_auction_account.status, 2);
        // let pubkey_hi: [u8; 16] = bytes[0..16].try_into().unwrap();
        // let pubkey_lo: [u8; 16] = bytes[16..32].try_into().unwrap();

        let auction = &mut ctx.accounts.vickrey_auction_account;
        auction.status = 3;

        emit!(VickreyAuctionRevealResultEvent {
            timestamp: Clock::get()?.unix_timestamp,
            pubkey_hi: winning_pubkey[0..16],
            pubkey_lo: winning_pubkey[16..32],
            bid,
        });

        Ok(())
    }
}

#[queue_computation_accounts("setup_vickrey_auction", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, auction_id: u64)]
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
    #[account(
        init,
        payer = payer,
        space = 8 + VickreyAuctionAccount::INIT_SPACE,
        seeds = [b"vickrey_auction_account", auction_id.to_le_bytes().as_ref()],
        bump
    )]
    pub vickrey_auction_account: Account<'info, VickreyAuctionAccount>,
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
    #[account(mut)]
    pub vickrey_auction_account: Account<'info, VickreyAuctionAccount>,
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

#[queue_computation_accounts("vickrey_auction_place_bid", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct VickreyAuctionPlaceBid<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VICKREY_AUCTION_PLACE_BID)
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
    pub vickrey_auction_account: Account<'info, VickreyAuctionAccount>,
}

#[callback_accounts("vickrey_auction_place_bid", payer)]
#[derive(Accounts)]
pub struct VickreyAuctionPlaceBidCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VICKREY_AUCTION_PLACE_BID)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub vickrey_auction_account: Account<'info, VickreyAuctionAccount>,
}

#[init_computation_definition_accounts("vickrey_auction_place_bid", payer)]
#[derive(Accounts)]
pub struct InitVickreyAuctionPlaceBidCompDef<'info> {
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

#[queue_computation_accounts("vickrey_auction_reveal_result", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct VickreyAuctionRevealResult<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VICKREY_AUCTION_REVEAL_RESULT)
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
    #[account(mut)]
    pub vickrey_auction_account: Account<'info, VickreyAuctionAccount>,
}

#[callback_accounts("vickrey_auction_reveal_result", payer)]
#[derive(Accounts)]
pub struct VickreyAuctionRevealResultCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VICKREY_AUCTION_REVEAL_RESULT)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub vickrey_auction_account: Account<'info, VickreyAuctionAccount>,
}

#[init_computation_definition_accounts("vickrey_auction_reveal_result", payer)]
#[derive(Accounts)]
pub struct InitVickreyAuctionRevealResultCompDef<'info> {
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
pub struct VickreyAuctionAccount {
    pub auctioneer: Pubkey,
    pub vickrey_auction_data: [[u8; 32]; 6],
    pub nonce: [u8; 16],
    pub bump: u8,
    pub status: u8, // 0: initialized, 1: bidding, 2: revealing, 3: revealed
}

#[event]
pub struct VickreyAuctionSetupEvent {
    pub timestamp: i64,
}

#[event]
pub struct VickreyAuctionPlaceBidEvent {
    pub timestamp: i64,
}

#[event]
pub struct VickreyAuctionRevealResultEvent {
    pub timestamp: i64,
    pub pubkey_hi: [u8; 16],
    pub pubkey_lo: [u8; 16],
    pub bid: u128,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
