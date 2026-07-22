//! Stateless program for MPC Ed25519: `sign_message` produces a signature from
//! the MXE's share-split key, `verify_signature` checks a signature against an
//! encrypted verifying key. Results are delivered via events (`SignMessageEvent`,
//! `VerifySignatureEvent`); no application state is stored on chain. See README.md.

use anchor_lang::prelude::*;
use arcium_anchor::prelude::*;

const COMP_DEF_OFFSET_SIGN_MESSAGE: u32 = comp_def_offset("sign_message");
const COMP_DEF_OFFSET_VERIFY_SIGNATURE: u32 = comp_def_offset("verify_signature");

declare_id!("BVDLKuPHre5sThUJVCG5Get4nEeBvki4hy3ZxFdpGu2p");

#[arcium_program]
pub mod ed_25519 {
    use super::*;

    /// Initializes the computation definition for the `sign_message` circuit.
    pub fn init_sign_message_comp_def(ctx: Context<InitSignMessageCompDef>) -> Result<()> {
        init_computation_def(ctx.accounts, None)?;
        Ok(())
    }

    /// Queues MPC signing of `message` with the MXE's distributed Ed25519 key.
    /// The 64-byte signature is emitted via `SignMessageEvent`.
    pub fn sign_message(
        ctx: Context<SignMessage>,
        computation_offset: u64,
        message: [u8; 5],
    ) -> Result<()> {
        // The Arcium signer PDA bump must be persisted before queue_computation.
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        let mut builder = ArgBuilder::new();
        for byte in message {
            builder = builder.plaintext_u8(byte);
        }
        queue_computation(
            ctx.accounts,
            computation_offset,
            builder.build(),
            vec![SignMessageCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
            0,
        )?;
        Ok(())
    }

    /// Reassembles the signature from the two 32-byte halves (r, s) the circuit
    /// outputs and emits it as a standard 64-byte Ed25519 signature.
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

    /// Initializes the computation definition for the `verify_signature` circuit.
    pub fn init_verify_signature_comp_def(ctx: Context<InitVerifySignatureCompDef>) -> Result<()> {
        init_computation_def(ctx.accounts, None)?;
        Ok(())
    }

    /// Queues MPC verification of `signature` against an encrypted verifying key.
    /// The encrypted boolean verdict is emitted via `VerifySignatureEvent`.
    pub fn verify_signature(
        ctx: Context<VerifySignature>,
        computation_offset: u64,
        one_time_pub_key: [u8; 32],
        one_time_nonce: u128,
        verifying_key_enc_lo: [u8; 32],
        verifying_key_enc_hi: [u8; 32],
        message: [u8; 5],
        signature: [u8; 64],
        observer_pub_key: [u8; 32],
        observer_nonce: u128,
    ) -> Result<()> {
        ctx.accounts.sign_pda_account.bump = ctx.bumps.sign_pda_account;
        // Argument order must match the circuit signature: Enc<Shared, Pack<VerifyingKey>>
        // expands to pubkey + nonce + two ciphertexts (a packed 32-byte key spans two
        // field elements), and the trailing `observer: Shared` to pubkey + nonce.
        let mut builder = ArgBuilder::new()
            .x25519_pubkey(one_time_pub_key)
            .plaintext_u128(one_time_nonce)
            .encrypted_u128(verifying_key_enc_lo)
            .encrypted_u128(verifying_key_enc_hi);
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
            vec![VerifySignatureCallback::callback_ix(
                computation_offset,
                &ctx.accounts.mxe_account,
                &[],
            )?],
            1,
            0,
            0,
        )?;
        Ok(())
    }

    /// Emits the encrypted verification verdict for the observer to decrypt.
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
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account)
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account)
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_SIGN_MESSAGE)
    )]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
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
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(address = ::arcium_anchor::solana_instructions_sysvar::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: UncheckedAccount<'info>,
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
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
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
    pub sign_pda_account: Account<'info, ArciumSignerAccount>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    #[account(
        mut,
        address = derive_mempool_pda!(mxe_account)
    )]
    /// CHECK: mempool_account, checked by the arcium program.
    pub mempool_account: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_execpool_pda!(mxe_account)
    )]
    /// CHECK: executing_pool, checked by the arcium program.
    pub executing_pool: UncheckedAccount<'info>,
    #[account(
        mut,
        address = derive_comp_pda!(computation_offset, mxe_account)
    )]
    /// CHECK: computation_account, checked by the arcium program.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_comp_def_pda!(COMP_DEF_OFFSET_VERIFY_SIGNATURE)
    )]
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        mut,
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(
        mut,
        address = ARCIUM_FEE_POOL_ACCOUNT_ADDRESS,
    )]
    pub pool_account: Account<'info, FeePool>,
    #[account(
        mut,
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
    pub comp_def_account: Box<Account<'info, ComputationDefinitionAccount>>,
    #[account(
        address = derive_mxe_pda!()
    )]
    pub mxe_account: Box<Account<'info, MXEAccount>>,
    /// CHECK: computation_account, checked by arcium program via constraints in the callback context.
    pub computation_account: UncheckedAccount<'info>,
    #[account(
        address = derive_cluster_pda!(mxe_account)
    )]
    pub cluster_account: Box<Account<'info, Cluster>>,
    #[account(address = ::arcium_anchor::solana_instructions_sysvar::ID)]
    /// CHECK: instructions_sysvar, checked by the account constraint
    pub instructions_sysvar: UncheckedAccount<'info>,
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
    #[account(mut, address = derive_mxe_lut_pda!(mxe_account.lut_offset_slot))]
    /// CHECK: address_lookup_table, checked by arcium program.
    pub address_lookup_table: UncheckedAccount<'info>,
    #[account(address = LUT_PROGRAM_ID)]
    /// CHECK: lut_program is the Address Lookup Table program.
    pub lut_program: UncheckedAccount<'info>,
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
