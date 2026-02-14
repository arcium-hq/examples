use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

const COMP_DEF_OFFSET_INIT_AUCTION_STATE: u32 = comp_def_offset("init_auction_state");
const COMP_DEF_OFFSET_PLACE_BID: u32 = comp_def_offset("place_bid");
const COMP_DEF_OFFSET_DETERMINE_WINNER_FIRST_PRICE: u32 =
    comp_def_offset("determine_winner_first_price");
const COMP_DEF_OFFSET_DETERMINE_WINNER_VICKREY: u32 = comp_def_offset("determine_winner_vickrey");

declare_id!("CHFR2eD8dmZ5NM7UbwM7nWFVTfWPpdtKfv6H4Bgtha3e");

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum AuctionType {
    FirstPrice,
    Vickrey,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum AuctionStatus {
    Open,
    Closed,
    Resolved,
}

#[arcium_program]
pub mod sealed_bid_auction {
    use super::*;

    pub fn init_auction_state_comp_def(ctx: Context<InitAuctionStateCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_place_bid_comp_def(ctx: Context<InitPlaceBidCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_determine_winner_first_price_comp_def(
        ctx: Context<InitDetermineWinnerFirstPriceCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn init_determine_winner_vickrey_comp_def(
        ctx: Context<InitDetermineWinnerVickreyCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    pub fn create_auction(
        ctx: Context<CreateAuction>,
        computation_offset: u64,
        auction_type: AuctionType,
        min_bid: u64,
        end_time: i64,
        nonce: u128,
    ) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        auction.bump = ctx.bumps.auction;
        auction.authority = ctx.accounts.authority.key();
        auction.auction_type = auction_type;
        auction.status = AuctionStatus::Open;
        auction.min_bid = min_bid;
        auction.end_time = end_time;
        auction.bid_count = 0;
        auction.state_nonce = nonce;
        auction.encrypted_state = [[0u8; 32]; 5];

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        let args = ArgBuilder::new().plaintext_u128(nonce).build();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![InitAuctionStateCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: ctx.accounts.auction.key(),
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "init_auction_state")]
    pub fn init_auction_state_callback(
        ctx: Context<InitAuctionStateCallback>,
        output: SignedComputationOutputs<InitAuctionStateOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(InitAuctionStateOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let auction_key = ctx.accounts.auction.key();
        let authority = ctx.accounts.auction.authority;
        let auction_type = ctx.accounts.auction.auction_type;
        let min_bid = ctx.accounts.auction.min_bid;
        let end_time = ctx.accounts.auction.end_time;

        let auction = &mut ctx.accounts.auction;
        auction.encrypted_state = o.ciphertexts;
        auction.state_nonce = o.nonce;

        emit!(AuctionCreatedEvent {
            auction: auction_key,
            authority,
            auction_type,
            min_bid,
            end_time,
        });

        Ok(())
    }

    pub fn place_bid(
        ctx: Context<PlaceBid>,
        computation_offset: u64,
        encrypted_bidder_lo: [u8; 32],
        encrypted_bidder_hi: [u8; 32],
        encrypted_amount: [u8; 32],
        bidder_pubkey: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let auction = &ctx.accounts.auction;
        require!(
            auction.status == AuctionStatus::Open,
            ErrorCode::AuctionNotOpen
        );

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        // Account offset: 8 (discriminator) + 1 + 32 + 1 + 1 + 8 + 8 + 1 + 16 = 76
        const ENCRYPTED_STATE_OFFSET: u32 = 76;
        const ENCRYPTED_STATE_SIZE: u32 = 32 * 5;

        let args = ArgBuilder::new()
            .x25519_pubkey(bidder_pubkey)
            .plaintext_u128(nonce)
            .encrypted_u128(encrypted_bidder_lo)
            .encrypted_u128(encrypted_bidder_hi)
            .encrypted_u64(encrypted_amount)
            .plaintext_u128(auction.state_nonce)
            .account(
                ctx.accounts.auction.key(),
                ENCRYPTED_STATE_OFFSET,
                ENCRYPTED_STATE_SIZE,
            )
            .build();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![PlaceBidCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: ctx.accounts.auction.key(),
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "place_bid")]
    pub fn place_bid_callback(
        ctx: Context<PlaceBidCallback>,
        output: SignedComputationOutputs<PlaceBidOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(PlaceBidOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let auction_key = ctx.accounts.auction.key();
        let auction = &mut ctx.accounts.auction;
        auction.encrypted_state = o.ciphertexts;
        auction.state_nonce = o.nonce;
        auction.bid_count += 1;

        emit!(BidPlacedEvent {
            auction: auction_key,
            bid_count: auction.bid_count,
        });

        Ok(())
    }

    pub fn close_auction(ctx: Context<CloseAuction>) -> Result<()> {
        let auction = &mut ctx.accounts.auction;
        require!(
            auction.status == AuctionStatus::Open,
            ErrorCode::AuctionNotOpen
        );
        auction.status = AuctionStatus::Closed;

        emit!(AuctionClosedEvent {
            auction: auction.key(),
            bid_count: auction.bid_count,
        });

        Ok(())
    }

    pub fn determine_winner_first_price(
        ctx: Context<DetermineWinnerFirstPrice>,
        computation_offset: u64,
    ) -> Result<()> {
        let auction = &ctx.accounts.auction;
        require!(
            auction.status == AuctionStatus::Closed,
            ErrorCode::AuctionNotClosed
        );
        require!(
            auction.auction_type == AuctionType::FirstPrice,
            ErrorCode::WrongAuctionType
        );

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        const ENCRYPTED_STATE_OFFSET: u32 = 8 + 1 + 32 + 1 + 1 + 8 + 8 + 1 + 16;
        const ENCRYPTED_STATE_SIZE: u32 = 32 * 5;

        let args = ArgBuilder::new()
            .plaintext_u128(auction.state_nonce)
            .account(
                ctx.accounts.auction.key(),
                ENCRYPTED_STATE_OFFSET,
                ENCRYPTED_STATE_SIZE,
            )
            .build();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![DetermineWinnerFirstPriceCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: ctx.accounts.auction.key(),
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "determine_winner_first_price")]
    pub fn determine_winner_first_price_callback(
        ctx: Context<DetermineWinnerFirstPriceCallback>,
        output: SignedComputationOutputs<DetermineWinnerFirstPriceOutput>,
    ) -> Result<()> {
        let (winner_lo, winner_hi, payment_amount) = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(DetermineWinnerFirstPriceOutput {
                field_0:
                    DetermineWinnerFirstPriceOutputStruct0 {
                        field_0:
                            DetermineWinnerFirstPriceOutputStruct00 {
                                field_0: winner_lo,
                                field_1: winner_hi,
                            },
                        field_1: payment_amount,
                    },
            }) => (winner_lo, winner_hi, payment_amount),
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let mut winner = [0u8; 32];
        winner[..16].copy_from_slice(&winner_lo.to_le_bytes());
        winner[16..].copy_from_slice(&winner_hi.to_le_bytes());

        let auction_key = ctx.accounts.auction.key();
        let auction_type = ctx.accounts.auction.auction_type;
        let auction = &mut ctx.accounts.auction;
        auction.status = AuctionStatus::Resolved;

        emit!(AuctionResolvedEvent {
            auction: auction_key,
            winner,
            payment_amount,
            auction_type,
        });

        Ok(())
    }

    pub fn determine_winner_vickrey(
        ctx: Context<DetermineWinnerVickrey>,
        computation_offset: u64,
    ) -> Result<()> {
        let auction = &ctx.accounts.auction;
        require!(
            auction.status == AuctionStatus::Closed,
            ErrorCode::AuctionNotClosed
        );
        require!(
            auction.auction_type == AuctionType::Vickrey,
            ErrorCode::WrongAuctionType
        );

        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;

        const ENCRYPTED_STATE_OFFSET: u32 = 8 + 1 + 32 + 1 + 1 + 8 + 8 + 1 + 16;
        const ENCRYPTED_STATE_SIZE: u32 = 32 * 5;

        let args = ArgBuilder::new()
            .plaintext_u128(auction.state_nonce)
            .account(
                ctx.accounts.auction.key(),
                ENCRYPTED_STATE_OFFSET,
                ENCRYPTED_STATE_SIZE,
            )
            .build();

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![DetermineWinnerVickreyCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[CallbackAccount {
                    pubkey: ctx.accounts.auction.key(),
                    is_writable: true,
                }],
            )?],
            1,
            0,
        )?;

        Ok(())
    }

    #[arcium_callback(encrypted_ix = "determine_winner_vickrey")]
    pub fn determine_winner_vickrey_callback(
        ctx: Context<DetermineWinnerVickreyCallback>,
        output: SignedComputationOutputs<DetermineWinnerVickreyOutput>,
    ) -> Result<()> {
        let (winner_lo, winner_hi, payment_amount) = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(DetermineWinnerVickreyOutput {
                field_0:
                    DetermineWinnerVickreyOutputStruct0 {
                        field_0:
                            DetermineWinnerVickreyOutputStruct00 {
                                field_0: winner_lo,
                                field_1: winner_hi,
                            },
                        field_1: payment_amount,
                    },
            }) => (winner_lo, winner_hi, payment_amount),
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        let mut winner = [0u8; 32];
        winner[..16].copy_from_slice(&winner_lo.to_le_bytes());
        winner[16..].copy_from_slice(&winner_hi.to_le_bytes());

        let auction_key = ctx.accounts.auction.key();
        let auction_type = ctx.accounts.auction.auction_type;
        let auction = &mut ctx.accounts.auction;
        auction.status = AuctionStatus::Resolved;

        emit!(AuctionResolvedEvent {
            auction: auction_key,
            winner,
            payment_amount,
            auction_type,
        });

        Ok(())
    }
}

#[account]
#[derive(InitSpace)]
pub struct Auction {
    pub bump: u8,
    pub authority: Pubkey,
    pub auction_type: AuctionType,
    pub status: AuctionStatus,
    pub min_bid: u64,
    pub end_time: i64,
    pub bid_count: u8,
    pub state_nonce: u128,
    pub encrypted_state: [[u8; 32]; 5],
}

#[queue_computation_accounts("init_auction_state", authority)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct CreateAuction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = 8 + Auction::INIT_SPACE,
        seeds = [b"auction", authority.key().as_ref()],
        bump,
    )]
    pub auction: Account<'info, Auction>,
    #[account(
        init_if_needed,
        space = 9,
        payer = authority,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_AUCTION_STATE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("init_auction_state")]
#[derive(Accounts)]
pub struct InitAuctionStateCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_INIT_AUCTION_STATE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
}

