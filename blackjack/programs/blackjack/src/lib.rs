use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;
use arcium_client::idl::arcium::types::CallbackAccount;

const COMP_DEF_OFFSET_SHUFFLE_AND_DEAL_CARDS: u32 = comp_def_offset("shuffle_and_deal_cards");
const COMP_DEF_OFFSET_PLAYER_HIT: u32 = comp_def_offset("player_hit");
const COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN: u32 = comp_def_offset("player_double_down");
const COMP_DEF_OFFSET_PLAYER_STAND: u32 = comp_def_offset("player_stand");
const COMP_DEF_OFFSET_DEALER_PLAY: u32 = comp_def_offset("dealer_play");
const COMP_DEF_OFFSET_RESOLVE_GAME: u32 = comp_def_offset("resolve_game");

declare_id!("A7sNeBnrQAFxmj6BVmoYC6PYnebURaar7xhKuaEyRh4j");

#[arcium_program]
pub mod blackjack {
    use super::*;

    /// Initializes the computation definition for shuffling and dealing cards.
    /// This sets up the MPC environment for the initial deck shuffle and card dealing operation.
    pub fn init_shuffle_and_deal_cards_comp_def(
        ctx: Context<InitShuffleAndDealCardsCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    /// Creates a new blackjack game session and initiates the deck shuffle.
    ///
    /// This function sets up a new game account with initial state and triggers the MPC computation
    /// to shuffle a standard 52-card deck and deal the opening hands (2 cards each to player and dealer).
    /// The actual shuffling and dealing happens confidentially within the Arcium network.
    ///
    /// # Arguments
    /// * `game_id` - Unique identifier for this game session
    /// * `mxe_nonce` - Cryptographic nonce for MXE operations  
    /// * `client_pubkey` - Player's encryption public key for receiving encrypted cards
    /// * `client_nonce` - Player's cryptographic nonce for encryption operations
    pub fn initialize_blackjack_game(
        ctx: Context<InitializeBlackjackGame>,
        computation_offset: u64,
        game_id: u64,
        mxe_nonce: u128,
        mxe_again_nonce: u128,
        client_pubkey: [u8; 32],
        client_nonce: u128,
        client_again_nonce: u128,
    ) -> Result<()> {
        // Initialize the blackjack game account
        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.bump = ctx.bumps.blackjack_game;
        blackjack_game.game_id = game_id;
        blackjack_game.player_pubkey = ctx.accounts.payer.key();
        blackjack_game.player_hand = [0; 32];
        blackjack_game.dealer_hand = [0; 32];
        blackjack_game.deck_nonce = 0;
        blackjack_game.client_nonce = 0;
        blackjack_game.dealer_nonce = 0;
        blackjack_game.player_enc_pubkey = client_pubkey;
        blackjack_game.game_state = GameState::Initial;
        blackjack_game.player_hand_size = 0;
        blackjack_game.dealer_hand_size = 0;

        // Queue the shuffle and deal cards computation
        let args = vec![
            Argument::PlaintextU128(mxe_nonce),
            Argument::PlaintextU128(mxe_again_nonce),
            Argument::ArcisPubkey(client_pubkey),
            Argument::PlaintextU128(client_nonce),
            Argument::ArcisPubkey(client_pubkey),
            Argument::PlaintextU128(client_again_nonce),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    /// Handles the result of the shuffle and deal cards MPC computation.
    ///
    /// This callback processes the shuffled deck and dealt cards from the MPC computation.
    /// It updates the game state with the new deck, initial hands, and sets the game to PlayerTurn.
    /// The player receives their encrypted hand while the dealer gets one face-up card visible to the player.
    #[arcium_callback(encrypted_ix = "shuffle_and_deal_cards")]
    pub fn shuffle_and_deal_cards_callback(
        ctx: Context<ShuffleAndDealCardsCallback>,
        output: ComputationOutputs<ShuffleAndDealCardsOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(ShuffleAndDealCardsOutput {
                field_0:
                    ShuffleAndDealCardsTupleStruct0 {
                        field_0: deck,
                        field_1: dealer_hand,
                        field_2: player_hand,
                        field_3: dealer_face_up_card,
                    },
            }) => (deck, dealer_hand, player_hand, dealer_face_up_card),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let deck_nonce = o.0.nonce;

        let deck: [[u8; 32]; 3] = o.0.ciphertexts;

        let dealer_nonce = o.1.nonce;

        let dealer_hand: [u8; 32] = o.1.ciphertexts[0];

        let client_pubkey: [u8; 32] = o.2.encryption_key;

        let client_nonce = o.2.nonce;

        let player_hand: [u8; 32] = o.2.ciphertexts[0];

        let dealer_client_pubkey: [u8; 32] = o.3.encryption_key;

        let dealer_client_nonce = o.3.nonce;

        let dealer_face_up_card: [u8; 32] = o.3.ciphertexts[0];

        // Update the blackjack game account
        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.deck = deck;
        blackjack_game.deck_nonce = deck_nonce;
        blackjack_game.client_nonce = client_nonce;
        blackjack_game.dealer_nonce = dealer_nonce;
        blackjack_game.player_enc_pubkey = client_pubkey;
        blackjack_game.game_state = GameState::PlayerTurn; // It is now the player's turn

        require!(
            dealer_client_pubkey == blackjack_game.player_enc_pubkey,
            ErrorCode::InvalidDealerClientPubkey
        );

        // Initialize player hand with first two cards
        blackjack_game.player_hand = player_hand;
        // Initialize dealer hand with face up card and face down card
        blackjack_game.dealer_hand = dealer_hand;
        blackjack_game.player_hand_size = 2;
        blackjack_game.dealer_hand_size = 2;

        emit!(CardsShuffledAndDealtEvent {
            client_nonce,
            dealer_client_nonce,
            player_hand,
            dealer_face_up_card,
            game_id: blackjack_game.game_id,
        });
        Ok(())
    }
    pub fn init_player_hit_comp_def(ctx: Context<InitPlayerHitCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    /// Allows the player to request an additional card (hit).
    ///
    /// This triggers an MPC computation that draws the next card from the shuffled deck
    /// and adds it to the player's hand. The computation also checks if the player busts (exceeds 21)
    /// and returns this information while keeping the actual card values encrypted.
    pub fn player_hit(
        ctx: Context<PlayerHit>,
        computation_offset: u64,
        _game_id: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.blackjack_game.game_state == GameState::PlayerTurn,
            ErrorCode::InvalidGameState
        );
        require!(
            !ctx.accounts.blackjack_game.player_has_stood,
            ErrorCode::InvalidMove
        );

        let args = vec![
            // Deck
            Argument::PlaintextU128(ctx.accounts.blackjack_game.deck_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
            // Player hand
            Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
            Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
            // Player hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
            // Dealer hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "player_hit")]
    pub fn player_hit_callback(
        ctx: Context<PlayerHitCallback>,
        output: ComputationOutputs<PlayerHitOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(PlayerHitOutput {
                field_0:
                    PlayerHitTupleStruct0 {
                        field_0: player_hand,
                        field_1: is_bust,
                    },
            }) => (player_hand, is_bust),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let client_nonce = o.0.nonce;

        let player_hand: [u8; 32] = o.0.ciphertexts[0];

        let is_bust: bool = o.1;

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.player_hand = player_hand;
        blackjack_game.client_nonce = client_nonce;

        if is_bust {
            blackjack_game.game_state = GameState::DealerTurn;
            emit!(PlayerBustEvent {
                client_nonce,
                game_id: blackjack_game.game_id,
            });
        } else {
            blackjack_game.game_state = GameState::PlayerTurn;
            emit!(PlayerHitEvent {
                player_hand,
                client_nonce,
                game_id: blackjack_game.game_id,
            });
            blackjack_game.player_hand_size += 1;
        }

        Ok(())
    }

    pub fn init_player_double_down_comp_def(
        ctx: Context<InitPlayerDoubleDownCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    pub fn player_double_down(
        ctx: Context<PlayerDoubleDown>,
        computation_offset: u64,
        _game_id: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.blackjack_game.game_state == GameState::PlayerTurn,
            ErrorCode::InvalidGameState
        );
        require!(
            !ctx.accounts.blackjack_game.player_has_stood,
            ErrorCode::InvalidMove
        );

        let args = vec![
            // Deck
            Argument::PlaintextU128(ctx.accounts.blackjack_game.deck_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
            // Player hand
            Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
            Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
            // Player hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
            // Dealer hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "player_double_down")]
    #[arcium_callback(encrypted_ix = "player_double_down")]
    pub fn player_double_down_callback(
        ctx: Context<PlayerDoubleDownCallback>,
        output: ComputationOutputs<PlayerDoubleDownOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(PlayerDoubleDownOutput {
                field_0:
                    PlayerDoubleDownTupleStruct0 {
                        field_0: player_hand,
                        field_1: is_bust,
                    },
            }) => (player_hand, is_bust),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let client_nonce = o.0.nonce;

        let player_hand: [u8; 32] = o.0.ciphertexts[0];

        let is_bust: bool = o.1;

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.player_hand = player_hand;
        blackjack_game.client_nonce = client_nonce;
        blackjack_game.player_has_stood = true;

        if is_bust {
            blackjack_game.game_state = GameState::DealerTurn;
            emit!(PlayerBustEvent {
                client_nonce,
                game_id: blackjack_game.game_id,
            });
        } else {
            blackjack_game.game_state = GameState::DealerTurn;
            emit!(PlayerDoubleDownEvent {
                player_hand,
                client_nonce,
                game_id: blackjack_game.game_id,
            });
        }

        Ok(())
    }

    pub fn init_player_stand_comp_def(ctx: Context<InitPlayerStandCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    pub fn player_stand(
        ctx: Context<PlayerStand>,
        computation_offset: u64,
        _game_id: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.blackjack_game.game_state == GameState::PlayerTurn,
            ErrorCode::InvalidGameState
        );
        require!(
            !ctx.accounts.blackjack_game.player_has_stood,
            ErrorCode::InvalidMove
        );

        let args = vec![
            // Player hand
            Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
            Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
            // Player hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "player_stand")]
    pub fn player_stand_callback(
        ctx: Context<PlayerStandCallback>,
        output: ComputationOutputs<PlayerStandOutput>,
    ) -> Result<()> {
        let is_bust = match output {
            ComputationOutputs::Success(PlayerStandOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.player_has_stood = true;

        if is_bust {
            // This should never happen
            blackjack_game.game_state = GameState::PlayerTurn;
            emit!(PlayerBustEvent {
                client_nonce: blackjack_game.client_nonce,
                game_id: blackjack_game.game_id,
            });
        } else {
            blackjack_game.game_state = GameState::DealerTurn;
            emit!(PlayerStandEvent {
                is_bust,
                game_id: blackjack_game.game_id
            });
        }

        Ok(())
    }

    pub fn init_dealer_play_comp_def(ctx: Context<InitDealerPlayCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    pub fn dealer_play(
        ctx: Context<DealerPlay>,
        computation_offset: u64,
        _game_id: u64,
        nonce: u128,
    ) -> Result<()> {
        require!(
            ctx.accounts.blackjack_game.game_state == GameState::DealerTurn,
            ErrorCode::InvalidGameState
        );

        let args = vec![
            // Deck
            Argument::PlaintextU128(ctx.accounts.blackjack_game.deck_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8, 32 * 3),
            // Dealer hand
            Argument::PlaintextU128(ctx.accounts.blackjack_game.dealer_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3 + 32, 32),
            // Client nonce
            Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
            Argument::PlaintextU128(nonce),
            // Player hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
            // Dealer hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "dealer_play")]
    pub fn dealer_play_callback(
        ctx: Context<DealerPlayCallback>,
        output: ComputationOutputs<DealerPlayOutput>,
    ) -> Result<()> {
        let o = match output {
            ComputationOutputs::Success(DealerPlayOutput {
                field_0:
                    DealerPlayTupleStruct0 {
                        field_0: dealer_hand,
                        field_1: dealer_client_hand,
                        field_2: dealer_hand_size,
                    },
            }) => (dealer_hand, dealer_client_hand, dealer_hand_size),
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        let dealer_nonce = o.0.nonce;
        let dealer_hand = o.0.ciphertexts[0];
        let dealer_client_hand = o.1.ciphertexts[0];
        let dealer_hand_size = o.2;
        let client_nonce = o.1.nonce;

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.dealer_hand = dealer_hand;
        blackjack_game.dealer_nonce = dealer_nonce;
        blackjack_game.dealer_hand_size = dealer_hand_size;
        blackjack_game.game_state = GameState::Resolving;

        emit!(DealerPlayEvent {
            dealer_hand: dealer_client_hand,
            dealer_hand_size,
            client_nonce,
            game_id: ctx.accounts.blackjack_game.game_id,
        });

        Ok(())
    }

    pub fn init_resolve_game_comp_def(ctx: Context<InitResolveGameCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, 0, None, None)?;
        Ok(())
    }

    pub fn resolve_game(
        ctx: Context<ResolveGame>,
        computation_offset: u64,
        _game_id: u64,
    ) -> Result<()> {
        require!(
            ctx.accounts.blackjack_game.game_state == GameState::Resolving,
            ErrorCode::InvalidGameState
        );

        let args = vec![
            // Player hand
            Argument::ArcisPubkey(ctx.accounts.blackjack_game.player_enc_pubkey),
            Argument::PlaintextU128(ctx.accounts.blackjack_game.client_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3, 32),
            // Dealer hand
            Argument::PlaintextU128(ctx.accounts.blackjack_game.dealer_nonce),
            Argument::Account(ctx.accounts.blackjack_game.key(), 8 + 32 * 3 + 32, 32),
            // Player hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.player_hand_size),
            // Dealer hand size
            Argument::PlaintextU8(ctx.accounts.blackjack_game.dealer_hand_size),
        ];

        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            vec![CallbackAccount {
                pubkey: ctx.accounts.blackjack_game.key(),
                is_writable: true,
            }],
            None,
        )?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "resolve_game")]
    pub fn resolve_game_callback(
        ctx: Context<ResolveGameCallback>,
        output: ComputationOutputs<ResolveGameOutput>,
    ) -> Result<()> {
        let result = match output {
            ComputationOutputs::Success(ResolveGameOutput { field_0 }) => field_0,
            _ => return Err(ErrorCode::AbortedComputation.into()),
        };

        if result == 0 {
            // Player busts (dealer wins)
            emit!(ResultEvent {
                winner: "Dealer".to_string(),
                game_id: ctx.accounts.blackjack_game.game_id,
            });
        } else if result == 1 {
            // Dealer busts (player wins)
            emit!(ResultEvent {
                winner: "Player".to_string(),
                game_id: ctx.accounts.blackjack_game.game_id,
            });
        } else if result == 2 {
            // Player wins
            emit!(ResultEvent {
                winner: "Player".to_string(),
                game_id: ctx.accounts.blackjack_game.game_id,
            });
        } else if result == 3 {
            // Dealer wins
            emit!(ResultEvent {
                winner: "Dealer".to_string(),
                game_id: ctx.accounts.blackjack_game.game_id,
            });
        } else {
            // Push (tie)
            emit!(ResultEvent {
                winner: "Tie".to_string(),
                game_id: ctx.accounts.blackjack_game.game_id,
            });
        }

        let blackjack_game = &mut ctx.accounts.blackjack_game;
        blackjack_game.game_state = GameState::Resolved;

        Ok(())
    }
}

#[queue_computation_accounts("shuffle_and_deal_cards", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, game_id: u64)]
pub struct InitializeBlackjackGame<'info> {
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
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
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
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(mut)]
    /// CHECK: comp_def_account, checked by arcium program.
    /// Can't check it here as it's not initialized yet.
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("player_hit", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct PlayerHit<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_HIT)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
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
    #[account(
        mut,
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("player_hit", payer)]
#[derive(Accounts)]
pub struct PlayerHitCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_HIT)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("player_hit", payer)]
#[derive(Accounts)]
pub struct InitPlayerHitCompDef<'info> {
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

#[queue_computation_accounts("player_double_down", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct PlayerDoubleDown<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
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
    #[account(
        mut,
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("player_double_down", payer)]
#[derive(Accounts)]
pub struct PlayerDoubleDownCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_DOUBLE_DOWN)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("player_double_down", payer)]
#[derive(Accounts)]
pub struct InitPlayerDoubleDownCompDef<'info> {
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

#[queue_computation_accounts("player_stand", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct PlayerStand<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_STAND)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
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
    #[account(
        mut,
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("player_stand", payer)]
#[derive(Accounts)]
pub struct PlayerStandCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_PLAYER_STAND)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("player_stand", payer)]
#[derive(Accounts)]
pub struct InitPlayerStandCompDef<'info> {
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

#[queue_computation_accounts("dealer_play", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct DealerPlay<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEALER_PLAY)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
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
    #[account(
        mut,
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("dealer_play", payer)]
#[derive(Accounts)]
pub struct DealerPlayCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_DEALER_PLAY)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("dealer_play", payer)]
#[derive(Accounts)]
pub struct InitDealerPlayCompDef<'info> {
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

#[queue_computation_accounts("resolve_game", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64, _game_id: u64)]
pub struct ResolveGame<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_RESOLVE_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
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
    #[account(
        mut,
        seeds = [b"blackjack_game".as_ref(), _game_id.to_le_bytes().as_ref()],
        bump = blackjack_game.bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[callback_accounts("resolve_game", payer)]
#[derive(Accounts)]
pub struct ResolveGameCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_RESOLVE_GAME)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub blackjack_game: Account<'info, BlackjackGame>,
}

