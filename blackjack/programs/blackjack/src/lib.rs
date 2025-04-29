#[allow(unused_assignments)]
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

const COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS: u32 = comp_def_offset("shuffle_and_deal_cards");
const COMP_DEF_OFFSET_DEAL_CARDS: u32 = comp_def_offset("deal_cards");

declare_id!("8YLMpSEWaLzpGqqGufakQ8FtPzvSm5kdm5VpsVPHeZTP");

#[arcium_program]
pub mod blackjack {
    use super::*;

    pub fn init_shuffle_and_deal_cards_comp_def(
        ctx: Context<InitShuffleAndDealCardsCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn initialize_blackjack_game(
        ctx: Context<InitializeBlackjackGame>,
        game_id: u64,
        mxe_nonce: u128,
        mxe_again_nonce: u128,
        client_pubkey: [u8; 32],
        client_nonce: u128,
    ) -> Result<()> {
        // Initialize the blackjack game account
        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.bump = ctx.bumps.blackjack_game;
        blackjack_game.game_id = game_id;
        blackjack_game.player_pubkey = ctx.accounts.payer.key();
        blackjack_game.player_hand = [[0; 32]; 11];
        blackjack_game.dealer_hand = [[0; 32]; 11];
        blackjack_game.deck_nonce = [0; 16];
        blackjack_game.client_nonce = [0; 16];
        blackjack_game.dealer_nonce = [0; 16];
        blackjack_game.player_enc_pubkey = client_pubkey;
        blackjack_game.game_state = 0;
        blackjack_game.player_hand_size = [0; 32];
        blackjack_game.dealer_hand_size = [0; 32];

        // Queue the shuffle and deal cards computation
        let args = vec![
            Argument::PlaintextU128(mxe_nonce),
            Argument::PlaintextU128(mxe_again_nonce),
            Argument::ArcisPubkey(client_pubkey),
            Argument::PlaintextU128(client_nonce),
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

    #[arcium_callback(encrypted_ix = "shuffle_and_deal_cards")]
    pub fn shuffle_and_deal_cards_callback(
        ctx: Context<ShuffleAndDealCardsCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        // Keep track of the offset in the bytes array
        let mut offset = 0;

        let deck_nonce: [u8; 16] = bytes[offset..(offset + 16)].try_into().unwrap();
        offset += 16;

        let deck: [[u8; 32]; 3] = bytes[offset..(offset + 32 * 3)]
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        offset += 32 * 3;

        let dealer_nonce: [u8; 16] = bytes[offset..(offset + 16)].try_into().unwrap();
        offset += 16;

        let dealer_face_down_card: [u8; 32] = bytes[offset..(offset + 32)].try_into().unwrap();
        offset += 32;

        let client_pubkey: [u8; 32] = bytes[offset..(offset + 32)].try_into().unwrap();
        offset += 32;

        let client_nonce: [u8; 16] = bytes[offset..(offset + 16)].try_into().unwrap();
        offset += 16;

        let visible_cards: [[u8; 32]; 3] = bytes[offset..(offset + 32 * 3)]
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        offset += 32 * 3;

        let player_num_cards: u8 = bytes[offset];
        offset += 1;

        let dealer_num_cards: u8 = bytes[offset];
        offset += 1;

        // Update the blackjack game account
        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.deck = deck;
        blackjack_game.deck_nonce = deck_nonce;
        blackjack_game.client_nonce = client_nonce;
        blackjack_game.dealer_nonce = dealer_nonce;
        blackjack_game.player_enc_pubkey = client_pubkey;
        blackjack_game.game_state = 1; // It is now the player's turn

        // Initialize player hand with first two cards
        blackjack_game.player_hand[0] = visible_cards[0];
        blackjack_game.player_hand[1] = visible_cards[1];
        // Initialize dealer hand with face up card and face down card
        blackjack_game.dealer_hand[0] = visible_cards[2];
        blackjack_game.dealer_hand[1] = dealer_face_down_card;
        blackjack_game.player_hand_size = player_num_cards;
        blackjack_game.dealer_hand_size = dealer_num_cards;

        // Assert that we have read the entire bytes array
        assert_eq!(offset, bytes.len());

        emit!(CardsShuffledAndDealtEvent {
            client_nonce,
            user_hand: [visible_cards[0], visible_cards[1]],
            dealer_face_up_card: visible_cards[2],
        });
        Ok(())
    }

    pub fn init_deal_cards_comp_def(ctx: Context<InitDealCardsCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn deal_cards(ctx: Context<DealCards>, _game_id: u64) -> Result<()> {
        let args = vec![
            Argument::PlaintextU128(u128::from_le_bytes(ctx.accounts.blackjack_game.deck_nonce)),
            Argument::Account(ctx.accounts.blackjack_game.key(), 0, 32 * 3),
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

        let card: u8 = bytes[0];

        emit!(CardDealtEvent { card });
        Ok(())
    }
}

#[queue_computation_accounts("shuffle_and_deal_cards", payer)]
#[derive(Accounts)]
#[instruction(game_id: u64)]
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS)
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
        seeds = [b"blackjack_game".as_ref(), game_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("shuffle_and_deal_cards", payer)]
#[derive(Accounts)]
pub struct ShuffleAndDealCardsCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("shuffle_and_deal_cards", payer)]
#[derive(Accounts)]
pub struct InitShuffleAndDealCardsCompDef<'info> {
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
#[instruction(_game_id: u64)]
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
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
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
    pub player_hand: [[u8; 32]; 11],
    pub dealer_hand: [[u8; 32]; 11],
    pub deck_nonce: [u8; 16],
    pub client_nonce: [u8; 16],
    pub dealer_nonce: [u8; 16],
    pub game_id: u64,
    pub player_pubkey: Pubkey,
    pub player_enc_pubkey: [u8; 32],
    pub bump: u8,
    pub game_state: u8, // 0 = initial, 1 = player turn, 2 = dealer turn, 3 = resolved
    pub player_hand_size: u8, // Number of cards in player's hand
    pub dealer_hand_size: u8, // Number of cards in dealer's hand
                        // pub player_bet: u64, // Player's current bet
                        // pub player_has_doubled: bool, // Whether player has doubled down
                        // pub game_result: u8, // Result of the game (0-4)
}

#[event]
pub struct CardsShuffledAndDealtEvent {
    pub client_nonce: [u8; 16],
    pub user_hand: [[u8; 32]; 2],
    pub dealer_face_up_card: [u8; 32],
}

#[event]
pub struct CardDealtEvent {
    pub card: u8,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Invalid game state")]
    InvalidGameState,
}
