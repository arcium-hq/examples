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

const COMP_DEF_OFFSET_GENERATE_DECK_OF_SHUFFLED_CARDS: u32 =
    comp_def_offset("generate_deck_of_shuffled_cards");
const COMP_DEF_OFFSET_DEAL_CARDS: u32 = comp_def_offset("deal_cards");

declare_id!("Co1qsWYzoSMhY6SvixqDNJp6ch7B9YgbJTZwgejcsH6L");

#[arcium_program]
pub mod blackjack {

    use super::*;

    pub fn init_generate_deck_of_shuffled_cards_comp_def(
        ctx: Context<InitGenerateDeckOfShuffledCardsCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn initialize_blackjack_game(
        ctx: Context<InitializeBlackjackGame>,
        nonce: u128,
    ) -> Result<()> {
        ctx.accounts.blackjack_game.bump = ctx.bumps.blackjack_game;

        let args = vec![Argument::PlaintextU128(nonce)];
        queue_computation(
            ctx.accounts,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "generate_deck_of_shuffled_cards")]
    pub fn generate_deck_of_shuffled_cards_callback(
        ctx: Context<GenerateDeckOfShuffledCardsCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        let deck: [[u8; 32]; 3] = bytes[16..]
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.deck = deck;

        emit!(CardsShuffledEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }

    pub fn init_deal_cards_comp_def(ctx: Context<InitDealCardsCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn deal_cards(ctx: Context<DealCards>, nonce: u128) -> Result<()> {
        let args = vec![
            Argument::PlaintextU128(nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
        ];
        queue_computation(
            ctx.accounts,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "deal_cards")]
    pub fn deal_cards_callback(
        ctx: Context<DealCardsCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        let deck: [[u8; 32]; 3] = bytes[16..]
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.deck = deck;

        emit!(CardsShuffledEvent {
            timestamp: Clock::get()?.unix_timestamp,
        });
        Ok(())
    }
}

#[queue_computation_accounts("generate_deck_of_shuffled_cards", payer)]
#[derive(Accounts)]
pub struct InitializeBlackjackGame<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_GENERATE_DECK_OF_SHUFFLED_CARDS)
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
    #[account(
        init,
        payer = payer,
        space = 8 + BlackjackGame::INIT_SPACE,
        seeds = [b"blackjack_game".as_ref()],
        bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("generate_deck_of_shuffled_cards", payer)]
#[derive(Accounts)]
pub struct GenerateDeckOfShuffledCardsCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_GENERATE_DECK_OF_SHUFFLED_CARDS)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("generate_deck_of_shuffled_cards", payer)]
#[derive(Accounts)]
pub struct InitGenerateDeckOfShuffledCardsCompDef<'info> {
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

#[queue_computation_accounts("deal_cards", payer)]
#[derive(Accounts)]
pub struct DealCards<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEAL_CARDS)
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
    #[account(
        seeds = [b"blackjack_game".as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("deal_cards", payer)]
#[derive(Accounts)]
pub struct DealCardsCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEAL_CARDS)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("deal_cards", payer)]
#[derive(Accounts)]
pub struct InitDealCardsCompDef<'info> {
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
pub struct BlackjackGame {
    pub deck: [[u8; 32]; 3],
    // pub player_hand: [u8; 2],
    // pub dealer_hand: [u8; 2],
    pub bump: u8,
}

#[event]
pub struct CardsShuffledEvent {
    pub timestamp: i64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
}
