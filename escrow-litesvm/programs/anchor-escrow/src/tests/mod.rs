#[cfg(test)]
mod tests {

    use {
        anchor_lang::{
            prelude::msg, solana_program::program_pack::Pack, AccountDeserialize, InstructionData,
            ToAccountMetas,
        },
        anchor_spl::{
            associated_token::{self, spl_associated_token_account},
            token::spl_token,
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, CreateAssociatedTokenAccount, CreateMint, MintTo,
        },
        solana_account::Account,
        solana_address::Address,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_rpc_client::rpc_client::RpcClient,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::{path::PathBuf, str::FromStr},
    };

    // Re-export Clock from anchor_lang
    use anchor_lang::solana_program::clock::Clock;

    static PROGRAM_ID: Pubkey = crate::ID;

    // 5 days in seconds
    const FIVE_DAYS_IN_SECONDS: i64 = 5 * 24 * 60 * 60;

    // Setup function to initialize LiteSVM and create a payer keypair
    // Also loads an account from devnet into the LiteSVM environment (for testing purposes)
    fn setup() -> (LiteSVM, Keypair) {
        // Initialize LiteSVM and payer
        let mut program = LiteSVM::new();
        let payer = Keypair::new();

        // Airdrop some SOL to the payer keypair
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");

        // Load program SO file
        let so_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/deploy/anchor_escrow.so");

        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");

        program.add_program(PROGRAM_ID, &program_data);

        // Example on how to Load an account from devnet
        // LiteSVM does not have access to real Solana network data since it does not have network access,
        // so we use an RPC client to fetch account data from devnet
        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let account_address =
            Address::from_str("DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2").unwrap();
        let fetched_account = rpc_client
            .get_account(&account_address)
            .expect("Failed to fetch account from devnet");

        // Set the fetched account in the LiteSVM environment
        // This allows us to simulate interactions with this account during testing
        program
            .set_account(
                payer.pubkey(),
                Account {
                    lamports: fetched_account.lamports,
                    data: fetched_account.data,
                    owner: Pubkey::from(fetched_account.owner.to_bytes()),
                    executable: fetched_account.executable,
                    rent_epoch: fetched_account.rent_epoch,
                },
            )
            .unwrap();

        msg!("Lamports of fetched account: {}", fetched_account.lamports);

        // Return the LiteSVM instance and payer keypair
        (program, payer)
    }

