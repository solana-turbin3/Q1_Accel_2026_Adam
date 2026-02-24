use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use std::collections::HashMap;

/// Tuktuk program ID (from on-chain IDL)
pub const TUKTUK_PROGRAM_ID: Pubkey =
    pubkey!("tuktukUrfhXT6ZT77QTU8RQtvgL967uRuVagWF57zVA");

/// Discriminator for the queue_task_v0 instruction (from tuktuk IDL)
pub const QUEUE_TASK_V0_DISCRIMINATOR: [u8; 8] = [177, 95, 195, 252, 241, 2, 178, 88];

// ---------------------------------------------------------------------------
// Tuktuk types (mirrors the on-chain IDL definitions)
// ---------------------------------------------------------------------------

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CompiledInstructionV0 {
    pub program_id_index: u8,
    pub accounts: Vec<u8>,
    pub data: Vec<u8>,
}

/// Field order must match the IDL: num_rw_signers, num_ro_signers, num_rw,
/// accounts, instructions, signer_seeds.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Default)]
pub struct CompiledTransactionV0 {
    pub num_rw_signers: u8,
    pub num_ro_signers: u8,
    pub num_rw: u8,
    pub accounts: Vec<Pubkey>,
    pub instructions: Vec<CompiledInstructionV0>,
    pub signer_seeds: Vec<Vec<Vec<u8>>>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum TransactionSourceV0 {
    CompiledV0(CompiledTransactionV0),
    RemoteV0 { url: String, signer: Pubkey },
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub enum TriggerV0 {
    Now,
    Timestamp(i64),
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct QueueTaskArgsV0 {
    pub id: u16,
    pub trigger: TriggerV0,
    pub transaction: TransactionSourceV0,
    pub crank_reward: Option<u64>,
    pub free_tasks: u8,
    pub description: String,
}

// ---------------------------------------------------------------------------
// compile_transaction  – ported from tuktuk-program
// ---------------------------------------------------------------------------

pub fn compile_transaction(
    instructions: Vec<Instruction>,
    signer_seeds: Vec<Vec<Vec<u8>>>,
) -> Result<(CompiledTransactionV0, Vec<AccountMeta>)> {
    let mut pubkeys_to_metadata: HashMap<Pubkey, AccountMeta> = HashMap::new();

    for ix in &instructions {
        pubkeys_to_metadata
            .entry(ix.program_id)
            .or_insert(AccountMeta {
                pubkey: ix.program_id,
                is_signer: false,
                is_writable: false,
            });

        for key in &ix.accounts {
            let entry = pubkeys_to_metadata
                .entry(key.pubkey)
                .or_insert(AccountMeta {
                    is_signer: false,
                    is_writable: false,
                    pubkey: key.pubkey,
                });
            entry.is_writable |= key.is_writable;
            entry.is_signer |= key.is_signer;
        }
    }

    let mut sorted_accounts: Vec<Pubkey> = pubkeys_to_metadata.keys().cloned().collect();
    sorted_accounts.sort_by(|a, b| {
        let a_meta = &pubkeys_to_metadata[a];
        let b_meta = &pubkeys_to_metadata[b];

        fn get_priority(meta: &AccountMeta) -> u8 {
            match (meta.is_signer, meta.is_writable) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 3,
            }
        }

        get_priority(a_meta).cmp(&get_priority(b_meta))
    });

    let mut num_rw_signers: u8 = 0;
    let mut num_ro_signers: u8 = 0;
    let mut num_rw: u8 = 0;

    for k in &sorted_accounts {
        let metadata = &pubkeys_to_metadata[k];
        if metadata.is_signer && metadata.is_writable {
            num_rw_signers = num_rw_signers.checked_add(1).unwrap();
        } else if metadata.is_signer {
            num_ro_signers = num_ro_signers.checked_add(1).unwrap();
        } else if metadata.is_writable {
            num_rw = num_rw.checked_add(1).unwrap();
        }
    }

    let accounts_to_index: HashMap<Pubkey, u8> = sorted_accounts
        .iter()
        .enumerate()
        .map(|(i, k)| (*k, i as u8))
        .collect();

    let compiled_instructions: Vec<CompiledInstructionV0> = instructions
        .iter()
        .map(|ix| CompiledInstructionV0 {
            program_id_index: *accounts_to_index.get(&ix.program_id).unwrap(),
            accounts: ix
                .accounts
                .iter()
                .map(|k| *accounts_to_index.get(&k.pubkey).unwrap())
                .collect(),
            data: ix.data.clone(),
        })
        .collect();

    let rw_signers_end = num_rw_signers as usize;
    let ro_signers_end = rw_signers_end.checked_add(num_ro_signers as usize).unwrap();
    let rw_end = ro_signers_end.checked_add(num_rw as usize).unwrap();

    let remaining_accounts = sorted_accounts
        .iter()
        .enumerate()
        .map(|(index, k)| AccountMeta {
            pubkey: *k,
            is_signer: false,
            is_writable: index < rw_signers_end
                || (index >= ro_signers_end && index < rw_end),
        })
        .collect();

    Ok((
        CompiledTransactionV0 {
            num_rw_signers,
            num_ro_signers,
            num_rw,
            accounts: sorted_accounts,
            instructions: compiled_instructions,
            signer_seeds,
        },
        remaining_accounts,
    ))
}