use anchor_lang::prelude::*;
use arcium_anchor::{
    comp_def_offset, init_comp_def, queue_computation, CLOCK_PDA_SEED, CLUSTER_PDA_SEED,
    COMP_DEF_PDA_SEED, DATA_OBJ_PDA_SEED, MEMPOOL_PDA_SEED, MXE_PDA_SEED, POOL_PDA_SEED,
};
use arcium_client::idl::arcium::{
    accounts::{
        ClockAccount, Cluster, ComputationDefinitionAccount, DataObjectAccount, Mempool,
        PersistentMXEAccount, StakingPoolAccount,
    },
    program::Arcium,
    types::{Argument, OffChainReference},
    ID_CONST as ARCIUM_PROG_ID,
};
use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    init_data_object_accounts, queue_computation_accounts,
};

const COMP_DEF_OFFSET_SETUP_BLACKJACK_GAME: u32 = comp_def_offset("setup_blackjack_game");
const COMP_DEF_OFFSET_REVEAL_DEALER_CARD: u32 = comp_def_offset("reveal_dealer_card");

declare_id!("GZFGjRTZnkmBim8gscDoBwEFWodQRAbkPqKv7GT5tG2c");

#[arcium_program]
pub mod blackjack {
    use arcium_anchor::init_da_object;

    use super::*;

    pub fn init_setup_blackjack_game_comp_def(
        ctx: Context<InitSetupBlackjackGameCompDef>,
    ) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn init_blackjack_game(
        ctx: Context<InitBlackjackGame>,
        id: u32,
        initial_player_hand: OffChainReference,
        initial_dealer_hand: OffChainReference,
    ) -> Result<()> {
        init_da_object(
            ctx.accounts,
            initial_player_hand,
            ctx.accounts.player_hand.to_account_info(),
            id + 0,
        )?;

        init_da_object(
            ctx.accounts,
            initial_dealer_hand,
            ctx.accounts.dealer_hand.to_account_info(),
            id + 1,
        )?;

        ctx.accounts.blackjack_game.state = GameState::Setup;
        ctx.accounts.blackjack_game.player_hand = ctx.accounts.player_hand.key();
        ctx.accounts.blackjack_game.dealer_hand = ctx.accounts.dealer_hand.key();
        ctx.accounts.blackjack_game.bump = ctx.bumps.blackjack_game;

        Ok(())
    }

    pub fn setup_blackjack_game(
        ctx: Context<SetupBlackjackGame>,
        seed: OffChainReference,
        id: u32,
    ) -> Result<()> {
        let args = vec![
            Argument::MU8(seed),
            Argument::DataObj(id + 0),
            Argument::DataObj(id + 1),
        ];

        queue_computation(
            ctx.accounts,
            args,
            vec![
                ctx.accounts.player_hand.to_account_info(),
                ctx.accounts.dealer_hand.to_account_info(),
            ],
            vec![],
        )?;

        Ok(())
    }

    #[arcium_callback(confidential_ix = "setup_blackjack_game")]
    pub fn setup_blackjack_game_callback(
        ctx: Context<SetupBlackjackGameCallback>,
        _output: Vec<u8>,
    ) -> Result<()> {
        Ok(())
    }

    pub fn reveal_dealer_card(
        ctx: Context<RevealDealerCard>,
        id: u32,
        card_index: u8,
    ) -> Result<()> {
        let args = vec![
            Argument::DataObj(id + 1),
            Argument::PlaintextU8(card_index),
        ];

        queue_computation(
            ctx.accounts,
            args,
            vec![ctx.accounts.dealer_hand.to_account_info()],
            vec![],
        )?;

        Ok(())
    }

    #[arcium_callback(confidential_ix = "reveal_dealer_card")]
    pub fn reveal_dealer_card_callback(
        ctx: Context<RevealDealerCardCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        let card = output[0];

        emit!(RevealDealerCardEvent { card });

        Ok(())
    }
}

#[init_data_object_accounts(payer)]
#[derive(Accounts)]
#[instruction(id: u32)]
pub struct InitBlackjackGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + BlackjackGame::INIT_SPACE,
        seeds = [b"blackjack", payer.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
    /// CHECK: Player hand data object will be initialized by CPI
    #[account(mut)]
    pub player_hand: UncheckedAccount<'info>,
    /// CHECK: Dealer hand data object will be initialized by CPI
    #[account(mut)]
    pub dealer_hand: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, ID_CONST.to_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_account.bump
    )]
    pub mxe_account: Account<'info, PersistentMXEAccount>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[queue_computation_accounts("setup_blackjack_game", payer)]
#[derive(Accounts)]
#[instruction(id: u32)]
pub struct SetupBlackjackGame<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"blackjack", payer.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_SETUP_BLACKJACK_GAME.to_le_bytes().as_ref()],
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
    #[account(
        mut,
        seeds = [DATA_OBJ_PDA_SEED, &ID_CONST.to_bytes().as_ref(), (id + 0).to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = player_hand.bump,
    )]
    pub player_hand: Account<'info, DataObjectAccount>,
    #[account(
        mut,
        seeds = [DATA_OBJ_PDA_SEED, &ID_CONST.to_bytes().as_ref(), (id + 1).to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = dealer_hand.bump,
    )]
    pub dealer_hand: Account<'info, DataObjectAccount>,
}

#[callback_accounts("setup_blackjack_game", payer)]
#[derive(Accounts)]
pub struct SetupBlackjackGameCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_SETUP_BLACKJACK_GAME.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("setup_blackjack_game", payer)]
#[derive(Accounts)]
pub struct InitSetupBlackjackGameCompDef<'info> {
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

#[queue_computation_accounts("reveal_dealer_card", payer)]
#[derive(Accounts)]
#[instruction(id: u32)]
pub struct RevealDealerCard<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"blackjack", payer.key().as_ref(), id.to_le_bytes().as_ref()],
        bump,
    )]
    pub blackjack_game: Account<'info, BlackjackGame>,
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
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_REVEAL_DEALER_CARD.to_le_bytes().as_ref()],
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
    #[account(
        mut,
        seeds = [DATA_OBJ_PDA_SEED, &ID_CONST.to_bytes().as_ref(), (id + 1).to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = dealer_hand.bump,
    )]
    pub dealer_hand: Account<'info, DataObjectAccount>,
}

#[callback_accounts("reveal_dealer_card", payer)]
#[derive(Accounts)]
pub struct RevealDealerCardCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &ID_CONST.to_bytes().as_ref(), COMP_DEF_OFFSET_REVEAL_DEALER_CARD.to_le_bytes().as_ref()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}


// Game state enum
#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Clone, InitSpace)]
pub enum GameState {
    Setup,
    Active,
    PlayerTurn,
    DealerTurn,
    Complete,
}

// Game actions enum
#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Clone)]
pub enum PlayerAction {
    Hit,
    Stand,
}

// Main game struct
#[derive(Debug, InitSpace)]
#[account]
pub struct BlackjackGame {
    state: GameState,
    player_hand: Pubkey,
    dealer_hand: Pubkey,
    bump: u8,
}

#[event]
pub struct RevealDealerCardEvent {
    pub card: u8,
}