#[queue_computation_accounts("place_bid", bidder)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct PlaceBid<'info> {
    #[account(mut)]
    pub bidder: Signer<'info>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
    #[account(
        init_if_needed,
        space = 9,
        payer = bidder,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLACE_BID))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("place_bid")]
#[derive(Accounts)]
pub struct PlaceBidCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLACE_BID))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
}

#[derive(Accounts)]
pub struct CloseAuction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        has_one = authority @ ErrorCode::Unauthorized,
    )]
    pub auction: Account<'info, Auction>,
}

#[queue_computation_accounts("determine_winner_first_price", authority)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct DetermineWinnerFirstPrice<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ ErrorCode::Unauthorized)]
    pub auction: Account<'info, Auction>,
    #[account(
        init_if_needed,
        space = 9,
        payer = authority,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DETERMINE_WINNER_FIRST_PRICE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("determine_winner_first_price")]
#[derive(Accounts)]
pub struct DetermineWinnerFirstPriceCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DETERMINE_WINNER_FIRST_PRICE))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
}

#[queue_computation_accounts("determine_winner_vickrey", authority)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct DetermineWinnerVickrey<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ ErrorCode::Unauthorized)]
    pub auction: Account<'info, Auction>,
    #[account(
        init_if_needed,
        space = 9,
        payer = authority,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(mut, address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(mut, address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet))]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DETERMINE_WINNER_VICKREY))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(mut, address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(mut, address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS)]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS)]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("determine_winner_vickrey")]
