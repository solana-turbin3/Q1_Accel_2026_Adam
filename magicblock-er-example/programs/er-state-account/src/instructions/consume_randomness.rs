use anchor_lang::prelude::*;

use crate::state::UserAccount;

/// Callback — the VRF oracle CPI-calls this after fulfilling the randomness
/// request. The `vrf_program_identity` signer proves authenticity.
#[derive(Accounts)]
pub struct CallbackConsumeRandomness<'info> {
    /// This check ensures that the vrf_program_identity (which is a PDA) is a signer,
    /// enforcing the callback is executed by the VRF program through CPI
    #[account(address = ephemeral_vrf_sdk::consts::VRF_PROGRAM_IDENTITY)]
    pub vrf_program_identity: Signer<'info>,
    /// The user account to write randomness into (passed via `accounts_metas` in the request)
    #[account(mut)]
    pub user_account: Account<'info, UserAccount>,
}

impl<'info> CallbackConsumeRandomness<'info> {
    pub fn callback_consume_randomness(&mut self, randomness: [u8; 32]) -> Result<()> {
        let random_value = ephemeral_vrf_sdk::rnd::random_u64(&randomness);
        msg!("Received randomness callback with random_value: {}", random_value);
        self.user_account.data = random_value;
        Ok(())
    }
}