use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_SIGN_MESSAGE: u32 = comp_def_offset("sign_message");
const COMP_DEF_OFFSET_VERIFY_SIGNATURE: u32 = comp_def_offset("verify_signature");

declare_id!("HnzU9vBq3TDwdxVvcdfpbBBB8iKMB4uoboJiN4jDHhgN");

#[arcium_program]
pub mod ed_25519 {
    use super::*;

    /// Initializes the computation definition for Ed25519 message signing.
    /// This sets up the MPC environment for performing distributed key signing operations.
    pub fn init_sign_message_comp_def(ctx: Context<InitSignMessageCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    /// Signs a message using the MXE's distributed Ed25519 private key.
    ///
    /// The message is signed within the MPC environment using the Arcium network's
    /// collective signing key. The private key never exists in a single location,
    /// yet a valid Ed25519 signature is produced through multi-party computation.
    ///
    /// # Arguments
    /// * `message` - The 5-byte message to be signed
    ///
    /// # Returns
    /// Returns the 64-byte Ed25519 signature via the SignMessageEvent
    pub fn sign_message(
        ctx: Context<SignMessage>,
        computation_offset: u64,
        message: [u8; 5],
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let mut builder = ArgBuilder::new();
        for byte in message {
            builder = builder.plaintext_u8(byte);
        }
        queue_computation(
            ctx.accounts,
            computation_offset,
            builder.build(),
            None,
            vec![SignMessageCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    /// Handles the result of the MPC signing computation.
    ///
    /// This callback receives the Ed25519 signature components (r and s) from the
    /// completed MPC computation and emits them as a standard 64-byte signature.
    #[arcium_callback(encrypted_ix = "sign_message")]
    pub fn sign_message_callback(
        ctx: Context<SignMessageCallback>,
        output: SignedComputationOutputs<SignMessageOutput>,
    ) -> Result<()> {
        let signature = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(SignMessageOutput {
                field_0:
                    SignMessageOutputStruct0 {
                        field_0: r_encoded,
                        field_1: s,
                    },
            }) => {
                let mut signature = [0u8; 64];
                signature[..32].copy_from_slice(&r_encoded);
                signature[32..].copy_from_slice(&s);
                signature
            }
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        emit!(SignMessageEvent { signature });
        Ok(())
    }

    /// Initializes the computation definition for Ed25519 signature verification.
    /// This sets up the MPC environment for verifying signatures against encrypted public keys.
    pub fn init_verify_signature_comp_def(ctx: Context<InitVerifySignatureCompDef>) -> Result<()> {
        init_comp_def(ctx.accounts, None, None)?;
        Ok(())
    }

    /// Verifies an Ed25519 signature against an encrypted verifying key.
    ///
    /// This function allows signature verification where the public key remains encrypted
    /// throughout the verification process. The verification happens within the MPC environment,
    /// and only the boolean result (valid/invalid) is revealed.
    ///
    /// # Arguments
    /// * `verifying_key_compressed_lo_enc` - Lower 128 bits of the encrypted compressed public key
    /// * `verifying_key_compressed_hi_enc` - Upper 128 bits of the encrypted compressed public key
    /// * `message` - The 5-byte message that was signed
    /// * `signature` - The 64-byte Ed25519 signature to verify
    /// * `observer_pub_key` - Public key for encrypting the verification result
    ///
    /// # Returns
    /// Returns encrypted verification result via VerifySignatureEvent
    pub fn verify_signature(
        ctx: Context<VerifySignature>,
        computation_offset: u64,
        one_time_pub_key: [u8; 32],
        one_time_nonce: u128,
        verifying_key_compressed_lo_enc: [u8; 32],
        verifying_key_compressed_hi_enc: [u8; 32],
        message: [u8; 5],
        signature: [u8; 64],
        observer_pub_key: [u8; 32],
        observer_nonce: u128,
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let mut builder = ArgBuilder::new()
            .x25519_pubkey(one_time_pub_key)
            .plaintext_u128(one_time_nonce)
            .encrypted_u128(verifying_key_compressed_lo_enc)
            .encrypted_u128(verifying_key_compressed_hi_enc);
        for byte in message {
            builder = builder.plaintext_u8(byte);
        }
        let args = builder
            .arcis_ed25519_signature(signature)
            .x25519_pubkey(observer_pub_key)
            .plaintext_u128(observer_nonce)
            .build();
        queue_computation(
            ctx.accounts,
            computation_offset,
            args,
            None,
            vec![VerifySignatureCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
        )?;
        Ok(())
    }

    /// Handles the result of the MPC signature verification computation.
    ///
    /// This callback receives the encrypted verification result and emits it for the observer
    /// to decrypt. The verification outcome remains encrypted until the observer decrypts it.
    #[arcium_callback(encrypted_ix = "verify_signature")]
    pub fn verify_signature_callback(
        ctx: Context<VerifySignatureCallback>,
        output: SignedComputationOutputs<VerifySignatureOutput>,
    ) -> Result<()> {
        let o = match output.verify_output(
            &ctx.accounts.cluster_account,
            &ctx.accounts.computation_account,
        ) {
            Ok(VerifySignatureOutput { field_0 }) => field_0,
            Err(_) => return Err(ErrorCode::AbortedComputation.into()),
        };

        emit!(VerifySignatureEvent {
            is_valid: o.ciphertexts[0],
            nonce: o.nonce.to_le_bytes(),
        });
        Ok(())
    }
}

#[queue_computation_accounts("sign_message", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct SignMessage<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SIGN_MESSAGE)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("sign_message")]
#[derive(Accounts)]
pub struct SignMessageCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SIGN_MESSAGE)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("sign_message", payer)]
#[derive(Accounts)]
pub struct InitSignMessageCompDef<'info> {
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

/// Event emitted when a message is signed using MPC Ed25519.
#[event]
pub struct SignMessageEvent {
    /// The 64-byte Ed25519 signature (r || s components)
    pub signature: [u8; 64],
}

#[queue_computation_accounts("verify_signature", payer)]
#[derive(Accounts)]
#[instruction(computation_offset: u64)]
pub struct VerifySignature<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        init_if_needed,
        space = 9,
        payer = payer,
        seeds = [&SIGN_PDA_SEED],
        bump,
        address = derive_sign_pda!(),
    )]
    pub sign_pda_account: Account<'info, SignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account, ErrorCode::ClusterNotSet)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VERIFY_SIGNATURE)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        address = ARCIUM_CLOCK_ACCOUNT_ADDRESS
    )]
    pub clock_account: Account<'info, ClockAccount>,
    pub system_program: Program<'info, System>,
    pub arcium_program: Program<'info, Arcium>,
}

#[callback_accounts("verify_signature")]
#[derive(Accounts)]
pub struct VerifySignatureCallback<'info> {
    pub arcium_program: Program<'info, Arcium>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VERIFY_SIGNATURE)
    )]
    pub comp_def_account: Account<'info, ComputationDefinitionAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Account<'info, MXEAccount>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account, ErrorCode::ClusterNotSet)
    )]
    pub cluster_account: Account<'info, Cluster>,
    #[account(address = ::anchor_lang::solana_program::sysvar::instructions::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: AccountInfo<'info>,
}

#[init_computation_definition_accounts("verify_signature", payer)]
#[derive(Accounts)]
pub struct InitVerifySignatureCompDef<'info> {
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

/// Event emitted when signature verification completes.
#[event]
pub struct VerifySignatureEvent {
    /// Encrypted verification result (true if signature is valid, false otherwise)
    pub is_valid: [u8; 32],
    /// Nonce used for encrypting the result
    pub nonce: [u8; 16],
}

#[error_code]
pub enum ErrorCode {
    #[msg("The computation was aborted")]
    AbortedComputation,
    #[msg("Cluster not set")]
    ClusterNotSet,
}
