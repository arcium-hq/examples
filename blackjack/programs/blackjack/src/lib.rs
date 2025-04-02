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
    types::Argument,
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    queue_computation_accounts,
};

declare_id!("CX9KuFpPeJzJ9kNWNLgESzys42FFhfZZ9CG2P7frEWgz");

const GAME_SEED: &[u8] = b"blackjack_game";
const PLAYER_HAND_SEED: &[u8] = b"player_hand";
const DEALER_HAND_SEED: &[u8] = b"dealer_hand";

const COMP_DEF_OFFSET_CALC_VALUE: u32 = comp_def_offset("calculate_hand_value");
const COMP_DEF_OFFSET_ADD_CARD: u32 = comp_def_offset("add_card_to_hand");

#[arcium_program]
pub mod blackjack {
    use super::*;

    pub fn init_calculate_value_comp_def(ctx: Context<InitCalculateValueCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn init_add_card_comp_def(ctx: Context<InitAddCardCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn initialize(ctx: Context<Initialize>, bet: u64) -> Result<()> {
        let game = &mut ctx.accounts.game;
        game.player = ctx.accounts.player.key();
        game.dealer_pubkey = ctx.accounts.dealer_hand.key();
        game.status = GameStatus::Active;
        game.player_bet = bet;
        game.is_player_turn = true;
        game.bump = *ctx.bumps.get("game").unwrap();

        let player_hand = &mut ctx.accounts.player_hand;
        player_hand.owner = ctx.accounts.player.key();
        player_hand.bump = *ctx.bumps.get("player_hand").unwrap();

        let dealer_hand = &mut ctx.accounts.dealer_hand;
        dealer_hand.owner = game.dealer_pubkey;
        dealer_hand.bump = *ctx.bumps.get("dealer_hand").unwrap();

        Ok(())
    }

    pub fn hit(ctx: Context<AddCard>, encrypted_card: [u8; 32], pub_key: [u8; 32], nonce: u128) -> Result<()> {
        let args = vec![
            Argument::EncryptedU8(ctx.accounts.hand.encrypted_data.try_into().unwrap()),
            Argument::EncryptedU8(encrypted_card),
            Argument::PublicKey(pub_key),
            Argument::PlaintextU128(nonce),
        ];
        
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    pub fn calculate_value(ctx: Context<CalculateValue>, hand_ciphertext: [u8; 32], pub_key: [u8; 32], nonce: u128) -> Result<()> {
        let args = vec![
            Argument::EncryptedU8(hand_ciphertext),
            Argument::PublicKey(pub_key),
            Argument::PlaintextU128(nonce),
        ];
        
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "add_card_to_hand")]
    pub fn add_card_callback(ctx: Context<AddCardCallback>, output: Vec<u8>) -> Result<()> {
        let hand = &mut ctx.accounts.hand;
        hand.encrypted_data = output;
        
        // Queue computation to calculate new hand value
        let args = vec![
            Argument::EncryptedU8(output.try_into().unwrap()),
            Argument::PublicKey(ctx.accounts.game.player.to_bytes()),
            Argument::PlaintextU128(0), // Use appropriate nonce
        ];
        
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "calculate_hand_value")]
    pub fn calculate_value_callback(ctx: Context<CalculateValueCallback>, output: Vec<u8>) -> Result<()> {
        let game = &mut ctx.accounts.game;
        let value = output[0];
        
        if value > 21 {
            game.status = GameStatus::DealerWon;
            game.is_player_turn = false;
        }
        
        Ok(())
    }
}

// Events
#[event]
pub struct HandValueEvent {
    pub value: [u8; 32],
}

// Enums
#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq)]
pub enum GameStatus {
    Active,
    PlayerWon,
    DealerWon,
    Push,
}

// Account Structures
#[account]
pub struct GameState {
    pub player: Pubkey,
    pub dealer_pubkey: Pubkey,
    pub status: GameStatus,
    pub player_bet: u64,
    pub is_player_turn: bool,
    pub bump: u8,
}

#[account]
pub struct Hand {
    pub owner: Pubkey,
    pub encrypted_data: Vec<u8>,
    pub bump: u8,
}

// Context Accounts
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = player,
        space = 8 + 32 + 32 + 1 + 8 + 1 + 1,
        seeds = [GAME_SEED, player.key().as_ref()],
        bump
    )]
    pub game: Account<'info, GameState>,
    
    #[account(
        init,
        payer = player,
        space = 8 + 32 + 200 + 1,
        seeds = [PLAYER_HAND_SEED, game.key().as_ref()],
        bump
    )]
    pub player_hand: Account<'info, Hand>,
    
    #[account(
        init,
        payer = player,
        space = 8 + 32 + 200 + 1,
        seeds = [DEALER_HAND_SEED, game.key().as_ref()],
        bump
    )]
    pub dealer_hand: Account<'info, Hand>,
    
    #[account(mut)]
    pub player: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("calculate_hand_value", payer)]
#[derive(Accounts)]
pub struct CalculateValue<'info> {
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_CALC_VALUE.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        seeds = [CLUSTER_PDA_SEED, mxe_account.cluster.offset.to_le_bytes().as_ref()],
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

#[queue_computation_accounts("add_card_to_hand", payer)]
#[derive(Accounts)]
pub struct AddCard<'info> {
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_ADD_CARD.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        seeds = [CLUSTER_PDA_SEED, mxe_account.cluster.offset.to_le_bytes().as_ref()],
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

#[callback_accounts("calculate_hand_value", payer)]
#[derive(Accounts)]
pub struct CalculateValueCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_CALC_VALUE.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub game: Account<'info, GameState>,
}

#[callback_accounts("add_card_to_hand", payer)]
#[derive(Accounts)]
pub struct AddCardCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_ADD_CARD.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
    #[account(mut)]
    pub hand: Account<'info, Hand>,
    #[account(mut)]
    pub game: Account<'info, GameState>,
}

#[init_computation_definition_accounts("calculate_hand_value", payer)]
#[derive(Accounts)]
pub struct InitCalculateValueCompDef<'info> {
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
    /// CHECK: comp_def_account, checked by arcium program
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("add_card_to_hand", payer)]
#[derive(Accounts)]
pub struct InitAddCardCompDef<'info> {
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
    /// CHECK: comp_def_account, checked by arcium program
    pub comp_def_account: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}
