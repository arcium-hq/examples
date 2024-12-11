use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer},
};
use arcium_anchor::{
    comp_def_offset, data_object_account, derive_seed, init_comp_def, init_da_object,
    queue_computation,
};
use arcium_client::idl::arcium::{
    accounts::{
        ClockAccount, Cluster, ComputationDefinitionAccount, DataObjectAccount, Mempool,
        PersistentMXEAccount, StakingPoolAccount,
    },
    program::Arcium,
    types::OffChainReference,
    ID_CONST as ARCIUM_PROG_ID,
};

use arcium_macros::{
    arcium_callback, arcium_program, callback_accounts, init_computation_definition_accounts,
    init_data_object_accounts, queue_computation_accounts,
};
use confidential_ixs::OrderBook;

const MXE_PDA_SEED: &'static [u8] = derive_seed!(PersistentMXEAccount);
const DA_OBJ_PDA_SEED: &'static [u8] = derive_seed!(DataObjectAccount);
const MEMPOOL_PDA_SEED: &'static [u8] = derive_seed!(Mempool);
const COMP_DEF_PDA_SEED: &'static [u8] = derive_seed!(ComputationDefinitionAccount);
const CLUSTER_PDA_SEED: &'static [u8] = derive_seed!(Cluster);
const POOL_PDA_SEED: &'static [u8] = derive_seed!(StakingPoolAccount);
const CLOCK_PDA_SEED: &'static [u8] = derive_seed!(ClockAccount);
const COMP_DEF_OFFSET_ADD_ORDER: u32 = comp_def_offset("add_order");
const COMP_DEF_OFFSET_FIND_MATCH: u32 = comp_def_offset("find_next_match");

const ORDERBOOK_DATA_OBJ_OFFSET: u32 = 42;

declare_id!("FkdM3cAbA91fgYKHeXJBoHb4y313boyKBSCLh8EtMyTe");

#[arcium_program]
pub mod dark_pool {
    use super::*;

    pub fn init(ctx: Context<Init>, initial_ob: OffChainReference) -> Result<()> {
        // Initialize the vault with the token pair for the orderbook
        let vault = &mut ctx.accounts.vault;
        // vault.token_a = ctx.accounts.token_a_mint.key();
        // vault.token_b = ctx.accounts.token_b_mint.key();
        vault.token_a = ctx.accounts.token_program.key();
        vault.token_b = ctx.accounts.token_program.key();

        // Initialize the orderbook data object
        init_da_object(
            ctx.accounts,
            initial_ob,
            ctx.accounts.ob.to_account_info(),
            ORDERBOOK_DATA_OBJ_OFFSET,
        )?;
        Ok(())
    }

    pub fn init_add_order_comp_def(ctx: Context<InitAOCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn init_next_match_comp_def(ctx: Context<InitFNMCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts)?;
        Ok(())
    }

    pub fn add_order(ctx: Context<AddOrder>, order: OffChainReference) -> Result<()> {
        // Call the Arcium program to queue the computation
        queue_computation(
            ctx.accounts,
            Some(order),
            vec![ORDERBOOK_DATA_OBJ_OFFSET],
            vec![ctx.accounts.ob.to_account_info()],
            vec![],
        )?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "add_order")]
    pub fn add_order_callback(ctx: Context<AddOrderCallback>, _output: Vec<u8>) -> Result<()> {
        // Do some additional processing once the order is added to the orderbook
        Ok(())
    }

    pub fn find_next_match(ctx: Context<NextMatch>) -> Result<()> {
        // Call the Arcium program to queue the computations
        queue_computation(
            ctx.accounts,
            None,
            vec![ORDERBOOK_DATA_OBJ_OFFSET],
            vec![ctx.accounts.ob.to_account_info()],
            vec![],
        )?;
        Ok(())
    }

    #[arcium_callback(confidential_ix = "find_next_match")]
    pub fn find_next_match_callback(
        ctx: Context<NextMatchCallback>,
        output: Vec<u8>,
    ) -> Result<()> {
        // Convert to u128
        let match_partner_one = u128::from_le_bytes(output.as_slice().try_into().unwrap());
        if match_partner_one != 0 {
            let match_partner_two = u128::from_le_bytes(output.as_slice().try_into().unwrap());
            msg!(
                "Match partners are: {} and {}",
                match_partner_one,
                match_partner_two
            );
        } else {
            msg!("No matches found!");
        }
        Ok(())
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;
        let token_account = &ctx.accounts.deposit_token_account;
        let user_deposit = &mut ctx.accounts.user_deposit;

        // Check if the deposited token is one of the two allowed tokens
        if token_account.mint != vault.token_a
            && token_account.mint != vault.token_b
            && token_account.mint != ctx.accounts.deposit_token_mint.key()
        {
            return Err(ErrorCode::InvalidToken.into());
        }

        // Transfer tokens from user to vault
        let cpi_accounts = Transfer {
            from: ctx.accounts.deposit_token_account.to_account_info(),
            to: ctx.accounts.vault_token_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();

        let vault_bump: &[u8; 1] = &[ctx.bumps.vault];

        let signer_seeds = &[&[
            b"vault".as_ref(),
            vault.to_account_info().key.as_ref(),
            vault_bump,
        ][..]];

        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        token::transfer(cpi_ctx, amount)?;

        // Update user's deposit record
        if token_account.mint == vault.token_a {
            user_deposit.token_a_amount += amount;
        } else {
            user_deposit.token_b_amount += amount;
        }

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        let vault = &ctx.accounts.vault;
        let token_account = &ctx.accounts.token_account;
        let user_deposit = &mut ctx.accounts.user_deposit;

        // Check if the withdrawn token is one of the two allowed tokens
        if token_account.mint != vault.token_a && token_account.mint != vault.token_b {
            return Err(ErrorCode::InvalidToken.into());
        }

        // Check if user has enough balance
        let user_balance = if token_account.mint == vault.token_a {
            &mut user_deposit.token_a_amount
        } else {
            &mut user_deposit.token_b_amount
        };

        if *user_balance < amount {
            return Err(ErrorCode::InsufficientBalance.into());
        }

        let vault_bump: &[u8; 1] = &[ctx.bumps.vault];

        // Transfer tokens from vault to user
        let vault_signer_seeds = &[&[
            b"vault".as_ref(),
            vault.to_account_info().key.as_ref(),
            vault_bump,
        ][..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.vault_token_account.to_account_info(),
            to: ctx.accounts.token_account.to_account_info(),
            authority: ctx.accounts.vault.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, vault_signer_seeds);
        token::transfer(cpi_ctx, amount)?;

        // Update user's deposit record
        *user_balance -= amount;

        Ok(())
    }
}

#[queue_computation_accounts("add_order", payer)]
pub struct AddOrder<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [MXE_PDA_SEED, &ID.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_account.bump
    )]
    pub mxe_account: Account<'info, PersistentMXEAccount>,
    #[account(
        mut,
        seeds = [MEMPOOL_PDA_SEED, &ID.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mempool_account.bump
    )]
    pub mempool_account: Account<'info, Mempool>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &COMP_DEF_OFFSET_ADD_ORDER.to_le_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        // TODO: Replace with actual cluster offset
        seeds = [CLUSTER_PDA_SEED, &0u32.to_le_bytes()],
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
        seeds = [DA_OBJ_PDA_SEED, &ORDERBOOK_DATA_OBJ_OFFSET.to_le_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        owner = ARCIUM_PROG_ID,
        bump = ob.bump,
    )]
    pub ob: Account<'info, DataObjectAccount>,
}

