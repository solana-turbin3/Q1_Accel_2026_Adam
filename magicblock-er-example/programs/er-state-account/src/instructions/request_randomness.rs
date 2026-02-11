use anchor_lang::prelude::*;

use ephemeral_vrf_sdk::anchor::vrf;
use ephemeral_vrf_sdk::instructions::{create_request_randomness_ix, RequestRandomnessParams};
use ephemeral_vrf_sdk::types::SerializableAccountMeta;

use crate::state::UserAccount;

// ── Task 1: Request randomness on L1 (DEFAULT_QUEUE) ────────────────────────

#[vrf]
#[derive(Accounts)]
pub struct RequestRandomness<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"user", payer.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: Oracle queue on L1
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RequestRandomness<'info> {
    pub fn request_randomness(&self, client_seed: u8) -> Result<()> {
        msg!("Requesting randomness on L1...");
        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.payer.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: crate::ID,
            callback_discriminator: crate::instruction::CallbackConsumeRandomness::DISCRIMINATOR.to_vec(),
            caller_seed: [client_seed; 32],
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            }]),
            ..Default::default()
        });
        self.invoke_signed_vrf(&self.payer.to_account_info(), &ix)?;
        Ok(())
    }
}

// ── Task 2: Request randomness inside ER (DEFAULT_EPHEMERAL_QUEUE, free) ────

#[vrf]
#[derive(Accounts)]
pub struct RequestRandomnessEr<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(
        seeds = [b"user", payer.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,
    /// CHECK: Oracle queue inside the Ephemeral Rollup
    #[account(mut, address = ephemeral_vrf_sdk::consts::DEFAULT_EPHEMERAL_QUEUE)]
    pub oracle_queue: AccountInfo<'info>,
}

impl<'info> RequestRandomnessEr<'info> {
    pub fn request_randomness_er(&self, client_seed: u8) -> Result<()> {
        msg!("Requesting randomness in ER...");
        let ix = create_request_randomness_ix(RequestRandomnessParams {
            payer: self.payer.key(),
            oracle_queue: self.oracle_queue.key(),
            callback_program_id: crate::ID,
            callback_discriminator: crate::instruction::CallbackConsumeRandomness::DISCRIMINATOR.to_vec(),
            caller_seed: [client_seed; 32],
            accounts_metas: Some(vec![SerializableAccountMeta {
                pubkey: self.user_account.key(),
                is_signer: false,
                is_writable: true,
            }]),
            ..Default::default()
        });
        self.invoke_signed_vrf(&self.payer.to_account_info(), &ix)?;
        Ok(())
    }
}