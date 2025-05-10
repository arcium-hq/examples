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

const COMP_DEF_OFFSET_REVEAL_SECRET: u32 = comp_def_offset("reveal_secret");

declare_id!("8so4imnbiDULdF6AcwyvV2sD3siSmkurPhBHzzrnbwi5");

#[arcium_program]
pub mod time_capsule {
    use super::*;

    pub fn store_secret(
        ctx: Context<StoreSecret>,
        secret: [u8; 32],
        timeout: u64,
        receiver_pub_key: [u8; 32],
        pub_key: [u8; 32],
        nonce: u128,
    ) -> Result<()> {
        let secret_account = &mut ctx.accounts.secret_account;
        secret_account.secret = secret;
        secret_account.pub_key = pub_key;
        secret_account.nonce = nonce;
        secret_account.timeout = timeout;
        secret_account.last_updated = Clock::get()?.unix_timestamp;
        secret_account.receiver_pub_key = receiver_pub_key;
        Ok(())
    }

    pub fn init_reveal_secret_comp_def(ctx: Context<InitRevealSecretCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, true, None, None)?;
        Ok(())
    }

    pub fn reveal_secret(
        ctx: Context<RevealSecret>,
        receiver_pub_key: [u8; 32],
        receiver_nonce: u128,
    ) -> Result<()> {
        let secret_account = &ctx.accounts.secret_account;

        if secret_account.last_updated + secret_account.timeout as i64
            > Clock::get()?.unix_timestamp
        {
            return Err(ErrorCode::SecretNotAccessibleYet.into());
        }

        if secret_account.receiver_pub_key != receiver_pub_key {
            return Err(ErrorCode::InvalidReceiverPubKey.into());
        }

        let args = vec![
            Argument::PublicKey(secret_account.pub_key),
            Argument::PlaintextU128(secret_account.nonce),
            Argument::Account(secret_account.key(), 8, 32),
            Argument::EncryptedU16(secret_account.secret),
            Argument::PublicKey(receiver_pub_key),
            Argument::PlaintextU128(receiver_nonce),
        ];
        queue_computation(ctx.accounts, args, vec![], None)?;
        Ok(())
    }

    #[arcium_callback(encrypted_ix = "reveal_secret")]
    pub fn reveal_secret_callback(
        ctx: Context<RevealSecretCallback>,
        output: ComputationOutputs,
    ) -> Result<()> {
        let bytes = if let ComputationOutputs::Bytes(bytes) = output {
            bytes
        } else {
            return Err(ErrorCode::AbortedComputation.into());
        };

        emit!(RevealSecretEvent {
            secret: bytes[..32].try_into().unwrap(),
        });
        Ok(())
    }
}

#[queue_computation_accounts("reveal_secret", payer)]
#[derive(Accounts)]
pub struct RevealSecret<'info> {
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
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_REVEAL_SECRET)
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
    pub secret_account: Account<'info, SecretAccount>,
}

#[derive(Accounts)]
pub struct StoreSecret<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(init,
        payer = payer,
        space = 8 + 32 + 16 + 8 + 8,
        seeds = [b"secret".as_ref(), payer.key().as_ref()],
        bump,
    )]
    pub secret_account: Account<'info, SecretAccount>,
    pub system_program: Program<'info, System>,
}

#[callback_accounts("reveal_secret", payer)]
#[derive(Accounts)]
pub struct RevealSecretCallback<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_REVEAL_SECRET)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("reveal_secret", payer)]
#[derive(Accounts)]
pub struct InitRevealSecretCompDef<'info> {
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
pub struct SecretAccount {
    pub secret: [u8; 32],
    pub pub_key: [u8; 32],
    pub nonce: u128,
    pub timeout: u64,
    pub last_updated: i64,
    pub receiver_pub_key: [u8; 32],
}

#[event]
pub struct RevealSecretEvent {
    pub secret: [u8; 32],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("The secret is not accessible yet")]
    SecretNotAccessibleYet,
    #[msg("The receiver pub key is not the one as stored in the secret account")]
    InvalidReceiverPubKey,
}
