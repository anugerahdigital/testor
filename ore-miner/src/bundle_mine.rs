use std::{
    sync::{atomic::AtomicU64, Arc},
    time::{Duration, Instant},
};

use anchor_lang::{InstructionData, ToAccountMetas};
use clap::Parser;
use guild::accounts::Mine;
use itertools::Itertools;
use ore::TREASURY_ADDRESS;
use solana_sdk::{
    instruction::Instruction,
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    system_program,
    sysvar,
    transaction::Transaction,
};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

use crate::{
    constant::{self, FEE_PER_SIGNER},
    format_duration,
    format_reward,
    jito::{self, subscribe_jito_tips, JitoTips},
    utils::{self, get_proof_pda},
    wait_continue,
    Miner,
};
#[derive(Debug, Clone, Parser)]
pub struct BundleMineArgs {
    #[arg(long, default_value = "4", help = "Number of threads to use for nonce calculation")]
    pub threads: usize,

    #[arg(
        long,
        default_value = "1",
        help = "Number of miner workers to mine nonce concurrently"
    )]
    pub concurrency: usize,

    #[arg(
        long,
        default_value = "0",
        help = "The maximum tip to pay for jito. Set to 0 to disable adaptive tip"
    )]
    pub max_adaptive_tip: u64,

    #[arg(long, default_value = "2", help = "The maximum number of buses to use for mining")]
    pub max_buses: usize,
}

impl Miner {
    pub async fn bundle_mine(&self, args: &BundleMineArgs) {
        let signer = Self::get_miners(0, 100);
        let semaphore = Arc::new(Semaphore::new(args.concurrency));
        let reward_counter = Arc::new(AtomicU64::new(0));
        let tips = Arc::new(RwLock::new(JitoTips::default()));

        subscribe_jito_tips(tips.clone()).await;

        for (i, keys) in signer.chunks(50).enumerate() {
            let miner = self.clone();
            let args = args.clone();
            let semaphore = semaphore.clone();
            let reward_counter = reward_counter.clone();
            let tips = tips.clone();
            let signers = keys.iter().map(|key| Arc::new(key.clone())).collect::<Vec<_>>();

            tokio::spawn(async move {
                miner
                    .bundle_mine_worker(i, args, signers, semaphore, reward_counter, tips)
                    .await;
            });
        }

        loop {
            tokio::time::sleep(Duration::from_secs(10 * 60)).await;

            let rewards = reward_counter.swap(0, std::sync::atomic::Ordering::Relaxed);
            if rewards > 0 {
                info!(rewards = format_reward!(rewards), "reward mined");
            }
        }
    }