#[init_computation_definition_accounts("resolve_game", payer)]
#[derive(Accounts)]
pub struct InitResolveGameCompDef<'info> {
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

/// Represents a single blackjack game session.
///
/// This account stores all the game state including encrypted hands, deck information,
/// and game progress. The deck is stored as three 32-byte encrypted chunks that together
/// represent all 52 cards in shuffled order. Hands are stored encrypted and only
/// decryptable by their respective owners (player) or the MPC network (dealer).
#[account]
#[derive(InitSpace)]
pub struct BlackjackGame {
    /// Encrypted deck split into 3 chunks for storage efficiency
    pub deck: [[u8; 32]; 3],
    /// Player's encrypted hand (only player can decrypt)
    pub player_hand: [u8; 32],
    /// Dealer's encrypted hand (handled by MPC)
    pub dealer_hand: [u8; 32],
    /// Cryptographic nonce for deck encryption
    pub deck_nonce: u128,
    /// Cryptographic nonce for player's hand encryption  
    pub client_nonce: u128,
    /// Cryptographic nonce for dealer's hand encryption
    pub dealer_nonce: u128,
    /// Unique identifier for this game session
    pub game_id: u64,
    /// Solana public key of the player
    pub player_pubkey: Pubkey,
    /// Player's encryption public key for MPC operations
    pub player_enc_pubkey: [u8; 32],
    /// PDA bump seed
    pub bump: u8,
    /// Current state of the game (initial, player turn, dealer turn, etc.)
    pub game_state: GameState,
    /// Number of cards currently in player's hand
    pub player_hand_size: u8,
    /// Number of cards currently in dealer's hand
    pub dealer_hand_size: u8,
    /// Whether the player has chosen to stand
    pub player_has_stood: bool,
    /// Final result of the game once resolved
    pub game_result: u8,
}

