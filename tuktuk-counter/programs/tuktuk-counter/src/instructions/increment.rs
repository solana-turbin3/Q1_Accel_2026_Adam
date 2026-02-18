use anchor_lang::prelude::*;

use crate::state::Counter;

#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(
        mut,
        seeds = [b"counter"],
        bump = counter.bump,
    )]
    pub counter: Account<'info, Counter>,
}

impl<'info> Increment<'info> {
    pub fn increment_counter(&mut self) -> Result<()> {
        self.counter.count += 1;
        Ok(())
    }
}