    async fn bundle_mine_worker(
        self,
        miner: usize,
        args: BundleMineArgs,
        signers: Vec<Arc<Pubkey>>,
        semaphore: Arc<Semaphore>,
        reward_counter: Arc<AtomicU64>,
        tips: Arc<RwLock<JitoTips>>,
    ) {
        info!(miner, accounts = signers.len(), "miner started");

        let client = Miner::get_client_confirmed(&self.rpc);
        let mut tip = self.priority_fee.expect("jito tip should set");

        let proof_pda = signers.iter().map(|k| utils::get_proof_pda_no_cache(**k)).collect_vec();

        loop {
            let now = Instant::now();
            let _permit = semaphore.clone().acquire_owned().await;
            let mining_queue_duration = now.elapsed();

            let (treasury, clock, buses) = match Self::get_system_accounts(&client).await {
                Ok(accounts) => accounts,
                Err(err) => {
                    error!(miner, "fail to fetch system accounts: {err:#}");
                    wait_continue!(500);
                }
            };

            let proofs = match Self::get_proof_accounts(&client, &proof_pda).await {
                Ok(proofs) => proofs,
                Err(err) => {
                    error!(miner, "fail to fetch proof accounts: {err:#}");
                    wait_continue!(500);
                }
            };

            let reset_threshold = treasury.last_reset_at.saturating_add(ore::EPOCH_DURATION);
            let time_to_next_epoch = Self::get_time_to_next_epoch(&treasury, &clock, reset_threshold);

            let (mining_duration, mining_results) = self
                .mine_hashes_cpu(
                    args.threads,
                    &treasury.difficulty.into(),
                    &signers
                        .iter()
                        .zip(proofs.iter())
                        .map(|(signer, proof)| (proof.hash.into(), **signer))
                        .collect::<Vec<_>>(),
                )
                .await;

            drop(_permit);

            debug!(
                miner,
                mining = format_duration!(mining_duration),
                queue = format_duration!(mining_queue_duration),
                "mining done"
            );

            let available_bus =
                Self::find_buses(buses, treasury.reward_rate.saturating_mul((signers.len() + 4) as u64));

            if available_bus.is_empty() {
                warn!(miner, "no bus available for mining, waiting for next epoch",);
                wait_continue!(time_to_next_epoch.as_millis() as u64);
            }

            let rewards = treasury.reward_rate.saturating_mul(signers.len() as u64);

            if args.max_adaptive_tip > 0 {
                let tips = *tips.read().await;

                if tips.p50() > 0 {
                    tip = args.max_adaptive_tip.min(30000.max(tips.p50() + 1));
                }
            }

            let signer_and_mining_results = signers.iter().zip(mining_results.into_iter()).collect::<Vec<_>>();

            let (send_at_slot, blockhash) = match Self::get_latest_blockhash_and_slot(&client).await {
                Ok(value) => value,
                Err(err) => {
                    error!(miner, "fail to get latest blockhash: {err:#}");
                    continue;
                }
            };

            let confirm_start = Instant::now();

            // Bundle limit
            let tasks = available_bus
                .into_iter()
                .take(args.max_buses)
                .map(|bus| {
                    let mut bundle = Vec::with_capacity(5);
                    let mut fee_payer_and_cost = vec![];

                    let payer = utils::get_payer();

                    let batch_size = 10;
                    let batch_count = signer_and_mining_results.chunks(batch_size).len();
                    for (i, batch) in signer_and_mining_results.chunks(batch_size).enumerate() {
                        let mut ixs = Vec::with_capacity(batch.len());

                        for (miner, (hash, nonce)) in batch {
                            let miner = ***miner;
                            let bus_id: u8 = bus.id.try_into().unwrap();

                            let mine_accounts = Mine {
                                guild: utils::get_guild(),
                                miner,
                                bus: Pubkey::find_program_address(&[b"bus", &[bus_id]], &ore::id()).0,
                                payer: payer.pubkey(),
                                ore_program: ore::id(),
                                ore_treasury: TREASURY_ADDRESS,
                                proof_account: get_proof_pda(miner),
                                slot_hashes: sysvar::slot_hashes::ID,
                                system_program: system_program::id(),
                            };

                            let mine_args = guild::instruction::Mine {
                                args: guild::MineArgs {
                                    hash: hash.0,
                                    nonce: *nonce,
                                },
                            };

                            let mine_ix = Instruction {
                                program_id: guild::id(),
                                accounts: mine_accounts.to_account_metas(None),
                                data: mine_args.data(),
                            };

                            ixs.push(mine_ix);

                            // if its the last batch add the bribe
                            if i >= batch_count - 1 {
                                ixs.push(jito::build_bribe_ix(&payer.pubkey(), tip));
                            }
                        }

                        let tx = Transaction::new_signed_with_payer(&ixs, Some(&payer.pubkey()), &[&payer], blockhash);

                        bundle.push(tx);

                        let cost = FEE_PER_SIGNER * 1 as u64 + tip;
                        fee_payer_and_cost.push((payer.insecure_clone(), cost));
                    }

                    (
                        tokio::spawn(async move { jito::send_bundle(bundle).await }),
                        fee_payer_and_cost,
                    )
                })
                .collect::<Vec<_>>();

            let mut signatures = vec![];

            for (task, fee_payer_and_cost) in tasks {
                let (signature, bundle_id) = match task.await.unwrap() {
                    Ok(r) => r,
                    Err(err) => {
                        error!(miner, "fail to send bundle: {err:#}");
                        continue;
                    }
                };

                debug!(miner, ?bundle_id, ?signature, "bundle sent");
                signatures.push(signature);
            }

            if signatures.is_empty() {
                warn!(miner, "no bundle sent");
                continue;
            }

            let tips = *tips.read().await;
            info!(
                miner,
                mining = format_duration!(mining_duration),
                queue = format_duration!(mining_queue_duration),
                tip,
                tip.p25 = tips.p25(),
                tip.p50 = tips.p50(),
                slot = send_at_slot,
                "bundles sent"
            );

            let mut latest_slot = send_at_slot;
            let mut landed_tx = vec![];

            while landed_tx.is_empty() && latest_slot < send_at_slot + constant::SLOT_EXPIRATION {
                tokio::time::sleep(Duration::from_secs(2)).await;
                debug!(miner, latest_slot, send_at_slot, "checking bundle status");

                let (statuses, slot) = match Self::get_signature_statuses(&client, &signatures).await {
                    Ok(value) => value,
                    Err(err) => {
                        error!(miner, latest_slot, send_at_slot, "fail to get bundle status: {err:#}");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                };

                latest_slot = slot;
                landed_tx = utils::find_landed_txs(&signatures, statuses);
            }

            if !landed_tx.is_empty() {
                info!(
                    miner,
                    mining = format_duration!(mining_duration),
                    queue = format_duration!(mining_queue_duration),
                    confirm = format_duration!(confirm_start.elapsed()),
                    rewards = format_reward!(rewards),
                    first_tx = ?landed_tx.first().unwrap(),
                    "bundle mined",
                );
                reward_counter.fetch_add(rewards, std::sync::atomic::Ordering::Relaxed);
            } else {
                warn!(
                    miner,
                    mining = format_duration!(mining_duration),
                    queue = format_duration!(mining_queue_duration),
                    confirm = format_duration!(confirm_start.elapsed()),
                    rewards = format_reward!(rewards),
                    tip,
                    %tips,
                    "bundle dropped"
                );
            }
        }
    }
}
