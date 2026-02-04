use anchor_lang::prelude::*;

use crate::state::whitelist::Whitelist;

#[derive(Accounts)]
#[instruction(address: Pubkey)]
pub struct AddToWhitelist<'info> {
    #[account(
        mut,
        //address = 
    )]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + 32 + 1,
        seeds = [b"whitelist", address.as_ref()],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}

impl<'info> AddToWhitelist<'info> {
    pub fn add_to_whitelist(&mut self, address: Pubkey, bumps: AddToWhitelistBumps) -> Result<()> {
        self.whitelist.set_inner(Whitelist {
            address,
            bump: bumps.whitelist,
        });
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(address: Pubkey)]
pub struct RemoveFromWhitelist<'info> {
    #[account(
        mut,
        //address = 
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"whitelist", address.as_ref()],
        bump = whitelist.bump,
        close = admin,
    )]
    pub whitelist: Account<'info, Whitelist>,
}

impl<'info> RemoveFromWhitelist<'info> {
    pub fn remove_from_whitelist(&mut self, _address: Pubkey) -> Result<()> {
        Ok(())
    }
}