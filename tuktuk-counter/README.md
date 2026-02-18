# TukTuk Counter

This example demonstrates how to use [TukTuk](https://github.com/helium/tuktuk) — Helium's decentralized task queue system — to automate on-chain actions on Solana.

In this example, a simple counter program is deployed, and TukTuk is used to schedule automatic increments of the counter either via on-chain task queuing or an off-chain cron job that fires every minute.

---

## Let's walk through the architecture:

For this program, we will have 1 main state account:

- A Counter account

A Counter account consists of:

```rust
#[account]
pub struct Counter {
    pub count: u64,
    pub bump: u8,
}
```

### In this state account, we will store:

- count: A 64-bit unsigned integer that tracks how many times the counter has been incremented.
- bump: The bump seed used to derive the counter PDA.

The Counter account uses a fixed-size layout with a total space of 17 bytes (8-byte discriminator + 8-byte count + 1-byte bump).

---

### Anyone can initialize a new Counter account. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(
        init,
        payer = user,
        seeds = [b"counter"],
        bump,
        space = Counter::INIT_SPACE,
    )]
    pub counter: Account<'info, Counter>,
    pub system_program: Program<'info, System>,
}
```

Let's have a closer look at the accounts that we are passing in this context:

- user: Will be the person creating the counter account. He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports from this account.

- counter: Will be the state account that we will initialize and the user will be paying for the initialization of the account. We derive the Counter PDA from the byte representation of the word "counter".

- system_program: Program responsible for the initialization of any new account.

### We then implement some functionality for our Initialize context:

```rust
impl<'info> Initialize<'info> {
    pub fn initialize(&mut self, bumps: &InitializeBumps) -> Result<()> {
        self.counter.count = 0;
        self.counter.bump = bumps.counter;

        Ok(())
    }
}
```

In here, we set the initial data of our Counter account: the count starts at 0 and we store the bump seed for future PDA derivation.

---

### The counter can be incremented by anyone. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct Increment<'info> {
    #[account(
        mut,
        seeds = [b"counter"],
        bump = counter.bump,
    )]
    pub counter: Account<'info, Counter>,
}
```

In this context, we are passing the only account needed to increment the counter:

- counter: The state account that we will modify. We derive the Counter PDA from the byte representation of the word "counter" and verify the bump.

### We then implement some functionality for our Increment context:

```rust
impl<'info> Increment<'info> {
    pub fn increment_counter(&mut self) -> Result<()> {
        self.counter.count += 1;
        Ok(())
    }
}
```

In here, we simply increment the count field by 1. No signer is required — this is intentional, as TukTuk's crankers need to be able to execute this instruction on behalf of the task queue.

---

### The schedule instruction queues an increment task on TukTuk. For that, we create the following context:

```rust
#[derive(Accounts)]
pub struct Schedule<'info> {
    #[account(
        mut,
        address = Pubkey::from_str("AHYic562KhgtAEkb1rSesqS87dFYRcfXb4WwWus3Zc9C").unwrap()
    )]
    pub user: Signer<'info>,
    #[account(
        mut,
        seeds = [b"counter"],
        bump,
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub task_queue: UncheckedAccount<'info>,
    pub task_queue_authority: UncheckedAccount<'info>,
    #[account(mut)]
    pub task: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"queue_authority"],
        bump
    )]
    pub queue_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub tuktuk_program: Program<'info, Tuktuk>,
}
```

In this context, we are passing all the accounts needed to queue a task on TukTuk:

- user: The authorized user who can schedule tasks. This is hardcoded to a specific public key (the update authority of the queue). He will be a signer of the transaction, and we mark his account as mutable as we will be deducting lamports to pay for the task.

- counter: The counter state account that the queued task will increment.

- task_queue: The TukTuk task queue where the task will be submitted.

- task_queue_authority: The authority PDA for the task queue, used to verify scheduling permissions.

- task: The account that will hold the queued task data, initialized during the CPI call.

- queue_authority: A PDA derived from "queue_authority" that acts as the program's signing authority when interacting with the TukTuk program via CPI.

- system_program: Program responsible for account creation.

- tuktuk_program: The TukTuk program that processes task queue operations.

### We then implement some functionality for our Schedule context:

```rust
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
```

In this implementation, we first compile an `increment` instruction into TukTuk's compiled transaction format using `compile_transaction`. Then we perform a CPI call to the TukTuk program's `queue_task_v0` instruction, signing with the program's `queue_authority` PDA. The task is configured with `TriggerV0::Now` so it executes immediately, a crank reward of 1,000,001 lamports to incentivize crankers, and a unique task ID provided by the caller.

---

## Flow of Actions

Here is the step-by-step flow to set up and run the TukTuk Counter:

### 1. Build and deploy the program

```bash
anchor build
anchor deploy
```

### 2. Initialize the Counter

Run the initialization test (or call the instruction directly) to create the on-chain counter account:

```bash
anchor test --skip-local-validator
```

This will initialize the counter PDA and set the count to 0.

### 3. Create a TukTuk Task Queue

Use the TukTuk CLI to create a task queue on devnet:

```bash
tuktuk --url https://api.devnet.solana.com task-queue create \
  --name tuktuk-counter \
  --capacity 5 \
  --funding-amount 100000000 \
  --queue-authority <QUEUE_AUTHORITY_PDA> \
  --min-crank-reward 1000000 \
  --stale-task-age 0
```

This creates a task queue named `tuktuk-counter` with a capacity of 5 tasks, funded with 0.1 SOL, and a minimum crank reward of 0.001 SOL.

### 4. Add a Queue Authority

Add your wallet (or the program's PDA) as a queue authority so it can submit tasks:

```bash
tuktuk --url https://api.devnet.solana.com task-queue add-queue-authority \
  --task-queue-name tuktuk-counter \
  --queue-authority <YOUR_QUEUE_AUTHORITY>
```

### 5. Option A — Schedule a one-off task (on-chain)

Call the `schedule` instruction to queue a single increment task that TukTuk crankers will execute immediately:

```bash
anchor test --skip-local-validator
```

The test file calls `program.methods.schedule(taskID)` which compiles an `increment` instruction, submits it to the task queue via CPI, and TukTuk crankers pick it up and execute it.

### 5. Option B — Set up a Cron Job (recurring automation)

Use the cron script to create a recurring cron job that increments the counter every minute:

```bash
anchor run cron
```

This script (located in `cron/cron.ts`):
1. Initializes the TukTuk and Cron SDK programs.
2. Creates a task queue authority if one doesn't exist.
3. Creates a cron job with a `"0 * * * * *"` schedule (every minute).
4. Funds the cron job with SOL for crank rewards.
5. Compiles and attaches the `increment` instruction as the cron transaction.

### 6. Monitor the Counter

Watch for transactions on the task queue to see the counter being incremented:

```
Task Queue: CMreFdKxT5oeZhiX8nWTGz9PtXM1AMYTh6dGR2UzdtrA
```

### 7. Stop the Cron Job

To stop the recurring cron job, use the TukTuk CLI:

```bash
tuktuk -u https://api.devnet.solana.com cron-transaction close \
  --cron-name tuktuk-counter-cron --id 0

tuktuk -u https://api.devnet.solana.com cron close \
  --cron-name tuktuk-counter-cron
```

---

This TukTuk Counter demonstrates how to integrate decentralized task automation into a Solana program, enabling both on-demand task scheduling via CPI and recurring cron-based automation — all without relying on centralized off-chain infrastructure.
