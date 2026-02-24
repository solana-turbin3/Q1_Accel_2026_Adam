use anchor_lang::prelude::*;

use crate::state::GptConfig;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = 8 + GptConfig::INIT_SPACE,
        seeds = [b"gpt_config"],
        bump,
    )]
    pub gpt_config: Account<'info, GptConfig>,

    /// CHECK: Oracle context account (created externally via oracle program)
    pub context_account: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> Initialize<'info> {
    pub fn handler(&mut self, prompt: String, bumps: &InitializeBumps) -> Result<()> {
        self.gpt_config.admin = self.admin.key();
        self.gpt_config.context_account = self.context_account.key();
        self.gpt_config.prompt = prompt;
        self.gpt_config.latest_response = String::new();
        self.gpt_config.bump = bumps.gpt_config;
        msg!("GptConfig initialized");
        Ok(())
    }
}