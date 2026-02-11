#![allow(unexpected_cfgs)]
#![allow(deprecated)]

use anchor_lang::prelude::*;
use ephemeral_rollups_sdk::anchor::ephemeral;

mod state;
mod instructions;

use instructions::*;

declare_id!("9hG187VazKdEZcYbsEcoPuPEWwkfF9HccUDTAJzuEcg3");

#[ephemeral]
#[program]
pub mod er_state_account {

    use super::*;

    pub fn initialize(ctx: Context<InitUser>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)?;
        
        Ok(())
    }

    pub fn update(ctx: Context<UpdateUser>, new_data: u64) -> Result<()> {
        ctx.accounts.update(new_data)?;
        
        Ok(())
    }

    pub fn update_commit(ctx: Context<UpdateCommit>, new_data: u64) -> Result<()> {
        ctx.accounts.update_commit(new_data)?;
        
        Ok(())
    }

    pub fn delegate(ctx: Context<Delegate>) -> Result<()> {
        ctx.accounts.delegate()?;
        
        Ok(())
    }

    pub fn undelegate(ctx: Context<Undelegate>) -> Result<()> {
        ctx.accounts.undelegate()?;
        
        Ok(())
    }

    pub fn close(ctx: Context<CloseUser>) -> Result<()> {
        ctx.accounts.close()?;
        
        Ok(())
    }

    // ── Task 1: Request VRF on L1 (DEFAULT_QUEUE) ──────────────
    pub fn request_randomness(ctx: Context<RequestRandomness>, client_seed: u8) -> Result<()> {
        ctx.accounts.request_randomness(client_seed)?;
        Ok(())
    }

    // ── Task 2: Request VRF inside Ephemeral Rollup (free) ─────
    pub fn request_randomness_er(ctx: Context<RequestRandomnessEr>, client_seed: u8) -> Result<()> {
        ctx.accounts.request_randomness_er(client_seed)?;
        Ok(())
    }

    // ── Callback: VRF oracle CPI-calls this with randomness ────
    pub fn callback_consume_randomness(
        ctx: Context<CallbackConsumeRandomness>,
        randomness: [u8; 32],
    ) -> Result<()> {
        let random_value = ephemeral_vrf_sdk::rnd::random_u64(&randomness);
        msg!("VRF callback! random_u64 = {}", random_value);
        ctx.accounts.user_account.data = random_value;
        Ok(())
    }
}

