use std::str::FromStr;

use anchor_lang::{prelude::*, InstructionData};
use anchor_lang::solana_program::instruction::Instruction;
use tuktuk_program::{
    TransactionSourceV0, 
    compile_transaction, 
    tuktuk::{
        cpi::{
            accounts::{
                QueueTaskV0, 
                InitializeTaskQueueV0
            },
            queue_task_v0, 
            initialize_task_queue_v0
        }, 
        program::Tuktuk, 
        types::TriggerV0
    }, types::QueueTaskArgsV0
};

use crate::state::Counter;

#[derive(Accounts)]
pub struct Schedule<'info> {
    #[account(
        mut,
        address = Pubkey::from_str("AHYic562KhgtAEkb1rSesqS87dFYRcfXb4WwWus3Zc9C").unwrap()
    )]
    pub user: Signer<'info>,
    /// CHECK: This is safe because we don't read or write from this account
    #[account(
        mut,
        seeds = [b"counter"],
        bump,
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue: UncheckedAccount<'info>,
    /// CHECK: Don't need to parse this account, just using it in CPI
    pub task_queue_authority: UncheckedAccount<'info>,
    /// CHECK: Initialized in CPI
    #[account(mut)]
    pub task: UncheckedAccount<'info>,
    /// CHECK: Via seeds
    #[account(
        mut,
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}

impl<'info> Schedule<'info> {
    pub fn schedule(&mut self, task_id: u16, bumps: ScheduleBumps) -> Result<()> {
        let (compiled_tx, _) = compile_transaction(
            vec![Instruction {
                program_id: crate::ID,
                accounts: crate::__cpi_client_accounts_increment::Increment {
                    counter: self.counter.to_account_info(),
                }
                .to_account_metas(None)
                .to_vec(),
                data: crate::instruction::Increment {}.data(),
            }],
        vec![],
        ).unwrap();

        queue_task_v0(
            CpiContext::new_with_signer(
                self.tuktuk_program.to_account_info(),
                QueueTaskV0 {
                    payer: self.user.to_account_info(),
                    queue_authority: self.queue_authority.to_account_info(),
                    task_queue: self.task_queue.to_account_info(),
                    task_queue_authority: self.task_queue_authority.to_account_info(),
                    task: self.task.to_account_info(),
                    system_program: self.system_program.to_account_info(),
                },
                &[&["queue_authority".as_bytes(), &[bumps.queue_authority]]],
            ),
            QueueTaskArgsV0 {
                trigger: TriggerV0::Now,
                transaction: TransactionSourceV0::CompiledV0(compiled_tx),
                crank_reward: Some(1000001),
                free_tasks: 1,
                id: task_id,
                description: "test".to_string(),
            },
        )?;
        
    Ok(())
    }
}