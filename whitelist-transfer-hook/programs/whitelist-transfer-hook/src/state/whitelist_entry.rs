use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct WhitelistEntry {
    pub user: Pubkey,
    pub bump: u8,
}

impl WhitelistEntry {
    pub const SIZE: usize = 8 + 32 + 1;
}