#[repr(u8)]
#[derive(InitSpace, AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameState {
    Initial = 0,
    PlayerTurn = 1,
    DealerTurn = 2,
    Resolving = 3,
    Resolved = 4,
}

#[event]
pub struct CardsShuffledAndDealtEvent {
    pub player_hand: [u8; 32],
    pub dealer_face_up_card: [u8; 32],
    pub client_nonce: u128,
    pub dealer_client_nonce: u128,
    pub game_id: u64,
}

#[event]
pub struct PlayerHitEvent {
    pub player_hand: [u8; 32],
    pub client_nonce: u128,
    pub game_id: u64,
}

#[event]
pub struct PlayerDoubleDownEvent {
    pub player_hand: [u8; 32],
    pub client_nonce: u128,
    pub game_id: u64,
}

#[event]
pub struct PlayerStandEvent {
    pub is_bust: bool,
    pub game_id: u64,
}

#[event]
pub struct PlayerBustEvent {
    pub client_nonce: u128,
    pub game_id: u64,
}

#[event]
pub struct DealerPlayEvent {
    pub dealer_hand: [u8; 32],
    pub dealer_hand_size: u8,
    pub client_nonce: u128,
    pub game_id: u64,
}

#[event]
pub struct ResultEvent {
    pub winner: String,
    pub game_id: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Invalid game state")]
    InvalidGameState,
    #[msg("Invalid move")]
    InvalidMove,
    #[msg("Invalid dealer client pubkey")]
    InvalidDealerClientPubkey,
    #[msg("Cluster not set")]
    ClusterNotSet,
}
