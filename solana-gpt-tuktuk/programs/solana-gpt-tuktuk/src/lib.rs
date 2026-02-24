use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;
pub mod types;

use instructions::*;

declare_id!("H8Tq9DAw82BcYzeeBpm3BLisK8sQn4Ntyj3AewhNTuvj");

#[program]
pub mod solana_gpt_tuktuk {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, prompt: String) -> Result<()> {
        ctx.accounts.handler(prompt, &ctx.bumps)
    }

    pub fn ask_gpt(ctx: Context<AskGpt>) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps)
    }

    pub fn receive(ctx: Context<ReceiveResponse>, response: String) -> Result<()> {
        ctx.accounts.handler(response)
    }

    pub fn schedule(
        ctx: Context<ScheduleAskGpt>,
        task_id: u16,
        trigger: crate::types::TriggerV0,
    ) -> Result<()> {
        ctx.accounts.handler(task_id, trigger, &ctx.bumps)
    }
}