#[derive(Accounts)]
pub struct DetermineWinnerVickreyCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(address = derive_comp_def_pda!(COMP_DEF_OFFSET_DETERMINE_WINNER_VICKREY))]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = derive_mxe_pda!())]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet))]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub auction: Account<'info, Auction>,
}

#[init_computation_definition_accounts("init_auction_state", payer)]
#[derive(Accounts)]
pub struct InitAuctionStateCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("place_bid", payer)]
#[derive(Accounts)]
pub struct InitPlaceBidCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("determine_winner_first_price", payer)]
#[derive(Accounts)]
pub struct InitDetermineWinnerFirstPriceCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("determine_winner_vickrey", payer)]
#[derive(Accounts)]
pub struct InitDetermineWinnerVickreyCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut, address = derive_mxe_pda!())]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    pub comp_def_account: UncheckedAccount<'info>,
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[event]
pub struct AuctionCreatedEvent {
    pub auction: Pubkey,
    pub authority: Pubkey,
    pub auction_type: AuctionType,
    pub min_bid: u64,
    pub end_time: i64,
}

#[event]
pub struct BidPlacedEvent {
    pub auction: Pubkey,
    pub bid_count: u8,
}

#[event]
pub struct AuctionClosedEvent {
    pub auction: Pubkey,
    pub bid_count: u8,
}

#[event]
pub struct AuctionResolvedEvent {
    pub auction: Pubkey,
    pub winner: [u8; 32],
    pub payment_amount: u64,
    pub auction_type: AuctionType,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
    #[msg("Auction is not open for bidding")]
    AuctionNotOpen,
    #[msg("Auction is not closed yet")]
    AuctionNotClosed,
    #[msg("Wrong auction type for this operation")]
    WrongAuctionType,
    #[msg("Unauthorized")]
    Unauthorized,
}
