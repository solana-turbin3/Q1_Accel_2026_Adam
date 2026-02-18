use anchor_lang::prelude::*;

declare_id!("6wpZxCkv6bZwosC1WHqqhm2sDZfV3oA7fSMU78vkipaR");

mod state;
mod instructions;
pub use instructions::*;

#[program]
pub mod tuktuk_counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        ctx.accounts.initialize(&ctx.bumps)
    }

    pub fn increment(ctx: Context<Increment>) -> Result<()> {
        ctx.accounts.increment_counter()
    }

    pub fn schedule(ctx: Context<Schedule>, task_id: u16) -> Result<()> {
        ctx.accounts.schedule(task_id, ctx.bumps)
    }
}