#[callback_accounts("add_order", payer)]
pub struct AddOrderCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &COMP_DEF_OFFSET_ADD_ORDER.to_le_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[queue_computation_accounts("find_next_match", payer)]
pub struct NextMatch<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [MXE_PDA_SEED, &ID.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_account.bump
    )]
    pub mxe_account: Account<'info, PersistentMXEAccount>,
    #[account(
        mut,
        seeds = [MEMPOOL_PDA_SEED, &ID.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mempool_account.bump
    )]
    pub mempool_account: Account<'info, Mempool>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &COMP_DEF_OFFSET_ADD_ORDER.to_le_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        // TODO: Replace with actual cluster offset
        seeds = [CLUSTER_PDA_SEED, &0u32.to_le_bytes()],
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
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        mut,
        seeds = [DA_OBJ_PDA_SEED, &ORDERBOOK_DATA_OBJ_OFFSET.to_le_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = ob.bump,
    )]
    pub ob: Account<'info, DataObjectAccount>,
}

#[callback_accounts("find_next_match", payer)]
pub struct NextMatchCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        seeds = [COMP_DEF_PDA_SEED, &COMP_DEF_OFFSET_FIND_MATCH.to_le_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = comp_def_account.bump
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_data_object_accounts(payer)]
pub struct Init<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 8 + 32 + 32,
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub ob: UncheckedAccount<'info>,
    // pub token_a_mint: Account<'info, Mint>,
    // pub token_b_mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, &ID_CONST.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe.bump
    )]
    // TODO: Rename to mxe_acc
    pub mxe: Account<'info, PersistentMXEAccount>,
    pub arcium_program: Program<'info, Arcium>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("add_order", payer)]
pub struct InitAOCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = ID_CONST
    )]
    pub mxe: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, &ID_CONST.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_acc.bump
    )]
    pub mxe_acc: Box<Account<'info, PersistentMXEAccount>>,
    #[account(mut)]
    pub comp_def_acc: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[init_computation_definition_accounts("find_next_match", payer)]
pub struct InitFNMCompDef<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        address = ID_CONST
    )]
    pub mxe: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [MXE_PDA_SEED, &ID_CONST.to_bytes()],
        seeds::program = ARCIUM_PROG_ID,
        bump = mxe_acc.bump
    )]
    pub mxe_acc: Box<Account<'info, PersistentMXEAccount>>,
    #[account(mut)]
    pub comp_def_acc: UncheckedAccount<'info>,
    pub arcium_program: Program<'info, Arcium>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(
        mut,
        seeds = [b"vault"],
        bump
    )]
    pub vault: Account<'info, Vault>,
    pub deposit_token_mint: Account<'info, Mint>,
    #[account(mut)]
    pub deposit_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = deposit_token_mint,
        associated_token::authority = vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + 8 + 8,
        seeds = [b"user_deposit", user.key().as_ref(), vault.key().as_ref()],
        bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(
		mut,
		seeds = [b"vault"],
		bump
	)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        associated_token::mint = token_account.mint,
        associated_token::authority = vault
    )]
    pub vault_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"user_deposit", user.key().as_ref(), vault.key().as_ref()],
        bump
    )]
    pub user_deposit: Account<'info, UserDeposit>,
    #[account(mut)]
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[account]
pub struct Vault {
    pub token_a: Pubkey,
    pub token_b: Pubkey,
}

#[account]
pub struct UserDeposit {
    pub token_a_amount: u64,
    pub token_b_amount: u64,
}

data_object_account!(OrderBookDataObject, OrderBook);

#[error_code]
pub enum ErrorCode {
    #[msg("The provided token is not one of the two allowed tokens")]
    InvalidToken,
    #[msg("Insufficient balance for withdrawal")]
    InsufficientBalance,
}
