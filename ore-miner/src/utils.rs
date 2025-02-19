use std::{collections::HashMap, env, path::PathBuf, str::FromStr};

use cached::proc_macro::cached;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair, Signature},
};
use solana_transaction_status::TransactionStatus;

#[cached]
pub fn get_proof_pda(authority: Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[ore::PROOF, authority.as_ref()], &ore::ID).0
}

#[cached]
pub fn get_treasury_ata() -> Pubkey {
    spl_associated_token_account::get_associated_token_address(&ore::TREASURY_ADDRESS, &ore::MINT_ADDRESS)
}

#[cached]
pub fn get_ore_ata(owner: Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(&owner, &ore::MINT_ADDRESS)
}

pub fn ore_ui_amount(amount: u64) -> f64 {
    spl_token::amount_to_ui_amount(amount, ore::TOKEN_DECIMALS)
}

#[cached]
pub fn get_gpu_nonce_worker_path() -> PathBuf {
    env::current_exe().unwrap().parent().unwrap().join("nonce-worker-gpu")
}

#[cached]
pub fn get_nonce_worker_path() -> PathBuf {
    env::current_exe().unwrap().parent().unwrap().join("nonce-worker")
}

pub fn find_landed_txs(signatures: &[Signature], statuses: Vec<Option<TransactionStatus>>) -> Vec<Signature> {
    let landed_tx = statuses
        .into_iter()
        .zip(signatures.iter())
        .filter_map(|(status, sig)| {
            if status?.satisfies_commitment(CommitmentConfig::confirmed()) {
                Some(*sig)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    landed_tx
}

pub fn get_payer() -> Keypair {
    read_keypair_file("payer.json").unwrap()
}

pub fn get_guild() -> Pubkey {
    Pubkey::from_str("ACrBnqZezHPuTQXrxXgAe2NWSAD25QLBFf7Eh2uexKe4").unwrap()
}

pub fn get_miners(start_index: u64, count: u64) -> Vec<Pubkey> {
    let mut miners: Vec<Pubkey> = vec![];

    for i in start_index..start_index + count {
        miners.push(get_miner(i));
    }

    miners
}

fn get_miner(id: u64) -> Pubkey {
    let miner_prefix: &[u8] = b"miner";
    let miner = Pubkey::find_program_address(&[miner_prefix, get_guild().as_ref(), &id.to_le_bytes()], &guild::ID).0;
    miner
}

#[macro_export]
macro_rules! format_duration {
    ($d: expr) => {
        format_args!("{:.1}s", $d.as_secs_f64())
    };
}

#[macro_export]
macro_rules! format_reward {
    ($r: expr) => {
        format_args!("{:.}", utils::ore_ui_amount($r))
    };
}

#[macro_export]
macro_rules! wait_return {
    ($duration: expr) => {{
        tokio::time::sleep(std::time::Duration::from_millis($duration)).await;
        return;
    }};

    ($duration: expr, $return: expr) => {{
        tokio::time::sleep(std::time::Duration::from_millis($duration)).await;
        return $return;
    }};
}

#[macro_export]
macro_rules! wait_continue {
    ($duration: expr) => {{
        tokio::time::sleep(std::time::Duration::from_millis($duration)).await;
        continue;
    }};
}