    #[test]
    fn test_make() {
        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        // Get the maker's public key from the payer keypair
        let maker = payer.pubkey();

        // Create two mints (Mint A and Mint B) with 6 decimal places and the maker as the authority
        // This done using litesvm-token's CreateMint utility which creates the mint in the LiteSVM environment
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        // This is done using litesvm-token's CreateAssociatedTokenAccount utility
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker)
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint 1,000 tokens (with 6 decimal places) of Mint A to the maker's associated token account
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker,
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: asspciated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 10,
                seed: 123u64,
                receive: 10,
            }
            .data(),
        };

        // Create and send the transaction containing the "Make" instruction
        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let recent_blockhash = program.latest_blockhash();

        let transaction = Transaction::new(&[&payer], message, recent_blockhash);

        // Send the transaction and capture the result
        let tx = program.send_transaction(transaction).unwrap();

        // Log transaction details
        msg!("\n\nMake transaction sucessfull");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify the vault account and escrow account data after the "Make" instruction
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 10);
        assert_eq!(vault_data.owner, escrow);
        assert_eq!(vault_data.mint, mint_a);

        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data =
            crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        assert_eq!(escrow_data.seed, 123u64);
        assert_eq!(escrow_data.maker, maker);
        assert_eq!(escrow_data.mint_a, mint_a);
        assert_eq!(escrow_data.mint_b, mint_b);
        assert_eq!(escrow_data.receive, 10);
    }

    #[test]
    fn test_take() {
        // Setup the test environment by initializing LiteSVM and creating a payer keypair
        let (mut program, payer) = setup();

        // Create separate maker and taker keypairs
        let maker = Keypair::new();
        let taker = Keypair::new();

        // Airdrop SOL to both maker and taker
        program
            .airdrop(&maker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();
        program
            .airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        // Create two mints (Mint A and Mint B) with 6 decimal places
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Create the taker's associated token account for Mint B (taker needs mint_b to send to maker)
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();
        msg!("Taker ATA B: {}\n", taker_ata_b);

        // Derive the PDA for the escrow account using the maker's public key and a seed value
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.pubkey().as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the PDA for the vault associated token account using the escrow PDA and Mint A
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        // Define program IDs for associated token program, token program, and system program
        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens of Mint A to the maker's ATA (maker will deposit these into escrow)
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        // Mint tokens of Mint B to the taker's ATA (taker will send these to maker during take)
        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000000000)
            .send()
            .unwrap();

        // ==================== MAKE INSTRUCTION ====================
        // Create the "Make" instruction to deposit tokens into the escrow
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 100,
                seed: 123u64,
                receive: 50,
            }
            .data(),
        };

        // Create and send the Make transaction
        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nMake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // Verify the vault has the deposited tokens
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 100);
        msg!("Vault balance after Make: {}", vault_data.amount);

        // ==================== WARP TIME FORWARD 5 DAYS ====================
        // Update the clock timestamp
        let mut clock: Clock = program.get_sysvar();
        clock.unix_timestamp += FIVE_DAYS_IN_SECONDS + 1;
        program.set_sysvar(&clock);

        // Also warp the slot forward and expire the blockhash
        program.warp_to_slot(clock.slot + 1_000_000);
        program.expire_blockhash();

        // ==================== TAKE INSTRUCTION ====================
        // Derive the taker's ATA for mint_a (will receive tokens from vault)
        let taker_ata_a = associated_token::get_associated_token_address(&taker.pubkey(), &mint_a);

        // Derive the maker's ATA for mint_b (will receive tokens from taker)
        let maker_ata_b = associated_token::get_associated_token_address(&maker.pubkey(), &mint_b);

        // Create the "Take" instruction
        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                taker_ata_a: taker_ata_a,
                taker_ata_b: taker_ata_b,
                maker_ata_b: maker_ata_b,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        // Create and send the Take transaction
        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);
        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nTake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);
        msg!("Tx Signature: {}", tx.signature);

        // ==================== VERIFY FINAL STATE ====================
        // Verify taker received mint_a tokens from vault
        let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
        let taker_ata_a_data =
            spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
        assert_eq!(taker_ata_a_data.amount, 100);
        msg!("Taker ATA A balance: {}", taker_ata_a_data.amount);

        // Verify maker received mint_b tokens from taker
        let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
        let maker_ata_b_data =
            spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
        assert_eq!(maker_ata_b_data.amount, 50);
        msg!("Maker ATA B balance: {}", maker_ata_b_data.amount);

        // Verify vault is closed (should have 0 lamports or not exist)
        match program.get_account(&vault) {
            Some(acc) => {
                // If account exists, it should have 0 lamports (closed)
                assert_eq!(
                    acc.lamports, 0,
                    "Vault should have 0 lamports after closing"
                );
                msg!("Vault closed (0 lamports)");
            }
            None => {
                msg!("Vault closed successfully (account removed)");
            }
        }

        // Verify escrow is closed
        match program.get_account(&escrow) {
            Some(acc) => {
                assert_eq!(
                    acc.lamports, 0,
                    "Escrow should have 0 lamports after closing"
                );
                msg!("Escrow closed (0 lamports)");
            }
            None => {
                msg!("Escrow closed successfully (account removed)");
            }
        }
    }

    #[test]
    fn test_refund() {
        // Setup the test environment
        let (mut program, payer) = setup();

        // Create maker keypair
        let maker = Keypair::new();
        program
            .airdrop(&maker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        // Create mint A
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        msg!("Mint A: {}\n", mint_a);

        // Create mint B (needed for escrow creation)
        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();
        msg!("Mint B: {}\n", mint_b);

        // Create the maker's associated token account for Mint A
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();
        msg!("Maker ATA A: {}\n", maker_ata_a);

        // Derive the PDA for the escrow account
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.pubkey().as_ref(), &456u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;
        msg!("Escrow PDA: {}\n", escrow);

        // Derive the vault ATA
        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);
        msg!("Vault PDA: {}\n", vault);

        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens to maker's ATA
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        // Check initial maker balance
        let maker_ata_account = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_data = spl_token::state::Account::unpack(&maker_ata_account.data).unwrap();
        let initial_balance = maker_ata_data.amount;
        msg!("Maker initial balance: {}", initial_balance);

        // ==================== MAKE INSTRUCTION ====================
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 200,
                seed: 456u64,
                receive: 100,
            }
            .data(),
        };

        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nMake transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);

        // Verify vault has tokens
        let vault_account = program.get_account(&vault).unwrap();
        let vault_data = spl_token::state::Account::unpack(&vault_account.data).unwrap();
        assert_eq!(vault_data.amount, 200);
        msg!("Vault balance after Make: {}", vault_data.amount);

        // ==================== REFUND INSTRUCTION ====================
        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Refund {
                maker: maker.pubkey(),
                mint_a: mint_a,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&maker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        let tx = program.send_transaction(transaction).unwrap();

        msg!("\n\nRefund transaction successful");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);

        // ==================== VERIFY FINAL STATE ====================
        // Verify maker received tokens back
        let maker_ata_account = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_data = spl_token::state::Account::unpack(&maker_ata_account.data).unwrap();
        assert_eq!(maker_ata_data.amount, initial_balance);
        msg!("Maker balance after refund: {}", maker_ata_data.amount);

        // Verify vault is closed
        match program.get_account(&vault) {
            Some(acc) => {
                assert_eq!(
                    acc.lamports, 0,
                    "Vault should have 0 lamports after closing"
                );
                msg!("Vault closed (0 lamports)");
            }
            None => {
                msg!("Vault closed successfully");
            }
        }

        // Verify escrow is closed
        match program.get_account(&escrow) {
            Some(acc) => {
                assert_eq!(
                    acc.lamports, 0,
                    "Escrow should have 0 lamports after closing"
                );
                msg!("Escrow closed (0 lamports)");
            }
            None => {
                msg!("Escrow closed successfully");
            }
        }
    }

    #[test]
    fn test_take_fails_before_5_days() {
        // Setup the test environment
        let (mut program, payer) = setup();

        // Create separate maker and taker keypairs
        let maker = Keypair::new();
        let taker = Keypair::new();

        program
            .airdrop(&maker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();
        program
            .airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        // Create mints
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        // Create maker's ATA for mint_a
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        // Create taker's ATA for mint_b
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Derive escrow and vault
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.pubkey().as_ref(), &789u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000000000)
            .send()
            .unwrap();

        // ==================== MAKE INSTRUCTION ====================
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 100,
                seed: 789u64,
                receive: 50,
            }
            .data(),
        };

        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        program.send_transaction(transaction).unwrap();

        msg!("Make transaction successful");

        // ==================== TAKE INSTRUCTION (should fail - too early) ====================
        let taker_ata_a = associated_token::get_associated_token_address(&taker.pubkey(), &mint_a);
        let maker_ata_b = associated_token::get_associated_token_address(&maker.pubkey(), &mint_b);

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                taker_ata_a: taker_ata_a,
                taker_ata_b: taker_ata_b,
                maker_ata_b: maker_ata_b,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);

        // This should fail because 5 days haven't passed
        let result = program.send_transaction(transaction);
        assert!(
            result.is_err(),
            "Take should fail before 5 days have passed"
        );
        msg!("Take correctly failed before 5 days: {:?}", result.err());
    }

    #[test]
    fn test_take_succeeds_after_5_days() {
        // Setup the test environment
        let (mut program, payer) = setup();

        // Create separate maker and taker keypairs
        let maker = Keypair::new();
        let taker = Keypair::new();

        program
            .airdrop(&maker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();
        program
            .airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL)
            .unwrap();

        // Create mints
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&payer.pubkey())
            .send()
            .unwrap();

        // Create maker's ATA for mint_a
        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker.pubkey())
            .send()
            .unwrap();

        // Create taker's ATA for mint_b
        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_b)
            .owner(&taker.pubkey())
            .send()
            .unwrap();

        // Derive escrow and vault
        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.pubkey().as_ref(), &999u64.to_le_bytes()],
            &PROGRAM_ID,
        )
        .0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let associated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        // Mint tokens
        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000)
            .send()
            .unwrap();

        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000000000)
            .send()
            .unwrap();

        // ==================== MAKE INSTRUCTION ====================
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                maker_ata_a: maker_ata_a,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Make {
                deposit: 100,
                seed: 999u64,
                receive: 50,
            }
            .data(),
        };

        let message = Message::new(&[make_ix], Some(&maker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&maker], message, recent_blockhash);
        program.send_transaction(transaction).unwrap();

        msg!("Make transaction successful");

        // ==================== WARP TIME FORWARD 5 DAYS ====================
        // Update the clock timestamp
        let mut clock: Clock = program.get_sysvar();
        msg!("Current slot: {}, timestamp: {}", clock.slot, clock.unix_timestamp);

        clock.unix_timestamp += FIVE_DAYS_IN_SECONDS + 1;
        program.set_sysvar(&clock);

        // Also warp the slot forward and expire the blockhash
        program.warp_to_slot(clock.slot + 1_000_000);
        program.expire_blockhash();

        let new_clock: Clock = program.get_sysvar();
        msg!("New slot: {}, timestamp: {}", new_clock.slot, new_clock.unix_timestamp);

        // ==================== TAKE INSTRUCTION (should succeed now) ====================
        let taker_ata_a = associated_token::get_associated_token_address(&taker.pubkey(), &mint_a);
        let maker_ata_b = associated_token::get_associated_token_address(&maker.pubkey(), &mint_b);

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker: maker.pubkey(),
                mint_a: mint_a,
                mint_b: mint_b,
                taker_ata_a: taker_ata_a,
                taker_ata_b: taker_ata_b,
                maker_ata_b: maker_ata_b,
                escrow: escrow,
                vault: vault,
                associated_token_program: associated_token_program,
                token_program: token_program,
                system_program: system_program,
            }
            .to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let recent_blockhash = program.latest_blockhash();
        let transaction = Transaction::new(&[&taker], message, recent_blockhash);

        // This should succeed because 5 days have passed
        let tx = program.send_transaction(transaction).unwrap();
        msg!("\n\nTake transaction successful after 5 days");
        msg!("CUs Consumed: {}", tx.compute_units_consumed);

        // Verify final state
        let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
        let taker_ata_a_data =
            spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
        assert_eq!(taker_ata_a_data.amount, 100);
        msg!(
            "Taker received {} tokens of mint_a",
            taker_ata_a_data.amount
        );

        let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
        let maker_ata_b_data =
            spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
        assert_eq!(maker_ata_b_data.amount, 50);
        msg!(
            "Maker received {} tokens of mint_b",
            maker_ata_b_data.amount
        );
    }
}
